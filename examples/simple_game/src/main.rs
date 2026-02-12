use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_ai_remote::BevyAiRemotePlugin;

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
        .add_systems(Update, (draw_gizmos, camera_controller))
        .run();
}

fn camera_controller(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera3d>>,
) {
    let speed = 20.0; // Faster for top-down view
    let mut velocity = Vec3::ZERO;

    for mut transform in query.iter_mut() {
        // Move in X/Z plane (Top-Down navigation)
        // Forward on screen is -Z (World North)
        // Right on screen is +X (World East)

        let forward = Vec3::NEG_Z;
        let right = Vec3::X;

        if keyboard_input.pressed(KeyCode::KeyW) {
            velocity += forward;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            velocity -= forward;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            velocity -= right;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            velocity += right;
        }

        // Zoom (Up/Down)
        if keyboard_input.pressed(KeyCode::KeyE) {
            velocity += Vec3::Y;
        }
        if keyboard_input.pressed(KeyCode::KeyQ) {
            velocity -= Vec3::Y;
        }

        if velocity != Vec3::ZERO {
            let translation = velocity.normalize() * speed * time.delta_secs();
            transform.translation += translation;
        }
    }
}

fn draw_gizmos(mut gizmos: Gizmos) {
    // X-axis (Red) -> Right
    gizmos.line(
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        Color::srgb(1.0, 0.0, 0.0),
    );
    // Y-axis (Green) -> Up (Vertical)
    gizmos.line(
        Vec3::ZERO,
        Vec3::new(0.0, 10.0, 0.0),
        Color::srgb(0.0, 1.0, 0.0),
    );
    // Z-axis (Blue) -> Down (Screen)
    gizmos.line(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, 10.0),
        Color::srgb(0.0, 0.0, 1.0),
    );

    // Grid (XZ Floor) - White
    // Shifted by 0.5 to make (0,0) appear as cell center (Tile-based look)
    gizmos.grid(
        Isometry3d::new(
            Vec3::new(0.5, 0.0, 0.5),
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        ),
        UVec2::new(20, 20),
        Vec2::new(1.0, 1.0),
        Color::srgba(1.0, 1.0, 1.0, 0.2),
    );
}

fn setup(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
    // Top-Down Orthographic-ish View
    // Pos: (0, 50, 0)
    // LookAt: (0, 0, 0) with Up = -Z (So -Z is "Up" on screen)
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 50.0, 0.0).looking_at(Vec3::ZERO, Vec3::NEG_Z),
    ));

    // Light
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 50.0, 4.0),
    ));

    println!("Simple Game Running with AI Remote Control (Top-Down)...");
}
