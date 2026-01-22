import requests
import json
import time

class BevyAiClient:
    def __init__(self, host="127.0.0.1", port=15703):
        self.base_url = f"http://{host}:{port}"
        print(f"[CONN] Connected to Bevy AI Editor at {self.base_url}")

    def spawn(self, asset_path, x, z, y=0.0, scale=1.0, rotation=0.0):
        """
        Spawn an entity in the game world.
        """
        payload = {
            "asset_path": asset_path,
            "x": float(x),
            "y": float(y),
            "z": float(z),
            "scale": float(scale),
            "rotation": float(rotation)
        }
        try:
            resp = requests.post(f"{self.base_url}/spawn", json=payload, timeout=1.0)
            return resp.status_code == 200
        except requests.exceptions.RequestException as e:
            print(f"[ERR] Failed to spawn {asset_path}: {e}")
            return False

    def save_scene(self, filename="level_latest.json"):
        """Save the current scene to a JSON file on the server side."""
        payload = {"filename": filename}
        try:
            resp = requests.post(f"{self.base_url}/save", json=payload, timeout=1.0)
            if resp.status_code == 200:
                print(f"[SAVE] Save request sent for {filename}")
                return True
        except requests.exceptions.RequestException as e:
            print(f"[ERR] Failed to save scene: {e}")
            return False
        return False

    def get_entities(self):
        """Get list of all AI-spawned entities."""
        try:
            resp = requests.get(f"{self.base_url}/entities", timeout=1.0)
            if resp.status_code == 200:
                return resp.json()
        except requests.exceptions.RequestException as e:
            print(f"[ERR] Failed to get entities: {e}")
        return []

    def clear_all(self):
        print("[WARN] clear_all not implemented on server side yet.")

# Example usage
if __name__ == "__main__":
    client = BevyAiClient()
    
    print("[INFO] Spawning a forest...")
    for i in range(5):
        for j in range(5):
            client.spawn("models/nature/tree.glb", x=i*2.0, z=j*2.0, rotation=0.5 * i)
            time.sleep(0.05)
    
    print("[DONE] Done!")
