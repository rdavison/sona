use super::SplashPageRoot;
use crate::state::{
    MidiFilePath, PlaybackState, PlaybackStatus, SoundFontPath, UiPage, UiSelection, UiState,
};
use bevy::prelude::{
    default, AlignItems, BackgroundColor, BorderColor, Color, Commands, Component, Display, Entity,
    FlexDirection, Font, Handle, JustifyContent, Node, Query, Res, Text, TextColor, TextFont,
    UiRect, Val, With, Without,
};

#[derive(Component)]
pub(super) struct MidiFileText;

#[derive(Component)]
pub(super) struct SoundFontText;

#[derive(Component)]
pub(super) struct PlayButton;

#[derive(Component)]
pub(super) struct StopButton;

#[derive(Component)]
pub(super) struct RewindButton;

#[derive(Component)]
pub(super) struct PlaybackStatusText;

pub(super) fn spawn_splash_page(commands: &mut Commands, parent: Entity, font: Handle<Font>) {
    commands.entity(parent).with_children(|parent| {
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
    });
}

pub(super) fn update_selection_visuals(
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
