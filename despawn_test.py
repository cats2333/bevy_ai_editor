import requests
import json

BEVY_RPC_URL = "http://127.0.0.1:15721"

def despawn_entity(entity_id):
    # Handle the weird format {'entity': ID} if needed, or just ID
    # The output was: {'entity': 4294967261}
    # Bevy Remote usually expects integer ID directly.
    
    eid = entity_id
    if isinstance(entity_id, dict):
        eid = entity_id.get('entity')
        
    print(f"Attempting to despawn entity: {eid}")
        
    payload = {
        "jsonrpc": "2.0",
        "method": "bevy/despawn",
        "id": 1,
        "params": {
            "entity": eid 
        }
    }
    
    try:
        res = requests.post(BEVY_RPC_URL, json=payload, timeout=2)
        print(f"Response Status: {res.status_code}")
        print(f"Response Body: {res.text}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    # TARGET ID from previous run
    TARGET_ID = 4294967261
    despawn_entity(TARGET_ID)
