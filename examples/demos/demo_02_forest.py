import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), '../../python'))
from bevy_ai_client import BevyAiClient
import random
import time
import math

client = BevyAiClient()

print("[START] Spawning Random Forest (GLB)...")

# Generate 50 trees in a 40x40 area centered at (0,0)
for i in range(50):
    rx = random.uniform(-20, 20)
    rz = random.uniform(-20, 20)
    
    # Random variation (Natural Scale)
    scale = random.uniform(0.8, 1.2)
    rotation = random.uniform(0, math.pi * 2)
    
    # Spawn the GLB model
    client.spawn("models/tree-small.glb", x=rx, z=rz, y=0.0, scale=scale, rotation=rotation)
    
    # Rock (Builtin Fallback) - scattered on ground
    if random.random() < 0.2:
        rock_x = rx + random.uniform(-1.5, 1.5)
        rock_z = rz + random.uniform(-1.5, 1.5)
        client.spawn("builtin://cube/gray", x=rock_x, z=rock_z, y=0.1, scale=random.uniform(0.3, 0.5))

    time.sleep(0.03)

print("[DONE] Forest Done!")
