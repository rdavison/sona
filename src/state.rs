use bevy::prelude::*;
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

#[derive(Resource, Default)]
pub struct UiState {
    pub selection: UiSelection,
}

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
