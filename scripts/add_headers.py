import os

header = """/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
"""

def process_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        print(f"Skipping {filepath} due to read error: {e}")
        return

    if "Copyright (C)" not in content and "Licensed under the Apache License" not in content:
        print(f"Adding header to {filepath}")
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(header + content)

for root, dirs, files in os.walk('.'):
    if "target" in dirs:
        dirs.remove("target")
    if "node_modules" in dirs:
        dirs.remove("node_modules")
    if ".git" in dirs:
        dirs.remove(".git")
    for filename in files:
        if filename.endswith(".rs"):
            process_file(os.path.join(root, filename))
