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
                match playback_status.state {
                    PlaybackState::Playing => {
                        playback_status.state = PlaybackState::Paused;
                        let _ = audio_tx.0.send(AudioCommand::Pause);
                    }
                    PlaybackState::Paused | PlaybackState::Stopped => {
                        if let (Some(midi), Some(sf)) = (&midi_path.0, &soundfont_path.0) {
                            playback_status.state = PlaybackState::Playing;
                            let _ = audio_tx
                                .0
                                .send(AudioCommand::Play(midi.clone(), sf.clone()));
                        }
                    }
                }
            }
            UiSelection::Stop => {
                playback_status.state = PlaybackState::Stopped;
                let _ = audio_tx.0.send(AudioCommand::Stop);
            }
            UiSelection::Rewind => {
                let _ = audio_tx.0.send(AudioCommand::Rewind);
            }
        }
    }

    if keyboard_input.just_pressed(play_key) {
        match playback_status.state {
            PlaybackState::Playing => {
                playback_status.state = PlaybackState::Paused;
                let _ = audio_tx.0.send(AudioCommand::Pause);
            }
            PlaybackState::Paused | PlaybackState::Stopped => {
                if let (Some(midi), Some(sf)) = (&midi_path.0, &soundfont_path.0) {
                    playback_status.state = PlaybackState::Playing;
                    let _ = audio_tx
                        .0
                        .send(AudioCommand::Play(midi.clone(), sf.clone()));
                }
            }
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

    let mut track_spans: Vec<Vec<(u8, u64, u64)>> = Vec::new();
    let mut track_info: Vec<(usize, Option<String>, usize, u64)> = Vec::new();
    let mut max_tick = 0u64;

    for (index, track) in smf.tracks.iter().enumerate() {
        let mut current_tick = 0u64;
        let mut last_tick = 0u64;
        let mut spans = Vec::new();
        let mut active_notes: Vec<Vec<u64>> = vec![Vec::new(); 128];
        let name = track.iter().find_map(|event| match event.kind {
            TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                Some(String::from_utf8_lossy(name).to_string())
            }
            _ => None,
        });

        for event in track.iter() {
            current_tick += event.delta.as_int() as u64;
            last_tick = current_tick;
            if let TrackEventKind::Midi { message, .. } = event.kind {
                match message {
                    midly::MidiMessage::NoteOn { key, vel } => {
                        if vel.as_int() > 0 {
                            active_notes[key.as_int() as usize].push(current_tick);
                        } else if let Some(start) = active_notes[key.as_int() as usize].pop() {
                            spans.push((key.as_int() as u8, start, current_tick));
                        }
                    }
                    midly::MidiMessage::NoteOff { key, .. } => {
                        if let Some(start) = active_notes[key.as_int() as usize].pop() {
                            spans.push((key.as_int() as u8, start, current_tick));
                        }
                    }
                    _ => {}
                }
            }
        }

        for (pitch, starts) in active_notes.iter_mut().enumerate() {
            for start in starts.drain(..) {
                spans.push((pitch as u8, start, last_tick));
            }
        }

        max_tick = max_tick.max(last_tick);
        track_spans.push(spans);
        track_info.push((index, name, track.len(), last_tick));
    }

    let preview_height = 64usize;
    let max_preview_width = 240usize;
    let ticks_per_column = ticks_per_column_for_width(max_tick, max_preview_width);
    let preview_width = (max_tick / ticks_per_column) as usize + 1;
    track_info
        .into_iter()
        .zip(track_spans.into_iter())
        .map(|((index, name, event_count, track_end), spans)| {
            let center = duration_weighted_mean_pitch(&spans);
            MidiTrackInfo {
                index,
                name,
                event_count,
                preview_width,
                preview_height,
                preview_cells: build_track_preview(
                    preview_width,
                    preview_height,
                    ticks_per_column,
                    max_tick,
                    track_end,
                    center,
                    &spans,
                ),
            }
        })
        .collect()
}

fn duration_weighted_mean_pitch(spans: &[(u8, u64, u64)]) -> f32 {
    let mut weighted_sum = 0.0f64;
    let mut total = 0.0f64;

    for &(pitch, start, end) in spans {
        let mut duration = end.saturating_sub(start);
        if duration == 0 {
            duration = 1;
        }
        weighted_sum += pitch as f64 * duration as f64;
        total += duration as f64;
    }

    if total > 0.0 {
        (weighted_sum / total) as f32
    } else {
        60.0
    }
}

fn ticks_per_column_for_width(max_tick: u64, max_width: usize) -> u64 {
    if max_width == 0 {
        return 1;
    }
    let denom = max_width.saturating_sub(1).max(1) as u64;
    let mut ticks_per_column = (max_tick + denom - 1) / denom;
    if ticks_per_column == 0 {
        ticks_per_column = 1;
    }
    ticks_per_column
}

fn build_track_preview(
    width: usize,
    height: usize,
    ticks_per_column: u64,
    max_tick: u64,
    track_end: u64,
    center_pitch: f32,
    spans: &[(u8, u64, u64)],
) -> Vec<u16> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let mut cells = vec![0u16; width * height];
    let max_tick = max_tick.max(1);
    let _ = track_end;
    let _ = max_tick;

    for &(pitch, start, end) in spans {
        let start_col = (start / ticks_per_column) as usize;
        let end_col = (end / ticks_per_column) as usize;
        let row = pitch_to_row(height, center_pitch, pitch);
        let row_offset = row * width;
        let end_col = end_col.min(width.saturating_sub(1));
        for col in start_col..=end_col {
            let idx = row_offset + col;
            if let Some(cell) = cells.get_mut(idx) {
                *cell = cell.saturating_add(1);
            }
        }
    }

    for col in (0..width).step_by(32) {
        for row in 0..height {
            let idx = row * width + col;
            if let Some(cell) = cells.get_mut(idx) {
                *cell = (*cell).max(1);
            }
        }
    }

    cells
}

fn pitch_to_row(height: usize, center_pitch: f32, pitch: u8) -> usize {
    if height == 0 {
        return 0;
    }
    let half = (height as f32 - 1.0) / 2.0;
    let scale = 128.0 / height as f32;
    let row = (half - (pitch as f32 - center_pitch) / scale).round();
    row.clamp(0.0, (height - 1) as f32) as usize
}
