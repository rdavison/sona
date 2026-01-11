use bevy::prelude::Resource;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiSelection {
    #[default]
    MidiFile,
    SoundFont,
    Play,
    Stop,
    Rewind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiPage {
    #[default]
    Splash,
    About,
    Tracks,
    PianoRoll,
}

#[derive(Resource, Default)]
pub struct UiState {
    pub selection: UiSelection,
    pub page: UiPage,
}

#[derive(Debug, Clone)]
pub struct MidiTrackInfo {
    pub index: usize,
    pub name: Option<String>,
    pub event_count: usize,
    pub end_tick: u64,
    pub ticks_per_beat: u32,
    pub note_count: usize,
    pub min_pitch: u8,
    pub max_pitch: u8,
    pub channels: Vec<u8>,
    pub programs: Vec<(u8, u8)>,
    pub banks: Vec<(u8, u8, u8)>,
    pub tempo_changes: usize,
    pub time_signature: Option<(u8, u8)>,
    pub key_signature: Option<(i8, bool)>,
    pub note_spans: Vec<NoteSpan>,
    pub preview_width: usize,
    pub preview_height: usize,
    pub preview_cells: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct NoteSpan {
    pub pitch: u8,
    pub start: u64,
    pub end: u64,
}

#[derive(Resource, Default)]
pub struct MidiTracks(pub Vec<MidiTrackInfo>);

#[derive(Resource, Default)]
pub struct MidiFilePath(pub Option<PathBuf>);

#[derive(Resource, Default)]
pub struct SoundFontPath(pub Option<PathBuf>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[derive(Resource, Default)]
pub struct PlaybackStatus {
    pub state: PlaybackState,
}

#[derive(Resource, Default)]
pub struct TracksFocus {
    pub index: usize,
}

#[derive(Resource, Default)]
pub struct TrackDetailsPopup {
    pub visible: bool,
    pub track_index: usize,
}

#[derive(Resource)]
pub struct PianoRollViewState {
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub offset_ticks: f32,
    pub offset_pitch: f32,
}

impl Default for PianoRollViewState {
    fn default() -> Self {
        Self {
            zoom_x: 1.0,
            zoom_y: 1.0,
            offset_ticks: 0.0,
            offset_pitch: 0.0,
        }
    }
}
