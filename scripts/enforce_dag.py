#!/usr/bin/env python3
import json
import subprocess
import sys
from pathlib import Path

def get_metadata():
    try:
        result = subprocess.run(
            ["cargo", "metadata", "--format-version", "1"],
            capture_output=True,
            text=True,
            check=True
        )
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as e:
        print(f"Error running cargo metadata: {e.stderr}", file=sys.stderr)
        sys.exit(1)

def is_in_dir(manifest_path, dir_name):
    return f"/{dir_name}/" in manifest_path

def main():
    metadata = get_metadata()
    workspace_members = metadata.get("workspace_members", [])
    packages = {pkg["id"]: pkg for pkg in metadata.get("packages", [])}
    
    violations = []
    
    # Pre-calculate paths for workspace members
    member_pkgs = [packages[pkg_id] for pkg_id in workspace_members]
    
    for pkg in member_pkgs:
        pkg_name = pkg["name"]
        manifest_path = pkg["manifest_path"]
        
        # Check dependencies
        for dep in pkg.get("dependencies", []):
            dep_name = dep["name"]
            
            # Optional dependencies are feature-gated and not always resolved;
            # they do not represent a hard architectural coupling.
            if dep.get("optional", False):
                continue
            
            # Find the dependency package in workspace if it's a workspace member
            # Note: cargo metadata dependencies array does not contain manifest_path directly, 
            # so we match by name.
            dep_pkg = next((p for p in member_pkgs if p["name"] == dep_name), None)
            if not dep_pkg:
                continue # External dependency, skip
                
            dep_manifest_path = dep_pkg["manifest_path"]
            
            # Rule 1: apps/ cannot depend on another apps/
            if is_in_dir(manifest_path, "apps") and is_in_dir(dep_manifest_path, "apps"):
                violations.append(f"DAG Violation: App '{pkg_name}' depends on another app '{dep_name}'.")
                
            # Rule 2: libs/ cannot depend on apps/ (Reverse flow)
            if is_in_dir(manifest_path, "libs") and is_in_dir(dep_manifest_path, "apps"):
                violations.append(f"DAG Violation: Library '{pkg_name}' depends on app '{dep_name}'. Reverse dependency detected!")
                
            # Rule 3: libs/shared/ cannot depend on libs/infrastructure/ or libs/core/
            if is_in_dir(manifest_path, "libs/shared"):
                if is_in_dir(dep_manifest_path, "libs/infrastructure") or is_in_dir(dep_manifest_path, "libs/core"):
                    violations.append(f"DAG Violation: Layer 1 (shared) '{pkg_name}' depends on higher layer '{dep_name}'.")
                    
            # Rule 4: libs/core/ cannot depend on libs/infrastructure/
            if is_in_dir(manifest_path, "libs/core"):
                if is_in_dir(dep_manifest_path, "libs/infrastructure"):
                    violations.append(f"DAG Violation: Layer 2 (core) '{pkg_name}' depends on Layer 3 (infrastructure) '{dep_name}'.")

    if violations:
        print("❌ Architecture DAG Violations Detected ❌")
        for v in violations:
            print(f"  - {v}")
        sys.exit(1)
    else:
        print("✅ DAG Topology is clean. The Sovereign Verifier is satisfied.")
        sys.exit(0)

if __name__ == "__main__":
    main()
