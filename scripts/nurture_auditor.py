import os
import re
import json
import ast
from pathlib import Path
from datetime import datetime

# Regex patterns for static analysis
RUST_STRUCT_PATTERN = re.compile(r'pub\s+struct\s+([A-Z][a-zA-Z0-9_]*)')
RUST_TRAIT_PATTERN = re.compile(r'pub\s+trait\s+([A-Z][a-zA-Z0-9_]*)')
RUST_ROUTE_PATTERN = re.compile(r'route\s*\(\s*"([^"]+)"')
TS_COMPONENT_PATTERN = re.compile(r'(?:export\s+default\s+function|const)\s+([A-Z][a-zA-Z0-9_]*)\s*(?::\s*React\.FC|[=(])')

# New Regex Patterns for Phase A (AST Graph Generation)
RUST_USE_PATTERN = re.compile(r'use\s+([^;]+);')
RUST_MOD_PATTERN = re.compile(r'(?:pub\s+)?mod\s+([a-zA-Z0-9_]+)\s*;')
RUST_IMPL_PATTERN = re.compile(r'impl\s+(.*?)\s*\{')
TS_IMPORT_SYMBOL_PATTERN = re.compile(r'import\s+(.*?)\s+from')
CSS_DEF_PATTERN = re.compile(r'(--[a-zA-Z0-9_-]+)\s*:')
CSS_USE_PATTERN = re.compile(r'var\s*\(\s*(--[a-zA-Z0-9_-]+)\s*\)')

def find_source_files(base_dir, extensions):
    """Recursively yield paths, strictly skipping ignored directories at the root level using os.walk."""
    ignored_dirs = {'node_modules', 'target', '.git', '.context', 'dist', 'build'}
    for root, dirs, files in os.walk(base_dir):
        # In-place modification of dirs to prevent os.walk from descending
        dirs[:] = [d for d in dirs if d not in ignored_dirs]
        for f in files:
            if any(f.endswith(ext) for ext in extensions):
                yield Path(root) / f

def analyze_rust_file(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
        structs = RUST_STRUCT_PATTERN.findall(content)
        traits = RUST_TRAIT_PATTERN.findall(content)
        routes = RUST_ROUTE_PATTERN.findall(content)
        
        edges = []
        name = Path(file_path).name
        
        for u in RUST_USE_PATTERN.findall(content):
            edges.append({"from": name, "to": u.strip(), "kind": "use"})
        for m in RUST_MOD_PATTERN.findall(content):
            edges.append({"from": name, "to": m.strip(), "kind": "mod"})
        for i in RUST_IMPL_PATTERN.findall(content):
            edges.append({"from": name, "to": i.strip(), "kind": "impl"})
            
        return {"structs": structs, "traits": traits, "routes": routes, "edges": edges}

def analyze_py_file(file_path):
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        tree = ast.parse(content)
    except Exception:
        return {"routes": [], "functions": [], "edges": []}

    routes = []
    functions = []
    edges = []
    name = Path(file_path).name

    for node in ast.walk(tree):
        if isinstance(node, ast.FunctionDef):
            functions.append(node.name)
            for decorator in node.decorator_list:
                if isinstance(decorator, ast.Call) and isinstance(decorator.func, ast.Attribute):
                    if decorator.func.attr in ('get', 'post', 'put', 'delete', 'patch', 'route'):
                        if decorator.args and isinstance(decorator.args[0], ast.Constant):
                            routes.append(str(decorator.args[0].value))
        elif isinstance(node, ast.Import):
            for alias in node.names:
                edges.append({"from": name, "to": alias.name, "kind": "py_import"})
        elif isinstance(node, ast.ImportFrom):
            if node.module:
                edges.append({"from": name, "to": node.module, "kind": "py_import"})

    return {"routes": routes, "functions": functions, "edges": edges}

def analyze_ts_file(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
        components = TS_COMPONENT_PATTERN.findall(content)
        
        edges = []
        name = Path(file_path).name
        
        imports = TS_IMPORT_SYMBOL_PATTERN.findall(content)
        for imp in imports:
            # Flatten alias/imports e.g "{ Button, Modal }" or "* as utils" or "DefaultComponent"
            cleaned = re.sub(r'[\{\}\*]', '', imp).replace(' as ', ' ')
            # Split by comma or space
            symbols = [s.strip() for s in re.split(r'[, ]+', cleaned) if s.strip() and s.strip() != 'type']
            for symbol in symbols:
                edges.append({"from": name, "to": symbol, "kind": "import"})
                
        # Also catch CSS usages
        for u in CSS_USE_PATTERN.findall(content):
            edges.append({"from": name, "to": u.strip(), "kind": "css_token"})
            
        return {"components": components, "edges": edges}

def analyze_css_file(file_path):
    edges = []
    name = Path(file_path).name
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
            for d in CSS_DEF_PATTERN.findall(content):
                edges.append({"from": name, "to": d.strip(), "kind": "css_token_def"})
            for u in CSS_USE_PATTERN.findall(content):
                edges.append({"from": name, "to": u.strip(), "kind": "css_token"})
    except Exception:
        pass
    return {"edges": edges}

def generate_audit_report(root_dir, output_file):
    report = {
        "timestamp": datetime.now().isoformat(),
        "crates": {},
        "apps": {}
    }
    
    impact_edges = []
    print(f"🔍 Starting AST Deep Scan from {root_dir}...")

    # Scan libs (Cargo Crates)
    libs_dir = root_dir / 'libs'
    if libs_dir.exists():
        for crate_dir in libs_dir.iterdir():
            if crate_dir.is_dir() and (crate_dir / 'Cargo.toml').exists():
                crate_name = crate_dir.name
                crate_data = {"structs": [], "traits": []}
                for path in find_source_files(crate_dir, ['.rs']):
                    res = analyze_rust_file(path)
                    crate_data["structs"].extend(res["structs"])
                    crate_data["traits"].extend(res["traits"])
                    impact_edges.extend(res["edges"])
                
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
                app_data = {"structs": [], "traits": [], "routes": [], "components": [], "functions": []}
                
                for path in find_source_files(app_dir, ['.rs', '.tsx', '.css', '.py']):
                    ext = path.suffix
                    if ext == '.rs':
                        res = analyze_rust_file(path)
                        app_data["structs"].extend(res["structs"])
                        app_data["traits"].extend(res["traits"])
                        app_data["routes"].extend(res["routes"])
                        impact_edges.extend(res["edges"])
                    elif ext == '.tsx':
                        res = analyze_ts_file(path)
                        app_data["components"].extend(res["components"])
                        impact_edges.extend(res["edges"])
                    elif ext == '.css':
                        res = analyze_css_file(path)
                        impact_edges.extend(res["edges"])
                    elif ext == '.py':
                        res = analyze_py_file(path)
                        app_data["routes"].extend(res.get("routes", []))
                        app_data["functions"].extend(res.get("functions", []))
                        impact_edges.extend(res.get("edges", []))

                # Deduplicate
                app_data["structs"] = sorted(list(set(app_data["structs"])))
                app_data["traits"] = sorted(list(set(app_data["traits"])))
                app_data["routes"] = sorted(list(set(app_data["routes"])))
                app_data["components"] = sorted(list(set(app_data["components"])))
                app_data["functions"] = sorted(list(set(app_data["functions"])))
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
            if data.get('functions'):
                f.write("**Python Functions**\n")
                f.write(f"- {', '.join(data['functions'])}\n")
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
            
    # Write Graph JSON
    context_dir = root_dir / '.context'
    if context_dir.exists():
        # Deduplicate edges (convert list of dicts to list of unique tuple-dicts)
        unique_edges = []
        seen = set()
        for e in impact_edges:
            sig = (e["from"], e["to"], e.get("kind", ""))
            if sig not in seen:
                seen.add(sig)
                unique_edges.append(e)

        graph_path = context_dir / 'impact_graph.json'
        with open(graph_path, 'w', encoding='utf-8') as f:
            json.dump({
                "generated_at": report["timestamp"],
                "edges": unique_edges
            }, f, indent=2)
            
    print(f"✅ Deep Scan Complete. Report created at: {output_file}")

import sys

if __name__ == '__main__':
    if len(sys.argv) > 1:
        project_root = Path(sys.argv[1]).resolve()
        output_path = project_root / 'deep_scan_matrix.md'
    else:
        project_root = Path(__file__).parent.parent
        output_path = project_root / 'docs' / 'architecture' / 'deep_scan_matrix.md'
    
    # Ensure parents exist
    output_path.parent.mkdir(parents=True, exist_ok=True)
    generate_audit_report(project_root, output_path)
