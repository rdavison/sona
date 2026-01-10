use crate::audio::{AudioCommand, AudioSender};
use crate::state::{
    MidiFilePath, MidiTrackInfo, MidiTracks, PlaybackState, PlaybackStatus, SoundFontPath, UiPage,
    UiSelection, UiState,
};
use bevy::prelude::{
    App, ButtonInput, Commands, Component, Entity, KeyCode, Plugin, Query, Res, ResMut, Resource,
    Startup, Update,
};
use bevy::tasks::IoTaskPool;
use futures_lite::future;
use midly::{MetaMessage, Smf, TrackEventKind};
use rfd::FileDialog;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Resource, Default, Deserialize)]
pub struct Keybindings {
    pub bindings: HashMap<String, String>,
}

impl Keybindings {
    pub fn get_keycode(&self, action: &str) -> Option<KeyCode> {
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

pub fn load_keybindings(mut keybindings: ResMut<Keybindings>) {
    println!("Loading keybindings...");
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

#[derive(Component)]
pub struct FileDialogTask(pub bevy::tasks::Task<Option<PathBuf>>, pub UiSelection);

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Keybindings>()
            .add_systems(Startup, load_keybindings)
            .add_systems(
                Update,
                (keyboard_navigation, handle_input, poll_file_dialogs),
            );
    }
}

fn keyboard_navigation(
    mut ui_state: ResMut<UiState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    keybindings: Res<Keybindings>,
) {
    if ui_state.page != UiPage::Splash {
        return;
    }

    let up = keybindings
        .get_keycode("NavigateUp")
        .unwrap_or(KeyCode::ArrowUp);
    let down = keybindings
        .get_keycode("NavigateDown")
        .unwrap_or(KeyCode::ArrowDown);
    let left = keybindings
        .get_keycode("NavigateLeft")
        .unwrap_or(KeyCode::ArrowLeft);
    let right = keybindings
        .get_keycode("NavigateRight")
        .unwrap_or(KeyCode::ArrowRight);

    if keyboard_input.just_pressed(down) {
        println!("Key: Down");
        ui_state.selection = match ui_state.selection {
            UiSelection::MidiFile => UiSelection::SoundFont,
            UiSelection::SoundFont => UiSelection::Play,
            _ => ui_state.selection,
        };
    } else if keyboard_input.just_pressed(up) {
        println!("Key: Up");
        ui_state.selection = match ui_state.selection {
            UiSelection::SoundFont => UiSelection::MidiFile,
            UiSelection::Play | UiSelection::Stop | UiSelection::Rewind => UiSelection::SoundFont,
            _ => ui_state.selection,
        };
    } else if keyboard_input.just_pressed(right) {
        println!("Key: Right");
        ui_state.selection = match ui_state.selection {
            UiSelection::Play => UiSelection::Stop,
            UiSelection::Stop => UiSelection::Rewind,
            _ => ui_state.selection,
        };
    } else if keyboard_input.just_pressed(left) {
        println!("Key: Left");
        ui_state.selection = match ui_state.selection {
            UiSelection::Rewind => UiSelection::Stop,
            UiSelection::Stop => UiSelection::Play,
            _ => ui_state.selection,
        };
    }
}

fn handle_input(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    midi_path: Res<MidiFilePath>,
    soundfont_path: Res<SoundFontPath>,
    mut playback_status: ResMut<PlaybackStatus>,
    audio_tx: Res<AudioSender>,
    keybindings: Res<Keybindings>,
) {
    let about_toggle = keyboard_input.just_pressed(KeyCode::Slash)
        && (keyboard_input.pressed(KeyCode::ShiftLeft)
            || keyboard_input.pressed(KeyCode::ShiftRight));
    if about_toggle {
        ui_state.page = match ui_state.page {
            UiPage::Splash => UiPage::About,
            UiPage::About => UiPage::Splash,
            UiPage::Tracks => UiPage::About,
        };
        return;
    }

    let tracks_key = keybindings.get_keycode("Tracks").unwrap_or(KeyCode::KeyT);
    if keyboard_input.just_pressed(tracks_key) {
        ui_state.page = if ui_state.page == UiPage::Tracks {
            UiPage::Splash
        } else {
            UiPage::Tracks
        };
        return;
    }

    if ui_state.page != UiPage::Splash {
        return;
    }

    let select_key = keybindings.get_keycode("Select").unwrap_or(KeyCode::Enter);
    let play_key = keybindings.get_keycode("Play").unwrap_or(KeyCode::KeyP);
    let stop_key = keybindings.get_keycode("Stop").unwrap_or(KeyCode::KeyS);

    if keyboard_input.just_pressed(select_key) {
        println!("Key: Select");
        match ui_state.selection {
            UiSelection::MidiFile => {
                let thread_pool = IoTaskPool::get();
                let task = thread_pool.spawn(async move {
                    FileDialog::new()
                        .add_filter("MIDI", &["mid", "midi"])
                        .pick_file()
                });
                commands.spawn(FileDialogTask(task, UiSelection::MidiFile));
            }
            UiSelection::SoundFont => {
                let thread_pool = IoTaskPool::get();
                let task = thread_pool.spawn(async move {
                    FileDialog::new()
                        .add_filter("SoundFont", &["sf2"])
                        .pick_file()
                });
                commands.spawn(FileDialogTask(task, UiSelection::SoundFont));
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
            let _ = audio_tx
                .0
                .send(AudioCommand::Play(midi.clone(), sf.clone()));
        }
    }

    if keyboard_input.just_pressed(stop_key) {
        playback_status.state = PlaybackState::Stopped;
        let _ = audio_tx.0.send(AudioCommand::Stop);
    }
}

fn poll_file_dialogs(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut FileDialogTask)>,
    mut midi_path: ResMut<MidiFilePath>,
    mut soundfont_path: ResMut<SoundFontPath>,
    mut midi_tracks: ResMut<MidiTracks>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            println!("File dialog result received.");
            if let Some(path) = result {
                match task.1 {
                    UiSelection::MidiFile => {
                        midi_path.0 = Some(path.clone());
                        midi_tracks.0 = load_midi_tracks(&path);
                    }
                    UiSelection::SoundFont => soundfont_path.0 = Some(path),
                    _ => {}
                }
            }
            commands.entity(entity).despawn();
        }
    }
}

fn load_midi_tracks(path: &PathBuf) -> Vec<MidiTrackInfo> {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to read MIDI file: {err}");
            return Vec::new();
        }
    };

    let smf = match Smf::parse(&data) {
        Ok(smf) => smf,
        Err(err) => {
            eprintln!("Failed to parse MIDI file: {err:?}");
            return Vec::new();
        }
    };

    let mut track_ticks: Vec<Vec<u64>> = Vec::new();
    let mut track_info: Vec<(usize, Option<String>, usize, u64)> = Vec::new();
    let mut max_tick = 0u64;

    for (index, track) in smf.tracks.iter().enumerate() {
        let mut current_tick = 0u64;
        let mut ticks = Vec::new();
        let name = track.iter().find_map(|event| match event.kind {
            TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                Some(String::from_utf8_lossy(name).to_string())
            }
            _ => None,
        });

        for event in track.iter() {
            current_tick += event.delta.as_int() as u64;
            if let TrackEventKind::Midi { message, .. } = event.kind {
                if let midly::MidiMessage::NoteOn { vel, .. } = message {
                    if vel.as_int() > 0 {
                        ticks.push(current_tick);
                    }
                }
            }
        }

        let track_end = ticks.last().copied().unwrap_or(0);
        max_tick = max_tick.max(track_end);
        track_ticks.push(ticks);
        track_info.push((index, name, track.len(), track_end));
    }

    let preview_width = 48usize;
    track_info
        .into_iter()
        .zip(track_ticks.into_iter())
        .map(|((index, name, event_count, track_end), ticks)| MidiTrackInfo {
            index,
            name,
            event_count,
            preview: build_track_preview(preview_width, max_tick, track_end, &ticks),
        })
        .collect()
}

fn build_track_preview(width: usize, max_tick: u64, track_end: u64, ticks: &[u64]) -> String {
    if width == 0 {
        return String::new();
    }

    let mut chars = vec!['.'; width];
    let denom = max_tick.max(1) as f64;

    for (i, slot) in chars.iter_mut().enumerate() {
        if i % 8 == 0 {
            *slot = '|';
        }
    }

    for &tick in ticks {
        let pos = ((tick as f64 / denom) * (width.saturating_sub(1)) as f64).round() as usize;
        if pos < width {
            chars[pos] = '#';
        }
    }

    let track_pos =
        ((track_end as f64 / denom) * (width.saturating_sub(1)) as f64).round() as usize;
    if track_pos < width {
        chars[track_pos] = '*';
    }

    chars.into_iter().collect()
}
