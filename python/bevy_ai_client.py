import requests
import json
import time

class BevyAiClient:
    def __init__(self, host="127.0.0.1", port=15703):
        self.base_url = f"http://{host}:{port}"
        print(f"[CONN] Connected to Bevy AI Editor at {self.base_url}")

    def spawn(self, asset_path, x, z, y=0.0, scale=1.0, rotation=0.0, name=None, physics="static"):
        """
        Spawn an entity.
        :param physics: "static" (default), "dynamic" (gravity), "kinematic" (scripted)
        :param name: Unique name for joint connection.
        """
        payload = {
            "asset_path": asset_path,
            "x": float(x),
            "y": float(y),
            "z": float(z),
            "scale": float(scale),
            "rotation": float(rotation),
            "name": name,
            "physics": physics
        }
        try:
            resp = requests.post(f"{self.base_url}/spawn", json=payload, timeout=1.0)
            return resp.status_code == 200
        except requests.exceptions.RequestException as e:
            print(f"[ERR] Failed to spawn {asset_path}: {e}")
            return False

    def joint(self, entity1, entity2, type="fixed", anchor1=(0,0,0), anchor2=(0,0,0), limits=None, motor=False, name=None):
        """
        Connect two entities with a joint.
        :param type: "fixed", "revolute", "spherical"
        :param motor: Enable motor for this joint?
        :param name: Name of the joint (required for motor control)
        """
        payload = {
            "entity1": entity1,
            "entity2": entity2,
            "type": type,
            "anchor1": anchor1,
            "anchor2": anchor2,
            "limits": limits,
            "motor": motor,
            "name": name
        }
        try:
            resp = requests.post(f"{self.base_url}/joint", json=payload, timeout=1.0)
            return resp.status_code == 200
        except requests.exceptions.RequestException as e:
            print(f"[ERR] Failed to create joint: {e}")
            return False

    def set_motor(self, joint_name, target, stiffness=10.0, damping=1.0):
        """
        Control a motorized joint.
        :param target: Target angle (radians)
        """
        payload = {
            "joint_name": joint_name,
            "target": float(target),
            "stiffness": float(stiffness),
            "damping": float(damping)
        }
        try:
            resp = requests.post(f"{self.base_url}/motor", json=payload, timeout=0.5)
            return resp.status_code == 200
        except requests.exceptions.RequestException as e:
            # Silent fail for motor updates to avoid spam
            return False

    def save_scene(self, filename="level_latest.json"):
        payload = {"filename": filename}
        try:
            resp = requests.post(f"{self.base_url}/save", json=payload, timeout=1.0)
            if resp.status_code == 200:
                print(f"[SAVE] Save request sent for {filename}")
                return True
        except requests.exceptions.RequestException as e:
            print(f"[ERR] Failed to save scene: {e}")
            return False

    def get_entities(self):
        try:
            resp = requests.get(f"{self.base_url}/entities", timeout=1.0)
            if resp.status_code == 200:
                return resp.json()
        except requests.exceptions.RequestException as e:
            print(f"[ERR] Failed to get entities: {e}")
        return []

# Example usage
if __name__ == "__main__":
    client = BevyAiClient()
    # Test Motor
    client.spawn("builtin://sphere/green", 0, 0, 5, name="base", physics="static")
    client.spawn("builtin://capsule/red", 0, 0, 3, name="arm", physics="dynamic")
    
    # Connect with a motor
    client.joint("base", "arm", type="revolute", 
                 anchor1=(0, -1, 0), anchor2=(0, 1, 0), 
                 motor=True, name="elbow_motor")
    
    print("Moving arm...")
    import math
    for i in range(100):
        angle = math.sin(i * 0.1) * 1.5
        client.set_motor("elbow_motor", angle)
        time.sleep(0.05)
