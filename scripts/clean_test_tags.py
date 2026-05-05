import sys
import os

def clean_file(filepath):
    try:
        with open(filepath, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except Exception:
        return False
        
    changed = False
    new_lines = []
    
    in_test_block = False
    brace_depth = 0
    test_entry_depth = -1
    
    is_test_file = filepath.endswith("test.rs") or filepath.endswith("tests.rs") or "tests/" in filepath
    
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
                
        is_test_context = is_test_file or in_test_block
        
        if is_test_context and "allow-anti-pattern" in raw_line:
            # Remove the comment
            if " // allow-anti-pattern" in raw_line:
                new_line = raw_line.replace(" // allow-anti-pattern", "")
            elif "// allow-anti-pattern" in raw_line:
                new_line = raw_line.replace("// allow-anti-pattern", "")
            else:
                new_line = raw_line
                
            new_lines.append(new_line)
            changed = True
        else:
            new_lines.append(raw_line)
            
    if changed:
        with open(filepath, "w", encoding="utf-8") as f:
            f.writelines(new_lines)
        return True
    return False

count = 0
for d in ["libs", "apps"]:
    for root, _, files in os.walk(d):
        for f in files:
            if f.endswith(".rs"):
                if clean_file(os.path.join(root, f)):
                    count += 1
print(f"Cleaned test tags in {count} files.")
