import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent
header = """/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

"""

fixed_count = 0
for d in ["libs", "apps"]:
    dir_path = PROJECT_ROOT / d
    if not dir_path.exists():
        continue
    for rs_file in dir_path.rglob("*.rs"):
        rel = str(rs_file.relative_to(PROJECT_ROOT))
        if "target/" in rel or "build/" in rel:
            continue
        try:
            with open(rs_file, 'r', encoding='utf-8') as f:
                content = f.read()
            if "Copyright (C)" not in content:
                with open(rs_file, 'w', encoding='utf-8') as f:
                    # skip existing #!/usr/bin/env or similar if needed... but rust doesn't use that
                    # Also skip // file level comments if any, but prepending is safe usually.
                    f.write(header + content)
                fixed_count += 1
        except Exception as e:
            print(f"Skipping {rel}: {e}")

print(f"Fixed {fixed_count} files")
