use bevy::prelude::*;
use bevy_ai_remote::BevyAiRemotePlugin;

use bevy::window::WindowResolution;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Simple Game (AI Host)".to_string(),
                resolution: WindowResolution::new(800, 600),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BevyAiRemotePlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_gizmos) // Draw axes every frame
        .run();
}

fn draw_gizmos(mut gizmos: Gizmos) {
    // X-axis (Red)
    gizmos.line(
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        Color::srgb(1.0, 0.0, 0.0),
    );
    // Y-axis (Green)
    gizmos.line(
        Vec3::ZERO,
        Vec3::new(0.0, 10.0, 0.0),
        Color::srgb(0.0, 1.0, 0.0),
    );
    // Z-axis (Blue)
    gizmos.line(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, 10.0),
        Color::srgb(0.0, 0.0, 1.0),
    );

    // Grid (White, faint)
    gizmos.grid(
        Isometry3d::default(),
        UVec2::new(20, 20),
        Vec2::new(1.0, 1.0),
        Color::srgba(1.0, 1.0, 1.0, 0.1),
    );
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Plane
    /*
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    */

    println!("Simple Game Running with AI Remote Control...");
}
