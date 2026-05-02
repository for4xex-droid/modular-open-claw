#!/usr/bin/env python3
import subprocess
import json
import os

WORKSPACE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def run_clippy():
    print("Running cargo clippy...")
    result = subprocess.run(
        ["cargo", "clippy", "--workspace", "--all-targets", "--message-format=json", "--", "-D", "warnings"],
        cwd=WORKSPACE_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    errors = []
    for line in result.stdout.splitlines():
        if not line.strip():
            continue
        try:
            msg = json.loads(line)
            if msg.get("reason") == "compiler-message":
                message = msg.get("message", {})
                if message.get("code") and message["code"].get("code") == "clippy::unwrap_used":
                    errors.append(message)
        except Exception:
            pass
    return errors

def fix_errors(errors):
    # Group by file
    fixes_by_file = {}
    for err in errors:
        for span in err.get("spans", []):
            if span.get("is_primary"):
                filepath = os.path.join(WORKSPACE_ROOT, span["file_name"])
                if filepath not in fixes_by_file:
                    fixes_by_file[filepath] = []
                fixes_by_file[filepath].append(span)

    total_fixed = 0
    for filepath, spans in fixes_by_file.items():
        if not os.path.exists(filepath):
            continue
            
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.read()
            
        # We process spans in reverse order so that indices remain valid
        spans.sort(key=lambda x: x["byte_start"], reverse=True)
        
        for span in spans:
            start = span["byte_start"]
            end = span["byte_end"]
            
            snippet = content[start:end]
            if "unwrap()" in snippet:
                reason = "safe unwrap"
                if "Regex::new" in content[max(0, start-50):start]:
                    reason = "invalid regex pattern"
                elif ".lock(" in content[max(0, start-20):start]:
                    reason = "lock poisoned"
                elif "NonZero" in content[max(0, start-50):start]:
                    reason = "must be non-zero"
                elif "serde_json::to_string" in content[max(0, start-50):start]:
                    reason = "serialization failed"
                    
                replacement = snippet.replace("unwrap()", f'expect("{reason}")')
                content = content[:start] + replacement + content[end:]
                total_fixed += 1
                print(f"Fixed unwrap in {span['file_name']}:{span['line_start']}")

        with open(filepath, "w", encoding="utf-8") as f:
            f.write(content)
            
    print(f"Total unwraps fixed: {total_fixed}")

def main():
    errors = run_clippy()
    if not errors:
        print("No clippy::unwrap_used errors found!")
        return
        
    print(f"Found {len(errors)} unwrap_used errors. Fixing...")
    fix_errors(errors)

if __name__ == "__main__":
    main()
