#!/usr/bin/env python3
import os

WORKSPACE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SEARCH_DIRS = [
    os.path.join(WORKSPACE_ROOT, "libs"),
    os.path.join(WORKSPACE_ROOT, "apps"),
]
SKIP_DIRS = {"target", "node_modules", ".git", "scripts"}

def find_rs_files(dirs):
    rs_files = []
    for d in dirs:
        for root, subdirs, files in os.walk(d):
            subdirs[:] = [s for s in subdirs if s not in SKIP_DIRS]
            for f in files:
                if f.endswith(".rs"):
                    rs_files.append(os.path.join(root, f))
    return sorted(rs_files)

def process_file(filepath):
    with open(filepath, "r", encoding="utf-8") as f:
        lines = f.readlines()

    is_crate_root = filepath.endswith("/src/lib.rs") or filepath.endswith("/src/main.rs")
    is_integration_test = "/tests/" in filepath and filepath.endswith(".rs")

    cfg_attr_line = "#![cfg_attr(test, allow(clippy::unwrap_used))]\n"
    allow_line_for_integration = "#![allow(clippy::unwrap_used)]\n"
    
    new_lines = []
    stripped_something = False

    for line in lines:
        if "allow(clippy::unwrap_used)" in line:
            stripped_something = True
            continue
        new_lines.append(line)

    needs_insert = False
    insert_str = ""
    if is_crate_root:
        if not any("cfg_attr(test, allow(clippy::unwrap_used))" in l for l in new_lines):
            needs_insert = True
            insert_str = cfg_attr_line
    elif is_integration_test:
        if not any("#![allow(clippy::unwrap_used)]" in l for l in new_lines) and not any("cfg_attr(test, allow(clippy::unwrap_used))" in l for l in new_lines):
            needs_insert = True
            insert_str = allow_line_for_integration

    modified = stripped_something or needs_insert

    if needs_insert:
        # Insert after license block if present
        insert_idx = 0
        if new_lines and new_lines[0].startswith("/*"):
            for i, line in enumerate(new_lines):
                if line.strip() == "*/":
                    insert_idx = i + 1
                    break
        new_lines.insert(insert_idx, insert_str)

    if modified:
        with open(filepath, "w", encoding="utf-8") as f:
            f.writelines(new_lines)
        return True
    return False

def main():
    rs_files = find_rs_files(SEARCH_DIRS)
    count = 0
    for filepath in rs_files:
        if process_file(filepath):
            count += 1
            print(f"Modified: {os.path.relpath(filepath, WORKSPACE_ROOT)}")
    
    print(f"Total files modified: {count}")

if __name__ == "__main__":
    main()
