use super::{TracksPageRoot, UiFonts};
use crate::audio::AudioState;
use crate::state::{MidiTrackInfo, MidiTracks, TrackDetailsPopup, TracksFocus, UiPage, UiState};
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::Window;
use bevy::prelude::{
    default, AlignItems, Assets, BackgroundColor, BorderColor, ButtonInput, Changed, Children,
    Color, ColorToPacked, Commands, Component, ComputedNode, DetectChanges, Display, Entity,
    FlexDirection, Font, Handle, Image, ImageNode, JustifyContent, KeyCode, Node, NodeImageMode,
    Overflow, PositionType, Query, Res, ResMut, Resource, Text, TextColor, TextFont, UiRect, Val,
    With, ZIndex,
};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::UiGlobalTransform;
use bevy::window::PrimaryWindow;

#[derive(Component)]
pub(super) struct TracksList;

#[derive(Component)]
pub(super) struct TracksListViewport;

#[derive(Component)]
pub(super) struct TrackRow {
    index: usize,
}

#[derive(Component)]
pub(super) struct TrackRuler {
    image_entity: Entity,
}

#[derive(Component)]
pub(super) struct DebugOverlayText;

#[derive(Component)]
pub(super) struct DebugOverlayRoot;

#[derive(Component)]
pub(super) struct TrackDetailsPopupRoot;

#[derive(Component)]
pub(super) struct TrackDetailsField {
    field: TrackDetailsFieldKind,
}

#[derive(Clone, Copy)]
enum TrackDetailsFieldKind {
    Title,
    Index,
    Name,
    Events,
    EndTick,
    TicksPerBeat,
    NoteCount,
    PitchRange,
    Channels,
    Programs,
    Banks,
    TempoChanges,
    TimeSignature,
    KeySignature,
}

#[derive(Component)]
pub(super) struct TrackPreview {
    track_index: usize,
    image: Handle<Image>,
    last_size: (u32, u32),
}

#[derive(Resource, Default)]
pub(super) struct DebugOverlayState {
    visible: bool,
}

#[derive(Resource, Default)]
pub(super) struct TracksScroll {
    offset: f32,
}

const TRACK_COL_WIDTH: f32 = 220.0;
const EVENT_COL_WIDTH: f32 = 80.0;
const PREVIEW_CELL_SIZE: f32 = 2.0;
const TRACK_LABEL_FONT_SIZE: f32 = 24.0;

fn max_label_chars(column_width: f32, font_size: f32) -> usize {
    let avg_char_width = font_size * 0.6;
    if avg_char_width <= 0.0 {
        return 0;
    }
    (column_width / avg_char_width).floor().max(0.0) as usize
}

fn ellipsize_text(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let mut out: String = text.chars().take(max_chars - 3).collect();
    out.push_str("...");
    out
}

fn clamp_scroll_offset(current: f32, delta: f32, viewport_height: f32, content_height: f32) -> f32 {
    let max_offset = (content_height - viewport_height).max(0.0);
    (current + delta).clamp(0.0, max_offset)
}

fn pitch_range_label(min_pitch: u8, max_pitch: u8) -> String {
    format!("{} - {}", min_pitch, max_pitch)
}

fn channel_list_label(channels: &[u8]) -> String {
    if channels.is_empty() {
        return "-".to_string();
    }
    let list = channels
        .iter()
        .map(|ch| (ch + 1).to_string())
        .collect::<Vec<_>>()
        .join(", ");
    list
}

fn key_signature_label(signature: Option<(i8, bool)>) -> String {
    match signature {
        Some((sharps, minor)) => format!("{} {}", sharps, if minor { "minor" } else { "major" }),
        None => "-".to_string(),
    }
}

fn time_signature_label(signature: Option<(u8, u8)>) -> String {
    match signature {
        Some((num, denom)) => format!("{}/{}", num, denom),
        None => "-".to_string(),
    }
}

fn program_label(program: u8) -> String {
    const GM_NAMES: [&str; 128] = [
        "Acoustic Grand Piano",
        "Bright Acoustic Piano",
        "Electric Grand Piano",
        "Honky-tonk Piano",
        "Electric Piano 1",
        "Electric Piano 2",
        "Harpsichord",
        "Clavinet",
        "Celesta",
        "Glockenspiel",
        "Music Box",
        "Vibraphone",
        "Marimba",
        "Xylophone",
        "Tubular Bells",
        "Dulcimer",
        "Drawbar Organ",
        "Percussive Organ",
        "Rock Organ",
        "Church Organ",
        "Reed Organ",
        "Accordion",
        "Harmonica",
        "Tango Accordion",
        "Acoustic Guitar (nylon)",
        "Acoustic Guitar (steel)",
        "Electric Guitar (jazz)",
        "Electric Guitar (clean)",
        "Electric Guitar (muted)",
        "Overdriven Guitar",
        "Distortion Guitar",
        "Guitar Harmonics",
        "Acoustic Bass",
        "Electric Bass (finger)",
        "Electric Bass (pick)",
        "Fretless Bass",
        "Slap Bass 1",
        "Slap Bass 2",
        "Synth Bass 1",
        "Synth Bass 2",
        "Violin",
        "Viola",
        "Cello",
        "Contrabass",
        "Tremolo Strings",
        "Pizzicato Strings",
        "Orchestral Harp",
        "Timpani",
        "String Ensemble 1",
        "String Ensemble 2",
        "SynthStrings 1",
        "SynthStrings 2",
        "Choir Aahs",
        "Voice Oohs",
        "Synth Voice",
        "Orchestra Hit",
        "Trumpet",
        "Trombone",
        "Tuba",
        "Muted Trumpet",
        "French Horn",
        "Brass Section",
        "SynthBrass 1",
        "SynthBrass 2",
        "Soprano Sax",
        "Alto Sax",
        "Tenor Sax",
        "Baritone Sax",
        "Oboe",
        "English Horn",
        "Bassoon",
        "Clarinet",
        "Piccolo",
        "Flute",
        "Recorder",
        "Pan Flute",
        "Blown Bottle",
        "Shakuhachi",
        "Whistle",
        "Ocarina",
        "Lead 1 (square)",
        "Lead 2 (sawtooth)",
        "Lead 3 (calliope)",
        "Lead 4 (chiff)",
        "Lead 5 (charang)",
        "Lead 6 (voice)",
        "Lead 7 (fifths)",
        "Lead 8 (bass + lead)",
        "Pad 1 (new age)",
        "Pad 2 (warm)",
        "Pad 3 (polysynth)",
        "Pad 4 (choir)",
        "Pad 5 (bowed)",
        "Pad 6 (metallic)",
        "Pad 7 (halo)",
        "Pad 8 (sweep)",
        "FX 1 (rain)",
        "FX 2 (soundtrack)",
        "FX 3 (crystal)",
        "FX 4 (atmosphere)",
        "FX 5 (brightness)",
        "FX 6 (goblins)",
        "FX 7 (echoes)",
        "FX 8 (sci-fi)",
        "Sitar",
        "Banjo",
        "Shamisen",
        "Koto",
        "Kalimba",
        "Bagpipe",
        "Fiddle",
        "Shanai",
        "Tinkle Bell",
        "Agogo",
        "Steel Drums",
        "Woodblock",
        "Taiko Drum",
        "Melodic Tom",
        "Synth Drum",
        "Reverse Cymbal",
        "Guitar Fret Noise",
        "Breath Noise",
        "Seashore",
        "Bird Tweet",
        "Telephone Ring",
        "Helicopter",
        "Applause",
        "Gunshot",
    ];
    let name = GM_NAMES.get(program as usize).copied().unwrap_or("Unknown");
    format!("{} {}", program + 1, name)
}

fn programs_label(programs: &[(u8, u8)]) -> String {
    if programs.is_empty() {
        return "-".to_string();
    }
    let list = programs
        .iter()
        .map(|(channel, program)| format!("Ch{}: {}", channel + 1, program_label(*program)))
        .collect::<Vec<_>>()
        .join(", ");
    list
}

fn banks_label(banks: &[(u8, u8, u8)]) -> String {
    if banks.is_empty() {
        return "-".to_string();
    }
    let list = banks
        .iter()
        .map(|(channel, msb, lsb)| format!("Ch{}: {}/{}", channel + 1, msb, lsb))
        .collect::<Vec<_>>()
        .join(", ");
    list
}

pub(super) fn spawn_tracks_page(commands: &mut Commands, parent: Entity, font: Handle<Font>) {
    let _ = commands.entity(parent).with_children(|parent| {
        let _ = parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    justify_content: JustifyContent::FlexStart,
                    display: Display::None,
                    ..default()
                },
                TracksPageRoot,
            ))
            .with_children(|parent| {
                let _ = parent
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(16.0),
                            right: Val::Px(16.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.9, 0.2, 0.2)),
                        BorderColor::all(Color::WHITE),
                        ZIndex(10),
                        DebugOverlayRoot,
                    ))
                    .with_children(|parent| {
                        let _ = parent.spawn((
                            Text::new("Debug"),
                            TextFont {
                                font: font.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            DebugOverlayText,
                        ));
                    });
                let _ = parent
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Percent(20.0),
                            left: Val::Percent(20.0),
                            width: Val::Percent(60.0),
                            padding: UiRect::all(Val::Px(16.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            display: Display::None,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.05, 0.05, 0.2)),
                        BorderColor::all(Color::WHITE),
                        ZIndex(20),
                        TrackDetailsPopupRoot,
                    ))
                    .with_children(|parent| {
                        let _ = parent.spawn((
                            Text::new("Track Details"),
                            TextFont {
                                font: font.clone(),
                                font_size: 28.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::Title,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Index:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::Index,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Name:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::Name,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Events:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::Events,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("End tick:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::EndTick,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Ticks per beat:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::TicksPerBeat,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Notes:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::NoteCount,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Pitch range:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::PitchRange,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Channels:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::Channels,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Programs:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::Programs,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Banks:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::Banks,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Tempo changes:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::TempoChanges,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Time signature:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::TimeSignature,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Key signature:"),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TrackDetailsField {
                                field: TrackDetailsFieldKind::KeySignature,
                            },
                        ));
                        let _ = parent.spawn((
                            Text::new("Press Esc to close."),
                            TextFont {
                                font: font.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                    });
                let _ = parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            padding: UiRect::all(Val::Px(20.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            row_gap: Val::Px(10.0),
                            align_items: AlignItems::Stretch,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.0, 0.0, 0.7)),
                        BorderColor::all(Color::WHITE),
                    ))
                    .with_children(|parent| {
                        let _ = parent.spawn((
                            Text::new("Tracks"),
                            TextFont {
                                font: font.clone(),
                                font_size: 40.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        let _ = parent.spawn((
                            Text::new("Press T to return to the splash page."),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                        let _ = parent.spawn((
                            Text::new("Press P to open the piano roll."),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                        let _ = parent.spawn((Node {
                            height: Val::Px(10.0),
                            ..default()
                        },));
                        let _ = parent
                            .spawn((Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(12.0),
                                ..default()
                            },))
                            .with_children(|parent| {
                                let _ = parent
                                    .spawn((Node {
                                        width: Val::Px(TRACK_COL_WIDTH),
                                        ..default()
                                    },))
                                    .with_children(|parent| {
                                        let _ = parent.spawn((
                                            Text::new("Track"),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 22.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });
                                let _ = parent
                                    .spawn((Node {
                                        width: Val::Px(EVENT_COL_WIDTH),
                                        ..default()
                                    },))
                                    .with_children(|parent| {
                                        let _ = parent.spawn((
                                            Text::new("Events"),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 22.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });
                                let _ = parent
                                    .spawn((Node {
                                        flex_grow: 1.0,
                                        ..default()
                                    },))
                                    .with_children(|parent| {
                                        let _ = parent.spawn((
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
                        let _ = parent
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    flex_grow: 1.0,
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                TracksListViewport,
                            ))
                            .with_children(|parent| {
                                let _ = parent.spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(6.0),
                                        position_type: PositionType::Absolute,
                                        top: Val::Px(0.0),
                                        width: Val::Percent(100.0),
                                        ..default()
                                    },
                                    TracksList,
                                ));
                            });
                    });
            });
    });
}

pub(super) fn update_tracks_list(
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

    let _ = commands.entity(list_entity).with_children(|parent| {
        if midi_tracks.0.is_empty() {
            let _ = parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    TrackRow { index: 0 },
                ))
                .with_children(|parent| {
                    let _ = parent.spawn((
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
            for (row_index, track) in midi_tracks.0.iter().enumerate() {
                let name = track
                    .name
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("Unnamed");
                let label = format!("[{:02}] {}", track.index + 1, name);
                let max_chars = max_label_chars(TRACK_COL_WIDTH, TRACK_LABEL_FONT_SIZE);
                let label = ellipsize_text(&label, max_chars);
                let _ = parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        TrackRow { index: row_index },
                    ))
                    .with_children(|parent| {
                        let _ = parent
                            .spawn((Node {
                                width: Val::Px(TRACK_COL_WIDTH),
                                ..default()
                            },))
                            .with_children(|parent| {
                                let _ = parent.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: TRACK_LABEL_FONT_SIZE,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        let _ = parent
                            .spawn((Node {
                                width: Val::Px(EVENT_COL_WIDTH),
                                ..default()
                            },))
                            .with_children(|parent| {
                                let _ = parent.spawn((
                                    Text::new(track.event_count.to_string()),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 24.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        let width_px = (track.preview_width as f32 * PREVIEW_CELL_SIZE).round();
                        let height_px = (track.preview_height as f32 * PREVIEW_CELL_SIZE).round();
                        let width_px = width_px.max(1.0) as u32;
                        let height_px = height_px.max(1.0) as u32;
                        let image = build_track_preview_image_scaled(
                            track,
                            width_px,
                            height_px,
                            &mut images,
                        );
                        let _ = parent
                            .spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    flex_grow: 1.0,
                                    height: Val::Px(
                                        track.preview_height as f32 * PREVIEW_CELL_SIZE,
                                    ),
                                    position_type: PositionType::Relative,
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                TrackPreview {
                                    track_index: track.index,
                                    image: image.clone(),
                                    last_size: (width_px, height_px),
                                },
                            ))
                            .with_children(|parent| {
                                let image_entity = parent
                                    .spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(0.0),
                                            top: Val::Px(0.0),
                                            width: Val::Percent(100.0),
                                            height: Val::Percent(100.0),
                                            ..default()
                                        },
                                        ImageNode {
                                            image: image.clone(),
                                            image_mode: NodeImageMode::Stretch,
                                            ..default()
                                        },
                                    ))
                                    .id();
                                let _ = parent.spawn((
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
                                    TrackRuler { image_entity },
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

fn scale_preview_cells(
    cells: &[u16],
    src_width: usize,
    src_height: usize,
    dst_width: u32,
    dst_height: u32,
) -> Vec<u16> {
    let dst_width = dst_width.max(1) as usize;
    let dst_height = dst_height.max(1) as usize;
    let src_width = src_width.max(1);
    let src_height = src_height.max(1);
    let mut scaled = vec![0u16; dst_width * dst_height];

    for y in 0..dst_height {
        let src_y = (y * src_height) / dst_height;
        let row_offset = src_y * src_width;
        for x in 0..dst_width {
            let src_x = (x * src_width) / dst_width;
            let idx = row_offset + src_x;
            scaled[y * dst_width + x] = *cells.get(idx).unwrap_or(&0);
        }
    }

    scaled
}

fn render_preview_rgba(cells: &[u16], width: u32, height: u32) -> Vec<u8> {
    let width = width.max(1);
    let height = height.max(1);
    let mut data = vec![0u8; (width * height * 4) as usize];
    let base_color = preview_color(0).to_srgba().to_u8_array();
    for pixel in data.chunks_exact_mut(4) {
        pixel.copy_from_slice(&base_color);
    }

    for (idx, intensity) in cells.iter().enumerate() {
        let color = if *intensity == 0 {
            preview_color(0).to_srgba().to_u8_array()
        } else {
            preview_color(1).to_srgba().to_u8_array()
        };
        let offset = idx * 4;
        if offset + 4 <= data.len() {
            data[offset..offset + 4].copy_from_slice(&color);
        }
    }

    data
}

fn compute_ruler_left(ratio: f32, width_px: f32) -> f32 {
    let max_left = (width_px - 1.0).max(0.0);
    (ratio * width_px).min(max_left)
}

pub(super) fn update_track_ruler(
    ui_state: Res<UiState>,
    audio_state: Res<AudioState>,
    mut rulers: Query<(&mut Node, &TrackRuler)>,
    computed_nodes: Query<&ComputedNode>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if ui_state.page != UiPage::Tracks {
        return;
    }

    let ratio = audio_state.current_tick_ratio();
    let scale = windows
        .iter()
        .next()
        .map(|window| window.scale_factor() as f32)
        .unwrap_or(1.0);
    for (mut node, ruler) in &mut rulers {
        let Ok(image_node) = computed_nodes.get(ruler.image_entity) else {
            node.display = Display::None;
            continue;
        };

        let Some(ratio) = ratio else {
            node.display = Display::None;
            continue;
        };

        let width_px = image_node.size.x / scale.max(1.0);
        let left_px = compute_ruler_left(ratio, width_px);
        node.display = Display::Flex;
        node.left = Val::Px(left_px);
        node.height = Val::Px(image_node.size.y);
    }
}

pub(super) fn update_debug_overlay(
    ui_state: Res<UiState>,
    audio_state: Res<AudioState>,
    overlay_state: Res<DebugOverlayState>,
    mut query: Query<&mut Text, With<DebugOverlayText>>,
    rulers: Query<(Entity, &TrackRuler)>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut overlay_nodes: Query<&mut Node, With<DebugOverlayRoot>>,
) {
    if ui_state.page != UiPage::Tracks {
        return;
    }

    let show_overlay = overlay_state.visible;
    for mut node in &mut overlay_nodes {
        node.display = if show_overlay {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !show_overlay {
        return;
    }

    let debug = audio_state.debug_state();
    let ratio = audio_state.current_tick_ratio().unwrap_or(0.0);
    let mut image_left = None;
    let mut image_right = None;
    let mut ruler_x = None;
    let mut ruler_left = None;

    let window = windows.iter().next();
    let center_x = window.map(|w| w.resolution.width() * 0.5).unwrap_or(0.0);

    if let Some((ruler_entity, ruler)) = rulers.iter().next() {
        if let Ok((ruler_node, ruler_transform)) = nodes.get(ruler_entity) {
            let ruler_center = center_x + ruler_transform.translation.x;
            let ruler_half = ruler_node.size.x * 0.5;
            ruler_x = Some(ruler_center);
            ruler_left = Some(ruler_center - ruler_half);
        }
        if let Ok((image_node, image_transform)) = nodes.get(ruler.image_entity) {
            let image_center = center_x + image_transform.translation.x;
            let half = image_node.size.x * 0.5;
            let left = image_center - half;
            let right = image_center + half;
            image_left = Some(left);
            image_right = Some(right);
        }
    }

    for mut text in &mut query {
        text.0 = format!(
            "samples: {}/{}\nlast: {} -> {}\nnext: {} -> {}\nmax_tick: {}\nratio: {:.4}\nimg_x: {:?}..{:?}\nruler_x: {:?}\nruler_left: {:?}",
            debug.samples_played,
            debug.total_samples,
            debug.last_event_sample,
            debug.last_event_tick,
            debug.next_event_sample,
            debug.next_event_tick,
            debug.max_tick,
            ratio,
            image_left,
            image_right,
            ruler_x,
            ruler_left
        );
    }
}

pub(super) fn toggle_debug_overlay(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut overlay_state: ResMut<DebugOverlayState>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        overlay_state.visible = !overlay_state.visible;
    }
}

pub(super) fn update_tracks_focus_visuals(
    ui_state: Res<UiState>,
    tracks_focus: Res<TracksFocus>,
    midi_tracks: Res<MidiTracks>,
    mut rows: Query<(&TrackRow, &mut BackgroundColor)>,
) {
    if ui_state.page != UiPage::Tracks {
        return;
    }

    let focused = if midi_tracks.0.is_empty() {
        None
    } else {
        Some(
            tracks_focus
                .index
                .min(midi_tracks.0.len().saturating_sub(1)),
        )
    };

    for (row, mut bg) in &mut rows {
        let is_focused = focused.map_or(false, |index| row.index == index);
        bg.0 = if is_focused {
            Color::srgb(0.2, 0.3, 0.6)
        } else {
            Color::NONE
        };
    }
}

pub(super) fn update_track_previews(
    ui_state: Res<UiState>,
    midi_tracks: Res<MidiTracks>,
    mut previews: Query<(&ComputedNode, &mut TrackPreview, &mut ImageNode), Changed<ComputedNode>>,
    mut images: ResMut<Assets<Image>>,
) {
    if ui_state.page != UiPage::Tracks {
        return;
    }

    for (computed, mut preview, mut image_node) in &mut previews {
        let width_px = computed.size.x.round().max(1.0) as u32;
        let height_px = computed.size.y.round().max(1.0) as u32;
        if preview.last_size == (width_px, height_px) {
            continue;
        }

        let Some(track) = midi_tracks.0.get(preview.track_index) else {
            continue;
        };

        let new_handle = build_track_preview_image_scaled(track, width_px, height_px, &mut images);
        let old_handle = std::mem::replace(&mut preview.image, new_handle.clone());
        preview.last_size = (width_px, height_px);
        image_node.image = new_handle;
        if old_handle != preview.image {
            let _image = images.remove(old_handle.id());
        }
    }
}

pub(super) fn update_track_details_popup(
    ui_state: Res<UiState>,
    popup: Res<TrackDetailsPopup>,
    midi_tracks: Res<MidiTracks>,
    mut root_query: Query<&mut Node, With<TrackDetailsPopupRoot>>,
    mut fields: Query<(&TrackDetailsField, &mut Text)>,
) {
    if ui_state.page != UiPage::Tracks {
        for mut node in &mut root_query {
            node.display = Display::None;
        }
        return;
    }

    let show = popup.visible;
    for mut node in &mut root_query {
        node.display = if show { Display::Flex } else { Display::None };
    }
    if !show {
        return;
    }

    let track = midi_tracks.0.get(popup.track_index);
    for (field, mut text) in &mut fields {
        text.0 = match field.field {
            TrackDetailsFieldKind::Title => "Track Details".to_string(),
            TrackDetailsFieldKind::Index => track
                .map(|t| format!("Index: {}", t.index + 1))
                .unwrap_or_else(|| "Index: -".to_string()),
            TrackDetailsFieldKind::Name => track
                .and_then(|t| t.name.as_deref().filter(|name| !name.is_empty()))
                .map(|name| name.to_string())
                .map(|name| format!("Name: {}", name))
                .unwrap_or_else(|| "Name: Unnamed".to_string()),
            TrackDetailsFieldKind::Events => track
                .map(|t| format!("Events: {}", t.event_count))
                .unwrap_or_else(|| "Events: -".to_string()),
            TrackDetailsFieldKind::EndTick => track
                .map(|t| format!("End tick: {}", t.end_tick))
                .unwrap_or_else(|| "End tick: -".to_string()),
            TrackDetailsFieldKind::TicksPerBeat => track
                .map(|t| format!("Ticks per beat: {}", t.ticks_per_beat))
                .unwrap_or_else(|| "Ticks per beat: -".to_string()),
            TrackDetailsFieldKind::NoteCount => track
                .map(|t| format!("Notes: {}", t.note_count))
                .unwrap_or_else(|| "Notes: -".to_string()),
            TrackDetailsFieldKind::PitchRange => track
                .map(|t| {
                    format!(
                        "Pitch range: {}",
                        pitch_range_label(t.min_pitch, t.max_pitch)
                    )
                })
                .unwrap_or_else(|| "Pitch range: -".to_string()),
            TrackDetailsFieldKind::Channels => track
                .map(|t| format!("Channels: {}", channel_list_label(&t.channels)))
                .unwrap_or_else(|| "Channels: -".to_string()),
            TrackDetailsFieldKind::Programs => track
                .map(|t| format!("Programs: {}", programs_label(&t.programs)))
                .unwrap_or_else(|| "Programs: -".to_string()),
            TrackDetailsFieldKind::Banks => track
                .map(|t| format!("Banks: {}", banks_label(&t.banks)))
                .unwrap_or_else(|| "Banks: -".to_string()),
            TrackDetailsFieldKind::TempoChanges => track
                .map(|t| format!("Tempo changes: {}", t.tempo_changes))
                .unwrap_or_else(|| "Tempo changes: -".to_string()),
            TrackDetailsFieldKind::TimeSignature => track
                .map(|t| format!("Time signature: {}", time_signature_label(t.time_signature)))
                .unwrap_or_else(|| "Time signature: -".to_string()),
            TrackDetailsFieldKind::KeySignature => track
                .map(|t| format!("Key signature: {}", key_signature_label(t.key_signature)))
                .unwrap_or_else(|| "Key signature: -".to_string()),
        };
    }
}

pub(super) fn update_tracks_scroll(
    ui_state: Res<UiState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut scroll: ResMut<TracksScroll>,
    mut content_query: Query<&mut Node, With<TracksList>>,
    viewport_query: Query<&ComputedNode, With<TracksListViewport>>,
    content_size_query: Query<&ComputedNode, With<TracksList>>,
) {
    if ui_state.page != UiPage::Tracks {
        return;
    }

    let ctrl = keyboard_input.pressed(KeyCode::ControlLeft)
        || keyboard_input.pressed(KeyCode::ControlRight);
    if ctrl {
        let mut delta = 0.0;
        if keyboard_input.just_pressed(KeyCode::KeyE) {
            delta += 40.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyY) {
            delta -= 40.0;
        }
        if delta != 0.0 {
            let viewport_height = viewport_query
                .iter()
                .next()
                .map(|node| node.size.y)
                .unwrap_or(0.0);
            let content_height = content_size_query
                .iter()
                .next()
                .map(|node| node.size.y)
                .unwrap_or(0.0);
            scroll.offset =
                clamp_scroll_offset(scroll.offset, delta, viewport_height, content_height);
        }
    }

    for mut node in &mut content_query {
        node.top = Val::Px(-scroll.offset);
    }
}

fn build_track_preview_image_scaled(
    track: &MidiTrackInfo,
    width: u32,
    height: u32,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let width = width.max(1);
    let height = height.max(1);
    let scaled = scale_preview_cells(
        &track.preview_cells,
        track.preview_width,
        track.preview_height,
        width,
        height,
    );
    let data = render_preview_rgba(&scaled, width, height);

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
    let mut image = image;
    image.sampler = ImageSampler::nearest();

    images.add(image)
}

#[cfg(test)]
mod tests {
    use super::{
        banks_label, channel_list_label, clamp_scroll_offset, compute_ruler_left, ellipsize_text,
        key_signature_label, max_label_chars, pitch_range_label, preview_color, program_label,
        programs_label, render_preview_rgba, scale_preview_cells, time_signature_label,
    };
    use bevy::prelude::ColorToPacked;

    #[test]
    fn scale_preview_cells_expands_nearest() {
        let src = vec![1, 2, 3, 4];
        let scaled = scale_preview_cells(&src, 2, 2, 4, 4);
        assert_eq!(scaled.len(), 16);
        assert_eq!(scaled[0], 1);
        assert_eq!(scaled[3], 2);
        assert_eq!(scaled[12], 3);
        assert_eq!(scaled[15], 4);
    }

    #[test]
    fn render_preview_rgba_writes_colors() {
        let cells = vec![0u16, 1u16, 0u16, 1u16];
        let data = render_preview_rgba(&cells, 2, 2);
        assert_eq!(data.len(), 16);
        let off = preview_color(0).to_srgba().to_u8_array();
        let on = preview_color(1).to_srgba().to_u8_array();
        assert_eq!(&data[0..4], &off);
        assert_eq!(&data[4..8], &on);
    }

    #[test]
    fn compute_ruler_left_clamps() {
        assert_eq!(compute_ruler_left(0.5, 100.0), 50.0);
        assert_eq!(compute_ruler_left(2.0, 10.0), 9.0);
    }

    #[test]
    fn ellipsize_text_truncates() {
        assert_eq!(ellipsize_text("Hello", 10), "Hello");
        assert_eq!(ellipsize_text("Hello", 5), "Hello");
        assert_eq!(ellipsize_text("Hello", 4), "H...");
        assert_eq!(ellipsize_text("Hello", 3), "...");
        assert_eq!(ellipsize_text("Hello", 2), "..");
    }

    #[test]
    fn max_label_chars_scales_with_width() {
        let small = max_label_chars(50.0, 20.0);
        let large = max_label_chars(200.0, 20.0);
        assert!(large > small);
    }

    #[test]
    fn clamp_scroll_offset_bounds() {
        let offset = clamp_scroll_offset(0.0, 10.0, 100.0, 50.0);
        assert_eq!(offset, 0.0);
        let offset = clamp_scroll_offset(0.0, 10.0, 50.0, 100.0);
        assert_eq!(offset, 10.0);
        let offset = clamp_scroll_offset(90.0, 20.0, 50.0, 100.0);
        assert_eq!(offset, 50.0);
        let offset = clamp_scroll_offset(10.0, -20.0, 50.0, 100.0);
        assert_eq!(offset, 0.0);
    }

    #[test]
    fn pitch_range_label_formats() {
        assert_eq!(pitch_range_label(60, 72), "60 - 72");
    }

    #[test]
    fn channel_list_label_formats() {
        assert_eq!(channel_list_label(&[]), "-");
        assert_eq!(channel_list_label(&[0, 2, 9]), "1, 3, 10");
    }

    #[test]
    fn time_signature_label_formats() {
        assert_eq!(time_signature_label(None), "-");
        assert_eq!(time_signature_label(Some((4, 4))), "4/4");
    }

    #[test]
    fn key_signature_label_formats() {
        assert_eq!(key_signature_label(None), "-");
        assert_eq!(key_signature_label(Some((2, false))), "2 major");
        assert_eq!(key_signature_label(Some((-3, true))), "-3 minor");
    }

    #[test]
    fn program_label_formats() {
        assert!(program_label(0).contains("Acoustic Grand Piano"));
        assert!(program_label(40).contains("Violin"));
    }

    #[test]
    fn programs_label_formats() {
        assert_eq!(programs_label(&[]), "-");
        let label = programs_label(&[(0, 0), (1, 40)]);
        assert!(label.contains("Ch1"));
        assert!(label.contains("Ch2"));
    }

    #[test]
    fn banks_label_formats() {
        assert_eq!(banks_label(&[]), "-");
        assert_eq!(banks_label(&[(0, 1, 2)]), "Ch1: 1/2");
    }
}
