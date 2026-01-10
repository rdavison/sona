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
}

#[derive(Resource, Default)]
pub struct UiState {
    pub selection: UiSelection,
    pub page: UiPage,
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
