mod state;
mod audio;
mod input;
mod ui;

use bevy::prelude::*;
use crate::state::*;
use crate::audio::AudioPlugin;
use crate::input::InputPlugin;
use crate::ui::UiPlugin;

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