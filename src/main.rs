use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // Spawn 2D Camera
    commands.spawn(Camera2d::default());

    // Spawn red square
    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.0, 0.0), // Red color
            custom_size: Some(Vec2::new(100.0, 100.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0), // Position at origin
        Visibility::Visible, // Make it visible
    ));
    println!("\nSUCCESS: App with sprite components compiled and ran.\n");
}
