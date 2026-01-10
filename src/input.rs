use crate::audio::{AudioCommand, AudioSender};
use crate::state::{
    MidiFilePath, PlaybackState, PlaybackStatus, SoundFontPath, UiPage, UiSelection, UiState,
};
use bevy::prelude::{
    App, ButtonInput, Commands, Component, Entity, KeyCode, Plugin, Query, Res, ResMut, Resource,
    Startup, Update,
};
use bevy::tasks::IoTaskPool;
use futures_lite::future;
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
    if ui_state.page == UiPage::About {
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
        };
        return;
    }

    if ui_state.page == UiPage::About {
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
) {
    for (entity, mut task) in &mut tasks {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            println!("File dialog result received.");
            if let Some(path) = result {
                match task.1 {
                    UiSelection::MidiFile => midi_path.0 = Some(path),
                    UiSelection::SoundFont => soundfont_path.0 = Some(path),
                    _ => {}
                }
            }
            commands.entity(entity).despawn();
        }
    }
}
