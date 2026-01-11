use crate::audio::{AudioCommand, AudioSender};
use crate::state::{
    MidiFilePath, MidiTrackInfo, MidiTracks, NoteSpan, PianoRollViewState, PlaybackState,
    PlaybackStatus, SoundFontPath, TrackDetailsPopup, TracksFocus, UiPage, UiSelection, UiState,
};
use bevy::prelude::{
    App, ButtonInput, Commands, Component, Entity, KeyCode, Plugin, Query, Res, ResMut, Resource,
    Startup, Update,
};
use bevy::tasks::IoTaskPool;
use futures_lite::future;
use midly::{MetaMessage, Smf, TrackEvent, TrackEventKind};
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
        self.bindings.get(action).and_then(|s| Self::of_str(s))
    }

    fn of_str(s: &str) -> Option<KeyCode> {
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

    pub fn load_from_conf(mut keybindings: ResMut<Keybindings>) {
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
}

#[derive(Component)]
pub struct FileDialogTask(pub bevy::tasks::Task<Option<PathBuf>>, pub UiSelection);

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Keybindings>()
            .add_systems(Startup, Keybindings::load_from_conf)
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
            UiSelection::Play | UiSelection::Stop | UiSelection::Rewind => ui_state.selection,
        };
    } else if keyboard_input.just_pressed(up) {
        println!("Key: Up");
        ui_state.selection = match ui_state.selection {
            UiSelection::SoundFont => UiSelection::MidiFile,
            UiSelection::Play | UiSelection::Stop | UiSelection::Rewind => UiSelection::SoundFont,
            UiSelection::MidiFile => ui_state.selection,
        };
    } else if keyboard_input.just_pressed(right) {
        println!("Key: Right");
        ui_state.selection = match ui_state.selection {
            UiSelection::Play => UiSelection::Stop,
            UiSelection::Stop => UiSelection::Rewind,
            UiSelection::MidiFile | UiSelection::SoundFont | UiSelection::Rewind => {
                ui_state.selection
            }
        };
    } else if keyboard_input.just_pressed(left) {
        println!("Key: Left");
        ui_state.selection = match ui_state.selection {
            UiSelection::Rewind => UiSelection::Stop,
            UiSelection::Stop => UiSelection::Play,
            UiSelection::MidiFile | UiSelection::SoundFont | UiSelection::Play => {
                ui_state.selection
            }
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
    mut tracks_focus: ResMut<TracksFocus>,
    midi_tracks: Res<MidiTracks>,
    mut track_popup: ResMut<TrackDetailsPopup>,
    mut piano_roll: ResMut<PianoRollViewState>,
) {
    if ui_state.page == UiPage::PianoRoll {
        if keyboard_input.just_pressed(KeyCode::Escape) {
            ui_state.page = UiPage::Tracks;
        }
        if let Some(track) = midi_tracks.0.get(tracks_focus.index) {
            let step_ticks = track.ticks_per_beat.max(1) as f32;
            let step_pitch = 12.0;
            if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
                piano_roll.offset_ticks -= step_ticks;
            }
            if keyboard_input.just_pressed(KeyCode::ArrowRight) {
                piano_roll.offset_ticks += step_ticks;
            }
            let shift = keyboard_input.pressed(KeyCode::ShiftLeft)
                || keyboard_input.pressed(KeyCode::ShiftRight);
            if shift {
                if keyboard_input.just_pressed(KeyCode::ArrowUp) {
                    piano_roll.zoom_y = (piano_roll.zoom_y * 1.25).min(16.0);
                }
                if keyboard_input.just_pressed(KeyCode::ArrowDown) {
                    piano_roll.zoom_y = (piano_roll.zoom_y / 1.25).max(1.0);
                }
            } else {
                if keyboard_input.just_pressed(KeyCode::ArrowUp) {
                    piano_roll.offset_pitch -= step_pitch;
                }
                if keyboard_input.just_pressed(KeyCode::ArrowDown) {
                    piano_roll.offset_pitch += step_pitch;
                }
            }
            if keyboard_input.just_pressed(KeyCode::Equal)
                || keyboard_input.just_pressed(KeyCode::NumpadAdd)
            {
                piano_roll.zoom_x = (piano_roll.zoom_x * 1.25).min(16.0);
            }
            if keyboard_input.just_pressed(KeyCode::Minus)
                || keyboard_input.just_pressed(KeyCode::NumpadSubtract)
            {
                piano_roll.zoom_x = (piano_roll.zoom_x / 1.25).max(1.0);
            }
        }
        return;
    }

    if ui_state.page == UiPage::Tracks && keyboard_input.just_pressed(KeyCode::KeyP) {
        ui_state.page = UiPage::PianoRoll;
        return;
    }

    let about_toggle = keyboard_input.just_pressed(KeyCode::Slash)
        && (keyboard_input.pressed(KeyCode::ShiftLeft)
            || keyboard_input.pressed(KeyCode::ShiftRight));
    if about_toggle {
        ui_state.page = match ui_state.page {
            UiPage::Splash => UiPage::About,
            UiPage::About => UiPage::Splash,
            UiPage::Tracks => UiPage::About,
            UiPage::PianoRoll => UiPage::About,
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
        if ui_state.page == UiPage::Tracks {
            tracks_focus.index = 0;
        }
        return;
    }

    if ui_state.page != UiPage::Splash {
        if ui_state.page == UiPage::Tracks {
            if keyboard_input.just_pressed(KeyCode::ArrowUp)
                || keyboard_input.just_pressed(KeyCode::ArrowDown)
            {
                let track_count = midi_tracks.0.len();
                if track_count == 0 {
                    return;
                }
                if keyboard_input.just_pressed(KeyCode::ArrowUp) {
                    tracks_focus.index = (tracks_focus.index + track_count - 1) % track_count;
                } else {
                    tracks_focus.index = (tracks_focus.index + 1) % track_count;
                }
            }
            if keyboard_input.just_pressed(KeyCode::Escape) {
                track_popup.visible = false;
            }
            if keyboard_input.just_pressed(KeyCode::Enter) {
                let track_count = midi_tracks.0.len();
                if track_count == 0 {
                    return;
                }
                track_popup.visible = true;
                track_popup.track_index = tracks_focus.index.min(track_count.saturating_sub(1));
            }
            if keyboard_input.just_pressed(KeyCode::Space) {
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
        }
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
            UiSelection::Play => match playback_status.state {
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
            },
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

pub(crate) fn load_midi_tracks(path: &PathBuf) -> Vec<MidiTrackInfo> {
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

    parse_midi_tracks(&smf)
}

struct TrackParse {
    name: Option<String>,
    event_count: usize,
    end_tick: u64,
    spans: Vec<NoteSpan>,
    note_end_tick: u64,
    channels: Vec<u8>,
    programs: Vec<(u8, u8)>,
    banks: Vec<(u8, u8, u8)>,
    tempo_changes: usize,
    time_signature: Option<(u8, u8)>,
    key_signature: Option<(i8, bool)>,
}

fn parse_track(track: &[TrackEvent<'_>]) -> TrackParse {
    let mut current_tick = 0u64;
    let mut last_tick = 0u64;
    let mut spans = Vec::new();
    let mut active_notes: Vec<Vec<u64>> = vec![Vec::new(); 128];
    let mut channels = std::collections::BTreeSet::new();
    let mut programs = std::collections::BTreeMap::new();
    let mut banks = std::collections::BTreeMap::<u8, (Option<u8>, Option<u8>)>::new();
    let mut tempo_changes = 0usize;
    let mut time_signature = None;
    let mut key_signature = None;
    let name = track.iter().find_map(|event| match event.kind {
        TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
            Some(String::from_utf8_lossy(name).to_string())
        }
        _ => None,
    });

    for event in track.iter() {
        current_tick += event.delta.as_int() as u64;
        last_tick = current_tick;
        match event.kind {
            TrackEventKind::Midi { channel, message } => {
                let channel = channel.as_int() as u8;
                channels.insert(channel);
                match message {
                    midly::MidiMessage::NoteOn { key, vel } => {
                        if vel.as_int() > 0 {
                            active_notes[key.as_int() as usize].push(current_tick);
                        } else if let Some(start) = active_notes[key.as_int() as usize].pop() {
                            spans.push(NoteSpan {
                                pitch: key.as_int() as u8,
                                start,
                                end: current_tick,
                            });
                        }
                    }
                    midly::MidiMessage::NoteOff { key, .. } => {
                        if let Some(start) = active_notes[key.as_int() as usize].pop() {
                            spans.push(NoteSpan {
                                pitch: key.as_int() as u8,
                                start,
                                end: current_tick,
                            });
                        }
                    }
                    midly::MidiMessage::ProgramChange { program } => {
                        programs.insert(channel, program.as_int() as u8);
                    }
                    midly::MidiMessage::Controller { controller, value } => {
                        let ctrl = controller.as_int() as u8;
                        if ctrl == 0 || ctrl == 32 {
                            let entry = banks.entry(channel).or_insert((None, None));
                            if ctrl == 0 {
                                entry.0 = Some(value.as_int() as u8);
                            } else {
                                entry.1 = Some(value.as_int() as u8);
                            }
                        }
                    }
                    _ => {}
                }
            }
            TrackEventKind::Meta(MetaMessage::Tempo(_)) => {
                tempo_changes += 1;
            }
            TrackEventKind::Meta(MetaMessage::TimeSignature(num, denom, _, _)) => {
                time_signature = Some((num, 2u8.pow(denom as u32)));
            }
            TrackEventKind::Meta(MetaMessage::KeySignature(sharps, is_minor)) => {
                key_signature = Some((sharps, is_minor));
            }
            _ => {}
        }
    }

    for (pitch, starts) in active_notes.iter_mut().enumerate() {
        for start in starts.drain(..) {
            spans.push(NoteSpan {
                pitch: pitch as u8,
                start,
                end: last_tick,
            });
        }
    }

    let note_end_tick = spans.iter().map(|span| span.end).max().unwrap_or(0);

    let programs = programs.into_iter().collect();
    let banks = banks
        .into_iter()
        .filter_map(|(channel, (msb, lsb))| match (msb, lsb) {
            (None, None) => None,
            (msb, lsb) => Some((channel, msb.unwrap_or(0), lsb.unwrap_or(0))),
        })
        .collect();

    TrackParse {
        name,
        event_count: track.len(),
        end_tick: last_tick,
        spans,
        note_end_tick,
        channels: channels.into_iter().collect(),
        programs,
        banks,
        tempo_changes,
        time_signature,
        key_signature,
    }
}

fn parse_midi_tracks(smf: &Smf) -> Vec<MidiTrackInfo> {
    let ticks_per_beat = match smf.header.timing {
        midly::Timing::Metrical(ticks) => ticks.as_int() as u32,
        _ => 480,
    }
    .max(1);
    let mut track_spans: Vec<Vec<NoteSpan>> = Vec::new();
    let mut track_info: Vec<TrackInfo> = Vec::new();
    let mut max_tick = 0u64;
    let mut max_note_tick = 0u64;

    for (index, track) in smf.tracks.iter().enumerate() {
        let parsed = parse_track(track);
        if parsed.note_end_tick > 0 {
            max_note_tick = max_note_tick.max(parsed.note_end_tick);
        }
        max_tick = max_tick.max(parsed.end_tick);
        track_spans.push(parsed.spans);
        track_info.push(TrackInfo {
            index,
            name: parsed.name,
            event_count: parsed.event_count,
            end_tick: parsed.end_tick,
            channels: parsed.channels,
            programs: parsed.programs,
            banks: parsed.banks,
            tempo_changes: parsed.tempo_changes,
            time_signature: parsed.time_signature,
            key_signature: parsed.key_signature,
        });
    }

    let preview_height = 64usize;
    let max_preview_width = 240usize;
    let ruler_max_tick = if max_note_tick > 0 {
        max_note_tick
    } else {
        max_tick
    };
    let ticks_per_column = ticks_per_column_for_width(ruler_max_tick, max_preview_width);
    let preview_width = (ruler_max_tick / ticks_per_column) as usize + 1;
    track_info
        .into_iter()
        .zip(track_spans.into_iter())
        .map(|(info, spans)| {
            let (min_pitch, max_pitch) = note_range(&spans);
            let note_count = spans.len();
            let preview_cells = build_track_preview(
                preview_width,
                preview_height,
                ticks_per_column,
                ruler_max_tick,
                info.end_tick,
                min_pitch,
                max_pitch,
                &spans,
            );
            MidiTrackInfo {
                index: info.index,
                name: info.name,
                event_count: info.event_count,
                end_tick: info.end_tick,
                ticks_per_beat,
                note_count,
                min_pitch,
                max_pitch,
                channels: info.channels,
                programs: info.programs,
                banks: info.banks,
                tempo_changes: info.tempo_changes,
                time_signature: info.time_signature,
                key_signature: info.key_signature,
                note_spans: spans,
                preview_width,
                preview_height,
                preview_cells,
            }
        })
        .collect()
}

struct TrackInfo {
    index: usize,
    name: Option<String>,
    event_count: usize,
    end_tick: u64,
    channels: Vec<u8>,
    programs: Vec<(u8, u8)>,
    banks: Vec<(u8, u8, u8)>,
    tempo_changes: usize,
    time_signature: Option<(u8, u8)>,
    key_signature: Option<(i8, bool)>,
}

fn note_range(spans: &[NoteSpan]) -> (u8, u8) {
    let mut min_pitch = 127u8;
    let mut max_pitch = 0u8;
    for span in spans {
        min_pitch = min_pitch.min(span.pitch);
        max_pitch = max_pitch.max(span.pitch);
    }
    if spans.is_empty() {
        (60, 60)
    } else {
        (min_pitch, max_pitch)
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
    min_pitch: u8,
    max_pitch: u8,
    spans: &[NoteSpan],
) -> Vec<u16> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let mut cells = vec![0u16; width * height];
    let max_tick = max_tick.max(1);
    let _ = track_end;
    let _ = max_tick;

    for span in spans {
        let pitch = span.pitch;
        let start = span.start;
        let end = span.end;
        let start_col = (start / ticks_per_column) as usize;
        let end_col = (end / ticks_per_column) as usize;
        let row = pitch_to_row_range(height, min_pitch, max_pitch, pitch);
        let row_offset = row * width;
        let end_col = end_col.min(width.saturating_sub(1));
        for col in start_col..=end_col {
            let idx = row_offset + col;
            if let Some(cell) = cells.get_mut(idx) {
                *cell = cell.saturating_add(1);
            }
        }
    }

    cells
}

fn pitch_to_row_range(height: usize, min_pitch: u8, max_pitch: u8, pitch: u8) -> usize {
    if height == 0 {
        return 0;
    }
    let padding = ((height as f32) * 0.08).round() as usize;
    let padding = padding.min(height.saturating_sub(1) / 2);
    let usable_height = height.saturating_sub(padding * 2).max(1);
    if min_pitch >= max_pitch {
        return padding + ((usable_height - 1) as f32 / 2.0).round() as usize;
    }
    let span = (max_pitch - min_pitch) as f32;
    let t = (max_pitch.saturating_sub(pitch) as f32) / span;
    let row = t * (usable_height as f32 - 1.0);
    (padding as f32 + row)
        .round()
        .clamp(0.0, (height - 1) as f32) as usize
}

#[cfg(test)]
mod tests {
    use super::{
        build_track_preview, note_range, parse_midi_tracks, parse_track, pitch_to_row_range,
        str_to_keycode, ticks_per_column_for_width,
    };
    use crate::state::MidiTrackInfo;
    use crate::state::NoteSpan;
    use midly::{Format, Smf, Timing, TrackEvent, TrackEventKind};

    #[test]
    fn str_to_keycode_handles_known_keys() {
        assert_eq!(str_to_keycode("up"), Some(bevy::prelude::KeyCode::ArrowUp));
        assert_eq!(str_to_keycode("P"), Some(bevy::prelude::KeyCode::KeyP));
        assert_eq!(str_to_keycode("unknown"), None);
    }

    #[test]
    fn parse_track_collects_spans_and_name() {
        let mut track = Vec::new();
        track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(midly::MetaMessage::TrackName(b"Test")),
        });
        track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Midi {
                channel: 1.into(),
                message: midly::MidiMessage::ProgramChange { program: 40.into() },
            },
        });
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
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: midly::MidiMessage::NoteOff {
                    key: 60.into(),
                    vel: 0.into(),
                },
            },
        });

        let parsed = parse_track(&track);
        assert_eq!(parsed.name.as_deref(), Some("Test"));
        assert_eq!(parsed.spans.len(), 1);
        assert_eq!(parsed.event_count, 4);
        assert_eq!(parsed.end_tick, 120);
        assert!(parsed.channels.contains(&0));
        assert!(parsed.channels.contains(&1));
        assert_eq!(parsed.programs, vec![(1, 40)]);
    }

    #[test]
    fn parse_midi_tracks_builds_track_info() {
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

        let tracks = parse_midi_tracks(&smf);
        assert_eq!(tracks.len(), 1);
        let MidiTrackInfo {
            preview_width,
            preview_height,
            preview_cells,
            end_tick,
            ticks_per_beat,
            note_count,
            min_pitch,
            max_pitch,
            channels,
            programs,
            banks,
            tempo_changes,
            time_signature,
            key_signature,
            note_spans,
            ..
        } = &tracks[0];
        assert_eq!(*preview_height, 64);
        assert!(preview_width > &0);
        assert_eq!(preview_cells.len(), preview_width * preview_height);
        assert_eq!(*end_tick, 120);
        assert_eq!(*ticks_per_beat, 480);
        assert_eq!(*note_count, 1);
        assert_eq!(*min_pitch, 60);
        assert_eq!(*max_pitch, 60);
        assert_eq!(channels.as_slice(), &[0]);
        assert!(programs.is_empty());
        assert!(banks.is_empty());
        assert_eq!(*tempo_changes, 0);
        assert!(time_signature.is_none());
        assert!(key_signature.is_none());
        assert_eq!(note_spans.len(), 1);
    }

    #[test]
    fn note_range_defaults_for_empty() {
        assert_eq!(note_range(&[]), (60, 60));
    }

    #[test]
    fn ticks_per_column_nonzero() {
        assert_eq!(ticks_per_column_for_width(0, 0), 1);
        assert_eq!(ticks_per_column_for_width(100, 1), 100);
        assert!(ticks_per_column_for_width(100, 10) > 0);
    }

    #[test]
    fn build_track_preview_marks_cells() {
        let spans = vec![NoteSpan {
            pitch: 60,
            start: 0,
            end: 10,
        }];
        let cells = build_track_preview(4, 4, 5, 10, 10, 60, 60, &spans);
        assert_eq!(cells.len(), 16);
        assert!(cells.iter().any(|cell| *cell > 0));
    }

    #[test]
    fn pitch_to_row_range_within_bounds() {
        let row = pitch_to_row_range(10, 40, 80, 60);
        assert!(row < 10);
    }
}
