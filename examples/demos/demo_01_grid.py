import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), '../../python'))
from bevy_ai_client import BevyAiClient
import time

client = BevyAiClient()

print("[START] Spawning Centered Chessboard Grid...")

# Size: 10x10 grid, centered at (0,0)
# Range -5 to 5 means x will be -5, -4, ... 0 ... 4
grid_radius = 5 

for x in range(-grid_radius, grid_radius):
    for z in range(-grid_radius, grid_radius):
        # Checkerboard pattern
        is_white = (x + z) % 2 == 0
        color = "white" if is_white else "black"
        
        # Ground tile
        # Scale 2.0 covers the 2.0 step size perfectly
        client.spawn(f"builtin://cube/{color}", x=x*2.0, z=z*2.0, y=0.0, scale=2.0)
        
        # Add a piece on the center ring (3x3 area)
        # abs(x) <= 1 and abs(z) <= 1 means the center 4 tiles
        if abs(x) <= 1 and abs(z) <= 1: 
            client.spawn("builtin://capsule/red", x=x*2.0, z=z*2.0, y=1.5, scale=0.8)

        # Add pieces on the outer ring (boundary)
        # if x == -grid_radius or x == grid_radius-1 or ...
        
        time.sleep(0.02)

print("[DONE] Grid Done!")
