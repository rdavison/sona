use super::PianoRollPageRoot;
use crate::audio::AudioState;
use crate::state::{MidiTracks, PianoRollViewState, TracksFocus, UiPage, UiState};
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::{
    default, AlignItems, Assets, BackgroundColor, BorderColor, Children, Color, ColorToPacked,
    Commands, Component, ComputedNode, DetectChanges, Display, Entity, FlexDirection, Font, Handle,
    Image, ImageNode, JustifyContent, Node, NodeImageMode, Overflow, PositionType, Query, Res,
    ResMut, Text, TextColor, TextFont, UiRect, Val, With,
};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[derive(Component)]
pub(super) struct PianoRollView {
    track_index: usize,
    image: Handle<Image>,
    last_size: (u32, u32),
}

const MAX_TEXTURE_SIZE: u32 = 16_384;

#[derive(Component)]
pub(super) struct PianoRollRuler {
    image_entity: bevy::prelude::Entity,
}

#[derive(Component)]
pub(super) struct PianoRollLabelsRoot {
    start: u8,
    end: u8,
    height: u32,
}

#[derive(Component)]
pub(super) struct PianoRollLabel;

const PIANO_BACKGROUND_COLOR: Color = Color::srgb(0.06, 0.06, 0.12);
const PIANO_NOTE_COLOR: Color = Color::srgb(0.95, 0.9, 0.25);

// TODO: instead of rendering pitch names, render a piano keyboard (white + black keys)
// and just label the octaves
fn note_name(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (pitch / 12) as i32 - 1;
    let name = NAMES[(pitch % 12) as usize];
    format!("{name}{octave}")
}

fn pitch_list(start: u8, end: u8) -> Vec<u8> {
    if end < start {
        return Vec::new();
    }
    (start..=end).rev().collect()
}

fn note_cell_band(height: u32, pitch_start: u8, pitch_end: u8, pitch: u8) -> (u32, u32) {
    if height == 0 || pitch_end < pitch_start {
        return (0, 0);
    }
    let pitch_count = (pitch_end - pitch_start + 1) as f32;
    let row_height = (height as f32 / pitch_count).max(1.0);
    let index = pitch_end.saturating_sub(pitch) as f32;
    let start = (index * row_height).floor() as u32;
    let end = ((index + 1.0) * row_height - 1.0)
        .ceil()
        .max(0.0)
        .min(height.saturating_sub(1) as f32) as u32;
    (start.min(height.saturating_sub(1)), end)
}

fn piano_grid_color() -> Color {
    Color::srgb(0.12, 0.12, 0.2)
}

fn piano_grid_major_color() -> Color {
    Color::srgb(0.18, 0.18, 0.28)
}

fn compute_visible_ticks(end_tick: u64, zoom_x: f32) -> f32 {
    let zoom = zoom_x.max(1.0);
    (end_tick.max(1) as f32 / zoom).max(1.0)
}

fn clamp_offset_ticks(offset: f32, end_tick: u64, zoom_x: f32) -> f32 {
    let visible = compute_visible_ticks(end_tick, zoom_x);
    let max_offset = (end_tick.max(1) as f32 - visible).max(0.0);
    offset.clamp(0.0, max_offset)
}

fn ruler_left_px(
    tick: u64,
    track_end: u64,
    view: &PianoRollViewState,
    width_px: f32,
) -> Option<f32> {
    if width_px <= 1.0 {
        return None;
    }
    let visible_ticks = compute_visible_ticks(track_end, view.zoom_x);
    let offset_ticks = clamp_offset_ticks(view.offset_ticks, track_end, view.zoom_x);
    let tick = tick as f32;
    if tick < offset_ticks || tick > offset_ticks + visible_ticks {
        return None;
    }
    let ratio = ((tick - offset_ticks) / visible_ticks).clamp(0.0, 1.0);
    let max_left = (width_px - 1.0).max(0.0);
    Some((ratio * width_px).min(max_left))
}

fn compute_visible_pitch_range(min_pitch: u8, max_pitch: u8, zoom_y: f32) -> f32 {
    let span = (max_pitch.saturating_sub(min_pitch).max(1) + 1) as f32;
    (span / zoom_y.max(1.0)).max(1.0)
}

fn clamp_offset_pitch(offset: f32, min_pitch: u8, max_pitch: u8, zoom_y: f32) -> f32 {
    let span = (max_pitch.saturating_sub(min_pitch).max(1) + 1) as f32;
    let visible = compute_visible_pitch_range(min_pitch, max_pitch, zoom_y);
    let max_offset = (span - visible).max(0.0);
    offset.clamp(0.0, max_offset)
}

fn visible_pitch_bounds(
    track: &crate::state::MidiTrackInfo,
    view: &PianoRollViewState,
) -> (u8, u8) {
    let visible_pitch = compute_visible_pitch_range(track.min_pitch, track.max_pitch, view.zoom_y);
    let offset_pitch = clamp_offset_pitch(
        view.offset_pitch,
        track.min_pitch,
        track.max_pitch,
        view.zoom_y,
    );
    let pitch_start = track.min_pitch as f32 + offset_pitch;
    let pitch_end = pitch_start + visible_pitch;
    let start_u8 = pitch_start.round().clamp(0.0, 127.0) as u8;
    let end_u8 = pitch_end
        .round()
        .clamp(start_u8 as f32, track.max_pitch as f32) as u8;
    (start_u8, end_u8)
}

fn should_rebuild_labels(root: &PianoRollLabelsRoot, start: u8, end: u8, height: u32) -> bool {
    let height_diff = root.height.max(height) - root.height.min(height);
    root.start != start || root.end != end || height_diff > 1
}

fn build_empty_piano_roll_data(width: u32, height: u32) -> Vec<u8> {
    let width = width.max(1);
    let height = height.max(1);
    let mut data = vec![0u8; (width * height * 4) as usize];
    let bg = PIANO_BACKGROUND_COLOR.to_srgba().to_u8_array();
    for pixel in data.chunks_exact_mut(4) {
        pixel.copy_from_slice(&bg);
    }
    data
}

fn pitch_to_row(height: u32, min_pitch: u8, max_pitch: u8, pitch: u8) -> u32 {
    if height <= 1 {
        return 0;
    }
    if min_pitch >= max_pitch {
        return height - 1;
    }
    let span = (max_pitch - min_pitch) as f32;
    let t = (max_pitch.saturating_sub(pitch) as f32) / span;
    (t * (height as f32 - 1.0))
        .round()
        .clamp(0.0, height as f32 - 1.0) as u32
}

fn build_piano_roll_data(
    track: &crate::state::MidiTrackInfo,
    width: u32,
    height: u32,
    view: &PianoRollViewState,
) -> Vec<u8> {
    let width = width.max(1);
    let height = height.max(1);
    let mut data = build_empty_piano_roll_data(width, height);

    let visible_ticks = compute_visible_ticks(track.end_tick, view.zoom_x);
    let offset_ticks = clamp_offset_ticks(view.offset_ticks, track.end_tick, view.zoom_x);
    let (pitch_start_u8, pitch_end_u8) = visible_pitch_bounds(track, view);
    let pitch_start = pitch_start_u8 as f32;
    let pitch_end = pitch_end_u8 as f32;

    let grid_color = piano_grid_color().to_srgba().to_u8_array();
    let grid_major = piano_grid_major_color().to_srgba().to_u8_array();
    let ticks_per_beat = track.ticks_per_beat.max(1) as f32;
    let beat_start = (offset_ticks / ticks_per_beat).floor() as i64;
    let beat_end = ((offset_ticks + visible_ticks) / ticks_per_beat).ceil() as i64;
    for beat in beat_start..=beat_end {
        let tick = beat as f32 * ticks_per_beat;
        let x = (((tick - offset_ticks) / visible_ticks) * (width as f32 - 1.0))
            .round()
            .clamp(0.0, width as f32 - 1.0) as u32;
        let color = if beat % 4 == 0 {
            grid_major
        } else {
            grid_color
        };
        for y in 0..height {
            let idx = ((y * width + x) * 4) as usize;
            if idx + 4 <= data.len() {
                data[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }

    let min_pitch = track.min_pitch as i32;
    let max_pitch = track.max_pitch as i32;
    for pitch in min_pitch..=max_pitch {
        if (pitch as f32) < pitch_start || (pitch as f32) > pitch_end {
            continue;
        }
        let row = pitch_to_row(height, pitch_start_u8, pitch_end_u8, pitch as u8);
        let color = if pitch % 12 == 0 {
            grid_major
        } else {
            grid_color
        };
        for x in 0..width {
            let idx = ((row * width + x) * 4) as usize;
            if idx + 4 <= data.len() {
                data[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }

    let note_color = PIANO_NOTE_COLOR.to_srgba().to_u8_array();
    for span in &track.note_spans {
        if (span.end as f32) < offset_ticks || (span.start as f32) > offset_ticks + visible_ticks {
            continue;
        }
        if (span.pitch as f32) < pitch_start || (span.pitch as f32) > pitch_end {
            continue;
        }
        let x0 = (((span.start as f32 - offset_ticks) / visible_ticks) * (width as f32 - 1.0))
            .round()
            .clamp(0.0, width as f32 - 1.0) as u32;
        let x1 = (((span.end as f32 - offset_ticks) / visible_ticks) * (width as f32 - 1.0))
            .round()
            .clamp(0.0, width as f32 - 1.0) as u32;
        let (row_start, row_end) = note_cell_band(height, pitch_start_u8, pitch_end_u8, span.pitch);
        let start = x0.min(width - 1);
        let end = x1.min(width - 1);
        for y in row_start..=row_end {
            for x in start..=end {
                let idx = ((y * width + x) * 4) as usize;
                if idx + 4 <= data.len() {
                    data[idx..idx + 4].copy_from_slice(&note_color);
                }
            }
        }
    }

    data
}

fn build_piano_roll_image(
    track: &crate::state::MidiTrackInfo,
    width: u32,
    height: u32,
    images: &mut Assets<Image>,
    view: &PianoRollViewState,
) -> Handle<Image> {
    let data = build_piano_roll_data(track, width, height, view);
    let image = Image::new(
        Extent3d {
            width: width.max(1),
            height: height.max(1),
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

pub(super) fn spawn_piano_roll_page(commands: &mut Commands, parent: Entity, font: Handle<Font>) {
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
                PianoRollPageRoot,
            ))
            .with_children(|parent| {
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
                            Text::new("Piano Roll"),
                            TextFont {
                                font: font.clone(),
                                font_size: 40.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        let _ = parent.spawn((
                            Text::new("Press Esc to return to the tracks page."),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                        let _ = parent.spawn((
                            Text::new("Arrows pan, +/- zoom time, Shift+Up/Down zoom pitch."),
                            TextFont {
                                font: font.clone(),
                                font_size: 20.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.7, 0.7, 0.8)),
                        ));
                        let _ = parent
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    flex_grow: 1.0,
                                    align_items: AlignItems::Stretch,
                                    position_type: PositionType::Relative,
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                BackgroundColor(PIANO_BACKGROUND_COLOR),
                            ))
                            .with_children(|parent| {
                                let _ = parent.spawn((
                                    Node {
                                        width: Val::Px(70.0),
                                        height: Val::Percent(100.0),
                                        flex_direction: FlexDirection::Column,
                                        overflow: Overflow::clip(),
                                        flex_grow: 0.0,
                                        flex_shrink: 0.0,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.04, 0.04, 0.08)),
                                    PianoRollLabelsRoot {
                                        start: 0,
                                        end: 0,
                                        height: 0,
                                    },
                                ));

                                let _ = parent
                                    .spawn((Node {
                                        flex_grow: 1.0,
                                        height: Val::Percent(100.0),
                                        position_type: PositionType::Relative,
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },))
                                    .with_children(|parent| {
                                        let handle = Handle::default();
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
                                                    image: handle.clone(),
                                                    image_mode: NodeImageMode::Stretch,
                                                    ..default()
                                                },
                                                PianoRollView {
                                                    track_index: usize::MAX,
                                                    image: handle,
                                                    last_size: (0, 0),
                                                },
                                            ))
                                            .id();
                                        let _ = parent.spawn((
                                            Node {
                                                position_type: PositionType::Absolute,
                                                left: Val::Px(0.0),
                                                top: Val::Px(0.0),
                                                width: Val::Px(2.0),
                                                height: Val::Percent(100.0),
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgb(1.0, 1.0, 1.0)),
                                            PianoRollRuler { image_entity },
                                        ));
                                    });
                            });
                    });
            });
    });
}

pub(super) fn update_piano_roll_view(
    ui_state: Res<UiState>,
    tracks_focus: Res<TracksFocus>,
    midi_tracks: Res<MidiTracks>,
    view_state: Res<PianoRollViewState>,
    mut views: Query<(&ComputedNode, &mut PianoRollView, &mut ImageNode)>,
    mut images: ResMut<Assets<Image>>,
) {
    if ui_state.page != UiPage::PianoRoll {
        return;
    }

    let track_index = tracks_focus.index;
    let track = midi_tracks.0.get(track_index);
    for (node, mut view, mut image_node) in &mut views {
        let width = node.size.x.round().max(1.0) as u32;
        let height = node.size.y.round().max(1.0) as u32;
        let width = width.min(MAX_TEXTURE_SIZE);
        let height = height.min(MAX_TEXTURE_SIZE);
        let size_changed = view.last_size != (width, height);
        let track_changed = view.track_index != track_index;
        if !size_changed && !track_changed && !midi_tracks.is_changed() && !view_state.is_changed()
        {
            continue;
        }

        let new_handle = if let Some(track) = track {
            build_piano_roll_image(track, width, height, &mut images, &view_state)
        } else {
            let data = build_empty_piano_roll_data(width, height);
            let image = Image::new(
                Extent3d {
                    width: width.max(1),
                    height: height.max(1),
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
        };

        let old_handle = std::mem::replace(&mut view.image, new_handle.clone());
        view.last_size = (width, height);
        view.track_index = track_index;
        image_node.image = new_handle;
        if old_handle != view.image && images.get(old_handle.id()).is_some() {
            let _image = images.remove(old_handle.id());
        }
    }
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

pub(super) fn update_piano_roll_labels(
    ui_state: Res<UiState>,
    tracks_focus: Res<TracksFocus>,
    midi_tracks: Res<MidiTracks>,
    view_state: Res<PianoRollViewState>,
    mut commands: Commands,
    mut roots: Query<(Entity, &mut PianoRollLabelsRoot, &ComputedNode, &Children)>,
    label_nodes: Query<(Entity, &Children), With<PianoRollLabel>>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text>,
    children_query: Query<&Children>,
    fonts: Res<super::UiFonts>,
) {
    if ui_state.page != UiPage::PianoRoll {
        return;
    }
    let Some(track) = midi_tracks.0.get(tracks_focus.index) else {
        return;
    };
    let (start_pitch, end_pitch) = visible_pitch_bounds(track, &view_state);

    for (root_entity, mut root, node, root_children) in &mut roots {
        let height = node.size.y.round().max(1.0) as u32;
        let pitches = pitch_list(start_pitch, end_pitch);
        if pitches.is_empty() {
            continue;
        }

        let label_entities: Vec<Entity> = root_children
            .iter()
            .filter(|child| label_nodes.get(**child).is_ok())
            .copied()
            .collect();
        let pitch_count = pitches.len() as f32;
        let row_height = (height as f32 / pitch_count).max(1.0);
        if !should_rebuild_labels(&root, start_pitch, end_pitch, height)
            && label_entities.len() == pitches.len()
        {
            for (label_entity, pitch) in label_entities.iter().zip(pitches.iter()) {
                if let Ok(mut node) = nodes.get_mut(*label_entity) {
                    node.height = Val::Px(row_height);
                }
                if let Ok((_, children)) = label_nodes.get(*label_entity) {
                    if let Some(text_entity) =
                        children.iter().find(|child| texts.get_mut(**child).is_ok())
                    {
                        if let Ok(mut text) = texts.get_mut(*text_entity) {
                            text.0 = note_name(*pitch);
                        }
                    }
                }
            }
        } else {
            let mut descendants = Vec::new();
            collect_descendants(root_entity, &children_query, &mut descendants);
            for entity in descendants {
                commands.entity(entity).despawn();
            }

            let _ = commands.entity(root_entity).with_children(|parent| {
                for pitch in pitches {
                    let _ = parent
                        .spawn((
                            Node {
                                height: Val::Px(row_height),
                                padding: UiRect::left(Val::Px(6.0)),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            PianoRollLabel,
                        ))
                        .with_children(|parent| {
                            let _ = parent.spawn((
                                Text::new(note_name(pitch)),
                                TextFont {
                                    font: fonts.main.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                }
            });
        }

        root.start = start_pitch;
        root.end = end_pitch;
        root.height = height;
    }
}

pub(super) fn update_piano_roll_ruler(
    ui_state: Res<UiState>,
    audio_state: Res<AudioState>,
    midi_tracks: Res<MidiTracks>,
    tracks_focus: Res<TracksFocus>,
    view_state: Res<PianoRollViewState>,
    mut rulers: Query<(&mut Node, &PianoRollRuler)>,
    computed_nodes: Query<&ComputedNode>,
) {
    if ui_state.page != UiPage::PianoRoll {
        return;
    }

    let Some(tick) = audio_state.current_tick() else {
        for (mut node, _) in &mut rulers {
            node.display = Display::None;
        }
        return;
    };
    let Some(track) = midi_tracks.0.get(tracks_focus.index) else {
        for (mut node, _) in &mut rulers {
            node.display = Display::None;
        }
        return;
    };

    for (mut node, ruler) in &mut rulers {
        let Ok(image_node) = computed_nodes.get(ruler.image_entity) else {
            node.display = Display::None;
            continue;
        };
        let Some(left_px) = ruler_left_px(tick, track.end_tick, &view_state, image_node.size.x)
        else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = Val::Px(left_px);
        node.height = Val::Px(image_node.size.y);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_empty_piano_roll_data, build_piano_roll_data, clamp_offset_pitch, clamp_offset_ticks,
        compute_visible_pitch_range, compute_visible_ticks, note_cell_band, note_name, pitch_list,
        pitch_to_row, ruler_left_px, should_rebuild_labels, visible_pitch_bounds,
        PianoRollLabelsRoot,
    };
    use crate::state::{MidiTrackInfo, NoteSpan, PianoRollViewState};

    #[test]
    fn pitch_to_row_maps_bounds() {
        assert_eq!(pitch_to_row(10, 60, 72, 72), 0);
        assert_eq!(pitch_to_row(10, 60, 72, 60), 9);
    }

    #[test]
    fn build_piano_roll_data_draws_notes() {
        let view = PianoRollViewState::default();
        let track = MidiTrackInfo {
            index: 0,
            name: None,
            event_count: 0,
            end_tick: 100,
            ticks_per_beat: 10,
            note_count: 1,
            min_pitch: 60,
            max_pitch: 60,
            channels: vec![0],
            programs: vec![],
            banks: vec![],
            tempo_changes: 0,
            time_signature: None,
            key_signature: None,
            note_spans: vec![NoteSpan {
                pitch: 60,
                start: 10,
                end: 20,
            }],
            preview_width: 1,
            preview_height: 1,
            preview_cells: vec![0],
        };
        let data = build_piano_roll_data(&track, 20, 10, &view);
        assert_eq!(data.len(), 20 * 10 * 4);
        assert!(data.iter().any(|value| *value > 0));
    }

    #[test]
    fn build_empty_piano_roll_data_fills() {
        let data = build_empty_piano_roll_data(4, 3);
        assert_eq!(data.len(), 4 * 3 * 4);
        assert!(data.iter().any(|value| *value > 0));
    }

    #[test]
    fn visible_tick_math_clamps() {
        assert_eq!(compute_visible_ticks(0, 2.0), 1.0);
        assert_eq!(clamp_offset_ticks(10.0, 100, 2.0), 10.0);
        assert_eq!(clamp_offset_ticks(200.0, 100, 2.0), 50.0);
    }

    #[test]
    fn visible_pitch_math_clamps() {
        assert_eq!(compute_visible_pitch_range(60, 72, 2.0), 6.5);
        assert_eq!(clamp_offset_pitch(0.0, 60, 72, 2.0), 0.0);
        assert_eq!(clamp_offset_pitch(20.0, 60, 72, 2.0), 6.5);
    }

    #[test]
    fn note_row_band_clamps() {
        assert_eq!(note_cell_band(0, 60, 72, 60), (0, 0));
        assert_eq!(note_cell_band(10, 60, 69, 69), (0, 0));
        assert_eq!(note_cell_band(10, 60, 69, 60), (9, 9));
    }

    #[test]
    fn note_cell_band_full_height() {
        assert_eq!(note_cell_band(10, 60, 60, 60), (0, 9));
    }

    #[test]
    fn ruler_left_px_within_view() {
        let view = PianoRollViewState::default();
        let left = ruler_left_px(50, 100, &view, 200.0);
        assert!(left.is_some());
    }

    #[test]
    fn ruler_left_px_outside_view() {
        let view = PianoRollViewState::default();
        let left = ruler_left_px(200, 100, &view, 200.0);
        assert!(left.is_none());
    }

    #[test]
    fn note_name_formats() {
        assert_eq!(note_name(60), "C4");
        assert_eq!(note_name(61), "C#4");
        assert_eq!(note_name(0), "C-1");
    }

    #[test]
    fn pitch_list_reversed() {
        assert_eq!(pitch_list(60, 62), vec![62, 61, 60]);
        assert!(pitch_list(62, 60).is_empty());
    }

    #[test]
    fn visible_pitch_bounds_clamps() {
        let view = PianoRollViewState::default();
        let track = MidiTrackInfo {
            index: 0,
            name: None,
            event_count: 0,
            end_tick: 1,
            ticks_per_beat: 1,
            note_count: 1,
            min_pitch: 60,
            max_pitch: 60,
            channels: vec![0],
            programs: vec![],
            banks: vec![],
            tempo_changes: 0,
            time_signature: None,
            key_signature: None,
            note_spans: vec![NoteSpan {
                pitch: 60,
                start: 0,
                end: 1,
            }],
            preview_width: 1,
            preview_height: 1,
            preview_cells: vec![0],
        };
        let (start, end) = visible_pitch_bounds(&track, &view);
        assert_eq!(start, 60);
        assert_eq!(end, 60);
    }

    #[test]
    fn should_rebuild_labels_only_on_change() {
        let root = PianoRollLabelsRoot {
            start: 60,
            end: 72,
            height: 100,
        };
        assert!(!should_rebuild_labels(&root, 60, 72, 100));
        assert!(should_rebuild_labels(&root, 61, 72, 100));
        assert!(should_rebuild_labels(&root, 60, 72, 102));
    }
}
