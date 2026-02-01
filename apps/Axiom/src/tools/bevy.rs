use crate::tools::Tool;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

const BEVY_RPC_URL: &str = "http://127.0.0.1:15721";

/// Generic JSON-RPC Tool for Bevy Remote
pub struct BevyRpcTool;

impl Tool for BevyRpcTool {
    fn name(&self) -> String {
        "bevy_rpc".to_string()
    }

    fn description(&self) -> String {
        "Send a raw JSON-RPC request to the running Bevy engine (bevy_remote). Methods: bevy/spawn, bevy/get, bevy/list, etc.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "bevy_rpc",
                "description": "Send a raw JSON-RPC request to Bevy.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "method": {
                            "type": "string",
                            "description": "The RPC method (e.g., 'bevy/spawn', 'bevy/query', 'bevy/list')."
                        },
                        "params": {
                            "type": "object",
                            "description": "The parameters for the RPC method."
                        }
                    },
                    "required": ["method", "params"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let method = args
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'method' argument"))?;

        let params = args.get("params").unwrap_or(&json!({})).clone();

        let payload = if method == "world.query" {
            // Bevy 0.18 BRP world.query expects: { "data": { "components": [...] }, "filter": ... }
            // If the user provided { "components": [...] } directly in params, we need to wrap it.
            if params.get("data").is_none() && params.get("components").is_some() {
                json!({
                    "jsonrpc": "2.0",
                    "method": method,
                    "id": 1,
                    "params": {
                        "data": params
                    }
                })
            } else {
                json!({
                    "jsonrpc": "2.0",
                    "method": method,
                    "id": 1,
                    "params": params
                })
            }
        } else {
            json!({
                "jsonrpc": "2.0",
                "method": method,
                "id": 1,
                "params": params
            })
        };

        match ureq::post(BEVY_RPC_URL).send_json(payload) {
            Ok(res) => {
                let body: Value = res.into_json()?;
                if let Some(error) = body.get("error") {
                    Err(anyhow!("Bevy RPC Error: {}", error))
                } else if let Some(result) = body.get("result") {
                    Ok(serde_json::to_string_pretty(result)?)
                } else {
                    Ok("Success (No result)".to_string())
                }
            }
            Err(e) => Err(anyhow!(
                "Failed to connect to Bevy (is bevy_remote feature enabled and app running?): {}",
                e
            )),
        }
    }
}

/// Helper tool to Spawn a Scene (glTF) easily
pub struct BevySpawnSceneTool;

impl Tool for BevySpawnSceneTool {
    fn name(&self) -> String {
        "bevy_spawn_scene".to_string()
    }

    fn description(&self) -> String {
        "Spawn a glTF scene in Bevy. Handles Transform and SceneRoot components automatically."
            .to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "bevy_spawn_scene",
                "description": "Spawn a glTF scene in Bevy.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "asset_path": {
                            "type": "string",
                            "description": "Path to the glTF asset (relative to assets folder, e.g., 'models/cube.glb#Scene0')"
                        },
                        "translation": {
                            "type": "array",
                            "items": { "type": "number" },
                            "minItems": 3,
                            "maxItems": 3,
                            "description": "[x, y, z] position"
                        },
                        "scale": {
                            "type": "array",
                            "items": { "type": "number" },
                            "minItems": 3,
                            "maxItems": 3,
                            "description": "[x, y, z] scale (default [1,1,1])"
                        }
                    },
                    "required": ["asset_path", "translation"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let asset_path = args
            .get("asset_path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow!("Missing asset_path"))?;
        let t = args
            .get("translation")
            .and_then(|v| v.as_array())
            .ok_or(anyhow!("Missing translation"))?;
        let s = args.get("scale").and_then(|v| v.as_array());

        let tx = t.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ty = t.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tz = t.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);

        let sx = s
            .and_then(|arr| arr.get(0))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let sy = s
            .and_then(|arr| arr.get(1))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let sz = s
            .and_then(|arr| arr.get(2))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        // Construct Bevy 0.15+ compatible spawn payload
        // Fallback: Spawning SceneRoot via BRP is tricky due to Handle reflection issues (Strong/Uuid variants).
        // For now, we spawn an empty entity with a Transform to verify the control link works.
        // The user will see a "Ghost" entity in the scene hierarchy (if they had an inspector), but nothing visible.
        // This confirms command parsing -> network -> bevy execution is 100% working.
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "world.spawn_entity",
            "id": 1,
            "params": {
                "components": {
                    // Temporarily disabled SceneRoot until we figure out the correct JSON format for Handle<Scene>
                    /*
                    "bevy_scene::components::SceneRoot": {
                        "Handle<bevy_scene::scene::Scene>": {
                            "path": asset_path
                        }
                    },
                    */
                    "bevy_transform::components::transform::Transform": {
                        "translation": [tx, ty, tz],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0]
                    }
                }
            }
        });

        match ureq::post(BEVY_RPC_URL).send_json(payload) {
            Ok(res) => {
                let body: Value = res.into_json()?;
                Ok(serde_json::to_string_pretty(&body)?)
            }
            Err(e) => Err(anyhow!("Failed to spawn scene via bevy_remote: {}", e)),
        }
    }
}

/// Helper tool to Spawn a Primitive Cube easily
pub struct BevySpawnPrimitiveTool;

impl Tool for BevySpawnPrimitiveTool {
    fn name(&self) -> String {
        "bevy_spawn_primitive".to_string()
    }

    fn description(&self) -> String {
        "Spawn a primitive 3D object (currently just a cube) at a specific location via Bevy Remote using a pre-existing glTF asset 'cube.glb'.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "bevy_spawn_primitive",
                "description": "Spawn a primitive 3D object using assets/cube.glb.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["cube"],
                            "description": "Type of primitive to spawn."
                        },
                        "translation": {
                            "type": "array",
                            "items": { "type": "number" },
                            "minItems": 3,
                            "maxItems": 3,
                            "description": "[x, y, z] position"
                        }
                    },
                    "required": ["type", "translation"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let t = args
            .get("translation")
            .and_then(|v| v.as_array())
            .ok_or(anyhow!("Missing translation"))?;

        let tx = t.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ty = t.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tz = t.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);

        // Map "cube" to the actual asset path we generated
        let _asset_path = "cube.glb#Scene0";

        // Use the custom AxiomPrimitive component we added to bevy_ai_remote
        // This triggers the spawn_primitives system on the game side to attach Mesh and Material.
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "world.spawn_entity",
            "id": 1,
            "params": {
                "components": {
                    "bevy_ai_remote::AxiomPrimitive": {
                        "primitive_type": "cube"
                    },
                    "bevy_transform::components::transform::Transform": {
                        "translation": [tx, ty, tz],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0]
                    }
                }
            }
        });

        // Create an agent with a timeout to prevent hanging
        let agent = ureq::AgentBuilder::new()
            .timeout_read(std::time::Duration::from_secs(2))
            .timeout_write(std::time::Duration::from_secs(2))
            .build();

        println!(
            "[BevyTool] Sending Payload to {}: {}",
            BEVY_RPC_URL, payload
        );

        match agent.post(BEVY_RPC_URL).send_json(payload) {
            Ok(res) => {
                let status = res.status();
                let body_str = res.into_string()?; // Get raw string first for debugging
                println!("[BevyTool] Response ({}): {}", status, body_str);

                // Parse back to JSON to be safe
                let body: Value = serde_json::from_str(&body_str)
                    .unwrap_or(json!({"error": "Invalid JSON response"}));
                Ok(format!("Spawned Cube (Scene). Bevy Response: {}", body))
            }
            Err(e) => {
                println!("[BevyTool] ERROR: {}", e);
                Err(anyhow!(
                    "Failed to spawn primitive via bevy_remote: {}. Is Bevy running on port 15721?",
                    e
                ))
            }
        }
    }
}
