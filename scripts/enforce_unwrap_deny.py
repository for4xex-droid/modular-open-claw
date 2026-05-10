#!/usr/bin/env python3
"""
enforce_unwrap_deny.py — Rust コードベース内の未承認 .unwrap() / .expect() を検出する CI ガード。

検出ルール:
  - .unwrap() および .expect(...) をプロダクションコードで禁止する。
  - `// allow-unwrap` または `// allow-anti-pattern` アノテーション付きの行は許可する。
  - `#[cfg(test)]` ブロック内、テスト専用ファイル、`tests/` ディレクトリは除外する。
  - コメント行 (`//` で始まる行) は無視する。

終了コード:
  0 = 違反なし
  1 = 違反あり
"""

import sys
import os
import argparse

# --- opt-out アノテーション ---
OPT_OUT_MARKERS = ("allow-unwrap", "allow-anti-pattern")

# --- テストファイル名パターン ---
TEST_FILE_SUFFIXES = ("_test.rs", "_tests.rs")
TEST_FILE_EXACT = ("tests.rs",)


def _strip_trailing_comment(line: str) -> str:
    """行末コメントを除去する。文字列リテラル内の `//` は保持する。

    簡易的なステートマシンで `"` のトグルを追跡し、
    文字列リテラル外で最初に出現する `//` 以降を切り捨てる。
    """
    in_string = False
    escape_next = False
    for i, ch in enumerate(line):
        if escape_next:
            escape_next = False
            continue
        if ch == '\\':
            escape_next = True
            continue
        if ch == '"':
            in_string = not in_string
            continue
        if not in_string and ch == '/' and i + 1 < len(line) and line[i + 1] == '/':
            return line[:i]
    return line


def check_line(line: str) -> bool:
    """単一行が .unwrap() / .expect() 違反かどうかを判定する。"""
    stripped = line.strip()

    # コメント行は無視
    if stripped.startswith("//"):
        return False

    # opt-out アノテーション付きなら許可
    for marker in OPT_OUT_MARKERS:
        if marker in stripped:
            return False

    # コード部分のみ（文字列リテラル安全な手法でコメント除去）
    code_part = _strip_trailing_comment(line)

    if ".unwrap()" in code_part or ".expect(" in code_part:
        return True

    # 抜け道（パニック誘発マクロ）の検出
    for macro in ("panic!(", "todo!(", "unimplemented!(", "unreachable!("):
        if macro in code_part:
            return True

    return False


def check_file(filepath: str) -> list:
    """ファイル内のプロダクションコード領域をスキャンし、違反を返す。"""
    violations = []

    try:
        with open(filepath, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except (OSError, UnicodeDecodeError) as e:
        print(f"⚠️  Warning: Could not read {filepath}: {e}", file=sys.stderr)
        return violations

    in_test_block = False
    brace_depth = 0
    test_entry_depth = -1

    for i, raw_line in enumerate(lines):
        line_num = i + 1

        # --- コメント外の波括弧のみカウント ---
        code_only = _strip_trailing_comment(raw_line)
        brace_depth += code_only.count("{") - code_only.count("}")

        # テストブロック終了判定 (波括弧がテスト開始時の深さを下回ったら終了)
        if in_test_block and test_entry_depth != -1 and brace_depth < test_entry_depth:
            in_test_block = False
            test_entry_depth = -1

        # #[cfg(test)] 検出 → 以降のブロックをテストとしてマーク
        if "#[cfg(test)]" in raw_line:
            if not in_test_block:
                in_test_block = True
                # The depth of the scope that CONTAINS the test module is the current depth minus any '{' on this line
                test_entry_depth = brace_depth - code_only.count("{")
            continue
        
        # If we are in test block but haven't entered the module body yet (e.g. annotations)
        if in_test_block and test_entry_depth != -1 and brace_depth == test_entry_depth and "{" not in code_only:
            pass # still waiting for the module block to start

        # テストブロック内はスキップ
        if in_test_block:
            continue

        if check_line(raw_line):
            # 前後の行にアノテーションがないか確認する (rustfmt対策)
            is_allowed = False
            for offset in (-1, 1):
                adj_idx = i + offset
                if 0 <= adj_idx < len(lines):
                    adj_line = lines[adj_idx].strip()
                    if adj_line.startswith("//"):
                        for marker in OPT_OUT_MARKERS:
                            if marker in adj_line:
                                is_allowed = True
                                break
                if is_allowed:
                    break

            if not is_allowed:
                if ".unwrap()" in code_only:
                    kind = ".unwrap()"
                elif ".expect(" in code_only:
                    kind = ".expect()"
                elif "panic!(" in code_only:
                    kind = "panic!()"
                elif "todo!(" in code_only:
                    kind = "todo!()"
                elif "unimplemented!(" in code_only:
                    kind = "unimplemented!()"
                elif "unreachable!(" in code_only:
                    kind = "unreachable!()"
                else:
                    kind = "Zero-Panic violation"

                violations.append({
                    "line_number": line_num,
                    "reason": f"Unsafe {kind} found",
                })

    return violations


def scan_directory(directory: str) -> list:
    """ディレクトリを再帰走査し、違反を収集する。"""
    all_violations = []

    for root, _dirs, files in os.walk(directory):
        # tests/ ディレクトリ全体を除外
        path_parts = root.split(os.sep)
        if "tests" in path_parts:
            continue

        for filename in files:
            if not filename.endswith(".rs"):
                continue

            # テストファイルを名前で除外
            if filename.endswith(TEST_FILE_SUFFIXES) or filename in TEST_FILE_EXACT:
                continue

            filepath = os.path.join(root, filename)
            file_violations = check_file(filepath)
            for v in file_violations:
                all_violations.append({
                    "file": filepath,
                    "line": v["line_number"],
                    "reason": v["reason"],
                })

    return all_violations


def main():
    parser = argparse.ArgumentParser(
        description="Enforce no unwrap() or expect() in Rust production code"
    )
    parser.add_argument("directories", nargs="+", help="Directories to scan")
    args = parser.parse_args()

    total_violations = []
    for d in args.directories:
        if not os.path.isdir(d):
            print(f"⚠️  Warning: '{d}' is not a directory, skipping.", file=sys.stderr)
            continue
        total_violations.extend(scan_directory(d))

    if total_violations:
        print(f"🚨 Found {len(total_violations)} illegal uses of .unwrap() or .expect():")
        for v in total_violations:
            print(f"  {v['file']}:{v['line']} — {v['reason']}")
        sys.exit(1)
    else:
        print("✅ No illegal unwraps found!")
        sys.exit(0)


if __name__ == "__main__":
    main()
