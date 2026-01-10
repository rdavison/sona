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
    pub note_count: usize,
    pub min_pitch: u8,
    pub max_pitch: u8,
    pub channels: Vec<u8>,
    pub programs: Vec<(u8, u8)>,
    pub banks: Vec<(u8, u8, u8)>,
    pub tempo_changes: usize,
    pub time_signature: Option<(u8, u8)>,
    pub key_signature: Option<(i8, bool)>,
    pub preview_width: usize,
    pub preview_height: usize,
    pub preview_cells: Vec<u16>,
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
