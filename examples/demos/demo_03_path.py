import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), '../../python'))
from bevy_ai_client import BevyAiClient
import math
import time

client = BevyAiClient()

print("[START] Spawning Circular Path & Lights...")

# Circle radius
radius = 25.0
steps = 60

for i in range(steps):
    # Angle in radians
    angle = (i / steps) * math.pi * 2
    
    x = math.cos(angle) * radius
    z = math.sin(angle) * radius
    
    # Path (Yellow Plane) - rotated to face tangent? (Optional, just cubes for now)
    client.spawn("builtin://cube/yellow", x=x, z=z, y=0.1, scale=1.5)
    
    # Street Light every 5 steps
    if i % 5 == 0:
        # Pole - slightly outside the path
        lx = x * 1.1
        lz = z * 1.1
        
        # Pole
        client.spawn("builtin://capsule/gray", x=lx, z=lz, y=2.0, scale=0.15)
        # Light Bulb
        client.spawn("builtin://sphere/white", x=lx, z=lz, y=4.0, scale=0.3)

    time.sleep(0.02)

print("[DONE] Path Done!")
