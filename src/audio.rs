use bevy::prelude::{App, Plugin, Resource};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use midly::{Smf, TrackEventKind};
use oxisynth::{MidiEvent, SoundFont, Synth};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub enum AudioCommand {
    Play(PathBuf, PathBuf),
    Pause,
    Stop,
    Rewind,
}

#[derive(Resource)]
pub struct AudioSender(pub Sender<AudioCommand>);

#[derive(Resource, Clone)]
pub struct AudioState {
    pub samples_played: Arc<AtomicU64>,
    pub total_samples: Arc<AtomicU64>,
    max_tick: Arc<AtomicU64>,
    last_event_sample: Arc<AtomicU64>,
    last_event_tick: Arc<AtomicU64>,
    next_event_sample: Arc<AtomicU64>,
    next_event_tick: Arc<AtomicU64>,
}

pub struct AudioDebugState {
    pub samples_played: u64,
    pub total_samples: u64,
    pub last_event_sample: u64,
    pub next_event_sample: u64,
    pub last_event_tick: u64,
    pub next_event_tick: u64,
    pub max_tick: u64,
}

impl AudioState {
    pub fn current_tick_ratio(&self) -> Option<f32> {
        let max_tick = self.max_tick.load(Ordering::Relaxed);
        if max_tick == 0 {
            return None;
        }

        let samples = self.samples_played.load(Ordering::Relaxed);
        let last_sample = self.last_event_sample.load(Ordering::Relaxed);
        let last_tick = self.last_event_tick.load(Ordering::Relaxed);
        let next_sample = self.next_event_sample.load(Ordering::Relaxed);
        let next_tick = self.next_event_tick.load(Ordering::Relaxed);

        let tick = if next_sample > last_sample && next_tick >= last_tick {
            let denom = (next_sample - last_sample) as f64;
            let t = ((samples.saturating_sub(last_sample)) as f64 / denom).clamp(0.0, 1.0);
            (last_tick as f64 + t * (next_tick - last_tick) as f64).round() as u64
        } else {
            last_tick
        };

        Some((tick as f64 / max_tick as f64).clamp(0.0, 1.0) as f32)
    }

    pub fn current_tick(&self) -> Option<u64> {
        let max_tick = self.max_tick.load(Ordering::Relaxed);
        if max_tick == 0 {
            return None;
        }

        let samples = self.samples_played.load(Ordering::Relaxed);
        let last_sample = self.last_event_sample.load(Ordering::Relaxed);
        let last_tick = self.last_event_tick.load(Ordering::Relaxed);
        let next_sample = self.next_event_sample.load(Ordering::Relaxed);
        let next_tick = self.next_event_tick.load(Ordering::Relaxed);

        let tick = if next_sample > last_sample && next_tick >= last_tick {
            let denom = (next_sample - last_sample) as f64;
            let t = ((samples.saturating_sub(last_sample)) as f64 / denom).clamp(0.0, 1.0);
            (last_tick as f64 + t * (next_tick - last_tick) as f64).round() as u64
        } else {
            last_tick
        };

        Some(tick.min(max_tick))
    }

    pub fn debug_state(&self) -> AudioDebugState {
        AudioDebugState {
            samples_played: self.samples_played.load(Ordering::Relaxed),
            total_samples: self.total_samples.load(Ordering::Relaxed),
            last_event_sample: self.last_event_sample.load(Ordering::Relaxed),
            next_event_sample: self.next_event_sample.load(Ordering::Relaxed),
            last_event_tick: self.last_event_tick.load(Ordering::Relaxed),
            next_event_tick: self.next_event_tick.load(Ordering::Relaxed),
            max_tick: self.max_tick.load(Ordering::Relaxed),
        }
    }
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let (cmd_tx, cmd_rx) = channel::<AudioCommand>();
        let samples_played = Arc::new(AtomicU64::new(0));
        let total_samples = Arc::new(AtomicU64::new(0));
        let max_tick = Arc::new(AtomicU64::new(0));
        let last_event_sample = Arc::new(AtomicU64::new(0));
        let last_event_tick = Arc::new(AtomicU64::new(0));
        let next_event_sample = Arc::new(AtomicU64::new(0));
        let next_event_tick = Arc::new(AtomicU64::new(0));
        let audio_state = AudioState {
            samples_played: Arc::clone(&samples_played),
            total_samples: Arc::clone(&total_samples),
            max_tick: Arc::clone(&max_tick),
            last_event_sample: Arc::clone(&last_event_sample),
            last_event_tick: Arc::clone(&last_event_tick),
            next_event_sample: Arc::clone(&next_event_sample),
            next_event_tick: Arc::clone(&next_event_tick),
        };

        // Start audio thread
        let samples_played_thread = Arc::clone(&samples_played);
        let total_samples_thread = Arc::clone(&total_samples);
        let max_tick_thread = Arc::clone(&max_tick);
        let last_event_sample_thread = Arc::clone(&last_event_sample);
        let last_event_tick_thread = Arc::clone(&last_event_tick);
        let next_event_sample_thread = Arc::clone(&next_event_sample);
        let next_event_tick_thread = Arc::clone(&next_event_tick);
        thread::spawn(move || {
            println!("Audio thread spawned.");
            audio_thread(
                cmd_rx,
                samples_played_thread,
                total_samples_thread,
                max_tick_thread,
                last_event_sample_thread,
                last_event_tick_thread,
                next_event_sample_thread,
                next_event_tick_thread,
            );
        });

        app.insert_resource(AudioSender(cmd_tx));
        app.insert_resource(audio_state);
    }
}

struct MidiPlaybackEvent {
    tick: u64,
    sample: u64,
    event: MidiEvent,
}

struct PlaybackSchedule {
    events: Vec<MidiPlaybackEvent>,
    ruler_max_tick: u64,
    total_samples: u64,
}

#[derive(Clone, Copy)]
struct TempoSegment {
    tick: u64,
    us_per_beat: u32,
    seconds_at_tick: f64,
}

fn build_tempo_segments(tempo_events: &[(u64, u32)], ticks_per_beat: f64) -> Vec<TempoSegment> {
    let mut segments = Vec::new();
    let mut sorted = tempo_events.to_vec();
    sorted.sort_by_key(|(tick, _)| *tick);

    let mut current = TempoSegment {
        tick: 0,
        us_per_beat: 500_000,
        seconds_at_tick: 0.0,
    };
    segments.push(current);

    for (tick, us_per_beat) in sorted {
        if tick == current.tick {
            current.us_per_beat = us_per_beat;
            segments.last_mut().unwrap().us_per_beat = us_per_beat;
            continue;
        }
        let delta_ticks = tick.saturating_sub(current.tick);
        let seconds_delta =
            (delta_ticks as f64 * current.us_per_beat as f64) / (1_000_000.0 * ticks_per_beat);
        current = TempoSegment {
            tick,
            us_per_beat,
            seconds_at_tick: current.seconds_at_tick + seconds_delta,
        };
        segments.push(current);
    }

    segments
}

fn ticks_to_seconds(tick: u64, segments: &[TempoSegment], ticks_per_beat: f64) -> f64 {
    let mut active = segments[0];
    for segment in segments.iter().skip(1) {
        if segment.tick > tick {
            break;
        }
        active = *segment;
    }
    let delta_ticks = tick.saturating_sub(active.tick);
    let seconds_delta =
        (delta_ticks as f64 * active.us_per_beat as f64) / (1_000_000.0 * ticks_per_beat);
    active.seconds_at_tick + seconds_delta
}

struct ParsedMidi {
    events: Vec<(u64, MidiEvent)>,
    tempo_events: Vec<(u64, u32)>,
    max_tick: u64,
    max_note_tick: u64,
}

fn midi_message_to_event(channel: u8, message: midly::MidiMessage) -> MidiEvent {
    match message {
        midly::MidiMessage::NoteOff { key, .. } => MidiEvent::NoteOff {
            channel,
            key: key.as_int() as u8,
        },
        midly::MidiMessage::NoteOn { key, vel } => MidiEvent::NoteOn {
            channel,
            key: key.as_int() as u8,
            vel: vel.as_int() as u8,
        },
        midly::MidiMessage::Aftertouch { key, vel } => MidiEvent::PolyphonicKeyPressure {
            channel,
            key: key.as_int() as u8,
            value: vel.as_int() as u8,
        },
        midly::MidiMessage::Controller { controller, value } => MidiEvent::ControlChange {
            channel,
            ctrl: controller.as_int() as u8,
            value: value.as_int() as u8,
        },
        midly::MidiMessage::ProgramChange { program } => MidiEvent::ProgramChange {
            channel,
            program_id: program.as_int() as u8,
        },
        midly::MidiMessage::ChannelAftertouch { vel } => MidiEvent::ChannelPressure {
            channel,
            value: vel.as_int() as u8,
        },
        midly::MidiMessage::PitchBend { bend } => MidiEvent::PitchBend {
            channel,
            value: bend.as_int() as u16,
        },
    }
}

fn parse_smf(smf: &Smf) -> ParsedMidi {
    let mut all_events = Vec::new();
    let mut tempo_events = Vec::new();
    let mut max_tick = 0u64;
    let mut max_note_tick = 0u64;

    for track in &smf.tracks {
        let mut current_tick = 0u64;
        let mut last_tick = 0u64;
        let mut active_notes: Vec<Vec<u64>> = vec![Vec::new(); 128];
        for event in track {
            current_tick += event.delta.as_int() as u64;
            last_tick = current_tick;
            max_tick = max_tick.max(current_tick);
            match event.kind {
                TrackEventKind::Midi { channel, message } => {
                    let channel = channel.as_int() as u8;
                    match message {
                        midly::MidiMessage::NoteOff { key, .. } => {
                            let idx = key.as_int() as usize;
                            if active_notes[idx].pop().is_some() {
                                max_note_tick = max_note_tick.max(current_tick);
                            }
                        }
                        midly::MidiMessage::NoteOn { key, vel } => {
                            let idx = key.as_int() as usize;
                            if vel.as_int() > 0 {
                                active_notes[idx].push(current_tick);
                                max_note_tick = max_note_tick.max(current_tick);
                            } else if active_notes[idx].pop().is_some() {
                                max_note_tick = max_note_tick.max(current_tick);
                            }
                        }
                        _ => {}
                    }
                    all_events.push((current_tick, midi_message_to_event(channel, message)));
                }
                TrackEventKind::Meta(midly::MetaMessage::Tempo(us)) => {
                    tempo_events.push((current_tick, us.as_int()));
                }
                _ => {}
            }
        }
        if active_notes.iter().any(|notes| !notes.is_empty()) {
            max_note_tick = max_note_tick.max(last_tick);
        }
    }

    all_events.sort_by_key(|(tick, _)| *tick);

    ParsedMidi {
        events: all_events,
        tempo_events,
        max_tick,
        max_note_tick,
    }
}

fn build_playback_schedule_from_smf(smf: &Smf, sample_rate: u32) -> PlaybackSchedule {
    let parsed = parse_smf(smf);
    let ticks_per_beat = match smf.header.timing {
        midly::Timing::Metrical(ticks) => ticks.as_int() as f64,
        _ => 480.0,
    }
    .max(1.0);
    let tempo_segments = build_tempo_segments(&parsed.tempo_events, ticks_per_beat);

    let mut playback = Vec::with_capacity(parsed.events.len());
    for (tick, event) in parsed.events {
        let seconds = ticks_to_seconds(tick, &tempo_segments, ticks_per_beat);
        let sample = (seconds * sample_rate as f64).round() as u64;
        playback.push(MidiPlaybackEvent {
            tick,
            sample,
            event,
        });
    }

    playback.sort_by_key(|e| e.sample);
    let ruler_max_tick = if parsed.max_note_tick > 0 {
        parsed.max_note_tick
    } else {
        parsed.max_tick
    };
    let total_seconds = ticks_to_seconds(ruler_max_tick, &tempo_segments, ticks_per_beat);
    let total_samples = (total_seconds * sample_rate as f64).round() as u64;

    PlaybackSchedule {
        events: playback,
        ruler_max_tick,
        total_samples,
    }
}

fn audio_thread(
    cmd_rx: Receiver<AudioCommand>,
    samples_played: Arc<AtomicU64>,
    total_samples: Arc<AtomicU64>,
    max_tick_shared: Arc<AtomicU64>,
    last_event_sample: Arc<AtomicU64>,
    last_event_tick: Arc<AtomicU64>,
    next_event_sample: Arc<AtomicU64>,
    next_event_tick: Arc<AtomicU64>,
) {
    println!("Audio thread: Initializing CPAL...");
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let config = device.default_output_config().unwrap();

    let sample_rate = config.sample_rate();
    let channels = config.channels() as usize;
    println!(
        "Audio thread: Sample rate: {:?}, Channels: {}",
        sample_rate, channels
    );

    let synth = Arc::new(Mutex::new(Synth::default()));
    synth.lock().unwrap().set_sample_rate(sample_rate as f32);

    let playback_events = Arc::new(Mutex::new(Vec::<MidiPlaybackEvent>::new()));
    let playback_index = Arc::new(Mutex::new(0usize));
    let is_playing = Arc::new(Mutex::new(false));
    let mut last_midi_path: Option<PathBuf> = None;
    let mut last_soundfont_path: Option<PathBuf> = None;
    let synth_clone_cb = Arc::clone(&synth);
    let playback_events_clone_cb = Arc::clone(&playback_events);
    let samples_played_clone_cb = Arc::clone(&samples_played);
    let playback_index_clone_cb = Arc::clone(&playback_index);
    let is_playing_clone_cb = Arc::clone(&is_playing);
    let total_samples_clone_cb = Arc::clone(&total_samples);
    let max_tick_clone_cb = Arc::clone(&max_tick_shared);
    let last_event_sample_clone_cb = Arc::clone(&last_event_sample);
    let last_event_tick_clone_cb = Arc::clone(&last_event_tick);
    let next_event_sample_clone_cb = Arc::clone(&next_event_sample);
    let next_event_tick_clone_cb = Arc::clone(&next_event_tick);

    println!("Audio thread: Building output stream...");
    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let Ok(mut synth) = synth_clone_cb.try_lock() else {
                    return;
                };
                let Ok(events) = playback_events_clone_cb.try_lock() else {
                    return;
                };
                let Ok(mut index) = playback_index_clone_cb.try_lock() else {
                    return;
                };
                let Ok(playing_guard) = is_playing_clone_cb.try_lock() else {
                    return;
                };
                let playing = *playing_guard;
                for frame in data.chunks_mut(channels) {
                    if playing {
                        let current_sample = samples_played_clone_cb.load(Ordering::Relaxed);
                        while *index < events.len() && events[*index].sample <= current_sample {
                            let ev = &events[*index];
                            let _ = synth.send_event(ev.event);
                            last_event_sample_clone_cb.store(ev.sample, Ordering::Relaxed);
                            last_event_tick_clone_cb.store(ev.tick, Ordering::Relaxed);
                            *index += 1;
                        }
                        if *index < events.len() {
                            let next = &events[*index];
                            next_event_sample_clone_cb.store(next.sample, Ordering::Relaxed);
                            next_event_tick_clone_cb.store(next.tick, Ordering::Relaxed);
                        } else {
                            next_event_sample_clone_cb.store(
                                total_samples_clone_cb.load(Ordering::Relaxed),
                                Ordering::Relaxed,
                            );
                            next_event_tick_clone_cb.store(
                                max_tick_clone_cb.load(Ordering::Relaxed),
                                Ordering::Relaxed,
                            );
                        }

                        let mut samples = [0.0f32; 2];
                        synth.write(&mut samples[..]);
                        for (i, s) in frame.iter_mut().enumerate() {
                            *s = samples[i % 2];
                        }
                        samples_played_clone_cb.fetch_add(1, Ordering::Relaxed);
                    } else {
                        for s in frame.iter_mut() {
                            *s = 0.0;
                        }
                    }
                }
            },
            |err| eprintln!("an error occurred on stream: {}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();
    println!("Audio thread: Stream started.");

    loop {
        if let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                AudioCommand::Play(midi_path, sf_path) => {
                    println!("Audio thread: Play command received.");
                    let soundfont_changed = last_soundfont_path.as_ref() != Some(&sf_path);
                    let should_reload = last_midi_path.as_ref() != Some(&midi_path)
                        || soundfont_changed
                        || playback_events.lock().unwrap().is_empty();
                    let mut should_start = !should_reload;

                    if should_reload {
                        *is_playing.lock().unwrap() = false;
                        send_all_notes_off(&mut synth.lock().unwrap());

                        if soundfont_changed {
                            if let Ok(mut file) = std::fs::File::open(&sf_path) {
                                if let Ok(font) = SoundFont::load(&mut file) {
                                    let mut s = synth.lock().unwrap();
                                    s.add_font(font, true);
                                    println!("Audio thread: SoundFont loaded.");
                                }
                            }
                        }

                        if let Ok(schedule) = build_playback_schedule(&midi_path, sample_rate) {
                            let next_event = schedule
                                .events
                                .first()
                                .map(|event| (event.sample, event.tick));
                            *playback_events.lock().unwrap() = schedule.events;
                            samples_played.store(0, Ordering::Relaxed);
                            total_samples.store(schedule.total_samples, Ordering::Relaxed);
                            max_tick_shared.store(schedule.ruler_max_tick, Ordering::Relaxed);
                            last_event_sample.store(0, Ordering::Relaxed);
                            last_event_tick.store(0, Ordering::Relaxed);
                            if let Some((next_sample, next_tick)) = next_event {
                                next_event_sample.store(next_sample, Ordering::Relaxed);
                                next_event_tick.store(next_tick, Ordering::Relaxed);
                            } else {
                                next_event_sample.store(schedule.total_samples, Ordering::Relaxed);
                                next_event_tick.store(schedule.ruler_max_tick, Ordering::Relaxed);
                            }
                            *playback_index.lock().unwrap() = 0;
                            last_midi_path = Some(midi_path);
                            last_soundfont_path = Some(sf_path);
                            should_start = true;
                        }
                    }
                    if should_start {
                        *is_playing.lock().unwrap() = true;
                        println!("Audio thread: Playback started.");
                    }
                }
                AudioCommand::Pause => {
                    println!("Audio thread: Pause command received.");
                    *is_playing.lock().unwrap() = false;
                    send_all_notes_off(&mut synth.lock().unwrap());
                }
                AudioCommand::Stop => {
                    println!("Audio thread: Stop command received.");
                    *is_playing.lock().unwrap() = false;
                    samples_played.store(0, Ordering::Relaxed);
                    *playback_index.lock().unwrap() = 0;
                    hard_reset_synth(
                        &mut synth.lock().unwrap(),
                        sample_rate as f32,
                        last_soundfont_path.as_ref(),
                    );
                }
                AudioCommand::Rewind => {
                    println!("Audio thread: Rewind command received.");
                    samples_played.store(0, Ordering::Relaxed);
                    *playback_index.lock().unwrap() = 0;
                    hard_reset_synth(
                        &mut synth.lock().unwrap(),
                        sample_rate as f32,
                        last_soundfont_path.as_ref(),
                    );
                }
            }
        }
    }
}

fn hard_reset_synth(synth: &mut Synth, sample_rate: f32, soundfont_path: Option<&PathBuf>) {
    *synth = Synth::default();
    synth.set_sample_rate(sample_rate);

    if let Some(path) = soundfont_path {
        if let Ok(mut file) = std::fs::File::open(path) {
            if let Ok(font) = SoundFont::load(&mut file) {
                synth.add_font(font, true);
            }
        }
    }
}

fn send_all_notes_off(synth: &mut Synth) {
    for channel in 0u8..16 {
        let _ = synth.send_event(MidiEvent::ControlChange {
            channel,
            ctrl: 64,
            value: 0,
        });
        let _ = synth.send_event(MidiEvent::ControlChange {
            channel,
            ctrl: 120,
            value: 0,
        });
        let _ = synth.send_event(MidiEvent::ControlChange {
            channel,
            ctrl: 123,
            value: 0,
        });
        let _ = synth.send_event(MidiEvent::ControlChange {
            channel,
            ctrl: 121,
            value: 0,
        });
        for key in 0u8..128 {
            let _ = synth.send_event(MidiEvent::NoteOff { channel, key });
        }
    }
}

fn build_playback_schedule(midi_path: &PathBuf, sample_rate: u32) -> Result<PlaybackSchedule, ()> {
    let data = std::fs::read(midi_path).map_err(|_| ())?;
    let smf = Smf::parse(&data).map_err(|_| ())?;
    Ok(build_playback_schedule_from_smf(&smf, sample_rate))
}

#[cfg(test)]
mod tests {
    use super::{build_playback_schedule_from_smf, midi_message_to_event, parse_smf};
    use midly::{Format, Smf, Timing, TrackEvent, TrackEventKind};
    use oxisynth::MidiEvent;

    #[test]
    fn build_playback_schedule_respects_note_range() {
        let mut track = Vec::new();
        track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: midly::MidiMessage::NoteOn {
                    key: 60.into(),
                    vel: 100.into(),
                },
            },
        });
        track.push(TrackEvent {
            delta: 120.into(),
            kind: TrackEventKind::Meta(midly::MetaMessage::TrackName(b"Test")),
        });
        track.push(TrackEvent {
            delta: 120.into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: midly::MidiMessage::NoteOff {
                    key: 60.into(),
                    vel: 0.into(),
                },
            },
        });

        let smf = Smf {
            header: midly::Header {
                format: Format::SingleTrack,
                timing: Timing::Metrical(480.into()),
            },
            tracks: vec![track],
        };

        let schedule = build_playback_schedule_from_smf(&smf, 48_000);
        assert!(schedule.ruler_max_tick > 0);
        assert_eq!(schedule.events.len(), 2);
        assert!(schedule.total_samples > 0);
    }

    #[test]
    fn midi_message_to_event_maps_note_on() {
        let event = midi_message_to_event(
            2,
            midly::MidiMessage::NoteOn {
                key: 64.into(),
                vel: 100.into(),
            },
        );
        match event {
            MidiEvent::NoteOn { channel, key, vel } => {
                assert_eq!(channel, 2);
                assert_eq!(key, 64);
                assert_eq!(vel, 100);
            }
            _ => panic!("expected note on"),
        }
    }

    #[test]
    fn parse_smf_collects_tempo_and_ticks() {
        let mut track = Vec::new();
        track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(midly::MetaMessage::Tempo(500_000.into())),
        });
        track.push(TrackEvent {
            delta: 120.into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: midly::MidiMessage::NoteOn {
                    key: 60.into(),
                    vel: 100.into(),
                },
            },
        });
        track.push(TrackEvent {
            delta: 120.into(),
            kind: TrackEventKind::Meta(midly::MetaMessage::Tempo(400_000.into())),
        });
        track.push(TrackEvent {
            delta: 240.into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: midly::MidiMessage::NoteOff {
                    key: 60.into(),
                    vel: 0.into(),
                },
            },
        });

        let smf = Smf {
            header: midly::Header {
                format: Format::SingleTrack,
                timing: Timing::Metrical(480.into()),
            },
            tracks: vec![track],
        };

        let parsed = parse_smf(&smf);
        assert_eq!(parsed.tempo_events.len(), 2);
        assert!(parsed.max_tick > 0);
        assert!(parsed.max_note_tick > 0);
        assert_eq!(parsed.events.len(), 2);
    }
}
