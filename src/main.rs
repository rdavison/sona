use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use midly::{Smf, TrackEventKind};
use oxisynth::{MidiEvent, SoundFont, Synth};
use rfd::FileDialog;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let (cmd_tx, cmd_rx) = channel::<AudioCommand>();

    // Start audio thread
    thread::spawn(move || {
        audio_thread(cmd_rx);
    });

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sona - Retro MIDI Player".to_string(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(AudioSender(cmd_tx))
        .init_resource::<UiState>()
        .init_resource::<MidiFilePath>()
        .init_resource::<SoundFontPath>()
        .init_resource::<PlaybackStatus>()
        .init_resource::<Keybindings>()
        .add_systems(Startup, (setup_ui, load_keybindings))
        .add_systems(
            Update,
            (keyboard_navigation, update_selection_visuals, handle_input),
        )
        .run();
}

enum AudioCommand {
    Play(PathBuf, PathBuf),
    Stop,
    Rewind,
}

#[derive(Resource)]
struct AudioSender(Sender<AudioCommand>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum UiSelection {
    #[default]
    MidiFile,
    SoundFont,
    Play,
    Stop,
    Rewind,
}

#[derive(Resource, Default)]
struct UiState {
    selection: UiSelection,
}

#[derive(Resource, Default)]
struct MidiFilePath(Option<std::path::PathBuf>);

#[derive(Resource, Default)]
struct SoundFontPath(Option<std::path::PathBuf>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[derive(Resource, Default)]
struct PlaybackStatus {
    state: PlaybackState,
}

#[derive(Resource, Default, Deserialize)]
struct Keybindings {
    bindings: HashMap<String, String>,
}

impl Keybindings {
    fn get_keycode(&self, action: &str) -> Option<KeyCode> {
        self.bindings.get(action).and_then(|s| str_to_keycode(s))
    }
}

fn str_to_keycode(s: &str) -> Option<KeyCode> {
    match s.to_lowercase().as_str() {
        "up" | "arrowup" => Some(KeyCode::ArrowUp),
        "down" | "arrowdown" => Some(KeyCode::ArrowDown),
        "left" | "arrowleft" => Some(KeyCode::ArrowLeft),
        "right" | "arrowright" => Some(KeyCode::ArrowRight),
        "enter" | "return" => Some(KeyCode::Enter),
        "space" => Some(KeyCode::Space),
        "tab" => Some(KeyCode::Tab),
        "backspace" => Some(KeyCode::Backspace),
        "escape" | "esc" => Some(KeyCode::Escape),
        "p" => Some(KeyCode::KeyP),
        "s" => Some(KeyCode::KeyS),
        "t" => Some(KeyCode::KeyT),
        _ => None,
    }
}

fn load_keybindings(mut keybindings: ResMut<Keybindings>) {
    if let Ok(content) = std::fs::read_to_string("keybindings.toml") {
        if let Ok(config) = toml::from_str::<Keybindings>(&content) {
            *keybindings = config;
            println!("Keybindings loaded successfully.");
        } else {
            eprintln!("Failed to parse keybindings.toml");
        }
    } else {
        eprintln!("Failed to read keybindings.toml");
    }
}

// Marker components for UI elements
#[derive(Component)]
struct MidiFileText;

#[derive(Component)]
struct SoundFontText;

#[derive(Component)]
struct PlayButton;

#[derive(Component)]
struct StopButton;

#[derive(Component)]
struct RewindButton;

#[derive(Component)]
struct PlaybackStatusText;

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d::default());

    let font = asset_server.load("PixelifySans-Regular.ttf");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 0.0, 0.5)), // ZSNES Blue
        ))
        .with_children(|parent| {
            // Main Window Container
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(20.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.0, 0.0, 0.7)),
                BorderColor::all(Color::WHITE),
            )).with_children(|parent| {
                // Status bar
                parent.spawn((
                    Text::new("Status: Stopped"),
                    TextFont {
                        font: font.clone(),
                        font_size: 30.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    PlaybackStatusText,
                ));

                // Spacer
                parent.spawn((Node { height: Val::Px(20.0), ..default() },));

                // File Selectors
                parent.spawn((
                    Text::new("MIDI File: [None]"),
                    TextFont {
                        font: font.clone(),
                        font_size: 40.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    MidiFileText,
                ));

                parent.spawn((
                    Text::new("SoundFont: [None]"),
                    TextFont {
                        font: font.clone(),
                        font_size: 40.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    SoundFontText,
                ));

                // Spacer
                parent.spawn((Node { height: Val::Px(20.0), ..default() },));

                // Playback Controls
                parent
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(20.0),
                        ..default()
                    },))
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new("[ Play ]"),
                            TextFont {
                                font: font.clone(),
                                font_size: 40.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            PlayButton,
                        ));
                        parent.spawn((
                            Text::new("[ Stop ]"),
                            TextFont {
                                font: font.clone(),
                                font_size: 40.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            StopButton,
                        ));
                        parent.spawn((
                            Text::new("[ Rewind ]"),
                            TextFont {
                                font: font.clone(),
                                font_size: 40.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            RewindButton,
                        ));
                    });
            });
        });
}

fn keyboard_navigation(
    mut ui_state: ResMut<UiState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    keybindings: Res<Keybindings>,
) {
    let up = keybindings.get_keycode("NavigateUp").unwrap_or(KeyCode::ArrowUp);
    let down = keybindings.get_keycode("NavigateDown").unwrap_or(KeyCode::ArrowDown);
    let left = keybindings.get_keycode("NavigateLeft").unwrap_or(KeyCode::ArrowLeft);
    let right = keybindings.get_keycode("NavigateRight").unwrap_or(KeyCode::ArrowRight);

    if keyboard_input.just_pressed(down) {
        ui_state.selection = match ui_state.selection {
            UiSelection::MidiFile => UiSelection::SoundFont,
            UiSelection::SoundFont => UiSelection::Play,
            _ => ui_state.selection,
        };
    } else if keyboard_input.just_pressed(up) {
        ui_state.selection = match ui_state.selection {
            UiSelection::SoundFont => UiSelection::MidiFile,
            UiSelection::Play | UiSelection::Stop | UiSelection::Rewind => UiSelection::SoundFont,
            _ => ui_state.selection,
        };
    } else if keyboard_input.just_pressed(right) {
        ui_state.selection = match ui_state.selection {
            UiSelection::Play => UiSelection::Stop,
            UiSelection::Stop => UiSelection::Rewind,
            _ => ui_state.selection,
        };
    } else if keyboard_input.just_pressed(left) {
        ui_state.selection = match ui_state.selection {
            UiSelection::Rewind => UiSelection::Stop,
            UiSelection::Stop => UiSelection::Play,
            _ => ui_state.selection,
        };
    }
}

fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    ui_state: Res<UiState>,
    mut midi_path: ResMut<MidiFilePath>,
    mut soundfont_path: ResMut<SoundFontPath>,
    mut playback_status: ResMut<PlaybackStatus>,
    audio_tx: Res<AudioSender>,
    keybindings: Res<Keybindings>,
) {
    let select_key = keybindings.get_keycode("Select").unwrap_or(KeyCode::Enter);
    let play_key = keybindings.get_keycode("Play").unwrap_or(KeyCode::KeyP);
    let stop_key = keybindings.get_keycode("Stop").unwrap_or(KeyCode::KeyS);

    if keyboard_input.just_pressed(select_key) {
        match ui_state.selection {
            UiSelection::MidiFile => {
                if let Some(path) = FileDialog::new()
                    .add_filter("MIDI", &["mid", "midi"])
                    .pick_file()
                {
                    midi_path.0 = Some(path);
                }
            }
            UiSelection::SoundFont => {
                if let Some(path) = FileDialog::new()
                    .add_filter("SoundFont", &["sf2"])
                    .pick_file()
                {
                    soundfont_path.0 = Some(path);
                }
            }
            UiSelection::Play => {
                if let (Some(midi), Some(sf)) = (&midi_path.0, &soundfont_path.0) {
                    playback_status.state = PlaybackState::Playing;
                    let _ = audio_tx
                        .0
                        .send(AudioCommand::Play(midi.clone(), sf.clone()));
                }
            }
            UiSelection::Stop => {
                playback_status.state = PlaybackState::Stopped;
                let _ = audio_tx.0.send(AudioCommand::Stop);
            }
            UiSelection::Rewind => {
                playback_status.state = PlaybackState::Stopped;
                let _ = audio_tx.0.send(AudioCommand::Rewind);
            }
        }
    }

    if keyboard_input.just_pressed(play_key) {
        if let (Some(midi), Some(sf)) = (&midi_path.0, &soundfont_path.0) {
            playback_status.state = PlaybackState::Playing;
            let _ = audio_tx.0.send(AudioCommand::Play(midi.clone(), sf.clone()));
        }
    }

    if keyboard_input.just_pressed(stop_key) {
        playback_status.state = PlaybackState::Stopped;
        let _ = audio_tx.0.send(AudioCommand::Stop);
    }
}

fn update_selection_visuals(
    ui_state: Res<UiState>,
    midi_path: Res<MidiFilePath>,
    soundfont_path: Res<SoundFontPath>,
    playback_status: Res<PlaybackStatus>,
    mut midi_query: Query<
        (&mut TextColor, &mut Text),
        (
            With<MidiFileText>,
            Without<SoundFontText>,
            Without<PlayButton>,
            Without<StopButton>,
            Without<RewindButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut soundfont_query: Query<
        (&mut TextColor, &mut Text),
        (
            With<SoundFontText>,
            Without<MidiFileText>,
            Without<PlayButton>,
            Without<StopButton>,
            Without<RewindButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut play_query: Query<
        &mut TextColor,
        (
            With<PlayButton>,
            Without<MidiFileText>,
            Without<SoundFontText>,
            Without<StopButton>,
            Without<RewindButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut stop_query: Query<
        &mut TextColor,
        (
            With<StopButton>,
            Without<MidiFileText>,
            Without<SoundFontText>,
            Without<PlayButton>,
            Without<RewindButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut rewind_query: Query<
        &mut TextColor,
        (
            With<RewindButton>,
            Without<MidiFileText>,
            Without<SoundFontText>,
            Without<PlayButton>,
            Without<StopButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut status_query: Query<
        &mut Text,
        (
            With<PlaybackStatusText>,
            Without<MidiFileText>,
            Without<SoundFontText>,
            Without<PlayButton>,
            Without<StopButton>,
            Without<RewindButton>,
        ),
    >,
) {
    let selected_color = Color::srgb(1.0, 1.0, 0.0); // Yellow
    let default_color = Color::WHITE;

    for (mut color, mut text) in &mut midi_query {
        color.0 = if ui_state.selection == UiSelection::MidiFile {
            selected_color
        } else {
            default_color
        };
        if let Some(path) = &midi_path.0 {
            text.0 = format!("MIDI File: {}", path.file_name().unwrap().to_string_lossy());
        }
    }
    for (mut color, mut text) in &mut soundfont_query {
        color.0 = if ui_state.selection == UiSelection::SoundFont {
            selected_color
        } else {
            default_color
        };
        if let Some(path) = &soundfont_path.0 {
            text.0 = format!("SoundFont: {}", path.file_name().unwrap().to_string_lossy());
        }
    }
    for mut color in &mut play_query {
        color.0 = if ui_state.selection == UiSelection::Play {
            selected_color
        } else {
            default_color
        };
    }
    for mut color in &mut stop_query {
        color.0 = if ui_state.selection == UiSelection::Stop {
            selected_color
        } else {
            default_color
        };
    }
    for mut color in &mut rewind_query {
        color.0 = if ui_state.selection == UiSelection::Rewind {
            selected_color
        } else {
            default_color
        };
    }
    for mut text in &mut status_query {
        text.0 = format!("Status: {:?}", playback_status.state);
    }
}

struct MidiPlaybackEvent {
    tick: u64,
    event: MidiEvent,
}

fn audio_thread(cmd_rx: Receiver<AudioCommand>) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let config = device.default_output_config().unwrap();

    let sample_rate = config.sample_rate();
    let channels = config.channels() as usize;

    let synth = Arc::new(Mutex::new(Synth::default()));
    synth.lock().unwrap().set_sample_rate(sample_rate as f32);

    let playback_events = Arc::new(Mutex::new(Vec::<MidiPlaybackEvent>::new()));
    let samples_played = Arc::new(Mutex::new(0u64));
    let is_playing = Arc::new(Mutex::new(false));
    let ticks_per_sample = Arc::new(Mutex::new(0.0f64));

    // Clones for the audio callback
    let synth_clone_cb = Arc::clone(&synth);
    let playback_events_clone_cb = Arc::clone(&playback_events);
    let samples_played_clone_cb = Arc::clone(&samples_played);
    let is_playing_clone_cb = Arc::clone(&is_playing);
    let ticks_per_sample_clone_cb = Arc::clone(&ticks_per_sample);

    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut synth = synth_clone_cb.lock().unwrap();
                let mut events = playback_events_clone_cb.lock().unwrap();
                let mut samples_count = samples_played_clone_cb.lock().unwrap();
                let playing = *is_playing_clone_cb.lock().unwrap();
                let tps = *ticks_per_sample_clone_cb.lock().unwrap();

                for frame in data.chunks_mut(channels) {
                    if playing {
                        // Trigger events for the current tick
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

    loop {
        if let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                AudioCommand::Play(midi_path, sf_path) => {
                    // Stop current playback
                    *is_playing.lock().unwrap() = false;

                    // Load SoundFont
                    if let Ok(mut file) = std::fs::File::open(sf_path) {
                        if let Ok(font) = SoundFont::load(&mut file) {
                            let mut s = synth.lock().unwrap();
                            s.add_font(font, true);
                        }
                    }

                    // Parse MIDI
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
                                            midly::MidiMessage::NoteOff { key, .. } => MidiEvent::NoteOff {
                                                channel: channel.as_int() as u8,
                                                key: key.as_int() as u8,
                                            },
                                            midly::MidiMessage::NoteOn { key, vel } => MidiEvent::NoteOn {
                                                channel: channel.as_int() as u8,
                                                key: key.as_int() as u8,
                                                vel: vel.as_int() as u8,
                                            },
                                            midly::MidiMessage::Aftertouch { key, vel } => {
                                                MidiEvent::PolyphonicKeyPressure {
                                                    channel: channel.as_int() as u8,
                                                    key: key.as_int() as u8,
                                                    value: vel.as_int() as u8,
                                                }
                                            }
                                            midly::MidiMessage::Controller { controller, value } => {
                                                MidiEvent::ControlChange {
                                                    channel: channel.as_int() as u8,
                                                    ctrl: controller.as_int() as u8,
                                                    value: value.as_int() as u8,
                                                }
                                            }
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

                            let bpm = 120.0; // Default BPM
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
                        }
                    }
                }
                AudioCommand::Stop => {
                    *is_playing.lock().unwrap() = false;
                }
                AudioCommand::Rewind => {
                    *is_playing.lock().unwrap() = false;
                    *samples_played.lock().unwrap() = 0;
                    // Resetting events would require reloading MIDI or keeping a copy.
                    // For now, simple stop and reset count is a start.
                }
            }
        }
    }
}
