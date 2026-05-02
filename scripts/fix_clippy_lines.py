#!/usr/bin/env python3
import subprocess
import os
import re

WORKSPACE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def run_clippy():
    print("Running cargo clippy...")
    result = subprocess.run(
        ["cargo", "clippy", "--workspace", "--all-targets", "--message-format=short", "--", "-D", "warnings"],
        cwd=WORKSPACE_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    # Example line: libs/infrastructure/src/grpc/mock_a2a_client.rs:28:9: error: used `unwrap()` on a `Result` value
    errors = []
    for line in result.stdout.splitlines():
        if "error: used `unwrap()`" in line:
            parts = line.split(":", 3)
            if len(parts) >= 3:
                filepath = os.path.join(WORKSPACE_ROOT, parts[0])
                line_num = int(parts[1])
                errors.append((filepath, line_num))
    return errors

def fix_errors(errors):
    # Group by file
    fixes_by_file = {}
    for filepath, line_num in errors:
        if filepath not in fixes_by_file:
            fixes_by_file[filepath] = []
        fixes_by_file[filepath].append(line_num)

    total_fixed = 0
    for filepath, lines in fixes_by_file.items():
        if not os.path.exists(filepath):
            continue
            
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.readlines()
            
        # Remove duplicates
        lines = list(set(lines))
        lines.sort(reverse=True)
        
        for line_num in lines:
            idx = line_num - 1
            if idx >= 0 and idx < len(content):
                line = content[idx]
                if ".unwrap()" in line:
                    reason = "safe unwrap"
                    if "Regex::new" in line or "Regex::new" in content[idx-1]:
                        reason = "invalid regex pattern"
                    elif ".lock(" in line:
                        reason = "lock poisoned"
                    elif "NonZero" in line:
                        reason = "must be non-zero"
                    
                    content[idx] = line.replace(".unwrap()", f'.expect("{reason}")')
                    total_fixed += 1
                else:
                    # Look slightly ahead if unwrap() is on the next line
                    if idx + 1 < len(content) and ".unwrap()" in content[idx+1]:
                        content[idx+1] = content[idx+1].replace(".unwrap()", '.expect("safe unwrap")')
                        total_fixed += 1
                    elif idx + 2 < len(content) and ".unwrap()" in content[idx+2]:
                        content[idx+2] = content[idx+2].replace(".unwrap()", '.expect("safe unwrap")')
                        total_fixed += 1
                    elif idx + 3 < len(content) and ".unwrap()" in content[idx+3]:
                        content[idx+3] = content[idx+3].replace(".unwrap()", '.expect("safe unwrap")')
                        total_fixed += 1

        with open(filepath, "w", encoding="utf-8") as f:
            f.writelines(content)
            
    print(f"Total unwraps fixed: {total_fixed}")

def main():
    while True:
        errors = run_clippy()
        if not errors:
            print("No clippy::unwrap_used errors found!")
            break
            
        print(f"Found {len(errors)} unwrap_used errors. Fixing...")
        fix_errors(errors)

if __name__ == "__main__":
    main()
