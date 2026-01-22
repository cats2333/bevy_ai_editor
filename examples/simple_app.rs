use avian3d::prelude::*;
use bevy::ecs::event::EventReader;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy_ai_editor::{AiEditorConfig, AiEditorPlugin};

// Camera Controller Component
#[derive(Component)]
struct CameraController {
    speed: f32,
    rotate_speed: f32,
    scroll_speed: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            speed: 20.0,
            rotate_speed: 0.3,
            scroll_speed: 2.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(AiEditorPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, camera_control_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera with Controller
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 15.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraController::default(),
    ));

    // Light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        RigidBody::Static,
        Collider::cuboid(100.0, 0.1, 100.0),
    ));

    info!("🚀 Bevy AI Editor Example running!");
    info!("🎮 Controls: WASD to move, QE to up/down, Scroll to zoom, Hold Right Click to rotate.");
}

fn camera_control_system(
    time: Res<Time>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &CameraController)>,
) {
    let dt = time.delta_secs();

    for (mut transform, controller) in query.iter_mut() {
        // 1. Rotation (Hold Right Click)
        if mouse_button.pressed(MouseButton::Right) {
            for event in mouse_motion.read() {
                let delta_yaw = -event.delta.x * controller.rotate_speed * dt;
                let delta_pitch = -event.delta.y * controller.rotate_speed * dt;

                // Bevy's camera rotation logic
                transform.rotate_y(delta_yaw);

                // Limit pitch or rotate locally? Local X is pitch.
                let right = transform.right();
                transform.rotate_axis(right, delta_pitch);
            }
        }

        // 2. Movement (WASD + QE)
        let mut velocity = Vec3::ZERO;
        // forward() returns Dir3, use as_vec3()
        let forward = transform.forward();
        let right = transform.right();
        let up = Vec3::Y;

        if keyboard.pressed(KeyCode::KeyW) {
            // Flatten forward for movement? Or fly mode? Fly mode is better for editor.
            velocity += forward.as_vec3();
        }
        if keyboard.pressed(KeyCode::KeyS) {
            velocity -= forward.as_vec3();
        }
        if keyboard.pressed(KeyCode::KeyD) {
            velocity += right.as_vec3();
        }
        if keyboard.pressed(KeyCode::KeyA) {
            velocity -= right.as_vec3();
        }
        if keyboard.pressed(KeyCode::KeyE) {
            velocity += up;
        }
        if keyboard.pressed(KeyCode::KeyQ) {
            velocity -= up;
        }

        if velocity.length_squared() > 0.0 {
            velocity = velocity.normalize() * controller.speed * dt;
            transform.translation += velocity;
        }

        // 3. Scroll Zoom
        for event in mouse_wheel.read() {
            let zoom_dir = transform.forward().as_vec3();
            transform.translation += zoom_dir * event.y * controller.scroll_speed;
        }
    }
}
