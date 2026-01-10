use bevy::prelude::{App, Plugin, Resource};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use midly::{Smf, TrackEventKind};
use oxisynth::{MidiEvent, SoundFont, Synth};
use std::path::PathBuf;
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

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let (cmd_tx, cmd_rx) = channel::<AudioCommand>();

        // Start audio thread
        thread::spawn(move || {
            println!("Audio thread spawned.");
            audio_thread(cmd_rx);
        });

        app.insert_resource(AudioSender(cmd_tx));
    }
}

struct MidiPlaybackEvent {
    sample: u64,
    event: MidiEvent,
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

fn audio_thread(cmd_rx: Receiver<AudioCommand>) {
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
    let samples_played = Arc::new(Mutex::new(0u64));
    let playback_index = Arc::new(Mutex::new(0usize));
    let is_playing = Arc::new(Mutex::new(false));
    let mut last_midi_path: Option<PathBuf> = None;
    let mut last_soundfont_path: Option<PathBuf> = None;
    let synth_clone_cb = Arc::clone(&synth);
    let playback_events_clone_cb = Arc::clone(&playback_events);
    let samples_played_clone_cb = Arc::clone(&samples_played);
    let playback_index_clone_cb = Arc::clone(&playback_index);
    let is_playing_clone_cb = Arc::clone(&is_playing);

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
                let Ok(mut samples_count) = samples_played_clone_cb.try_lock() else {
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
                        let current_sample = *samples_count;
                        while *index < events.len()
                            && events[*index].sample <= current_sample
                        {
                            let ev = &events[*index];
                            let _ = synth.send_event(ev.event);
                            *index += 1;
                        }

                        let mut samples = [0.0f32; 2];
                        synth.write(&mut samples[..]);
                        for (i, s) in frame.iter_mut().enumerate() {
                            *s = samples[i % 2];
                        }
                        *samples_count += 1;
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

                        if let Ok(data) = std::fs::read(&midi_path) {
                            if let Ok(smf) = Smf::parse(&data) {
                                let timing = smf.header.timing;
                                let mut all_events = Vec::new();
                                let mut tempo_events = Vec::new();
                                for track in smf.tracks {
                                    let mut current_tick = 0u64;
                                    for event in track {
                                        current_tick += event.delta.as_int() as u64;
                                        match event.kind {
                                            TrackEventKind::Midi { channel, message } => {
                                                let ev = match message {
                                                    midly::MidiMessage::NoteOff { key, .. } => {
                                                        MidiEvent::NoteOff {
                                                            channel: channel.as_int() as u8,
                                                            key: key.as_int() as u8,
                                                        }
                                                    }
                                                    midly::MidiMessage::NoteOn { key, vel } => {
                                                        MidiEvent::NoteOn {
                                                            channel: channel.as_int() as u8,
                                                            key: key.as_int() as u8,
                                                            vel: vel.as_int() as u8,
                                                        }
                                                    }
                                                    midly::MidiMessage::Aftertouch { key, vel } => {
                                                        MidiEvent::PolyphonicKeyPressure {
                                                            channel: channel.as_int() as u8,
                                                            key: key.as_int() as u8,
                                                            value: vel.as_int() as u8,
                                                        }
                                                    }
                                                    midly::MidiMessage::Controller {
                                                        controller,
                                                        value,
                                                    } => MidiEvent::ControlChange {
                                                        channel: channel.as_int() as u8,
                                                        ctrl: controller.as_int() as u8,
                                                        value: value.as_int() as u8,
                                                    },
                                                    midly::MidiMessage::ProgramChange { program } => {
                                                        MidiEvent::ProgramChange {
                                                            channel: channel.as_int() as u8,
                                                            program_id: program.as_int() as u8,
                                                        }
                                                    }
                                                    midly::MidiMessage::ChannelAftertouch { vel } => {
                                                        MidiEvent::ChannelPressure {
                                                            channel: channel.as_int() as u8,
                                                            value: vel.as_int() as u8,
                                                        }
                                                    }
                                                    midly::MidiMessage::PitchBend { bend } => {
                                                        MidiEvent::PitchBend {
                                                            channel: channel.as_int() as u8,
                                                            value: bend.as_int() as u16,
                                                        }
                                                    }
                                                };
                                                all_events.push((current_tick, ev));
                                            }
                                            TrackEventKind::Meta(midly::MetaMessage::Tempo(us)) => {
                                                tempo_events.push((current_tick, us.as_int()));
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                all_events.sort_by_key(|(tick, _)| *tick);

                                let ticks_per_beat = match timing {
                                    midly::Timing::Metrical(ticks) => ticks.as_int() as f64,
                                    _ => 480.0,
                                }
                                .max(1.0);
                                let tempo_segments =
                                    build_tempo_segments(&tempo_events, ticks_per_beat);

                                let mut playback = Vec::with_capacity(all_events.len());
                                for (tick, event) in all_events {
                                    let seconds =
                                        ticks_to_seconds(tick, &tempo_segments, ticks_per_beat);
                                    let sample = (seconds * sample_rate as f64).round() as u64;
                                    playback.push(MidiPlaybackEvent { sample, event });
                                }

                                playback.sort_by_key(|e| e.sample);
                                *playback_events.lock().unwrap() = playback;
                                *samples_played.lock().unwrap() = 0;
                                *playback_index.lock().unwrap() = 0;
                                last_midi_path = Some(midi_path);
                                last_soundfont_path = Some(sf_path);
                                should_start = true;
                            }
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
                    *samples_played.lock().unwrap() = 0;
                    *playback_index.lock().unwrap() = 0;
                    send_all_notes_off(&mut synth.lock().unwrap());
                }
                AudioCommand::Rewind => {
                    println!("Audio thread: Rewind command received.");
                    *samples_played.lock().unwrap() = 0;
                    *playback_index.lock().unwrap() = 0;
                    send_all_notes_off(&mut synth.lock().unwrap());
                }
            }
        }
    }
}

fn send_all_notes_off(synth: &mut Synth) {
    for channel in 0u8..16 {
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
    }
}
