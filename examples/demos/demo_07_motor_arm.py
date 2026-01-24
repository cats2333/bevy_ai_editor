import time
import math
import sys
import os

# Ensure we can import the client
sys.path.append(os.path.join(os.path.dirname(__file__), "../../"))
from python.bevy_ai_client import BevyAiClient

def main():
    client = BevyAiClient()
    print("[Demo] Motorized Arm Test")

    # 1. Spawn Base (Kinematic to ensure it hangs in air)
    print("1. Spawning Base...")
    # Height 5 (visible range)
    client.spawn("builtin://sphere/green", 0, 5, 0, physics="kinematic", name="base")

    # 2. Spawn Dynamic Arm
    print("2. Spawning Arm...")
    client.spawn("builtin://capsule/red", 2, 5, 0, rotation=1.57, physics="dynamic", name="arm")

    # 3. Connect with Motorized Joint
    print("3. Connecting Joint (Motor=True)...")
    
    # Base: (0, 5, 0)
    # Arm:  (2, 5, 0) -> Horizontal
    # Joint at (0, 5, 0).
    client.joint(
        "base", "arm", 
        type="revolute", 
        anchor1=[0,0,0], 
        anchor2=[-2,0,0], 
        motor=True, 
        name="shoulder"
    )

    print("Wait for physics engine to initialize joint...")
    time.sleep(1.0)

    print("4. Starting Motor Loop (High Power)...")
    t = 0.0
    try:
        while True:
            # Sine wave from -90 to +90 degrees (amplitude 1.5 rad)
            target_angle = math.sin(t) * 1.5
            
            # Massive stiffness to force movement
            client.set_motor("shoulder", target_angle, stiffness=10000.0, damping=100.0)
            
            time.sleep(0.05) 
            t += 0.1
    except KeyboardInterrupt:
        print("Stopped.")

if __name__ == "__main__":
    main()
