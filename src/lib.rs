use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, RwLock};
use std::thread;
use tiny_http::{Method, Response, Server};

pub mod scanner;

// --- Configurations ---

#[derive(Resource, Clone, Debug)]
pub struct AiEditorConfig {
    pub http_port: u16,
    pub manifest_path: String,
    pub save_dir: String,
}

impl Default for AiEditorConfig {
    fn default() -> Self {
        Self {
            http_port: 15703,
            manifest_path: "assets/asset_manifest.json".to_string(),
            save_dir: "assets/levels".to_string(),
        }
    }
}

// --- Manifest Definitions ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifestItem {
    pub collider: String, // "trimesh", "hull", "capsule", "cuboid"
    pub size: [f32; 3],   // Extents
    pub center_offset: [f32; 3],
    #[serde(default)]
    pub snap_to_ground: bool,
    #[serde(default)]
    pub vertical_offset: f32,
}

#[derive(Resource, Default)]
pub struct AiAssetManifest(pub HashMap<String, AssetManifestItem>);

#[derive(Resource, Default)]
pub struct AiNamedEntities(pub HashMap<String, Entity>);

// --- Plugin ---

pub struct AiEditorPlugin;

impl Plugin for AiEditorPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<AiEditorConfig>() {
            app.init_resource::<AiEditorConfig>();
        }

        let config = app.world().resource::<AiEditorConfig>().clone();
        let (tx, rx) = flume::unbounded::<AiCommand>();
        let shared_state = SharedSceneState(Arc::new(RwLock::new(Vec::new())));

        app.insert_resource(AiCommandReceiver(rx))
            .insert_resource(shared_state.clone())
            .init_resource::<AiAssetManifest>()
            .init_resource::<AiNamedEntities>()
            .register_type::<AiSpawned>()
            .add_plugins(scanner::AssetScannerPlugin)
            .add_systems(Startup, load_manifest)
            .add_systems(
                Update,
                (handle_ai_commands, apply_snap_to_ground, sync_scene_state),
            );

        thread::spawn(move || {
            start_http_server(tx, shared_state, config);
        });
    }
}

// --- Components & Resources ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DebugEntityInfo {
    pub asset_path: String,
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub name: Option<String>,
}

#[derive(Resource, Clone)]
pub struct SharedSceneState(pub Arc<RwLock<Vec<DebugEntityInfo>>>);

#[derive(Component, Reflect, Serialize, Deserialize, Debug)]
#[reflect(Component)]
pub struct AiSpawned {
    pub asset_path: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AiCommand {
    Spawn(AiSpawnCommand),
    Joint(AiJointCommand),
    Motor(AiMotorCommand),
    Save(String),
}

#[derive(Debug, Clone)]
pub struct AiSpawnCommand {
    pub asset_path: String,
    pub position: Vec3,
    pub scale: f32,
    pub rotation: f32,
    pub name: Option<String>,
    pub physics: String, // "static", "dynamic", "kinematic"
}

#[derive(Debug, Clone)]
pub struct AiJointCommand {
    pub entity1: String,
    pub entity2: String,
    pub joint_type: String, // "fixed", "revolute", "spherical"
    pub anchor1: Vec3,
    pub anchor2: Vec3,
    pub limits: Option<[f32; 2]>,
    pub motor: bool,
    pub name: Option<String>, // Joint name for motor control
}

#[derive(Debug, Clone)]
pub struct AiMotorCommand {
    pub joint_name: String,
    pub target_pos: f32,
    pub stiffness: f32,
    pub damping: f32,
}

#[derive(Resource)]
struct AiCommandReceiver(flume::Receiver<AiCommand>);

// --- JSON Payloads ---

#[derive(Deserialize, Debug)]
struct SpawnRequestPayload {
    asset_path: String,
    x: f32,
    y: Option<f32>,
    z: f32,
    scale: Option<f32>,
    rotation: Option<f32>,
    name: Option<String>,
    physics: Option<String>,
}

#[derive(Deserialize, Debug)]
struct JointRequestPayload {
    entity1: String,
    entity2: String,
    #[serde(default = "default_joint_type")]
    r#type: String,
    anchor1: Option<[f32; 3]>,
    anchor2: Option<[f32; 3]>,
    limits: Option<[f32; 2]>,
    #[serde(default)]
    motor: bool,
    name: Option<String>,
}

fn default_joint_type() -> String {
    "fixed".to_string()
}

#[derive(Deserialize, Debug)]
struct MotorRequestPayload {
    joint_name: String,
    target: f32,
    stiffness: Option<f32>,
    damping: Option<f32>,
}

#[derive(Deserialize, Debug)]
struct SaveRequestPayload {
    filename: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LevelData {
    entities: Vec<LevelEntity>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LevelEntity {
    asset_path: String,
    position: [f32; 3],
    scale: [f32; 3],
    rotation: [f32; 4],
    name: Option<String>,
}

#[derive(Component)]
pub struct SnapToGround {
    pub offset: f32,
}

#[derive(Component)]
pub struct NeedCollider {
    pub collider_type: String,
    pub size: Vec3,
    pub offset: Vec3,
}

// --- Systems ---

fn load_manifest(mut commands: Commands, config: Res<AiEditorConfig>) {
    let path = &config.manifest_path;
    if let Ok(file) = File::open(path) {
        if let Ok(manifest) = serde_json::from_reader::<_, HashMap<String, AssetManifestItem>>(file)
        {
            info!(
                "📄 [BevyAiEditor] Loaded Asset Manifest with {} items.",
                manifest.len()
            );
            commands.insert_resource(AiAssetManifest(manifest));
            return;
        }
    }
    warn!(
        "⚠️ [BevyAiEditor] Failed to load Asset Manifest at {}. Physics inference disabled.",
        path
    );
}

fn apply_snap_to_ground(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &SnapToGround)>,
    spatial_query: SpatialQuery,
) {
    for (entity, mut transform, snap) in query.iter_mut() {
        let origin = Vec3::new(transform.translation.x, 2000.0, transform.translation.z);
        let dir = Dir3::NEG_Y;
        let filter = SpatialQueryFilter::from_excluded_entities([entity]);

        if let Some(hit) = spatial_query.cast_ray(origin, dir, 4000.0, true, &filter) {
            let hit_point = origin + dir.as_vec3() * hit.distance;
            let mut final_y = hit_point.y;

            // Simple heuristic corrections
            if final_y.abs() > 500.0 {
                final_y /= 1000.0;
            } else if final_y.abs() > 50.0 {
                final_y /= 100.0;
            }

            final_y += snap.offset;
            transform.translation.y = final_y;
            commands.entity(entity).remove::<SnapToGround>();
        }
    }
}

fn handle_ai_commands(
    mut commands: Commands,
    receiver: Res<AiCommandReceiver>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&Transform, &AiSpawned)>,
    manifest: Res<AiAssetManifest>,
    config: Res<AiEditorConfig>,
    mut named_entities: ResMut<AiNamedEntities>,
    mut revolute_joints: Query<&mut RevoluteJoint>,
    mut spherical_joints: Query<&mut SphericalJoint>,
) {
    let mut processed = 0;
    while let Ok(cmd) = receiver.0.try_recv() {
        if processed > 50 {
            break;
        }
        processed += 1;

        match cmd {
            AiCommand::Spawn(spawn_cmd) => {
                let position = spawn_cmd.position;
                let scale = Vec3::splat(spawn_cmd.scale);
                let rotation = Quat::from_rotation_y(spawn_cmd.rotation);
                let asset_path = spawn_cmd.asset_path.clone();
                let entity_name = spawn_cmd.name.clone();
                let physics_type = spawn_cmd.physics.as_str();

                let mut entity_cmds = if asset_path.starts_with("builtin://") {
                    let parts: Vec<&str> = asset_path
                        .trim_start_matches("builtin://")
                        .split('/')
                        .collect();
                    let shape = parts[0];
                    let color_name = parts.get(1).unwrap_or(&"white");

                    let color = match *color_name {
                        "red" => Color::srgb(1.0, 0.0, 0.0),
                        "green" => Color::srgb(0.0, 1.0, 0.0),
                        "blue" => Color::srgb(0.0, 0.0, 1.0),
                        "yellow" => Color::srgb(1.0, 1.0, 0.0),
                        "brown" => Color::srgb(0.5, 0.25, 0.0),
                        "black" => Color::BLACK,
                        "gray" => Color::srgb(0.5, 0.5, 0.5),
                        "white" => Color::WHITE,
                        _ => Color::WHITE,
                    };

                    let mesh_handle = match shape {
                        "cube" | "box" => meshes.add(Cuboid::default()),
                        "sphere" => meshes.add(Sphere::default()),
                        "capsule" => meshes.add(Capsule3d::default()),
                        "plane" => meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
                        _ => meshes.add(Cuboid::default()),
                    };

                    let mut e = commands.spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: color,
                            ..default()
                        })),
                        Transform::from_translation(position)
                            .with_scale(scale)
                            .with_rotation(rotation),
                        AiSpawned {
                            asset_path: asset_path.clone(),
                            name: entity_name.clone(),
                        },
                    ));

                    // Physics
                    match physics_type {
                        "dynamic" => {
                            e.insert(RigidBody::Dynamic);
                        }
                        "kinematic" => {
                            e.insert(RigidBody::Kinematic);
                        }
                        _ => {
                            e.insert(RigidBody::Static);
                        }
                    }

                    match shape {
                        "sphere" => {
                            e.insert(Collider::sphere(0.5));
                        }
                        "capsule" => {
                            e.insert(Collider::capsule(0.5, 1.0));
                        }
                        _ => {
                            e.insert(Collider::cuboid(1.0, 1.0, 1.0));
                        }
                    }
                    e
                } else {
                    let scene_handle = asset_server
                        .load(GltfAssetLabel::Scene(0).from_asset(spawn_cmd.asset_path.clone()));
                    let manifest_item = manifest.0.get(&spawn_cmd.asset_path);

                    let mut e = commands.spawn((
                        SceneRoot(scene_handle),
                        Transform::from_translation(spawn_cmd.position)
                            .with_scale(Vec3::splat(spawn_cmd.scale))
                            .with_rotation(Quat::from_rotation_y(spawn_cmd.rotation)),
                        AiSpawned {
                            asset_path: spawn_cmd.asset_path.clone(),
                            name: entity_name.clone(),
                        },
                    ));

                    if let Some(item) = manifest_item {
                        // For imported assets, "static" is default unless specified
                        match physics_type {
                            "dynamic" => {
                                e.insert(RigidBody::Dynamic);
                            }
                            "kinematic" => {
                                e.insert(RigidBody::Kinematic);
                            }
                            _ => {
                                e.insert(RigidBody::Static);
                            }
                        }

                        e.insert(NeedCollider {
                            collider_type: item.collider.clone(),
                            size: Vec3::from(item.size),
                            offset: Vec3::from(item.center_offset),
                        });
                        if item.snap_to_ground {
                            e.insert(SnapToGround {
                                offset: item.vertical_offset,
                            });
                        }
                    }
                    e.observe(on_scene_ready_add_collider);
                    e
                };

                // Register name
                if let Some(name) = entity_name {
                    let id = entity_cmds.id();
                    info!("🏷️ [Registry] Registered '{}' -> {:?}", name, id);
                    named_entities.0.insert(name, id);
                }
            }

            AiCommand::Joint(joint_cmd) => {
                if let (Some(&e1), Some(&e2)) = (
                    named_entities.0.get(&joint_cmd.entity1),
                    named_entities.0.get(&joint_cmd.entity2),
                ) {
                    let anchor1 = joint_cmd.anchor1;
                    let anchor2 = joint_cmd.anchor2;
                    let joint_name = joint_cmd.name.clone();

                    let mut joint_entity_cmd = match joint_cmd.joint_type.as_str() {
                        "spherical" => {
                            let mut joint = SphericalJoint::new(e1, e2)
                                .with_local_anchor1(anchor1)
                                .with_local_anchor2(anchor2);
                            // Spherical motors are complex, skipping simple motor for now or treating as special case
                            // But Avian doesn't support simple position motor for spherical easily yet in 0.1?
                            // Checking Avian docs: SphericalJoint has limit support.
                            commands.spawn(joint)
                        }
                        "revolute" => {
                            let mut joint = RevoluteJoint::new(e1, e2)
                                .with_local_anchor1(anchor1)
                                .with_local_anchor2(anchor2)
                                .with_aligned_axis(Vec3::X); // Default axis X, maybe make configurable?

                            if let Some(limits) = joint_cmd.limits {
                                joint = joint.with_angle_limits(limits[0], limits[1]);
                            }

                            // Enable motor if requested (Positional by default for robotics)
                            if joint_cmd.motor {
                                // Default stiff motor
                                // TODO: Fix Motor API in Avian 0.5
                                // joint = joint.with_angular_position_target(0.0)
                                //    .with_angular_velocity_target(0.0);
                            }

                            commands.spawn(joint)
                        }
                        _ => {
                            // Fixed
                            let joint = FixedJoint::new(e1, e2)
                                .with_local_anchor1(anchor1)
                                .with_local_anchor2(anchor2);
                            commands.spawn(joint)
                        }
                    };

                    // Register joint name if provided (for motor control)
                    if let Some(name) = joint_name {
                        let id = joint_entity_cmd.id();
                        info!("🔗 [Registry] Registered Joint '{}' -> {:?}", name, id);
                        named_entities.0.insert(name, id);
                    }
                } else {
                    warn!(
                        "❌ [Joint] Failed to find entities: {} or {}",
                        joint_cmd.entity1, joint_cmd.entity2
                    );
                }
            }

            AiCommand::Motor(motor_cmd) => {
                if let Some(&joint_entity) = named_entities.0.get(&motor_cmd.joint_name) {
                    // Try to get RevoluteJoint
                    if let Ok(mut joint) = revolute_joints.get_mut(joint_entity) {
                        // Update target
                        // Avian 0.2 API: set_angular_position_target
                        // joint.angular_position_target = Some(motor_cmd.target_pos);
                        // TODO: Fix Motor API
                        warn!("⚠️ [Motor] Motor control temporarily disabled due to API mismatch in Avian 0.5");
                        info!(
                            "⚙️ [Motor] Request received for '{}' -> {}",
                            motor_cmd.joint_name, motor_cmd.target_pos
                        );
                    } else {
                        warn!(
                            "❌ [Motor] Entity '{}' is not a RevoluteJoint",
                            motor_cmd.joint_name
                        );
                    }
                } else {
                    warn!("❌ [Motor] Joint '{}' not found", motor_cmd.joint_name);
                }
            }

            AiCommand::Save(filename) => {
                let mut level_data = LevelData { entities: vec![] };
                for (transform, ai_spawned) in query.iter() {
                    level_data.entities.push(LevelEntity {
                        asset_path: ai_spawned.asset_path.clone(),
                        position: transform.translation.to_array(),
                        scale: transform.scale.to_array(),
                        rotation: transform.rotation.to_array(),
                        name: ai_spawned.name.clone(),
                    });
                }

                if std::fs::create_dir_all(&config.save_dir).is_err() {
                    error!("Failed to create directory: {}", config.save_dir);
                    continue;
                }

                let path = format!("{}/{}", config.save_dir, filename);
                if let Ok(mut file) = File::create(&path) {
                    let json = serde_json::to_string_pretty(&level_data).unwrap();
                    let _ = file.write_all(json.as_bytes());
                    info!("✅ Scene saved to {}", path);
                }
            }
        }
    }
}

fn on_scene_ready_add_collider(
    trigger: On<SceneInstanceReady>,
    mut commands: Commands,
    query: Query<&NeedCollider>,
    children: Query<&Children>,
    meshes: Res<Assets<Mesh>>,
    mesh_handles: Query<(&Mesh3d, &Transform)>,
) {
    let entity = trigger.entity;
    if let Ok(config) = query.get(entity) {
        let mut found_mesh = false;
        let mut stack = vec![entity];

        while let Some(curr) = stack.pop() {
            if let Ok((mesh3d, _)) = mesh_handles.get(curr) {
                if let Some(mesh) = meshes.get(&mesh3d.0) {
                    let collider = match config.collider_type.as_str() {
                        "trimesh" => Collider::trimesh_from_mesh(mesh),
                        "hull" => Collider::convex_hull_from_mesh(mesh),
                        "capsule" => Some(Collider::capsule(
                            config.size.x.min(config.size.z) / 2.0,
                            config.size.y,
                        )),
                        _ => Some(Collider::cuboid(
                            config.size.x,
                            config.size.y,
                            config.size.z,
                        )),
                    };
                    if let Some(c) = collider {
                        commands.entity(curr).insert(c);
                        found_mesh = true;
                    }
                }
            }
            if let Ok(kids) = children.get(curr) {
                stack.extend(kids);
            }
        }

        if !found_mesh && (config.collider_type == "capsule" || config.collider_type == "cuboid") {
            commands.entity(entity).with_children(|parent| {
                let mut child = parent.spawn(Transform::from_translation(config.offset));
                match config.collider_type.as_str() {
                    "capsule" => {
                        child.insert(Collider::capsule(
                            config.size.x.min(config.size.z) / 2.0,
                            config.size.y,
                        ));
                    }
                    _ => {
                        child.insert(Collider::cuboid(
                            config.size.x,
                            config.size.y,
                            config.size.z,
                        ));
                    }
                }
            });
        }
        commands.entity(entity).remove::<NeedCollider>();
    }
}

fn sync_scene_state(query: Query<(&Transform, &AiSpawned)>, shared_state: Res<SharedSceneState>) {
    if let Ok(mut lock) = shared_state.0.write() {
        lock.clear();
        for (transform, ai_spawned) in query.iter() {
            lock.push(DebugEntityInfo {
                asset_path: ai_spawned.asset_path.clone(),
                position: transform.translation.to_array(),
                scale: transform.scale.to_array(),
                name: ai_spawned.name.clone(),
            });
        }
    }
}

fn start_http_server(
    tx: flume::Sender<AiCommand>,
    shared_state: SharedSceneState,
    config: AiEditorConfig,
) {
    let addr = format!("127.0.0.1:{}", config.http_port);
    let server = Server::http(&addr).expect("Failed to start AI Editor Server");

    info!("🤖 [BevyAiEditor] Server listening on http://{}", addr);

    for mut request in server.incoming_requests() {
        let mut content = String::new();
        let _ = request.as_reader().read_to_string(&mut content);

        match (request.method(), request.url()) {
            (&Method::Get, "/entities") => {
                let json = if let Ok(lock) = shared_state.0.read() {
                    serde_json::to_string(&*lock).unwrap_or("[]".into())
                } else {
                    "[]".into()
                };
                let _ = request.respond(Response::from_string(json));
            }
            (&Method::Post, "/spawn") => {
                if let Ok(payload) = serde_json::from_str::<SpawnRequestPayload>(&content) {
                    let cmd = AiCommand::Spawn(AiSpawnCommand {
                        asset_path: payload.asset_path,
                        position: Vec3::new(payload.x, payload.y.unwrap_or(0.0), payload.z),
                        scale: payload.scale.unwrap_or(1.0),
                        rotation: payload.rotation.unwrap_or(0.0),
                        name: payload.name,
                        physics: payload.physics.unwrap_or("static".to_string()),
                    });
                    let _ = tx.send(cmd);
                    let _ = request.respond(Response::from_string("{\"status\": \"spawned\"}"));
                } else {
                    let _ = request.respond(
                        Response::from_string("{\"error\": \"Invalid JSON\"}")
                            .with_status_code(400),
                    );
                }
            }
            (&Method::Post, "/joint") => {
                if let Ok(payload) = serde_json::from_str::<JointRequestPayload>(&content) {
                    let cmd = AiCommand::Joint(AiJointCommand {
                        entity1: payload.entity1,
                        entity2: payload.entity2,
                        joint_type: payload.r#type,
                        anchor1: Vec3::from(payload.anchor1.unwrap_or([0.0, 0.0, 0.0])),
                        anchor2: Vec3::from(payload.anchor2.unwrap_or([0.0, 0.0, 0.0])),
                        limits: payload.limits,
                        motor: payload.motor,
                        name: payload.name,
                    });
                    let _ = tx.send(cmd);
                    let _ =
                        request.respond(Response::from_string("{\"status\": \"joint_created\"}"));
                } else {
                    let _ = request.respond(
                        Response::from_string("{\"error\": \"Invalid JSON\"}")
                            .with_status_code(400),
                    );
                }
            }
            (&Method::Post, "/motor") => {
                if let Ok(payload) = serde_json::from_str::<MotorRequestPayload>(&content) {
                    let cmd = AiCommand::Motor(AiMotorCommand {
                        joint_name: payload.joint_name,
                        target_pos: payload.target,
                        stiffness: payload.stiffness.unwrap_or(10.0),
                        damping: payload.damping.unwrap_or(1.0),
                    });
                    let _ = tx.send(cmd);
                    let _ =
                        request.respond(Response::from_string("{\"status\": \"motor_updated\"}"));
                } else {
                    let _ = request.respond(
                        Response::from_string("{\"error\": \"Invalid JSON\"}")
                            .with_status_code(400),
                    );
                }
            }
            (&Method::Post, "/save") => {
                if let Ok(payload) = serde_json::from_str::<SaveRequestPayload>(&content) {
                    let _ = tx.send(AiCommand::Save(payload.filename));
                    let _ = request.respond(Response::from_string("{\"status\": \"saving\"}"));
                } else {
                    let _ = request.respond(
                        Response::from_string("{\"error\": \"Invalid JSON\"}")
                            .with_status_code(400),
                    );
                }
            }
            _ => {
                let _ = request.respond(
                    Response::from_string("{\"error\": \"Not Found\"}").with_status_code(404),
                );
            }
        }
    }
}
