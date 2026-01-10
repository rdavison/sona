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
