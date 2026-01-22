import sys
import os
import time
import math
import random

# Ensure we can import the python client
sys.path.append(os.path.join(os.path.dirname(__file__), "../../"))
from python.bevy_ai_client import BevyAiClient

def main():
    client = BevyAiClient()

    print("[START] Action! Starting Cinematic Build...")

    # --- Phase 1: The Core (金色核心) ---
    print("Phase 1: The Core")
    client.spawn("builtin://sphere/yellow", x=0, y=2, z=0, scale=3.0)
    time.sleep(1.0) # Pause for dramatic effect

    # --- Phase 2: The Inner Ring (红色护盾) ---
    print("Phase 2: Inner Ring")
    radius = 5.0
    count = 12
    for i in range(count):
        angle = (i / count) * math.pi * 2
        x = math.cos(angle) * radius
        z = math.sin(angle) * radius

        # Rapid fire spawning
        client.spawn("builtin://cube/red", x=x, y=0.5, z=z, rotation=angle)
        time.sleep(0.05)

    time.sleep(0.5)

    # --- Phase 3: The Spiral Growth (绿色森林/建筑) ---
    print("Phase 3: Spiral Growth")
    spirals = 3
    points_per_spiral = 30
    max_radius = 20.0

    for i in range(points_per_spiral * spirals):
        progress = i / (points_per_spiral * spirals)
        current_radius = 6.0 + (max_radius - 6.0) * progress
        angle = i * 0.5  # Controls tightness of spiral

        x = math.cos(angle) * current_radius
        z = math.sin(angle) * current_radius

        # Scale grows as we go out
        scale_y = 1.0 + progress * 3.0

        # Use capsules to look like trees/towers
        client.spawn("builtin://capsule/green", x=x, y=scale_y, z=z, scale=1.0)

        # Rhythm: Accelerate slightly as we go out
        delay = max(0.02, 0.1 - (progress * 0.08))
        time.sleep(delay)

    time.sleep(0.5)

    # --- Phase 4: The Outer Sentinels (蓝色巨塔) ---
    print("Phase 4: Outer Sentinels")
    tower_count = 8
    tower_radius = 25.0

    for i in range(tower_count):
        angle = (i / tower_count) * math.pi * 2
        x = math.cos(angle) * tower_radius
        z = math.sin(angle) * tower_radius

        client.spawn("builtin://cube/blue", x=x, y=4, z=z, scale=2.0)
        time.sleep(0.2) # Boom... Boom... Boom... heavy impact

    print("[DONE] Cut! Scene generated.")

if __name__ == "__main__":
    main()
