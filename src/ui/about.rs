use super::AboutPageRoot;
use bevy::prelude::{
    default, AlignItems, BackgroundColor, BorderColor, Color, Commands, Display, Entity,
    FlexDirection, Font, Handle, JustifyContent, Node, Text, TextColor, TextFont, UiRect, Val,
};

pub(super) fn spawn_about_page(commands: &mut Commands, parent: Entity, font: Handle<Font>) {
    let _ = commands.entity(parent).with_children(|parent| {
        let _ = parent
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
                let _ = parent
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
                        let _ = parent.spawn((
                            Text::new("Sona"),
                            TextFont {
                                font: font.clone(),
                                font_size: 50.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        let _ = parent.spawn((
                            Text::new("Retro MIDI player built with Bevy + OxiSynth."),
                            TextFont {
                                font: font.clone(),
                                font_size: 26.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                        let _ = parent.spawn((Node {
                            height: Val::Px(20.0),
                            ..default()
                        },));
                        let _ = parent.spawn((
                            Text::new("Controls:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 28.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        let _ = parent.spawn((
                            Text::new("Arrow keys to move, Enter to select."),
                            TextFont {
                                font: font.clone(),
                                font_size: 24.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                        let _ = parent.spawn((
                            Text::new("P to play/pause, S to stop."),
                            TextFont {
                                font: font.clone(),
                                font_size: 24.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                        let _ = parent.spawn((Node {
                            height: Val::Px(20.0),
                            ..default()
                        },));
                        let _ = parent.spawn((
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
    });
}
