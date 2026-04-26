import json
import struct

def make_glb(json_dict):
    json_bytes = json.dumps(json_dict, separators=(',', ':')).encode('utf-8')
    pad_len = (4 - (len(json_bytes) % 4)) % 4
    json_bytes += b' ' * pad_len
    
    bin_bytes = b''
    total_len = 12 + 8 + len(json_bytes) + 8 + len(bin_bytes)
    header = struct.pack('<4sII', b'glTF', 2, total_len)
    json_chunk_header = struct.pack('<I4s', len(json_bytes), b'JSON')
    bin_chunk_header = struct.pack('<I4s', len(bin_bytes), b'BIN\x00')
    
    return header + json_chunk_header + json_bytes + bin_chunk_header + bin_bytes

# Adult: ratio ~ 7.5
vrm_json = {
    "asset": { "version": "2.0" },
    "scenes": [{ "nodes": [0] }],
    "nodes": [
        { "name": "root", "children": [1, 2] },
        { "name": "head", "translation": [0.0, 1.4, 0.0] },
        { "name": "neck", "translation": [0.0, 1.3, 0.0] }
    ],
    "extensionsUsed": ["VRMC_vrm"],
    "extensions": {
        "VRMC_vrm": {
            "humanoid": {
                "humanBones": {
                    "head": { "node": 1 },
                    "neck": { "node": 2 }
                }
            }
        }
    }
}

# Child: ratio ~ 4.0
child_vrm_json = {
    "asset": { "version": "2.0" },
    "scenes": [{ "nodes": [0] }],
    "nodes": [
        { "name": "root", "children": [1, 2] },
        { "name": "head", "translation": [0.0, 0.8, 0.0] },
        { "name": "neck", "translation": [0.0, 0.7, 0.0] }
    ],
    "extensionsUsed": ["VRMC_vrm"],
    "extensions": {
        "VRMC_vrm": {
            "humanoid": {
                "humanBones": {
                    "head": { "node": 1 },
                    "neck": { "node": 2 }
                }
            }
        }
    }
}

with open("libs/avatar-engine/tests/fixtures/adult.vrm", "wb") as f:
    f.write(make_glb(vrm_json))

with open("libs/avatar-engine/tests/fixtures/child.vrm", "wb") as f:
    f.write(make_glb(child_vrm_json))
