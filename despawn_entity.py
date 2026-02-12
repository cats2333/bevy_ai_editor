import requests
import json

BEVY_RPC_URL = "http://127.0.0.1:15721"

def despawn_entity(entity_id):
    # Handle the weird format {'entity': ID} if needed, or just ID
    eid = entity_id
    if isinstance(entity_id, dict):
        eid = entity_id.get('entity')
    
    if eid is None:
        return

    print(f"Requesting despawn for entity: {eid} via AxiomDespawnRequest")
        
    # Spawn a request entity that carries the target ID
    payload = {
        "jsonrpc": "2.0",
        "method": "world.spawn_entity",
        "id": 1,
        "params": {
            "components": {
                "bevy_ai_remote::AxiomDespawnRequest": {
                    "target": eid
                }
            }
        }
    }
    
    try:
        res = requests.post(BEVY_RPC_URL, json=payload, timeout=2)
        print(f"Despawn Request Sent. Status: {res.status_code}")
        # print(res.text)
    except Exception as e:
        print(f"Error sending despawn request: {e}")

if __name__ == "__main__":
    # TARGET ID from previous run
    TARGET_ID = 4294967261
    despawn_entity(TARGET_ID)
