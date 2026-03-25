#!/usr/bin/env python3
"""
License Compliance Test Suite
TDD Red-Green-Refactor: This script validates that all license obligations are met.

Tests:
  1. NOTICE file exists
  2. THIRD_PARTY_NOTICES.md exists and contains required entries
  3. All .rs source files have copyright headers
  4. LICENSE file exists and is Apache 2.0
  5. All Cargo.toml files have license field

Exit code 0 = all pass, 1 = failures found.
"""

import os
import sys
import re
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent
PASS = "✅"
FAIL = "❌"
WARN = "⚠️"

results = []
warnings = []

def test(name, passed, detail=""):
    status = PASS if passed else FAIL
    results.append((name, passed, detail))
    print(f"  {status} {name}" + (f" — {detail}" if detail and not passed else ""))

def warn(name, detail=""):
    warnings.append((name, detail))
    print(f"  {WARN} {name} — {detail}")

# ──────────────────────────────────────────────
# Test 1: LICENSE file
# ──────────────────────────────────────────────
print("\n📋 Test 1: LICENSE file")
license_path = PROJECT_ROOT / "LICENSE"
test("LICENSE file exists", license_path.exists())
if license_path.exists():
    content = license_path.read_text()
    test("LICENSE is Apache 2.0", "Apache License" in content and "Version 2.0" in content)

# ──────────────────────────────────────────────
# Test 2: NOTICE file
# ──────────────────────────────────────────────
print("\n📋 Test 2: NOTICE file")
notice_path = PROJECT_ROOT / "NOTICE"
test("NOTICE file exists", notice_path.exists())
if notice_path.exists():
    content = notice_path.read_text()
    test("NOTICE contains project name", "Aiome" in content or "aiome" in content)
    test("NOTICE contains copyright", "Copyright" in content)

# ──────────────────────────────────────────────
# Test 3: THIRD_PARTY_NOTICES.md
# ──────────────────────────────────────────────
print("\n📋 Test 3: THIRD_PARTY_NOTICES.md")
tp_path = PROJECT_ROOT / "THIRD_PARTY_NOTICES.md"
test("THIRD_PARTY_NOTICES.md exists", tp_path.exists())
if tp_path.exists():
    content = tp_path.read_text()
    required_entries = [
        ("AutoResearchClaw", "Apache"),
        ("MetaClaw", "Apache"),
        ("Trojan's Whisper", "CC BY"),
    ]
    for name, license_type in required_entries:
        has_name = name in content
        has_license = license_type in content
        test(f"Contains {name} entry", has_name and has_license,
             f"name={'found' if has_name else 'MISSING'}, license={'found' if has_license else 'MISSING'}")

# ──────────────────────────────────────────────
# Test 4: Copyright headers in .rs files
# ──────────────────────────────────────────────
print("\n📋 Test 4: Copyright headers in .rs files")
HEADER_PATTERN = re.compile(r"Copyright \(C\) \d{4}")
DIRS_TO_CHECK = ["libs", "apps"]
missing_headers = []

for dir_name in DIRS_TO_CHECK:
    dir_path = PROJECT_ROOT / dir_name
    if not dir_path.exists():
        continue
    for rs_file in dir_path.rglob("*.rs"):
        # Skip generated files and build artifacts
        rel = str(rs_file.relative_to(PROJECT_ROOT))
        if "target/" in rel or "build/" in rel:
            continue
        try:
            with open(rs_file, 'r', errors='replace') as f:
                # Read first 10 lines for header check
                head = "".join(f.readline() for _ in range(10))
            if not HEADER_PATTERN.search(head):
                missing_headers.append(rel)
        except (IOError, OSError, UnicodeDecodeError):
            warn(f"Could not read {rel}")

if missing_headers:
    test(f"All .rs files have copyright headers", False,
         f"{len(missing_headers)} files missing headers")
    for mh in missing_headers[:10]:  # Show first 10
        print(f"      → {mh}")
    if len(missing_headers) > 10:
        print(f"      → ... and {len(missing_headers) - 10} more")
else:
    test("All .rs files have copyright headers", True)

# ──────────────────────────────────────────────
# Test 5: Cargo.toml license fields
# ──────────────────────────────────────────────
print("\n📋 Test 5: Cargo.toml license fields")
missing_license_field = []
for toml_file in PROJECT_ROOT.rglob("Cargo.toml"):
    rel = str(toml_file.relative_to(PROJECT_ROOT))
    if "target/" in rel or ".cargo/" in rel:
        continue
    try:
        content = toml_file.read_text()
        # Only check [package] sections (not workspace Cargo.toml)
        if "[package]" in content:
            # Extract [package] section only to avoid false positives
            # Find next section header (line starting with [) after [package]
            lines = content.split('\n')
            in_pkg = False
            pkg_lines = []
            for line in lines:
                stripped = line.strip()
                if stripped == "[package]":
                    in_pkg = True
                    continue
                elif in_pkg and stripped.startswith("[") and not stripped.startswith("[["):
                    break
                elif in_pkg:
                    pkg_lines.append(line)
            pkg_section = '\n'.join(pkg_lines)
            if 'license' not in pkg_section.lower():
                missing_license_field.append(rel)
    except (IOError, OSError, ValueError):
        warn(f"Could not parse {rel}")

if missing_license_field:
    test("All Cargo.toml have license field", False,
         f"{len(missing_license_field)} files missing")
    for mf in missing_license_field:
        print(f"      → {mf}")
else:
    test("All Cargo.toml have license field", True)

# ──────────────────────────────────────────────
# Summary
# ──────────────────────────────────────────────
print("\n" + "=" * 50)
total = len(results)
passed = sum(1 for _, p, _ in results if p)
failed = total - passed

if failed == 0:
    print(f"🎉 License Compliance: ALL {total} TESTS PASSED")
    sys.exit(0)
else:
    print(f"🚨 License Compliance: {failed}/{total} TESTS FAILED")
    print("\nFailed tests:")
    for name, p, detail in results:
        if not p:
            print(f"  {FAIL} {name}" + (f" — {detail}" if detail else ""))
    sys.exit(1)
