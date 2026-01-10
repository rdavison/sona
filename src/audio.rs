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
    tick: u64,
    event: MidiEvent,
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
    let is_playing = Arc::new(Mutex::new(false));
    let ticks_per_sample = Arc::new(Mutex::new(0.0f64));

    let synth_clone_cb = Arc::clone(&synth);
    let playback_events_clone_cb = Arc::clone(&playback_events);
    let samples_played_clone_cb = Arc::clone(&samples_played);
    let is_playing_clone_cb = Arc::clone(&is_playing);
    let ticks_per_sample_clone_cb = Arc::clone(&ticks_per_sample);

    println!("Audio thread: Building output stream...");
    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let Ok(mut synth) = synth_clone_cb.try_lock() else {
                    return;
                };
                let Ok(mut events) = playback_events_clone_cb.try_lock() else {
                    return;
                };
                let Ok(mut samples_count) = samples_played_clone_cb.try_lock() else {
                    return;
                };
                let Ok(playing_guard) = is_playing_clone_cb.try_lock() else {
                    return;
                };
                let playing = *playing_guard;
                let Ok(tps_guard) = ticks_per_sample_clone_cb.try_lock() else {
                    return;
                };
                let tps = *tps_guard;

                for frame in data.chunks_mut(channels) {
                    if playing {
                        let current_tick = (*samples_count as f64 * tps) as u64;
                        while !events.is_empty() && events[0].tick <= current_tick {
                            let ev = events.remove(0);
                            let _ = synth.send_event(ev.event);
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
                    *is_playing.lock().unwrap() = false;

                    if let Ok(mut file) = std::fs::File::open(sf_path) {
                        if let Ok(font) = SoundFont::load(&mut file) {
                            let mut s = synth.lock().unwrap();
                            s.add_font(font, true);
                            println!("Audio thread: SoundFont loaded.");
                        }
                    }

                    if let Ok(data) = std::fs::read(midi_path) {
                        if let Ok(smf) = Smf::parse(&data) {
                            let timing = smf.header.timing;
                            let mut all_events = Vec::new();
                            for track in smf.tracks {
                                let mut current_tick = 0u64;
                                for event in track {
                                    current_tick += event.delta.as_int() as u64;
                                    if let TrackEventKind::Midi { channel, message } = event.kind {
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
                                        all_events.push(MidiPlaybackEvent {
                                            tick: current_tick,
                                            event: ev,
                                        });
                                    }
                                }
                            }
                            all_events.sort_by_key(|e| e.tick);

                            let bpm = 120.0;
                            let tpb = match timing {
                                midly::Timing::Metrical(ticks) => ticks.as_int() as f64,
                                _ => 480.0,
                            };

                            let ticks_per_second = (bpm * tpb) / 60.0;
                            let tps = ticks_per_second / sample_rate as f64;

                            *ticks_per_sample.lock().unwrap() = tps;
                            *playback_events.lock().unwrap() = all_events;
                            *samples_played.lock().unwrap() = 0;
                            *is_playing.lock().unwrap() = true;
                            println!("Audio thread: MIDI parsed and playback started.");
                        }
                    }
                }
                AudioCommand::Stop => {
                    println!("Audio thread: Stop command received.");
                    *is_playing.lock().unwrap() = false;
                }
                AudioCommand::Rewind => {
                    println!("Audio thread: Rewind command received.");
                    *is_playing.lock().unwrap() = false;
                    *samples_played.lock().unwrap() = 0;
                }
            }
        }
    }
}
