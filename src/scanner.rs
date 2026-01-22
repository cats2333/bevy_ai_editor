use crate::{AiEditorConfig, AssetManifestItem};
use bevy::gltf::{Gltf, GltfMesh};
use bevy::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct AssetScannerPlugin;

impl Plugin for AssetScannerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetScanState>()
            .add_systems(Startup, start_auto_scan)
            .add_systems(Update, process_scan);
    }
}

#[derive(Resource, Default)]
enum ScanStatus {
    #[default]
    Idle,
    Scanning {
        handles: Vec<(String, Handle<Gltf>)>,
    },
}

#[derive(Resource, Default)]
struct AssetScanState {
    status: ScanStatus,
}

fn start_auto_scan(mut state: ResMut<AssetScanState>, asset_server: Res<AssetServer>) {
    info!("🔍 [AssetScanner] Starting Auto Scan...");
    let mut handles = Vec::new();
    let root_dir = Path::new("assets/models");

    // Only scan if directory exists
    if root_dir.exists() {
        scan_dir(root_dir, &asset_server, &mut handles);
        if !handles.is_empty() {
            info!(
                "📦 [AssetScanner] Found {} assets. Loading...",
                handles.len()
            );
            state.status = ScanStatus::Scanning { handles };
        } else {
            info!("⚠️ [AssetScanner] No assets found in assets/models.");
        }
    } else {
        warn!("⚠️ [AssetScanner] assets/models directory not found. Skipping scan.");
    }
}

fn scan_dir(dir: &Path, asset_server: &AssetServer, handles: &mut Vec<(String, Handle<Gltf>)>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, asset_server, handles);
            } else if let Some(ext) = path.extension() {
                if ext == "glb" || ext == "gltf" {
                    // Normalize path: assets/models/foo.glb -> models/foo.glb
                    let path_str = path.to_string_lossy().replace("\\", "/");
                    if let Some(idx) = path_str.find("assets/") {
                        let relative_path = path_str[idx + 7..].to_string(); // Skip "assets/"
                        let handle = asset_server.load(relative_path.clone());
                        handles.push((relative_path, handle));
                    }
                }
            }
        }
    }
}

fn process_scan(
    mut state: ResMut<AssetScanState>,
    asset_server: Res<AssetServer>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    mesh_assets: Res<Assets<Mesh>>,
    config: Res<AiEditorConfig>,
    mut manifest_resource: ResMut<crate::AiAssetManifest>,
) {
    if let ScanStatus::Scanning { handles } = &mut state.status {
        let all_loaded = handles.iter().all(|(_, h)| {
            matches!(
                asset_server.get_load_state(h),
                Some(bevy::asset::LoadState::Loaded)
            )
        });

        if !all_loaded {
            return;
        }

        info!("✅ [AssetScanner] All assets loaded. Computing AABBs...");
        let mut manifest: HashMap<String, AssetManifestItem> = HashMap::new();

        for (path, handle) in handles.iter() {
            if let Some(gltf) = gltf_assets.get(handle) {
                let mut min = Vec3::splat(f32::MAX);
                let mut max = Vec3::splat(f32::MIN);
                let mut found_mesh = false;

                for gltf_mesh_handle in gltf.meshes.iter() {
                    if let Some(gltf_mesh) = gltf_meshes.get(gltf_mesh_handle) {
                        for primitive in &gltf_mesh.primitives {
                            if let Some(mesh) = mesh_assets.get(&primitive.mesh) {
                                if let Some(vertex_values) =
                                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                                {
                                    if let Some(positions) = vertex_values.as_float3() {
                                        for pos in positions {
                                            let vec = Vec3::from(*pos);
                                            min = min.min(vec);
                                            max = max.max(vec);
                                            found_mesh = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if found_mesh {
                    let size = max - min;
                    let center = (min + max) / 2.0;

                    // Simple Inference Rules
                    let (collider, snap, offset) =
                        if path.contains("tile") || path.contains("ground") {
                            ("trimesh", false, 0.0)
                        } else if path.contains("tree") || path.contains("nature") {
                            ("capsule", true, 0.0)
                        } else {
                            ("cuboid", true, 0.0)
                        };

                    manifest.insert(
                        path.clone(),
                        AssetManifestItem {
                            collider: collider.to_string(),
                            size: [size.x, size.y, size.z],
                            center_offset: [center.x, center.y, center.z],
                            snap_to_ground: snap,
                            vertical_offset: offset,
                        },
                    );
                }
            }
        }

        // Update in-memory resource
        manifest_resource.0 = manifest.clone();
        info!(
            "🔄 [AssetScanner] In-memory manifest updated with {} items.",
            manifest.len()
        );

        // Write to file (using config path)
        let save_path = &config.manifest_path;
        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            if let Err(e) = fs::write(save_path, json) {
                error!("❌ Failed to write manifest: {}", e);
            } else {
                info!("💾 [AssetScanner] Manifest saved to {}", save_path);
            }
        }
        state.status = ScanStatus::Idle;
    }
}
