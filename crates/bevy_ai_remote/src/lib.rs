use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bevy::prelude::*;
use bevy_remote::{http::RemoteHttpPlugin, RemotePlugin};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Component to tag entities that should be rendered as a primitive shape.
#[derive(Component, Reflect, Default, Debug, Serialize, Deserialize)]
#[reflect(Component)]
pub struct AxiomPrimitive {
    pub primitive_type: String,
}

/// Component to receive a Base64 encoded asset file from the Editor.
/// usage: spawn an entity with this component. The system will write the file
/// to `assets/_remote_cache/` and then attach a SceneRoot to the entity.
#[derive(Component, Reflect, Default, Debug, Serialize, Deserialize)]
#[reflect(Component, Default)]
pub struct AxiomRemoteAsset {
    pub filename: String,
    pub data_base64: String,
    // Optional sub-path relative to _remote_cache (e.g., "Textures")
    pub subdir: Option<String>,
}

/// Component to request despawning of another entity.
/// Usage: spawn an entity with this component. The system will despawn the target entity and then the request entity.
#[derive(Component, Reflect, Default, Debug, Serialize, Deserialize)]
#[reflect(Component)]
pub struct AxiomDespawnRequest {
    pub target: u64,
}

/// Component to request clearing of all Axiom-spawned assets.
#[derive(Component, Reflect, Default, Debug, Serialize, Deserialize)]
#[reflect(Component)]
pub struct AxiomClearSceneRequest;

/// Add this plugin to your Bevy app to enable remote control via Axiom.
pub struct BevyAiRemotePlugin;

impl Plugin for BevyAiRemotePlugin {
    fn build(&self, app: &mut App) {
        // Ensure RemotePlugin is added if not already
        if !app.is_plugin_added::<RemotePlugin>() {
            app.add_plugins(RemotePlugin::default());
        }

        use std::net::IpAddr;

        // Ensure HTTP transport is enabled with correct config
        if !app.is_plugin_added::<RemoteHttpPlugin>() {
            app.add_plugins(
                RemoteHttpPlugin::default()
                    .with_address("127.0.0.1".parse::<IpAddr>().unwrap())
                    .with_port(15721),
            );
        }

        // Register our custom components
        app.register_type::<AxiomPrimitive>();
        app.register_type::<AxiomRemoteAsset>();
        app.register_type::<AxiomDespawnRequest>();
        app.register_type::<AxiomClearSceneRequest>();

        // Add systems
        app.add_systems(
            Update,
            (
                spawn_primitives,
                handle_remote_assets,
                handle_despawn_requests,
                handle_clear_scene_requests,
            ),
        );

        info!("Bevy AI Remote Plugin initialized on port 15721");
    }
}

fn handle_clear_scene_requests(
    mut commands: Commands,
    query: Query<Entity, Added<AxiomClearSceneRequest>>,
    targets: Query<Entity, Or<(With<AxiomRemoteAsset>, With<AxiomPrimitive>)>>,
) {
    for request_entity in query.iter() {
        info!("Processing clear scene request: Despawning all Axiom assets");

        let mut count = 0;
        for target in targets.iter() {
            if let Ok(mut cmds) = commands.get_entity(target) {
                cmds.despawn(); // Use despawn() as it's safe and recursive in 0.18 for components
                count += 1;
            }
        }
        info!("Cleared {} entities.", count);

        // Despawn the request entity itself
        commands.entity(request_entity).despawn();
    }
}

fn handle_despawn_requests(
    mut commands: Commands,
    query: Query<(Entity, &AxiomDespawnRequest), Added<AxiomDespawnRequest>>,
) {
    for (request_entity, request) in query.iter() {
        // Construct Entity from bits (u64)
        let target_entity = Entity::from_bits(request.target);
        info!("Processing despawn request for entity: {:?}", target_entity);

        // Despawn the target recursively (to clean up children like Scene parts)
        // Note: If entity doesn't exist, this might panic in older bevy or just warn.
        // In recent Bevy (0.13+), `commands.entity(e)` panics if e is invalid generation?
        // No, commands are deferred. `get_entity` returns Option.
        // But `commands.entity(e)` creates an EntityCommands struct.
        // Safe approach: use `commands.get_entity(e)` if available or just try.
        // Standard way is just do it. If it doesn't exist, Bevy handles it gracefully usually.

        if let Ok(mut entity_cmds) = commands.get_entity(target_entity) {
            entity_cmds.despawn();
        }

        // Despawn the request entity itself so it doesn't pile up
        commands.entity(request_entity).despawn();
    }
}

fn spawn_primitives(
    mut commands: Commands,
    query: Query<(Entity, &AxiomPrimitive), Added<AxiomPrimitive>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, primitive) in query.iter() {
        info!("Hydrating primitive: {:?}", primitive.primitive_type);
        match primitive.primitive_type.as_str() {
            "cube" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Cuboid::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                ));
            }
            "sphere" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Sphere::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                ));
            }
            _ => {
                warn!("Unknown primitive type: {}", primitive.primitive_type);
            }
        }
    }
}

fn handle_remote_assets(
    mut commands: Commands,
    query: Query<(Entity, &AxiomRemoteAsset), Added<AxiomRemoteAsset>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, asset) in query.iter() {
        info!("Receiving remote asset: {}", asset.filename);

        // 1. Decode Base64
        let decoded = match BASE64.decode(&asset.data_base64) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to decode base64 for {}: {}", asset.filename, e);
                continue;
            }
        };

        // 2. Ensure cache directory exists
        let mut cache_dir = Path::new("assets/_remote_cache").to_path_buf();

        // Handle subdirectory if provided
        if let Some(sub) = &asset.subdir {
            if !sub.is_empty() {
                cache_dir = cache_dir.join(sub);
            }
        }

        if !cache_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                error!("Failed to create cache dir {:?}: {}", cache_dir, e);
                continue;
            }
        }

        // 3. Write file to disk
        let file_path = cache_dir.join(&asset.filename);

        // Prevent redundant writes / race conditions for same content
        let mut should_write = true;
        if file_path.exists() {
            if let Ok(existing_bytes) = std::fs::read(&file_path) {
                if existing_bytes == decoded {
                    info!(
                        "File {:?} already exists and matches content. Skipping write.",
                        file_path
                    );
                    should_write = false;
                }
            }
        }

        if should_write {
            let mut file = match File::create(&file_path) {
                Ok(f) => f,
                Err(e) => {
                    error!("Failed to create file {:?}: {}", file_path, e);
                    continue;
                }
            };

            if let Err(e) = file.write_all(&decoded) {
                error!("Failed to write file {:?}: {}", file_path, e);
                continue;
            }
            info!("Saved remote asset to {:?}", file_path);
        } else {
            // Touch the file to ensure asset server notices if it's hot reloading?
            // Actually, if content is same, we don't want to trigger reload.
        }

        // 4. Load the asset using AssetServer
        // Note: AssetServer paths are relative to "assets" folder
        // We need to construct the path relative to "assets"
        let mut relative_path_str = "_remote_cache".to_string();
        if let Some(sub) = &asset.subdir {
            if !sub.is_empty() {
                relative_path_str = format!("{}/{}", relative_path_str, sub);
            }
        }
        relative_path_str = format!("{}/{}", relative_path_str, asset.filename);

        // Only load as Scene if it's a model file. If it's a texture, we just write it and stop.
        if asset.filename.ends_with(".glb") || asset.filename.ends_with(".gltf") {
            let scene_path = format!("{}#Scene0", relative_path_str);
            info!("Loading scene from: {}", scene_path);
            let scene_handle: Handle<Scene> = asset_server.load(scene_path);
            // 5. Attach SceneRoot to the entity
            commands.entity(entity).insert(SceneRoot(scene_handle));
        } else {
            info!("Saved auxiliary asset (texture/bin), not spawning SceneRoot.");
            // Just cleanup the component so it doesn't stay on the entity forever
            commands.entity(entity).remove::<AxiomRemoteAsset>();
            // Also despawn the entity itself if it has no other components, to keep hierarchy clean
            // commands.entity(entity).despawn();
        }
    }
}
