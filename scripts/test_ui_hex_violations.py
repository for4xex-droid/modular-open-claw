import os
import re
import sys

def run_test():
    target_dir = "apps/management-console/src/components"
    hex_pattern = re.compile(r'#[0-9a-fA-F]{3,8}')
    
    # Exclude files that legitimately require HEX (like the CSS bridge files once we build them, 
    # but for now we expect no HEX anywhere except possibly tokens.css, which is not in this dir)
    exceptions = []

    failed = False
    violations = 0
    file_count = 0

    for root, _, files in os.walk(target_dir):
        for file in files:
            if file.endswith(('.tsx', '.ts')) and file not in exceptions:
                filepath = os.path.join(root, file)
                with open(filepath, 'r', encoding='utf-8') as f:
                    content = f.read()
                
                # Ignore hex codes that are properly wrapped in our cssVar bridge fallback
                content = re.sub(r"cssVar\([^,]+,\s*['\"]#[0-9a-fA-F]{3,8}['\"]\)", "", content)
                
                matches = hex_pattern.findall(content)
                if matches:
                    failed = True
                    violations += len(matches)
                    file_count += 1
                    print(f"❌ {file} contains {len(matches)} HEX violations: {set(matches)}")

    if failed:
        print(f"\n[RED] Test Failed! Found {violations} HEX violations across {file_count} files.")
        print("Golden Rule U-002 Violation. Please replace with CSS tokens.")
        sys.exit(1)
    else:
        print("\n[GREEN] Test Passed! Zero HEX violations in UI components.")
        sys.exit(0)

if __name__ == "__main__":
    print("Running UI Theme Enforcement Test...")
    run_test()
