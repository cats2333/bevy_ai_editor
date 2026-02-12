import requests
import json
import base64
import os
import math
import time

BEVY_RPC_URL = "http://127.0.0.1:15721"
MODELS_DIR = r"D:\workspace\bevy_ai_editor\apps\Axiom\resources\models"

# --- Global State for Python Driver ---
# NOTE: In a real persistent app this would be kept in memory.
# For this test script, we assume a fresh start or we manually manage it.
GLOBAL_ROAD_MAP = {} 
GLOBAL_ENTITY_MAP = {}

def get_quat(y_deg):
    rad = math.radians(y_deg)
    s = math.sin(rad / 2)
    c = math.cos(rad / 2)
    return [0.0, s, 0.0, c]

def encode_file(filename):
    path = os.path.join(MODELS_DIR, filename)
    with open(path, "rb") as f:
        return base64.b64encode(f.read()).decode("utf-8")

def spawn_entity(filename, x, z, y_rot):
    b64 = encode_file(filename)
    quat = get_quat(y_rot)
    
    payload = {
        "jsonrpc": "2.0",
        "method": "world.spawn_entity",
        "id": 1,
        "params": {
            "components": {
                "bevy_ai_remote::AxiomRemoteAsset": {
                    "filename": filename,
                    "data_base64": b64
                },
                "bevy_transform::components::transform::Transform": {
                    "translation": [float(x), 0.0, float(z)],
                    "rotation": quat,
                    "scale": [1.0, 1.0, 1.0]
                }
            }
        }
    }
    
    try:
        res = requests.post(BEVY_RPC_URL, json=payload, timeout=2)
        if res.status_code == 200:
            result = res.json().get("result")
            return result
    except Exception as e:
        print(f"Failed to spawn {filename}: {e}")
    return None

def main():
    print("Test: Spawning Single Straight Road at (0,0)...")
    
    # 1. Clear everything first (Optional, but good for clean test)
    # Actually, let's just spawn.
    
    # Spawn Straight facing East (+X)
    eid = spawn_entity("road-straight.glb", 0, 0, 0.0)
    print(f"Spawned Straight Road. Entity ID: {eid}")

if __name__ == "__main__":
    main()
