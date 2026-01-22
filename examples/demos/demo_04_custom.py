import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), '../../python'))
from bevy_ai_client import BevyAiClient
import math
import time

client = BevyAiClient()

print("[START] Spawning Custom GLB Trees...")

# Circle of Real Trees
radius = 15.0
count = 12

for i in range(count):
    angle = (i / count) * math.pi * 2
    x = math.cos(angle) * radius
    z = math.sin(angle) * radius
    
    # Use the GLB path relative to assets folder
    # Note: rotation is in radians
    client.spawn("models/tree-small.glb", x=x, z=z, y=0.0, scale=1.0, rotation=-angle)
    
    time.sleep(0.1)

print("[DONE] Custom Trees Done!")
