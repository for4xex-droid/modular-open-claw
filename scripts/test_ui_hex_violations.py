import os
import re
import sys

def run_test():
    # Phase 3-B v3: スコープを管理コンソール全体に拡大 (App.tsx, lib/, components/ 全域)
    target_dirs = [
        "apps/management-console/src/components",
        "apps/management-console/src/lib",
    ]
    # App.tsx / biome-popup-entry は単独ファイルとして追加
    extra_files = [
        "apps/management-console/src/App.tsx",
        "apps/management-console/src/biome-popup-entry.tsx",
    ]
    # .css もスキャン対象（src 全域）。ただしトークン定義ファイル自体は生値を持つのが正しいため除外
    css_root = "apps/management-console/src"
    css_excludes = {
        os.path.normpath("apps/management-console/src/styles/tokens.css"),
    }

    hex_pattern = re.compile(r'#[0-9a-fA-F]{3,8}')
    # rgba(r, g, b, a) / rgb(r, g, b) および hsl(h, s, l) / hsla(h, s, l, a) ハードコードも U-002 違反として検出
    rgba_pattern = re.compile(r'(?:rgba?|hsla?)\(\s*\d+')

    failed = False
    violations = 0
    file_count = 0

    def scan_file(filepath, filename):
        nonlocal failed, violations, file_count
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        # Strip cssVar() bridge calls — these are the sanctioned pattern with HEX fallbacks
        content = re.sub(r"cssVar\([^)]+\)", "", content)

        # Strip comment lines (// ...)
        content = re.sub(r'//.*$', '', content, flags=re.MULTILINE)

        # Strip // allow-anti-pattern tagged lines entirely
        content = re.sub(r'.*allow-anti-pattern.*\n?', '', content)

        hex_matches = hex_pattern.findall(content)
        rgba_matches = rgba_pattern.findall(content)

        total = len(hex_matches) + len(rgba_matches)
        if total > 0:
            failed = True
            violations += total
            file_count += 1
            details = []
            if hex_matches:
                details.append(f"HEX: {set(hex_matches)}")
            if rgba_matches:
                details.append(f"rgba/rgb: {len(rgba_matches)} occurrences")
            print(f"❌ {filename} — {total} violations ({', '.join(details)})")

    # Scan directories recursively
    for target_dir in target_dirs:
        if not os.path.isdir(target_dir):
            continue
        for root, _, files in os.walk(target_dir):
            for file in files:
                if file.endswith(('.tsx', '.ts')):
                    filepath = os.path.join(root, file)
                    scan_file(filepath, file)

    # Scan extra individual files
    for filepath in extra_files:
        if os.path.isfile(filepath):
            scan_file(filepath, os.path.basename(filepath))

    # Scan .css files (tokens.css excluded)
    if os.path.isdir(css_root):
        for root, _, files in os.walk(css_root):
            for file in files:
                if file.endswith('.css'):
                    filepath = os.path.join(root, file)
                    if os.path.normpath(filepath) in css_excludes:
                        continue
                    scan_file(filepath, file)

    if failed:
        print(f"\n[RED] Test Failed! Found {violations} violations across {file_count} files.")
        print("Golden Rule U-002: Replace hardcoded colors with CSS tokens (var(--token)).")
        sys.exit(1)
    else:
        print("\n[GREEN] Test Passed! Zero color violations in UI components.")
        sys.exit(0)

if __name__ == "__main__":
    print("Running UI Theme Enforcement Test (U-002)...")
    print("Scope: components/ + lib/ + App.tsx + biome-popup-entry.tsx + src/**/*.css (tokens.css excluded) | Patterns: HEX + rgba/rgb")
    print("=" * 60)
    run_test()
