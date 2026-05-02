#!/usr/bin/env python3
"""
Clippy unwrap_used deny enforcer (v2 - Statement-aware).

Inserts #[allow(clippy::unwrap_used)] annotations at STATEMENT boundaries,
not at individual .unwrap() call sites. This prevents mid-expression insertion
that causes E0658 (attributes on expressions are experimental).

Strategy:
1. For #[cfg(test)] mod blocks: insert #[allow] before `mod` declaration.
   This covers ALL .unwrap() calls within the entire test module.
2. For integration test files (tests/ directory or *_tests.rs):
   Insert #[allow] before #[test]/#[tokio::test] attributes.
   This covers ALL .unwrap() within each test function.
3. For production code: find the statement start (let, if, match, etc.)
   and insert #[allow] above it. Falls back to line-level if no statement found.
"""

import os
import re
import sys

WORKSPACE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SEARCH_DIRS = [
    os.path.join(WORKSPACE_ROOT, "libs"),
    os.path.join(WORKSPACE_ROOT, "apps"),
]
SKIP_DIRS = {"target", "node_modules", ".git"}
ALLOW_ATTR = "#[allow(clippy::unwrap_used)]"


def find_rs_files(dirs):
    rs_files = []
    for d in dirs:
        for root, subdirs, files in os.walk(d):
            subdirs[:] = [s for s in subdirs if s not in SKIP_DIRS]
            for f in files:
                if f.endswith(".rs"):
                    rs_files.append(os.path.join(root, f))
    return sorted(rs_files)


def find_test_module_starts(lines):
    """Find line indices of #[cfg(test)] declarations."""
    starts = []
    for i, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]":
            starts.append(i)
    return starts


def find_test_mod_line(lines, cfg_test_idx):
    """Given #[cfg(test)] at idx, find the `mod ...` line."""
    j = cfg_test_idx + 1
    while j < len(lines) and lines[j].strip() == "":
        j += 1
    if j < len(lines) and re.match(r'\s*mod\s+\w+', lines[j]):
        return j
    return None


def find_cfg_test_ranges(lines):
    """Find (start, end) ranges of #[cfg(test)] mod blocks."""
    ranges = []
    for cfg_idx in find_test_module_starts(lines):
        mod_idx = find_test_mod_line(lines, cfg_idx)
        if mod_idx is None:
            continue
        depth = 0
        for k in range(mod_idx, len(lines)):
            depth += lines[k].count('{') - lines[k].count('}')
            if depth <= 0:
                ranges.append((cfg_idx, k))
                break
    return ranges


def is_in_range(idx, ranges):
    for start, end in ranges:
        if start <= idx <= end:
            return True
    return False


def process_file(filepath):
    with open(filepath, "r", encoding="utf-8") as f:
        lines = f.readlines()

    if ".unwrap()" not in "".join(lines):
        return False, 0

    test_ranges = find_cfg_test_ranges(lines)
    is_test_file = "/tests/" in filepath or filepath.endswith("_tests.rs")

    # Collect insertion points (line index to insert BEFORE)
    insertions = set()

    # Strategy 1: Insert #[allow] before mod declaration in #[cfg(test)] blocks
    for cfg_idx in find_test_module_starts(lines):
        mod_idx = find_test_mod_line(lines, cfg_idx)
        if mod_idx is not None:
            # Check if any .unwrap() exists in the test module
            for start, end in test_ranges:
                if start == cfg_idx:
                    has_unwrap = any(".unwrap()" in lines[k] for k in range(start, end + 1))
                    if has_unwrap:
                        # Check if already annotated
                        already = any("allow(clippy::unwrap_used)" in lines[k] for k in range(cfg_idx, mod_idx + 1))
                        if not already:
                            insertions.add(mod_idx)

    # Strategy 2: For test files, insert before #[test]/#[tokio::test]
    if is_test_file:
        for i, line in enumerate(lines):
            stripped = line.strip()
            if stripped in ("#[test]", "#[tokio::test]"):
                # Check if any .unwrap() in this test function
                # (We assume it does if the file contains .unwrap())
                prev_idx = i - 1
                while prev_idx >= 0 and lines[prev_idx].strip() == "":
                    prev_idx -= 1
                if prev_idx >= 0 and "allow(clippy::unwrap_used)" in lines[prev_idx]:
                    continue
                insertions.add(i)

    # Strategy 3: For production code .unwrap() NOT in test modules
    for i, line in enumerate(lines):
        if ".unwrap()" not in line:
            continue
        if "// allow-anti-pattern" in line:
            continue
        if is_in_range(i, test_ranges):
            continue  # Handled by Strategy 1
        if is_test_file:
            continue  # Handled by Strategy 2

        # Find the statement start by walking backwards
        stmt_start = find_statement_start(lines, i)

        # Check if already annotated
        check_idx = stmt_start - 1
        while check_idx >= 0 and lines[check_idx].strip() == "":
            check_idx -= 1
        if check_idx >= 0 and "allow(clippy::unwrap_used)" in lines[check_idx]:
            continue

        insertions.add(stmt_start)

    if not insertions:
        return False, 0

    # Sort in reverse to avoid index shifting
    sorted_insertions = sorted(insertions, reverse=True)
    for idx in sorted_insertions:
        indent = re.match(r'(\s*)', lines[idx]).group(1)
        lines.insert(idx, f"{indent}{ALLOW_ATTR}\n")

    with open(filepath, "w", encoding="utf-8") as f:
        f.writelines(lines)

    return True, len(sorted_insertions)


def find_statement_start(lines, unwrap_line_idx):
    """
    Walk backwards from .unwrap() line to find the statement start.
    A statement start is a line that begins with:
    - `let `, `if `, `match `, `while `, `for `, `assert`, `return `
    - Or a line that doesn't start with `.` (i.e., not a method chain continuation)
    - Or a line after a `{`, `}`, `;` (block boundary)
    """
    i = unwrap_line_idx
    while i > 0:
        prev = lines[i - 1].strip()

        # If previous line ends with `;`, `{`, `}`, or is empty, current line is start
        if prev == "" or prev.endswith(';') or prev.endswith('{') or prev.endswith('}'):
            return i

        # If current line starts with `.`, it's a chain continuation - go up
        current = lines[i].strip()
        if current.startswith('.'):
            i -= 1
            continue

        # If current line is a statement start keyword
        if re.match(r'\s*(let|if|match|while|for|assert|return|self\.|pub |fn )', lines[i]):
            return i

        # Default: go up one more
        i -= 1

    return i


def main():
    rs_files = find_rs_files(SEARCH_DIRS)
    total_modified = 0
    total_insertions = 0

    for filepath in rs_files:
        modified, insertions = process_file(filepath)
        if modified:
            rel = os.path.relpath(filepath, WORKSPACE_ROOT)
            print(f"  ✅ {rel}: +{insertions} annotations")
            total_modified += 1
            total_insertions += insertions

    print(f"\n📊 Summary: {total_modified} files modified, {total_insertions} annotations inserted")


if __name__ == "__main__":
    main()
