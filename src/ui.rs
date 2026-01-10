use crate::state::{MidiFilePath, PlaybackStatus, SoundFontPath, UiSelection, UiState};
use bevy::prelude::{
    default, AlignItems, App, AssetServer, BackgroundColor, BorderColor, Camera2d, Color, Commands,
    Component, FlexDirection, JustifyContent, Node, Plugin, Query, Res, Startup, Text, TextColor,
    TextFont, UiRect, Update, Val, With, Without,
};

#[derive(Component)]
pub struct MidiFileText;

#[derive(Component)]
pub struct SoundFontText;

#[derive(Component)]
pub struct PlayButton;

#[derive(Component)]
pub struct StopButton;

#[derive(Component)]
pub struct RewindButton;

#[derive(Component)]
pub struct PlaybackStatusText;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(Update, update_selection_visuals);
    }
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("Setting up UI...");
    commands.spawn(Camera2d::default());

    let font = asset_server.load("PixelifySans-Regular.ttf");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 0.0, 0.5)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(20.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.0, 0.0, 0.7)),
                    BorderColor::all(Color::WHITE),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Status: Stopped"),
                        TextFont {
                            font: font.clone(),
                            font_size: 30.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        PlaybackStatusText,
                    ));

                    parent.spawn((Node {
                        height: Val::Px(20.0),
                        ..default()
                    },));

                    parent.spawn((
                        Text::new("MIDI File: [None]"),
                        TextFont {
                            font: font.clone(),
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        MidiFileText,
                    ));

                    parent.spawn((
                        Text::new("SoundFont: [None]"),
                        TextFont {
                            font: font.clone(),
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        SoundFontText,
                    ));

                    parent.spawn((Node {
                        height: Val::Px(20.0),
                        ..default()
                    },));

                    parent
                        .spawn((Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(20.0),
                            ..default()
                        },))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("[ Play ]"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 40.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                PlayButton,
                            ));
                            parent.spawn((
                                Text::new("[ Stop ]"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 40.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                StopButton,
                            ));
                            parent.spawn((
                                Text::new("[ Rewind ]"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 40.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                RewindButton,
                            ));
                        });
                });
        });
    println!("UI setup complete.");
}

fn update_selection_visuals(
    ui_state: Res<UiState>,
    midi_path: Res<MidiFilePath>,
    soundfont_path: Res<SoundFontPath>,
    playback_status: Res<PlaybackStatus>,
    mut midi_query: Query<
        (&mut TextColor, &mut Text),
        (
            With<MidiFileText>,
            Without<SoundFontText>,
            Without<PlayButton>,
            Without<StopButton>,
            Without<RewindButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut soundfont_query: Query<
        (&mut TextColor, &mut Text),
        (
            With<SoundFontText>,
            Without<MidiFileText>,
            Without<PlayButton>,
            Without<StopButton>,
            Without<RewindButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut play_query: Query<
        &mut TextColor,
        (
            With<PlayButton>,
            Without<MidiFileText>,
            Without<SoundFontText>,
            Without<StopButton>,
            Without<RewindButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut stop_query: Query<
        &mut TextColor,
        (
            With<StopButton>,
            Without<MidiFileText>,
            Without<SoundFontText>,
            Without<PlayButton>,
            Without<RewindButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut rewind_query: Query<
        &mut TextColor,
        (
            With<RewindButton>,
            Without<MidiFileText>,
            Without<SoundFontText>,
            Without<PlayButton>,
            Without<StopButton>,
            Without<PlaybackStatusText>,
        ),
    >,
    mut status_query: Query<
        &mut Text,
        (
            With<PlaybackStatusText>,
            Without<MidiFileText>,
            Without<SoundFontText>,
            Without<PlayButton>,
            Without<StopButton>,
            Without<RewindButton>,
        ),
    >,
) {
    let selected_color = Color::srgb(1.0, 1.0, 0.0);
    let default_color = Color::WHITE;

    for (mut color, mut text) in &mut midi_query {
        color.0 = if ui_state.selection == UiSelection::MidiFile {
            selected_color
        } else {
            default_color
        };
        if let Some(path) = &midi_path.0 {
            text.0 = format!("MIDI File: {}", path.file_name().unwrap().to_string_lossy());
        }
    }
    for (mut color, mut text) in &mut soundfont_query {
        color.0 = if ui_state.selection == UiSelection::SoundFont {
            selected_color
        } else {
            default_color
        };
        if let Some(path) = &soundfont_path.0 {
            text.0 = format!("SoundFont: {}", path.file_name().unwrap().to_string_lossy());
        }
    }
    for mut color in &mut play_query {
        color.0 = if ui_state.selection == UiSelection::Play {
            selected_color
        } else {
            default_color
        };
    }
    for mut color in &mut stop_query {
        color.0 = if ui_state.selection == UiSelection::Stop {
            selected_color
        } else {
            default_color
        };
    }
    for mut color in &mut rewind_query {
        color.0 = if ui_state.selection == UiSelection::Rewind {
            selected_color
        } else {
            default_color
        };
    }
    for mut text in &mut status_query {
        text.0 = format!("Status: {:?}", playback_status.state);
    }
}
