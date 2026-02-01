use bevy::prelude::*;
use bevy_remote::{http::RemoteHttpPlugin, RemotePlugin};
use serde::{Deserialize, Serialize};

/// Component to tag entities that should be rendered as a primitive shape.
/// This simplifies the remote protocol: instead of sending complex Handle serialization,
/// we just send "I want a Cube".
#[derive(Component, Reflect, Default, Debug, Serialize, Deserialize)]
#[reflect(Component)]
pub struct AxiomPrimitive {
    pub primitive_type: String,
}

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

        // Register our custom component so BRP can see it
        app.register_type::<AxiomPrimitive>();

        // Add system to "hydrate" primitives with actual meshes
        app.add_systems(Update, spawn_primitives);

        info!("Bevy AI Remote Plugin initialized on port 15721");
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
