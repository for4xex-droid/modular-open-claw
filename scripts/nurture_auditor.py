import os
import re
import json
from pathlib import Path
from datetime import datetime

# Regex patterns for static analysis
RUST_STRUCT_PATTERN = re.compile(r'pub\s+struct\s+([A-Z][a-zA-Z0-9_]*)')
RUST_TRAIT_PATTERN = re.compile(r'pub\s+trait\s+([A-Z][a-zA-Z0-9_]*)')
RUST_ROUTE_PATTERN = re.compile(r'route\("([^"]+)"')
TS_COMPONENT_PATTERN = re.compile(r'const\s+([A-Z][a-zA-Z0-9_]*)\s*:\s*React\.FC')

def analyze_rust_file(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
        structs = RUST_STRUCT_PATTERN.findall(content)
        traits = RUST_TRAIT_PATTERN.findall(content)
        routes = RUST_ROUTE_PATTERN.findall(content)
        return {"structs": structs, "traits": traits, "routes": routes}

def analyze_ts_file(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
        components = TS_COMPONENT_PATTERN.findall(content)
        return {"components": components}

def generate_audit_report(root_dir, output_file):
    report = {
        "timestamp": datetime.now().isoformat(),
        "crates": {},
        "apps": {}
    }

    print(f"🔍 Starting AST Deep Scan from {root_dir}...")

    # Scan libs (Cargo Crates)
    libs_dir = root_dir / 'libs'
    if libs_dir.exists():
        for crate_dir in libs_dir.iterdir():
            if crate_dir.is_dir() and (crate_dir / 'Cargo.toml').exists():
                crate_name = crate_dir.name
                crate_data = {"structs": [], "traits": []}
                for path in crate_dir.rglob('*.rs'):
                    res = analyze_rust_file(path)
                    crate_data["structs"].extend(res["structs"])
                    crate_data["traits"].extend(res["traits"])
                
                # Deduplicate
                crate_data["structs"] = sorted(list(set(crate_data["structs"])))
                crate_data["traits"] = sorted(list(set(crate_data["traits"])))
                report["crates"][crate_name] = crate_data

    # Scan apps
    apps_dir = root_dir / 'apps'
    if apps_dir.exists():
        for app_dir in apps_dir.iterdir():
            if app_dir.is_dir():
                app_name = app_dir.name
                app_data = {"structs": [], "traits": [], "routes": [], "components": []}
                
                # Rust files
                for path in app_dir.rglob('*.rs'):
                    res = analyze_rust_file(path)
                    app_data["structs"].extend(res["structs"])
                    app_data["traits"].extend(res["traits"])
                    app_data["routes"].extend(res["routes"])
                
                # TSX files
                for path in app_dir.rglob('*.tsx'):
                    res = analyze_ts_file(path)
                    app_data["components"].extend(res["components"])

                # Deduplicate
                app_data["structs"] = sorted(list(set(app_data["structs"])))
                app_data["traits"] = sorted(list(set(app_data["traits"])))
                app_data["routes"] = sorted(list(set(app_data["routes"])))
                app_data["components"] = sorted(list(set(app_data["components"])))
                report["apps"][app_name] = app_data

    # Write Markdown Report
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write("# 📡 Aiome Deep Scan AST Matrix\n\n")
        f.write(f"> Generated at: {report['timestamp']}\n\n")
        f.write("This file contains the AST-extracted structural matrix of the codebase. Use it to cross-reference against Project NURTURE requirements without hitting LLM context limits.\n\n")
        
        f.write("## 📦 APPS (Endpoints & Services)\n")
        for app, data in report["apps"].items():
            f.write(f"### `{app}`\n")
            if data['routes']:
                f.write("**REST / Websocket Routes**\n")
                for r in data['routes']:
                    f.write(f"- `{r}`\n")
            if data['components']:
                f.write("**React Components**\n")
                f.write(f"- {', '.join(data['components'])}\n")
            if data['structs']:
                f.write("**Key Structs**\n")
                f.write(f"- {', '.join(data['structs'])}\n")
            f.write("\n")

        f.write("## 📚 LIBS (Core Domain & Infrastructure)\n")
        for crate, data in report["crates"].items():
            f.write(f"### `{crate}`\n")
            if data['traits']:
                f.write("**Traits (Interfaces)**\n")
                f.write(f"- {', '.join(data['traits'])}\n")
            if data['structs']:
                f.write("**Domain Structs**\n")
                f.write(f"- {', '.join(data['structs'])}\n")
            f.write("\n")
            
    print(f"✅ Deep Scan Complete. Report created at: {output_file}")


if __name__ == '__main__':
    project_root = Path(__file__).parent.parent
    output_path = project_root / 'docs' / 'architecture' / 'deep_scan_matrix.md'
    generate_audit_report(project_root, output_path)
