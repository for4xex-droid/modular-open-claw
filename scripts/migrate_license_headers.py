import os
import re
from pathlib import Path

HEADER_TEMPLATE = """/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
"""

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content
    modified = False

    # Pattern A: Has Apache 2.0 license -> Replace
    if "Licensed under the Apache License, Version 2.0" in content:
        content = content.replace(
            "Licensed under the Apache License, Version 2.0.",
            "Licensed under the Business Source License 1.1."
        )
        content = content.replace(
            "Licensed under the Apache License, Version 2.0",
            "Licensed under the Business Source License 1.1"
        )
        modified = True
        
    # Pattern B & C: Missing BUSL
    elif "Licensed under the Business Source License 1.1" not in content and "Licensed under the Apache License" not in content:
        # Pattern B: Has Copyright but no license
        if "Copyright (C)" in content and "motivationstudio, LLC" in content:
            content = re.sub(
                r'(Copyright \(C\) 202[0-9] motivationstudio, LLC)(\s*\*)',
                r'\1\n * Licensed under the Business Source License 1.1.\2',
                content
            )
            modified = True
        else:
            # Pattern C: No header at all. Prepend it.
            # Safety check: Don't blindly inject if there's already some OTHER copyright or it's empty
            if len(content.strip()) > 0:
                content = HEADER_TEMPLATE + content
                modified = True

    if modified:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)

def main():
    skip_dirs = {"aiome-contracts", "wasm-skills", "target", "node_modules", ".git", "dist", "build"}
    
    for root, dirs, files in os.walk("."):
        # Exactly match the dir names for exclusion to avoid preventing aiome-core-contracts
        dirs[:] = [d for d in dirs if d not in skip_dirs]
        
        path_obj = Path(root)
        
        # Additional safety check against nested occurrences
        parts = path_obj.parts
        if "aiome-contracts" in parts or "wasm-skills" in parts:
            continue
            
        for file in files:
            if file.endswith((".rs", ".ts", ".tsx")):
                filepath = os.path.join(root, file)
                process_file(filepath)

if __name__ == "__main__":
    main()
