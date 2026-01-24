import sys
import os
import math
import random

# Add python directory to path
sys.path.append(os.path.join(os.path.dirname(__file__), '../../python'))
from bevy_ai_client import BevyAiClient

def main():
    client = BevyAiClient()
    
    print("[INIT] Starting procedural village generation...")
    
    # 1. Configuration
    TILE_SIZE = 1.0  # Corrected tile size from manifest (was 4.0)
    ORIGIN = (0, 0)
    
    # Asset paths
    ROAD_STRAIGHT = "models/city_road/road-straight.glb"
    ROAD_INTERSECTION = "models/city_road/road-intersection.glb"
    ROAD_END = "models/city_road/road-end.glb" # Optional, looks better under the house
    
    HOUSE_VARIANTS = [
        "models/city_city/building-type-a.glb",
        "models/city_city/building-type-b.glb",
        "models/city_city/building-type-c.glb"
    ]

    # 2. Define the Graph (Topology)
    # Structure: A central intersection with 3 arms extending OUTWARD.
    # ARM LENGTH: 4 tiles of road, then house at the 5th tile.
    
    layout = []
    
    # --- 1. Central Hub ---
    layout.append({"x": 0, "z": 0, "type": "hub", "rot": 0})
    
    # --- 2. Arm South (Z+) ---
    # Extending downwards: (0,1), (0,2), (0,3), (0,4)
    # HYPOTHESIS: Model is X-aligned. We need to rotate 90 to make it Z-aligned (vertical).
    for z in range(1, 5):
        layout.append({"x": 0, "z": z, "type": "road", "rot": 90}) 
    # House at end (0, 5)
    layout.append({"x": 0, "z": 5, "type": "house", "rot": 180})

    # --- 3. Arm East (X+) ---
    # Extending right: (1,0), (2,0), (3,0), (4,0)
    # HYPOTHESIS: Model is X-aligned. No rotation needed to be horizontal.
    for x in range(1, 5):
        layout.append({"x": x, "z": 0, "type": "road", "rot": 0}) 
    # House at end (5, 0)
    layout.append({"x": 5, "z": 0, "type": "house", "rot": 90}) 

    # --- 4. Arm West (X-) ---
    # Extending left: (-1,0), (-2,0), (-3,0), (-4,0)
    # HYPOTHESIS: Model is X-aligned. No rotation needed to be horizontal.
    for x in range(1, 5):
        layout.append({"x": -x, "z": 0, "type": "road", "rot": 0})
    # House at end (-5, 0)
    layout.append({"x": -5, "z": 0, "type": "house", "rot": -90})


    # 3. Execution Loop
    for item in layout:
        # Convert Grid coordinates to World coordinates
        world_x = item["x"] * TILE_SIZE
        world_z = item["z"] * TILE_SIZE
        
        # Convert degrees to radians
        rot_rad = math.radians(item["rot"])
        
        spawned_something = False
        
        if item["type"] == "hub":
            print(f"  -> Spawning Hub at ({item['x']}, {item['z']})")
            client.spawn(ROAD_INTERSECTION, x=world_x, z=world_z, rotation=rot_rad)
            spawned_something = True
            
        elif item["type"] == "road":
            print(f"  -> Spawning Road at ({item['x']}, {item['z']})")
            client.spawn(ROAD_STRAIGHT, x=world_x, z=world_z, rotation=rot_rad)
            spawned_something = True
            
        elif item["type"] == "house":
            print(f"  -> Spawning House at ({item['x']}, {item['z']})")
            
            # FIX: Do NOT spawn a road under the house. 
            # It causes the house to stack on top of the road collider, making it float.
            # client.spawn(ROAD_END, x=world_x, z=world_z, rotation=rot_rad)
            
            # Place the house directly
            house_asset = random.choice(HOUSE_VARIANTS)
            client.spawn(house_asset, x=world_x, z=world_z, rotation=rot_rad)
            spawned_something = True

    print("[DONE] Village generation complete.")

if __name__ == "__main__":
    main()
