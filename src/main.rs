mod audio;
mod input;
mod state;
mod ui;

use crate::audio::AudioPlugin;
use crate::input::{load_midi_tracks, InputPlugin};
use crate::state::{
    MidiFilePath, MidiTracks, PianoRollViewState, PlaybackStatus, SoundFontPath, TrackDetailsPopup,
    TracksFocus, UiState,
};
use crate::ui::UiPlugin;
use bevy::prelude::{
    default, App, DefaultPlugins, PluginGroup, Query, Startup, Window, WindowPlugin, With,
};
use bevy::window::PrimaryWindow;
use clap::Parser;
use std::path::PathBuf;

fn main() {
    println!("Starting Sona...");
    let cli = CliArgs::parse();
    let original_midi = cli.midi.clone();
    let original_soundfont = cli.soundfont.clone();
    let cli = validate_cli_paths_with(cli.midi, cli.soundfont, |path| path.is_file());
    if original_midi.is_some() && cli.midi.is_none() {
        eprintln!("MIDI file not found: {}", original_midi.unwrap().display());
    }
    if original_soundfont.is_some() && cli.soundfont.is_none() {
        eprintln!(
            "SoundFont file not found: {}",
            original_soundfont.unwrap().display()
        );
    }
    let midi_tracks = cli.midi.as_ref().map(load_midi_tracks).unwrap_or_default();

    let start_on_tracks = cli.midi.is_some() && cli.soundfont.is_some();
    let mut ui_state = UiState::default();
    if start_on_tracks {
        ui_state.page = crate::state::UiPage::Tracks;
    }

    let _app = App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sona - Retro MIDI Player".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, maximize_primary_window)
        .insert_resource(ui_state)
        .insert_resource(MidiTracks(midi_tracks))
        .insert_resource(MidiFilePath(cli.midi))
        .insert_resource(SoundFontPath(cli.soundfont))
        .init_resource::<PlaybackStatus>()
        .init_resource::<TrackDetailsPopup>()
        .init_resource::<PianoRollViewState>()
        .init_resource::<TracksFocus>()
        .add_plugins(AudioPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin)
        .run();
}

#[derive(Parser)]
#[command(
    name = "sona",
    version,
    about = "Retro MIDI player built with Bevy + OxiSynth"
)]
struct CliArgs {
    #[arg(short, long)]
    midi: Option<PathBuf>,
    #[arg(short, long)]
    soundfont: Option<PathBuf>,
}

fn validate_cli_paths_with<F>(
    midi: Option<PathBuf>,
    soundfont: Option<PathBuf>,
    exists: F,
) -> CliArgs
where
    F: Fn(&PathBuf) -> bool,
{
    let midi = midi.filter(|path| exists(path));
    let soundfont = soundfont.filter(|path| exists(path));
    CliArgs { midi, soundfont }
}

fn maximize_primary_window(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    window.set_maximized(true);
}

#[cfg(test)]
mod tests {
    use super::{validate_cli_paths_with, CliArgs};
    use clap::Parser;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn parse_cli_args_reads_paths() {
        let args = vec!["sona", "--midi", "song.mid", "--soundfont", "piano.sf2"];
        let parsed = CliArgs::try_parse_from(args).expect("parse args");
        assert_eq!(parsed.midi.unwrap().to_string_lossy(), "song.mid");
        assert_eq!(parsed.soundfont.unwrap().to_string_lossy(), "piano.sf2");
    }

    #[test]
    fn parse_cli_args_short_flags() {
        let args = vec!["sona", "-m", "song.mid"];
        let parsed = CliArgs::try_parse_from(args).expect("parse args");
        assert_eq!(parsed.midi.unwrap().to_string_lossy(), "song.mid");
        assert!(parsed.soundfont.is_none());
    }

    #[test]
    fn start_on_tracks_when_both_paths_present() {
        let args = vec!["sona", "--midi", "song.mid", "--soundfont", "piano.sf2"];
        let parsed = CliArgs::try_parse_from(args).expect("parse args");
        assert!(parsed.midi.is_some());
        assert!(parsed.soundfont.is_some());
    }

    #[test]
    fn validate_cli_paths_with_filters_missing() {
        let valid = HashSet::from([PathBuf::from("song.mid"), PathBuf::from("piano.sf2")]);
        let result = validate_cli_paths_with(
            Some(PathBuf::from("song.mid")),
            Some(PathBuf::from("missing.sf2")),
            |path| valid.contains(path),
        );
        assert!(result.midi.is_some());
        assert!(result.soundfont.is_none());
    }
}
