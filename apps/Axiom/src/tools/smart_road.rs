use crate::tools::Tool;
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

// --- Shared State for Road Driver ---

#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    North, // -Z
    South, // +Z
    East,  // +X
    West,  // -X
}

impl Direction {
    fn to_string(&self) -> String {
        match self {
            Direction::North => "-Z".to_string(),
            Direction::South => "+Z".to_string(),
            Direction::East => "+X".to_string(),
            Direction::West => "-X".to_string(),
        }
    }

    fn turn_left(&self) -> Direction {
        match self {
            Direction::North => Direction::West,
            Direction::West => Direction::South,
            Direction::South => Direction::East,
            Direction::East => Direction::North,
        }
    }

    fn turn_right(&self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    fn to_vector(&self) -> (i64, i64) {
        // (dx, dz)
        match self {
            Direction::North => (0, -1),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
        }
    }

    fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }
}

struct DriverState {
    x: i64,
    z: i64,
    heading: Direction,
}

lazy_static! {
    static ref GLOBAL_DRIVER_STATE: Mutex<DriverState> = Mutex::new(DriverState {
        x: 0,
        z: 0,
        heading: Direction::East, // Default start heading
    });

    // Memory Canvas: Map coordinates to connections
    static ref GLOBAL_ROAD_MAP: Mutex<HashMap<(i64, i64), HashSet<String>>> = Mutex::new(HashMap::new());

    // Entity Tracker: Map coordinates to Entity ID
    static ref GLOBAL_ENTITY_MAP: Mutex<HashMap<(i64, i64), u64>> = Mutex::new(HashMap::new());
}

// --- Tools ---

/// Tool 1: Smart Road Tool (Legacy & Precision)
pub struct SmartRoadTool;
impl Tool for SmartRoadTool {
    fn name(&self) -> String {
        "spawn_smart_road".to_string()
    }
    fn description(&self) -> String {
        "Spawn a road piece based on connectivity.".to_string()
    }
    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "spawn_smart_road",
                "description": "Spawn a road piece based on connectivity.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "integer" },
                        "z": { "type": "integer" },
                        "connections": {
                            "type": "array",
                            "items": { "type": "string", "enum": ["+X", "-X", "+Z", "-Z"] }
                        }
                    },
                    "required": ["x", "z", "connections"]
                }
            }
        })
    }
    fn execute(&self, args: Value) -> Result<String> {
        let x = args
            .get("x")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow!("Missing x"))?;
        let z = args
            .get("z")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow!("Missing z"))?;
        let connections_val = args
            .get("connections")
            .and_then(|v| v.as_array())
            .ok_or(anyhow!("Missing connections"))?;

        let mut connections: HashSet<String> = HashSet::new();
        for c in connections_val {
            if let Some(s) = c.as_str() {
                connections.insert(s.to_string());
            }
        }

        let (model_name, rotation) = solve_road_piece(&connections)?;
        let resource_path = resolve_model_path(&model_name)?;

        let _ = upload_asset_to_bevy(
            resource_path.to_str().unwrap(),
            [x as f64, 0.0, z as f64],
            rotation,
        )?;
        Ok("Spawned road piece (SmartRoadTool logic)".to_string())
    }
}

/// Tool 2: Batch Grid Tool (Vision)
pub struct SpawnRoadGridTool;
impl Tool for SpawnRoadGridTool {
    fn name(&self) -> String {
        "spawn_road_grid".to_string()
    }
    fn description(&self) -> String {
        "Spawn a road grid from 2D array.".to_string()
    }
    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "spawn_road_grid",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "start_x": { "type": "integer" },
                        "start_z": { "type": "integer" },
                        "grid": { "type": "array", "items": { "type": "array", "items": { "type": "integer" } } }
                    },
                    "required": ["start_x", "start_z", "grid"]
                }
            }
        })
    }
    fn execute(&self, args: Value) -> Result<String> {
        // ... (Existing implementation, kept same logic but condensed for brevity)
        // Re-implementing fully to ensure it works
        let start_x = args.get("start_x").and_then(|v| v.as_i64()).unwrap_or(0);
        let start_z = args.get("start_z").and_then(|v| v.as_i64()).unwrap_or(0);
        let grid = args
            .get("grid")
            .and_then(|v| v.as_array())
            .ok_or(anyhow!("Missing grid"))?;

        let mut road_cells: HashSet<(i64, i64)> = HashSet::new();
        for (r_idx, row_val) in grid.iter().enumerate() {
            let row = row_val.as_array().ok_or(anyhow!("Invalid grid"))?;
            for (c_idx, cell) in row.iter().enumerate() {
                if cell.as_i64().unwrap_or(0) == 1 {
                    road_cells.insert((start_x + c_idx as i64, start_z + r_idx as i64));
                }
            }
        }

        let mut commands = Vec::new();
        for &(x, z) in &road_cells {
            let mut connections = HashSet::new();
            if road_cells.contains(&(x + 1, z)) {
                connections.insert("+X".to_string());
            }
            if road_cells.contains(&(x - 1, z)) {
                connections.insert("-X".to_string());
            }
            if road_cells.contains(&(x, z + 1)) {
                connections.insert("+Z".to_string());
            }
            if road_cells.contains(&(x, z - 1)) {
                connections.insert("-Z".to_string());
            }

            let (model_name, rotation) = solve_road_piece(&connections)?;
            let resource_path = resolve_model_path(&model_name)?;
            let filename = resource_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let b64_data = encode_file_base64(&resource_path)?;
            let q = euler_to_quat(rotation);

            commands.push(json!({
                "jsonrpc": "2.0",
                "method": "world.spawn_entity",
                "id": commands.len() + 1,
                "params": {
                    "components": {
                        "bevy_ai_remote::AxiomRemoteAsset": { "filename": filename, "data_base64": b64_data },
                        "bevy_transform::components::transform::Transform": {
                            "translation": [x as f64, 0.0, z as f64],
                            "rotation": [q.0, q.1, q.2, q.3],
                            "scale": [1.0, 1.0, 1.0]
                        }
                    }
                }
            }));
        }

        let agent = ureq::AgentBuilder::new()
            .timeout_read(std::time::Duration::from_secs(30))
            .build();
        let resp = agent
            .post("http://127.0.0.1:15721")
            .send_json(Value::Array(commands))?;
        Ok(format!("Grid Spawned. Status: {}", resp.status()))
    }
}

/// Tool 3: Road Driver Tool (Cursor-based)
pub struct RoadDriverTool;
impl Tool for RoadDriverTool {
    fn name(&self) -> String {
        "road_driver".to_string()
    }
    fn description(&self) -> String {
        "Drive a virtual road paver. Commands: INIT, RESET, FORWARD, TURN_LEFT, TURN_RIGHT. Automatically merges intersections (deletes old, spawns new). Accepts single action or list of actions.".to_string()
    }
    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "road_driver",
                "description": "Drive a road building cursor.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "description": "Single action or list of actions. Use 'SYNC' to recover state from game.",
                            "anyOf": [
                                { "type": "string", "enum": ["INIT", "RESET", "SYNC", "FORWARD", "TURN_LEFT", "TURN_RIGHT"] },
                                { "type": "array", "items": { "type": "string", "enum": ["INIT", "RESET", "SYNC", "FORWARD", "TURN_LEFT", "TURN_RIGHT"] } }
                            ]
                        },
                        "x": { "type": "integer", "description": "Start X (Only for INIT)" },
                        "z": { "type": "integer", "description": "Start Z (Only for INIT)" },
                        "heading": { "type": "string", "enum": ["+X", "-X", "+Z", "-Z"], "description": "Start Heading (Only for INIT)" }
                    },
                    "required": ["action"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let actions = if let Some(arr) = args.get("action").and_then(|v| v.as_array()) {
            arr.iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect::<Vec<_>>()
        } else if let Some(s) = args.get("action").and_then(|v| v.as_str()) {
            vec![s.to_string()]
        } else {
            return Err(anyhow!("Missing action"));
        };

        let mut results = Vec::new();
        let mut state = GLOBAL_DRIVER_STATE.lock().unwrap();
        let mut map = GLOBAL_ROAD_MAP.lock().unwrap();
        let mut entities = GLOBAL_ENTITY_MAP.lock().unwrap();

        for action in actions {
            match action.as_str() {
                "SYNC" => {
                    // Rehydrate state from Bevy scene
                    match sync_from_scene(&mut map, &mut entities) {
                        Ok(count) => {
                            results.push(format!("Synced {} road segments from scene.", count))
                        }
                        Err(e) => results.push(format!("Failed to sync: {}", e)),
                    }
                }
                "RESET" => {
                    // Despawn all tracked entities
                    for (_, entity_id) in entities.iter() {
                        let _ = despawn_entity(*entity_id);
                    }
                    map.clear();
                    entities.clear();
                    results.push("Cleared Global Road Map & Despawned Entities.".to_string());
                }
                "INIT" => {
                    // Auto-Sync if map is empty, to be safe
                    if map.is_empty() {
                        let _ = sync_from_scene(&mut map, &mut entities);
                    }

                    state.x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                    state.z = args.get("z").and_then(|v| v.as_i64()).unwrap_or(0);
                    let h_str = args.get("heading").and_then(|v| v.as_str()).unwrap_or("+X");
                    state.heading = match h_str {
                        "+X" => Direction::East,
                        "-X" => Direction::West,
                        "+Z" => Direction::South,
                        "-Z" => Direction::North,
                        _ => Direction::East,
                    };
                    results.push(format!(
                        "Initialized Driver at ({}, {}) facing {:?}",
                        state.x, state.z, state.heading
                    ));
                }
                "FORWARD" => {
                    // 1. Add OUTGOING connection to CURRENT tile
                    let outgoing = state.heading.to_string();
                    add_connection(&mut map, state.x, state.z, outgoing);
                    spawn_tile_from_map(&map, &mut entities, state.x, state.z)?;

                    // 2. Move Cursor
                    let (dx, dz) = state.heading.to_vector();
                    state.x += dx;
                    state.z += dz;

                    // 3. Add INCOMING connection to NEW tile
                    let incoming = state.heading.opposite().to_string();
                    add_connection(&mut map, state.x, state.z, incoming);
                    // Also spawn new tile immediately so we see where we are
                    spawn_tile_from_map(&map, &mut entities, state.x, state.z)?;

                    results.push(format!("Moved Forward to ({}, {})", state.x, state.z));
                }
                "TURN_LEFT" => {
                    // Turn implies: leaving Current via NEW Heading (Left)
                    // Incoming (from back) is already handled by previous move (or implicit start)

                    let new_heading = state.heading.turn_left();

                    // 1. Add OUTGOING (new heading) to CURRENT tile
                    let outgoing = new_heading.to_string();
                    add_connection(&mut map, state.x, state.z, outgoing);
                    spawn_tile_from_map(&map, &mut entities, state.x, state.z)?;

                    // 2. Update Heading
                    state.heading = new_heading;

                    // 3. Move Cursor
                    let (dx, dz) = state.heading.to_vector();
                    state.x += dx;
                    state.z += dz;

                    // 4. Add INCOMING (from back) to NEW tile
                    let incoming = state.heading.opposite().to_string();
                    add_connection(&mut map, state.x, state.z, incoming);
                    spawn_tile_from_map(&map, &mut entities, state.x, state.z)?;

                    results.push(format!(
                        "Turned LEFT. Now at ({}, {}) facing {:?}",
                        state.x, state.z, state.heading
                    ));
                }
                "TURN_RIGHT" => {
                    // Same logic
                    let new_heading = state.heading.turn_right();

                    let outgoing = new_heading.to_string();
                    add_connection(&mut map, state.x, state.z, outgoing);
                    spawn_tile_from_map(&map, &mut entities, state.x, state.z)?;

                    state.heading = new_heading;

                    let (dx, dz) = state.heading.to_vector();
                    state.x += dx;
                    state.z += dz;

                    let incoming = state.heading.opposite().to_string();
                    add_connection(&mut map, state.x, state.z, incoming);
                    spawn_tile_from_map(&map, &mut entities, state.x, state.z)?;

                    results.push(format!(
                        "Turned RIGHT. Now at ({}, {}) facing {:?}",
                        state.x, state.z, state.heading
                    ));
                }
                _ => return Err(anyhow!("Unknown action: {}", action)),
            }
        }
        Ok(results.join("\n"))
    }
}

// --- Helpers ---

fn sync_from_scene(
    map: &mut HashMap<(i64, i64), HashSet<String>>,
    entities: &mut HashMap<(i64, i64), u64>,
) -> Result<usize> {
    // 1. Query Bevy for all entities with AxiomRemoteAsset
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "bevy/query",
        "id": 1,
        "params": {
            "data": {
                "components": [
                    "bevy_transform::components::transform::Transform",
                    "bevy_ai_remote::AxiomRemoteAsset"
                ],
                "entity": true
            },
            "filter": {
                "with": ["bevy_ai_remote::AxiomRemoteAsset"]
            }
        }
    });

    let agent = ureq::AgentBuilder::new()
        .timeout_read(std::time::Duration::from_secs(5))
        .build();
    let resp = agent.post("http://127.0.0.1:15721").send_json(payload)?;

    let body: Value = resp.into_json()?;
    let mut count = 0;

    if let Some(result) = body.get("result").and_then(|v| v.as_array()) {
        for item in result {
            let entity_id = item.get("entity").and_then(|v| v.as_u64());
            let transform = item
                .get("components")
                .and_then(|c| c.get("bevy_transform::components::transform::Transform"));
            let asset = item
                .get("components")
                .and_then(|c| c.get("bevy_ai_remote::AxiomRemoteAsset"));

            if let (Some(eid), Some(trans), Some(asset)) = (entity_id, transform, asset) {
                let translation = trans.get("translation").and_then(|v| v.as_array());
                let rotation = trans.get("rotation").and_then(|v| v.as_array()); // Quaternion [x,y,z,w]
                let filename = asset.get("filename").and_then(|v| v.as_str());

                if let (Some(pos), Some(rot_quat), Some(fname)) = (translation, rotation, filename)
                {
                    let x = pos.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0).round() as i64;
                    let z = pos.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0).round() as i64;

                    // Recover Connections from Filename + Rotation
                    let rot_y_deg = quat_to_euler_y_deg(rot_quat);
                    let conns = infer_connections(fname, rot_y_deg);

                    // Update Local State
                    map.insert((x, z), conns);
                    entities.insert((x, z), eid);
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

fn quat_to_euler_y_deg(q: &Vec<Value>) -> f64 {
    // Basic quat conversion for Y rotation
    // q = [x, y, z, w]
    let x = q.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = q.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let z = q.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let w = q.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0);

    // Yaw (Y-axis rotation)
    let siny_cosp = 2.0 * (w * y + z * x);
    let cosy_cosp = 1.0 - 2.0 * (x * x + y * y);
    let yaw_rad = siny_cosp.atan2(cosy_cosp);

    let deg = yaw_rad.to_degrees();
    // Normalize to 0, 90, 180, 270
    let rounded = (deg / 90.0).round() * 90.0;
    let normalized = ((rounded as i64 % 360) + 360) % 360;
    normalized as f64
}

fn infer_connections(filename: &str, rot_deg: f64) -> HashSet<String> {
    let mut conns = HashSet::new();
    let r = rot_deg as i64;

    // Map Local Connections to Global based on Rotation
    // 0 deg: Default
    // 90 deg: Rotated CW

    // Base connections for 0 degree rotation
    let base_conns = match filename {
        "road-straight.glb" => vec!["+X", "-X"],
        "road-bend.glb" => vec!["+Z", "+X"], // Based on observation: Bend 0 connects +Z and +X? Or we re-derive.
        // Re-deriving from solve_road_piece:
        // if has_pz && has_nx -> 0.0 => +Z, -X (Wait, logic in solve_road_piece says 0.0 is +Z & -X??)
        // Let's check solve_road_piece logic again:
        // if has_pz && has_nx -> 0.0. OK.
        "road-intersection.glb" => vec!["+X", "-X", "+Z"], // T-Junction.
        // solve_road_piece: 3 conns.
        // if !has_nz -> 0.0. So 0.0 is missing -Z. Has +X, -X, +Z.
        "road-crossroad.glb" => vec!["+X", "-X", "+Z", "-Z"],
        _ => vec![],
    };

    // Helper to rotate a direction string by N * 90 degrees
    let rotate_dir = |d: &str, angle: i64| -> String {
        let dirs = ["-Z", "+X", "+Z", "-X"]; // N, E, S, W
        if let Some(idx) = dirs.iter().position(|&x| x == d) {
            let steps = (angle / 90) as usize;
            return dirs[(idx + steps) % 4].to_string();
        }
        d.to_string()
    };

    // Adjust base conns manually if my base assumptions above are wrong vs solve_road_piece
    // solve_road_piece 0.0 mappings:
    // Straight: +X, -X
    // Bend: +Z, -X (Wait, earlier comment said: "if has_pz && has_nx -> 0.0")
    // Intersection: +X, -X, +Z (missing -Z)
    // Crossroad: All

    let actual_base = match filename {
        "road-straight.glb" => vec!["+X", "-X"],
        "road-bend.glb" => vec!["+Z", "-X"],
        "road-intersection.glb" => vec!["+X", "-X", "+Z"],
        "road-crossroad.glb" => vec!["+X", "-X", "+Z", "-Z"],
        _ => vec![],
    };

    for c in actual_base {
        conns.insert(rotate_dir(c, r));
    }

    conns
}

fn add_connection(map: &mut HashMap<(i64, i64), HashSet<String>>, x: i64, z: i64, conn: String) {
    map.entry((x, z)).or_insert_with(HashSet::new).insert(conn);
}

fn spawn_tile_from_map(
    map: &HashMap<(i64, i64), HashSet<String>>,
    entities: &mut HashMap<(i64, i64), u64>,
    x: i64,
    z: i64,
) -> Result<()> {
    if let Some(conns) = map.get(&(x, z)) {
        let (model, rot) = solve_road_piece(conns)?;
        let res_path = resolve_model_path(&model)?;

        // CHECK AND DELETE OLD ENTITY
        if let Some(old_id) = entities.get(&(x, z)) {
            // Best effort despawn
            let _ = despawn_entity(*old_id);
        }

        // SPAWN NEW ENTITY
        let new_id =
            upload_asset_to_bevy(res_path.to_str().unwrap(), [x as f64, 0.0, z as f64], rot)?;

        // UPDATE MAP
        entities.insert((x, z), new_id);
    }
    Ok(())
}

fn despawn_entity(entity_id: u64) -> Result<()> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "world.spawn_entity", // Using the new Hack: Spawn Request Entity
        "id": 1,
        "params": {
            "components": {
                "bevy_ai_remote::AxiomDespawnRequest": {
                    "target": entity_id
                }
            }
        }
    });

    // We don't necessarily need the response, just send it.
    let _ = ureq::post("http://127.0.0.1:15721").send_json(payload);
    Ok(())
}

fn resolve_model_path(model_name: &str) -> Result<PathBuf> {
    let resource_path = std::env::current_dir()?
        .join("apps")
        .join("axiom")
        .join("resources")
        .join("models")
        .join(model_name);
    if !resource_path.exists() {
        return Err(anyhow!("Model not found: {:?}", resource_path));
    }
    Ok(resource_path)
}

fn solve_road_piece(conns: &HashSet<String>) -> Result<(String, [f64; 3])> {
    let has_px = conns.contains("+X");
    let has_nx = conns.contains("-X");
    let has_pz = conns.contains("+Z");
    let has_nz = conns.contains("-Z");
    let count = conns.len();

    match count {
        4 => Ok(("road-crossroad.glb".to_string(), [0.0, 0.0, 0.0])),
        3 => {
            if !has_nz {
                return Ok(("road-intersection.glb".to_string(), [0.0, 0.0, 0.0]));
            }
            if !has_pz {
                return Ok(("road-intersection.glb".to_string(), [0.0, 180.0, 0.0]));
            }
            if !has_nx {
                return Ok(("road-intersection.glb".to_string(), [0.0, 90.0, 0.0]));
            }
            if !has_px {
                return Ok(("road-intersection.glb".to_string(), [0.0, 270.0, 0.0]));
            }
            Ok(("road-intersection.glb".to_string(), [0.0, 0.0, 0.0]))
        }
        2 => {
            if has_px && has_nx {
                return Ok(("road-straight.glb".to_string(), [0.0, 0.0, 0.0]));
            }
            if has_pz && has_nz {
                return Ok(("road-straight.glb".to_string(), [0.0, 90.0, 0.0]));
            }
            if has_nz && has_px {
                return Ok(("road-bend.glb".to_string(), [0.0, 180.0, 0.0]));
            }
            if has_px && has_pz {
                return Ok(("road-bend.glb".to_string(), [0.0, 90.0, 0.0]));
            }
            if has_pz && has_nx {
                return Ok(("road-bend.glb".to_string(), [0.0, 0.0, 0.0]));
            }
            if has_nx && has_nz {
                return Ok(("road-bend.glb".to_string(), [0.0, 270.0, 0.0]));
            }
            Ok(("road-straight.glb".to_string(), [0.0, 0.0, 0.0]))
        }
        // Handle Single Connections (Dead Ends / Starts) - Defaulting to Straight aligned to connection
        1 => {
            if has_px || has_nx {
                return Ok(("road-straight.glb".to_string(), [0.0, 0.0, 0.0]));
            }
            if has_pz || has_nz {
                return Ok(("road-straight.glb".to_string(), [0.0, 90.0, 0.0]));
            }
            Ok(("road-straight.glb".to_string(), [0.0, 0.0, 0.0]))
        }
        _ => Ok(("road-straight.glb".to_string(), [0.0, 0.0, 0.0])),
    }
}

fn encode_file_base64(path: &PathBuf) -> Result<String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(BASE64.encode(&buffer))
}

fn euler_to_quat(rot: [f64; 3]) -> (f32, f32, f32, f32) {
    use glam::Quat;
    let q = Quat::from_euler(
        glam::EulerRot::XYZ,
        (rot[0] as f32).to_radians(),
        (rot[1] as f32).to_radians(),
        (rot[2] as f32).to_radians(),
    );
    (q.x, q.y, q.z, q.w)
}

fn upload_asset_to_bevy(
    local_path: &str,
    translation: [f64; 3],
    rotation_euler: [f64; 3],
) -> Result<u64> {
    let path = std::path::Path::new(local_path);
    let filename = path.file_name().unwrap().to_string_lossy().to_string();
    let b64_data = encode_file_base64(&path.to_path_buf())?;
    let q = euler_to_quat(rotation_euler);
    let payload = json!({
        "jsonrpc": "2.0", "method": "world.spawn_entity", "id": 1,
        "params": {
            "components": {
                "bevy_ai_remote::AxiomRemoteAsset": { "filename": filename, "data_base64": b64_data },
                "bevy_transform::components::transform::Transform": { "translation": translation, "rotation": [q.0, q.1, q.2, q.3], "scale": [1.0, 1.0, 1.0] }
            }
        }
    });
    let agent = ureq::AgentBuilder::new()
        .timeout_read(std::time::Duration::from_secs(10))
        .build();
    let resp = agent.post("http://127.0.0.1:15721").send_json(payload)?;

    // Parse result to get Entity ID
    let body: Value = resp.into_json()?;
    if let Some(result) = body.get("result") {
        if let Some(id) = result.as_u64() {
            return Ok(id);
        }
    }

    // Fallback if parsing fails (shouldn't happen if BRP works)
    Err(anyhow!("Failed to parse Entity ID from response: {}", body))
}
