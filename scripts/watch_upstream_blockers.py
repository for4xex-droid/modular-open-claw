#!/usr/bin/env python3
import urllib.request
import json
import sys

def check_crate_target(crate_name: str, target_version: str) -> dict:
    """
    Check if the specified crate has reached or exceeded the target version.
    Returns a dict with 'reached' (bool) and 'current_version' (str).
    """
    url = f"https://crates.io/api/v1/crates/{crate_name}"
    req = urllib.request.Request(url, headers={"User-Agent": "Aiome-Upstream-Watcher/1.0"})
    
    with urllib.request.urlopen(req) as response:
        data = json.loads(response.read().decode('utf-8'))
        
    max_version = data.get("crate", {}).get("max_stable_version", "0.0.0")
    
    # Very simple version comparison (assumes semver X.Y.Z format)
    def parse_version(v):
        try:
            return [int(x) for x in v.split('.')]
        except ValueError:
            return [0, 0, 0]
            
    current_parts = parse_version(max_version)
    target_parts = parse_version(target_version)
    
    reached = current_parts >= target_parts
    
    return {
        "reached": reached,
        "current_version": max_version
    }

if __name__ == "__main__":
    # Define our targets based on Phase 3 Epic
    targets = [
        {"crate": "serenity", "version": "0.13.0", "issue": "Issue A (Discord Bot TLS)"},
        {"crate": "tauri", "version": "3.0.0", "issue": "Issue D (Tauri GTK4/unic)"},
        # For extism, we wait for a release that bumps wasmtime. 
        # Currently it's at 1.21.0. We will alert if it goes above that.
        {"crate": "extism", "version": "1.22.0", "issue": "Issue C (Extism Wasmtime 43+)"}
    ]
    
    all_unblocked = []
    
    for target in targets:
        print(f"Checking {target['crate']} (waiting for >= {target['version']})...")
        try:
            result = check_crate_target(target["crate"], target["version"])
            print(f"  Current stable version: {result['current_version']}")
            if result["reached"]:
                print(f"  [!] UNBLOCKED: {target['crate']} has reached {target['version']}!")
                all_unblocked.append(target)
            else:
                print(f"  [ ] STILL BLOCKED.")
        except Exception as e:
            print(f"  Error checking {target['crate']}: {e}")
            
    if all_unblocked:
        print("\n" + "="*50)
        print("🚨 ACTION REQUIRED: UPSTREAM DEPENDENCIES UNBLOCKED! 🚨")
        print("="*50)
        for t in all_unblocked:
            print(f"- {t['issue']} can now be executed. Crate {t['crate']} is ready.")
        sys.exit(1) # Exit with 1 to trigger CI/Heartbeat alerts
    else:
        print("\nAll watched upstream dependencies are still blocked.")
        sys.exit(0)
