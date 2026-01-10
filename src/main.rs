mod audio;
mod input;
mod state;
mod ui;

use crate::audio::AudioPlugin;
use crate::input::InputPlugin;
use crate::state::{
    MidiFilePath, MidiTracks, PlaybackStatus, SoundFontPath, TracksFocus, UiState,
};
use crate::ui::UiPlugin;
use bevy::prelude::{
    default, App, DefaultPlugins, PluginGroup, Query, Startup, Window, WindowPlugin, With,
};
use bevy::window::PrimaryWindow;

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
        .add_systems(Startup, maximize_primary_window)
        .init_resource::<UiState>()
        .init_resource::<MidiTracks>()
        .init_resource::<MidiFilePath>()
        .init_resource::<SoundFontPath>()
        .init_resource::<PlaybackStatus>()
        .init_resource::<TracksFocus>()
        .add_plugins(AudioPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin)
        .run();
}

fn maximize_primary_window(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    window.set_maximized(true);
}
