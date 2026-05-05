import sys
import os

def check_file(filepath):
    try:
        with open(filepath, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except Exception:
        return
    in_test_block = False
    brace_depth = 0
    test_entry_depth = -1
    for i, raw_line in enumerate(lines):
        code_only = raw_line.split("//")[0]
        brace_depth += code_only.count("{") - code_only.count("}")
        if in_test_block and brace_depth <= test_entry_depth:
            in_test_block = False
            test_entry_depth = -1
        if "#[cfg(test)]" in raw_line:
            if not in_test_block:
                in_test_block = True
                test_entry_depth = brace_depth
            continue
        if in_test_block:
            continue
        if "allow-anti-pattern" in raw_line and "vendor" not in filepath:
            print(f"{filepath}:{i+1}:{raw_line.strip()}")

for root, _, files in os.walk("."):
    if "tests" in root.split(os.sep) or "vendor" in root.split(os.sep):
        continue
    for f in files:
        if f.endswith(".rs") and not f.endswith("test.rs") and not f.endswith("tests.rs"):
            check_file(os.path.join(root, f))
