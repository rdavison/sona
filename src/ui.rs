use crate::audio::AudioState;
use crate::state::{
    MidiFilePath, MidiTrackInfo, MidiTracks, PlaybackState, PlaybackStatus, SoundFontPath, UiPage,
    UiSelection, UiState,
};
use bevy::prelude::{
    default, AlignItems, App, AssetServer, Assets, BackgroundColor, BorderColor, Camera2d, Children,
    Color, Commands, Component, DetectChanges, Display, Entity, FlexDirection, Font, Handle,
    ColorToPacked, Image, ImageNode, JustifyContent, Node, Plugin, PositionType, Query, Res,
    ResMut, Resource, Startup, Text, TextColor, TextFont, UiRect, Update, Val, With, Without,
    ZIndex, Overflow,
};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::asset::RenderAssetUsages;
use std::sync::atomic::Ordering;

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
pub struct TrackRuler {
    preview_width: usize,
    preview_height: usize,
}

#[derive(Resource)]
struct UiFonts {
    main: Handle<Font>,
}

const TRACK_COL_WIDTH: f32 = 220.0;
const EVENT_COL_WIDTH: f32 = 80.0;
const PREVIEW_CELL_SIZE: f32 = 2.0;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui).add_systems(
            Update,
            (
                update_page_visibility,
                update_tracks_list,
                update_track_ruler,
                update_selection_visuals,
            ),
        );
    }
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("Setting up UI...");
    commands.spawn(Camera2d::default());

    let font = asset_server.load("PixelifySans-Regular.ttf");
    commands.insert_resource(UiFonts { main: font.clone() });

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
                                Text::new("P to play/pause, S to stop."),
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
                                    column_gap: Val::Px(12.0),
                                    ..default()
                                },))
                                .with_children(|parent| {
                                    parent
                                        .spawn((Node {
                                            width: Val::Px(TRACK_COL_WIDTH),
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
                                            width: Val::Px(EVENT_COL_WIDTH),
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
                                        .spawn((Node { ..default() },))
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
    children_query: Query<&Children>,
    fonts: Res<UiFonts>,
    mut images: ResMut<Assets<Image>>,
) {
    if !midi_tracks.is_changed() && !track_row_query.is_empty() {
        return;
    }

    let font = fonts.main.clone();

    let mut list_iter = list_query.iter();
    let Some(list_entity) = list_iter.next() else {
        return;
    };
    if list_iter.next().is_some() {
        return;
    }

    let mut descendants = Vec::new();
    for row in &track_row_query {
        collect_descendants(row, &children_query, &mut descendants);
        for entity in descendants.drain(..) {
            commands.entity(entity).despawn();
        }
        commands.entity(row).despawn();
    }

    commands.entity(list_entity).with_children(|parent| {
        if midi_tracks.0.is_empty() {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
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
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                        TrackRow,
                    ))
                    .with_children(|parent| {
                        parent
                            .spawn((Node {
                                width: Val::Px(TRACK_COL_WIDTH),
                                ..default()
                            },))
                            .with_children(|parent| {
                                parent.spawn((
                                    Text::new(format!("[{:02}] {}", track.index + 1, name)),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 24.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        parent
                            .spawn((Node {
                                width: Val::Px(EVENT_COL_WIDTH),
                                ..default()
                            },))
                            .with_children(|parent| {
                                parent.spawn((
                                    Text::new(track.event_count.to_string()),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 24.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        parent
                            .spawn((Node {
                                width: Val::Px(track.preview_width as f32 * PREVIEW_CELL_SIZE),
                                height: Val::Px(track.preview_height as f32 * PREVIEW_CELL_SIZE),
                                position_type: PositionType::Relative,
                                overflow: Overflow::clip(),
                                ..default()
                            },))
                            .with_children(|parent| {
                                parent.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    ImageNode::new(build_track_preview_image(track, &mut images)),
                                ));
                                parent.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        width: Val::Px(2.0),
                                        height: Val::Px(
                                            track.preview_height as f32 * PREVIEW_CELL_SIZE,
                                        ),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(1.0, 1.0, 1.0)),
                                    ZIndex(1),
                                    TrackRuler {
                                        preview_width: track.preview_width,
                                        preview_height: track.preview_height,
                                    },
                                ));
                            });
                    });
            }
        }
    });
}

fn collect_descendants(entity: Entity, children_query: &Query<&Children>, out: &mut Vec<Entity>) {
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter() {
        collect_descendants(*child, children_query, out);
        out.push(*child);
    }
}

fn preview_color(intensity: u16) -> Color {
    if intensity == 0 {
        return Color::srgb(0.15, 0.15, 0.25);
    }
    let level = (intensity as f32).min(6.0);
    let bright = 0.25 + level * 0.12;
    Color::srgb(bright, bright * 0.9, 0.2 + level * 0.08)
}

fn update_track_ruler(
    ui_state: Res<UiState>,
    audio_state: Res<AudioState>,
    mut rulers: Query<(&mut Node, &TrackRuler)>,
) {
    if ui_state.page != UiPage::Tracks {
        return;
    }

    let total_samples = audio_state.total_samples.load(Ordering::Relaxed);
    let samples_played = audio_state.samples_played.load(Ordering::Relaxed);

    for (mut node, ruler) in &mut rulers {
        if total_samples == 0 {
            node.display = Display::None;
            continue;
        }

        let width_px = ruler.preview_width as f32 * PREVIEW_CELL_SIZE;
        let ratio = (samples_played as f32 / total_samples as f32).clamp(0.0, 1.0);
        let max_left = (width_px - 1.0).max(0.0);
        let left_px = (ratio * width_px).min(max_left);
        node.display = Display::Flex;
        node.left = Val::Px(left_px);
        node.height = Val::Px(ruler.preview_height as f32 * PREVIEW_CELL_SIZE);
    }
}

fn build_track_preview_image(track: &MidiTrackInfo, images: &mut Assets<Image>) -> Handle<Image> {
    let width = track.preview_width.max(1) as u32;
    let height = track.preview_height.max(1) as u32;
    let mut data = vec![0u8; (width * height * 4) as usize];
    let base_color = preview_color(0).to_srgba().to_u8_array();
    for pixel in data.chunks_exact_mut(4) {
        pixel.copy_from_slice(&base_color);
    }

    let max_cells = (width as usize) * (height as usize);
    for (idx, intensity) in track.preview_cells.iter().take(max_cells).enumerate() {
        let color = preview_color(*intensity).to_srgba().to_u8_array();
        let offset = idx * 4;
        data[offset..offset + 4].copy_from_slice(&color);
    }

    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    images.add(image)
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
        (&mut TextColor, &mut Text),
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
    for (mut color, mut text) in &mut play_query {
        color.0 = if ui_state.selection == UiSelection::Play {
            selected_color
        } else {
            default_color
        };
        text.0 = if playback_status.state == PlaybackState::Playing {
            "[ Pause ]".to_string()
        } else {
            "[ Play ]".to_string()
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
