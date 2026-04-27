#!/bin/bash
python3 -m venv .venv
source .venv/bin/activate
pip install tomlkit
cat << 'PYEOF' > scripts/migrate_deps.py
import tomlkit
from pathlib import Path

def main():
    root_cargo_path = Path("Cargo.toml")
    with open(root_cargo_path, "r") as f:
        root_toml = tomlkit.parse(f.read())
    
    workspace_deps = root_toml.get("workspace", {}).get("dependencies", {})
    workspace_dep_names = set(workspace_deps.keys())
    
    # We will also add more deps to workspace_deps if they are widely used, but for now we just convert existing ones.
    
    members = root_toml.get("workspace", {}).get("members", [])
    
    for member in members:
        member_cargo_path = Path(member) / "Cargo.toml"
        if not member_cargo_path.exists():
            continue
            
        print(f"Processing {member_cargo_path}")
        with open(member_cargo_path, "r") as f:
            member_toml = tomlkit.parse(f.read())
            
        changed = False
        for dep_type in ["dependencies", "dev-dependencies", "build-dependencies"]:
            if dep_type in member_toml:
                deps = member_toml[dep_type]
                for dep_name in list(deps.keys()):
                    if dep_name in workspace_dep_names:
                        current_val = deps[dep_name]
                        if isinstance(current_val, str):
                            deps[dep_name] = tomlkit.inline_table()
                            deps[dep_name]["workspace"] = True
                            changed = True
                        elif isinstance(current_val, dict):
                            if "workspace" not in current_val:
                                new_val = tomlkit.inline_table()
                                new_val["workspace"] = True
                                if "features" in current_val:
                                    new_val["features"] = current_val["features"]
                                if "optional" in current_val:
                                    new_val["optional"] = current_val["optional"]
                                deps[dep_name] = new_val
                                changed = True
        
        if changed:
            with open(member_cargo_path, "w") as f:
                f.write(tomlkit.dumps(member_toml))
            print(f"  -> Updated {member_cargo_path}")

if __name__ == "__main__":
    main()
PYEOF
python scripts/migrate_deps.py
