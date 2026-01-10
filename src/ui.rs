use crate::state::{
    MidiFilePath, MidiTracks, PlaybackStatus, SoundFontPath, UiPage, UiSelection, UiState,
};
use bevy::prelude::{
    default, AlignItems, App, AssetServer, BackgroundColor, BorderColor, Camera2d, Color, Commands,
    Component, DetectChanges, Display, Entity, FlexDirection, JustifyContent, Node, Plugin, Query,
    Res, Startup, Text, TextColor, TextFont, UiRect, Update, Val, With, Without,
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

#[derive(Component)]
pub struct SplashPageRoot;

#[derive(Component)]
pub struct AboutPageRoot;

#[derive(Component)]
pub struct TracksPageRoot;

#[derive(Component)]
pub struct TracksList;

#[derive(Component)]
pub struct TrackRow;

#[derive(Component)]
pub struct TrackRowCell;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui).add_systems(
            Update,
            (
                update_page_visibility,
                update_tracks_list,
                update_selection_visuals,
            ),
        );
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
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 0.0, 0.5)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        display: Display::Flex,
                        ..default()
                    },
                    SplashPageRoot,
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

            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        display: Display::None,
                        ..default()
                    },
                    AboutPageRoot,
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
                                Text::new("Sona"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 50.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                            parent.spawn((
                                Text::new("Retro MIDI player built with Bevy + OxiSynth."),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 26.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            ));
                            parent.spawn((Node {
                                height: Val::Px(20.0),
                                ..default()
                            },));
                            parent.spawn((
                                Text::new("Controls:"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 28.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                            parent.spawn((
                                Text::new("Arrow keys to move, Enter to select."),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            ));
                            parent.spawn((
                                Text::new("P to play, S to stop."),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            ));
                            parent.spawn((Node {
                                height: Val::Px(20.0),
                                ..default()
                            },));
                            parent.spawn((
                                Text::new("Press ? to return to the splash page."),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                });

            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        display: Display::None,
                        ..default()
                    },
                    TracksPageRoot,
                ))
                .with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(20.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                row_gap: Val::Px(10.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.0, 0.0, 0.7)),
                            BorderColor::all(Color::WHITE),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Tracks"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 40.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                            parent.spawn((
                                Text::new("Press T to return to the splash page."),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 22.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            ));
                            parent.spawn((Node {
                                height: Val::Px(10.0),
                                ..default()
                            },));
                            parent
                                .spawn((Node {
                                    flex_direction: FlexDirection::Row,
                                    width: Val::Percent(100.0),
                                    column_gap: Val::Px(12.0),
                                    ..default()
                                },))
                                .with_children(|parent| {
                                    parent
                                        .spawn((Node {
                                            width: Val::Percent(35.0),
                                            ..default()
                                        },))
                                        .with_children(|parent| {
                                            parent.spawn((
                                                Text::new("Track"),
                                                TextFont {
                                                    font: font.clone(),
                                                    font_size: 22.0,
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });
                                    parent
                                        .spawn((Node {
                                            width: Val::Percent(15.0),
                                            ..default()
                                        },))
                                        .with_children(|parent| {
                                            parent.spawn((
                                                Text::new("Events"),
                                                TextFont {
                                                    font: font.clone(),
                                                    font_size: 22.0,
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });
                                    parent
                                        .spawn((Node {
                                            width: Val::Percent(50.0),
                                            ..default()
                                        },))
                                        .with_children(|parent| {
                                            parent.spawn((
                                                Text::new("Preview"),
                                                TextFont {
                                                    font: font.clone(),
                                                    font_size: 22.0,
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });
                                });
                            parent.spawn((
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    ..default()
                                },
                                TracksList,
                            ));
                        });
                });
        });
    println!("UI setup complete.");
}

fn update_page_visibility(
    ui_state: Res<UiState>,
    mut splash_query: Query<&mut Node, With<SplashPageRoot>>,
    mut about_query: Query<&mut Node, (With<AboutPageRoot>, Without<SplashPageRoot>)>,
    mut tracks_query: Query<
        &mut Node,
        (
            With<TracksPageRoot>,
            Without<SplashPageRoot>,
            Without<AboutPageRoot>,
        ),
    >,
) {
    let splash_display = if ui_state.page == UiPage::Splash {
        Display::Flex
    } else {
        Display::None
    };
    let about_display = if ui_state.page == UiPage::About {
        Display::Flex
    } else {
        Display::None
    };
    let tracks_display = if ui_state.page == UiPage::Tracks {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut splash_query {
        node.display = splash_display;
    }
    for mut node in &mut about_query {
        node.display = about_display;
    }
    for mut node in &mut tracks_query {
        node.display = tracks_display;
    }
}

fn update_tracks_list(
    midi_tracks: Res<MidiTracks>,
    mut commands: Commands,
    list_query: Query<Entity, With<TracksList>>,
    track_row_query: Query<Entity, With<TrackRow>>,
    track_row_cell_query: Query<Entity, With<TrackRowCell>>,
    asset_server: Res<AssetServer>,
) {
    if !midi_tracks.is_changed() && !track_row_query.is_empty() {
        return;
    }

    let font = asset_server.load("PixelifySans-Regular.ttf");

    let mut list_iter = list_query.iter();
    let Some(list_entity) = list_iter.next() else {
        return;
    };
    if list_iter.next().is_some() {
        return;
    }

    for row in &track_row_query {
        commands.entity(row).despawn();
    }
    for cell in &track_row_cell_query {
        commands.entity(cell).despawn();
    }

    commands.entity(list_entity).with_children(|parent| {
        if midi_tracks.0.is_empty() {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        width: Val::Percent(100.0),
                        ..default()
                    },
                    TrackRow,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("No tracks loaded."),
                        TextFont {
                            font: font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        TrackRowCell,
                    ));
                });
        } else {
            for track in &midi_tracks.0 {
                let name = track
                    .name
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("Unnamed");
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            width: Val::Percent(100.0),
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                        TrackRow,
                    ))
                    .with_children(|parent| {
                        parent
                            .spawn((Node {
                                width: Val::Percent(35.0),
                                ..default()
                            },
                            TrackRowCell,
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    Text::new(format!("[{:02}] {}", track.index + 1, name)),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 24.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    TrackRowCell,
                                ));
                            });
                        parent
                            .spawn((Node {
                                width: Val::Percent(15.0),
                                ..default()
                            },
                            TrackRowCell,
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    Text::new(track.event_count.to_string()),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 24.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    TrackRowCell,
                                ));
                            });
                        parent
                            .spawn((Node {
                                width: Val::Percent(50.0),
                                ..default()
                            },
                            TrackRowCell,
                            ))
                            .with_children(|parent| {
                                parent
                                    .spawn((Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(2.0),
                                        ..default()
                                    },
                                    TrackRowCell,
                                    ))
                                    .with_children(|parent| {
                                        for ch in track.preview.chars() {
                                            parent.spawn((
                                                Node {
                                                    width: Val::Px(6.0),
                                                    height: Val::Px(16.0),
                                                    ..default()
                                                },
                                                BackgroundColor(preview_color(ch)),
                                                TrackRowCell,
                                            ));
                                        }
                                    });
                            });
                    });
            }
        }
    });
}

fn preview_color(ch: char) -> Color {
    match ch {
        '#' => Color::srgb(1.0, 0.9, 0.3),
        '*' => Color::srgb(1.0, 1.0, 1.0),
        '|' => Color::srgb(0.4, 0.4, 0.6),
        _ => Color::srgb(0.15, 0.15, 0.25),
    }
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
    if ui_state.page != UiPage::Splash {
        return;
    }

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
