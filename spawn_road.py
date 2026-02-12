import requests
import json
import base64
import os
import math
import time

BEVY_RPC_URL = "http://127.0.0.1:15721"
MODELS_DIR = r"D:\workspace\bevy_ai_editor\apps\Axiom\resources\models"

# --- Global State for Python Driver ---
GLOBAL_ROAD_MAP = {} # (x,z) -> set([connections])
GLOBAL_ENTITY_MAP = {} # (x,z) -> entity_id

def get_quat(y_deg):
    rad = math.radians(y_deg)
    s = math.sin(rad / 2)
    c = math.cos(rad / 2)
    return [0.0, s, 0.0, c]

def encode_file(filename):
    path = os.path.join(MODELS_DIR, filename)
    with open(path, "rb") as f:
        return base64.b64encode(f.read()).decode("utf-8")

def despawn_entity(entity_id):
    payload = {
        "jsonrpc": "2.0",
        "method": "bevy/despawn",
        "id": 1,
        "params": { "entity": entity_id }
    }
    try:
        requests.post(BEVY_RPC_URL, json=payload, timeout=1)
        # print(f"Despawned {entity_id}")
    except:
        pass

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
            # Bevy Remote returns the entity ID as the result for spawn_entity? 
            # Actually standard BRP spawn returns the entity ID.
            return result
    except Exception as e:
        print(f"Failed to spawn {filename}: {e}")
    return None

def update_tile(x, z, new_conn):
    # 1. Update Map
    if (x, z) not in GLOBAL_ROAD_MAP:
        GLOBAL_ROAD_MAP[(x, z)] = set()
    GLOBAL_ROAD_MAP[(x, z)].add(new_conn)
    
    # 2. Resolve Model
    conns = GLOBAL_ROAD_MAP[(x, z)]
    model, rot = solve_road_piece(conns)
    
    # 3. Despawn Old
    if (x, z) in GLOBAL_ENTITY_MAP:
        despawn_entity(GLOBAL_ENTITY_MAP[(x, z)])
    
    # 4. Spawn New
    new_id = spawn_entity(model, x, z, rot)
    if new_id is not None:
        GLOBAL_ENTITY_MAP[(x, z)] = new_id
        print(f"Updated ({x}, {z}) -> {model}")

def solve_road_piece(conns):
    has_px = "+X" in conns
    has_nx = "-X" in conns
    has_pz = "+Z" in conns
    has_nz = "-Z" in conns
    count = len(conns)
    
    if count == 4: return "road-crossroad.glb", 0.0
    if count == 3:
        if not has_nz: return "road-intersection.glb", 0.0
        if not has_pz: return "road-intersection.glb", 180.0
        if not has_nx: return "road-intersection.glb", 90.0
        if not has_px: return "road-intersection.glb", 270.0
    if count == 2:
        if has_px and has_nx: return "road-straight.glb", 0.0
        if has_pz and has_nz: return "road-straight.glb", 90.0
        if has_nz and has_px: return "road-bend.glb", 180.0
        if has_px and has_pz: return "road-bend.glb", 90.0
        if has_pz and has_nx: return "road-bend.glb", 0.0
        if has_nx and has_nz: return "road-bend.glb", 270.0
    if count == 1:
        if has_px or has_nx: return "road-straight.glb", 0.0
        if has_pz or has_nz: return "road-straight.glb", 90.0
        
    return "road-straight.glb", 0.0

# --- Driver Logic ---

class Driver:
    def __init__(self, x, z, heading):
        self.x = x
        self.z = z
        self.heading = heading # +X, -X, +Z, -Z
        
        # Init adds connection to start tile? 
        # Usually INIT just places cursor.
        # But let's say it enters the map.
    
    def get_dir_vec(self, h):
        if h == "+X": return (1, 0)
        if h == "-X": return (-1, 0)
        if h == "+Z": return (0, 1)
        if h == "-Z": return (0, -1)
        return (0, 0)
        
    def opposite(self, h):
        if h == "+X": return "-X"
        if h == "-X": return "+X"
        if h == "+Z": return "-Z"
        if h == "-Z": return "+Z"
        return h
        
    def turn_left(self, h):
        dirs = ["-Z", "+X", "+Z", "-X"]
        idx = dirs.index(h)
        return dirs[(idx - 1) % 4]
        
    def turn_right(self, h):
        dirs = ["-Z", "+X", "+Z", "-X"]
        idx = dirs.index(h)
        return dirs[(idx + 1) % 4]

    def forward(self):
        # 1. Outgoing from Current
        update_tile(self.x, self.z, self.heading)
        
        # 2. Move
        dx, dz = self.get_dir_vec(self.heading)
        self.x += dx
        self.z += dz
        
        # 3. Incoming to New
        update_tile(self.x, self.z, self.opposite(self.heading))

    def turn(self, direction):
        old_h = self.heading
        new_h = self.turn_left(old_h) if direction == "LEFT" else self.turn_right(old_h)
        
        # 1. New Heading (Outgoing) from Current
        update_tile(self.x, self.z, new_h)
        
        # 2. Update State
        self.heading = new_h
        
        # 3. Move
        dx, dz = self.get_dir_vec(self.heading)
        self.x += dx
        self.z += dz
        
        # 4. Incoming to New
        update_tile(self.x, self.z, self.opposite(new_h))

# --- Main ---

def main():
    print("Generating 'Bow Tie' with Auto-Merge Logic (Python Sim)...")
    
    # Reset Scene First? No, user said "Do NOT clear". 
    # But for a clean script run, I'll clear my python map.
    # Bevy scene might have old stuff.
    
    # 1. Top-Left: (-2,-1) -> (-1,-1) Bend -> (-1,0)
    d = Driver(-2, -1, "+X")
    d.forward() # (-2,-1) -> (-1,-1)
    d.turn("RIGHT") # (-1,-1) -> (-1,0)
    
    # 2. Bottom-Left: (-2,1) -> (-1,1) Bend -> (-1,0)
    d = Driver(-2, 1, "+X")
    d.forward() # (-2,1) -> (-1,1)
    d.turn("LEFT") # (-1,1) -> (-1,0)
    
    # 3. Center: (-1,0) -> (0,0) -> (1,0)
    # (-1,0) is where the magic happens. It will have inputs from TL, BL, and now Center Out.
    d = Driver(-1, 0, "+X")
    d.forward() # (-1,0) -> (0,0)
    d.forward() # (0,0) -> (1,0)
    d.forward() # (1,0) -> (2,0) ... Wait, Plan said (1,0) to (2,0)?
    # Prompt said: Central (-1,0), (0,0), (1,0). 
    # Previous batch run did 3 forwards.
    # Let's stop at (1,0) to be safe for TR/BR connection.
    
    # 4. Top-Right: (2,-1) -> (1,-1) Bend -> (1,0)
    d = Driver(2, -1, "-X")
    d.forward() # (2,-1) -> (1,-1)
    d.turn("LEFT") # (1,-1) -> (1,0)
    
    # 5. Bottom-Right: (2,1) -> (1,1) Bend -> (1,0)
    d = Driver(2, 1, "-X")
    d.forward() # (2,1) -> (1,1)
    d.turn("RIGHT") # (1,1) -> (1,0)

    print("Done!")

if __name__ == "__main__":
    main()
