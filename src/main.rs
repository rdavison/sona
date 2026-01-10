mod audio;
mod input;
mod state;
mod ui;

use crate::audio::AudioPlugin;
use crate::input::InputPlugin;
use crate::state::{MidiFilePath, PlaybackStatus, SoundFontPath, UiState};
use crate::ui::UiPlugin;
use bevy::prelude::{default, App, DefaultPlugins, PluginGroup, Window, WindowPlugin};

fn main() {
    println!("Starting Sona...");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sona - Retro MIDI Player".to_string(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<UiState>()
        .init_resource::<MidiFilePath>()
        .init_resource::<SoundFontPath>()
        .init_resource::<PlaybackStatus>()
        .add_plugins(AudioPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin)
        .run();
}
