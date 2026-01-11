mod about;
mod piano;
mod splash;
mod tracks;

use crate::state::{UiPage, UiState};
use bevy::prelude::{
    default, App, AssetServer, BackgroundColor, Camera2d, Color, Commands, Component, Display,
    Font, Handle, Node, Plugin, Query, Res, Resource, Startup, Update, Val, With, Without,
};

#[derive(Component)]
pub struct SplashPageRoot;

#[derive(Component)]
pub struct AboutPageRoot;

#[derive(Component)]
pub struct TracksPageRoot;

#[derive(Component)]
pub struct PianoRollPageRoot;

#[derive(Resource)]
pub(super) struct UiFonts {
    main: Handle<Font>,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (
                    update_page_visibility,
                    splash::update_selection_visuals,
                    tracks::update_tracks_list,
                    tracks::update_track_ruler,
                    tracks::update_track_previews,
                    tracks::update_track_details_popup,
                    tracks::update_tracks_scroll,
                    tracks::toggle_debug_overlay,
                    tracks::update_tracks_focus_visuals,
                    tracks::update_debug_overlay,
                    piano::update_piano_roll_view,
                    piano::update_piano_roll_ruler,
                    piano::update_piano_roll_labels,
                ),
            )
            .init_resource::<tracks::DebugOverlayState>()
            .init_resource::<tracks::TracksScroll>();
    }
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("Setting up UI...");
    commands.spawn(Camera2d::default());

    let font = asset_server.load("PixelifySans-Regular.ttf");
    commands.insert_resource(UiFonts { main: font.clone() });

    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 0.0, 0.5)),
        ))
        .id();
    splash::spawn_splash_page(&mut commands, root, font.clone());
    about::spawn_about_page(&mut commands, root, font.clone());
    tracks::spawn_tracks_page(&mut commands, root, font.clone());
    piano::spawn_piano_roll_page(&mut commands, root, font.clone());
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
    mut piano_query: Query<
        &mut Node,
        (
            With<PianoRollPageRoot>,
            Without<SplashPageRoot>,
            Without<AboutPageRoot>,
            Without<TracksPageRoot>,
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
    let piano_display = if ui_state.page == UiPage::PianoRoll {
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
    for mut node in &mut piano_query {
        node.display = piano_display;
    }
}
