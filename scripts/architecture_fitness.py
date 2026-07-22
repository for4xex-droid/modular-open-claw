#!/usr/bin/env python3
"""Architecture Fitness Harness (OP-090).

Thin checks only:
  F-1 shared ↛ infrastructure / api-server
  F-2 aiome-core-contracts ↛ infrastructure
  F-3 soul ↛ infrastructure
  F-4 prod .rs line counts (tests reported separately)
  F-5 optional: call pattern-enforcer / deep-scan (no reimplementation)

Local use by default. CI gating requires explicit follow-up permission.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent

DEP_EDGE_CHECKS = (
    ("F-1", "libs/shared/Cargo.toml", ("infrastructure", "api-server")),
    ("F-2", "libs/aiome-core-contracts/Cargo.toml", ("infrastructure",)),
    ("F-3", "libs/soul/Cargo.toml", ("infrastructure",)),
)

PROD_LINE_WARN = 800
PROD_TOP_N = 15

# Paths that count as test code for F-4 separation
TEST_PATH_MARKERS = (
    "/tests/",
    "/tests.rs",
    "_tests.rs",
    "/test_utils.rs",
    "/testing.rs",
)


@dataclass
class CheckResult:
    id: str
    passed: bool
    detail: str
    extras: dict[str, Any] = field(default_factory=dict)


def _repo_root(explicit: str | None) -> Path:
    if explicit:
        return Path(explicit).resolve()
    return REPO_ROOT


def _dep_keys_from_cargo_toml(text: str) -> set[str]:
    """Collect dependency package keys from [dependencies] and target tables."""
    keys: set[str] = set()
    in_deps = False
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].rstrip()
        if not line.strip():
            continue
        if line.startswith("["):
            header = line.strip()
            in_deps = header == "[dependencies]" or (
                header.startswith("[target.") and ".dependencies]" in header
            )
            continue
        if not in_deps:
            continue
        m = re.match(r'^([A-Za-z0-9_-]+)\s*=', line.strip())
        if m:
            keys.add(m.group(1))
    return keys


def check_dep_edges(root: Path) -> list[CheckResult]:
    results: list[CheckResult] = []
    for check_id, rel, forbidden in DEP_EDGE_CHECKS:
        path = root / rel
        if not path.is_file():
            results.append(
                CheckResult(check_id, False, f"missing Cargo.toml: {rel}")
            )
            continue
        deps = _dep_keys_from_cargo_toml(path.read_text(encoding="utf-8"))
        hits = sorted(d for d in forbidden if d in deps)
        if hits:
            results.append(
                CheckResult(
                    check_id,
                    False,
                    f"{rel} must not depend on {hits}; found among {sorted(deps)}",
                    {"forbidden_hits": hits},
                )
            )
        else:
            results.append(
                CheckResult(
                    check_id,
                    True,
                    f"{rel} has no forbidden deps {list(forbidden)}",
                )
            )
    return results


def _is_test_path(path: Path, root: Path) -> bool:
    # Resolve both sides (macOS /var vs /private/var symlinks).
    rel = "/" + path.resolve().relative_to(root.resolve()).as_posix()
    return any(m in rel for m in TEST_PATH_MARKERS)


def _count_lines(path: Path) -> int:
    try:
        with path.open("rb") as f:
            return sum(1 for _ in f)
    except OSError:
        return 0


def check_file_sizes(root: Path, warn_at: int = PROD_LINE_WARN, top_n: int = PROD_TOP_N) -> CheckResult:
    skip_dirs = {
        ".git",
        "target",
        "node_modules",
        "static.bak",
        "vendor",
        ".venv",
    }
    prod: list[tuple[int, str]] = []
    tests: list[tuple[int, str]] = []

    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [
            d
            for d in dirnames
            if d not in skip_dirs and not d.startswith("static.bak")
        ]
        for name in filenames:
            if not name.endswith(".rs"):
                continue
            path = Path(dirpath) / name
            # Ignore generated / backup trees
            rel = path.relative_to(root).as_posix()
            if "static.bak" in rel or "/target/" in f"/{rel}/":
                continue
            n = _count_lines(path)
            entry = (n, rel)
            if _is_test_path(path, root):
                tests.append(entry)
            else:
                prod.append(entry)

    prod.sort(reverse=True)
    tests.sort(reverse=True)
    warnings = [(n, p) for n, p in prod if n >= warn_at]
    detail = (
        f"prod files ≥{warn_at}: {len(warnings)}; "
        f"top prod={prod[0] if prod else None}; "
        f"top test={tests[0] if tests else None}"
    )
    return CheckResult(
        "F-4",
        True,  # informational; does not fail the harness by default
        detail,
        {
            "prod_top": [{"lines": n, "path": p} for n, p in prod[:top_n]],
            "test_top": [{"lines": n, "path": p} for n, p in tests[:top_n]],
            "prod_warn_ge": [{"lines": n, "path": p} for n, p in warnings],
            "warn_at": warn_at,
        },
    )


def run_optional_delegates(root: Path, run_enforcer: bool, run_deep_scan: bool) -> list[CheckResult]:
    out: list[CheckResult] = []
    if run_enforcer:
        script = root / "scripts" / "pattern-enforcer.sh"
        if script.is_file():
            proc = subprocess.run(
                ["bash", str(script)],
                cwd=str(root),
                capture_output=True,
                text=True,
            )
            out.append(
                CheckResult(
                    "F-5a",
                    proc.returncode == 0,
                    f"pattern-enforcer exit={proc.returncode}",
                    {"stdout_tail": proc.stdout[-2000:], "stderr_tail": proc.stderr[-2000:]},
                )
            )
        else:
            out.append(CheckResult("F-5a", False, "pattern-enforcer.sh missing"))
    if run_deep_scan:
        script = root / "scripts" / "deep-scan.sh"
        if script.is_file():
            proc = subprocess.run(
                ["bash", str(script), "--ci"],
                cwd=str(root),
                capture_output=True,
                text=True,
            )
            out.append(
                CheckResult(
                    "F-5b",
                    proc.returncode == 0,
                    f"deep-scan --ci exit={proc.returncode}",
                    {"stdout_tail": proc.stdout[-2000:], "stderr_tail": proc.stderr[-2000:]},
                )
            )
        else:
            out.append(CheckResult("F-5b", False, "deep-scan.sh missing"))
    return out


def run_fitness(
    root: Path,
    *,
    run_enforcer: bool = False,
    run_deep_scan: bool = False,
) -> dict[str, Any]:
    checks = check_dep_edges(root)
    checks.append(check_file_sizes(root))
    checks.extend(run_optional_delegates(root, run_enforcer, run_deep_scan))

    # Gate exit on F-1..F-3 only (F-4 is report; F-5 optional)
    hard = [c for c in checks if c.id in {"F-1", "F-2", "F-3"}]
    passed = all(c.passed for c in hard)
    return {
        "passed": passed,
        "checks": [
            {
                "id": c.id,
                "passed": c.passed,
                "detail": c.detail,
                **({"extras": c.extras} if c.extras else {}),
            }
            for c in checks
        ],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Architecture Fitness Harness (OP-090)")
    parser.add_argument("--root", default=None, help="Repository root (default: auto)")
    parser.add_argument("--json", action="store_true", help="Emit JSON report")
    parser.add_argument(
        "--with-pattern-enforcer",
        action="store_true",
        help="Optionally invoke scripts/pattern-enforcer.sh (F-5a)",
    )
    parser.add_argument(
        "--with-deep-scan",
        action="store_true",
        help="Optionally invoke scripts/deep-scan.sh --ci (F-5b)",
    )
    args = parser.parse_args(argv)
    root = _repo_root(args.root)
    report = run_fitness(
        root,
        run_enforcer=args.with_pattern_enforcer,
        run_deep_scan=args.with_deep_scan,
    )
    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        status = "PASS" if report["passed"] else "FAIL"
        print(f"[architecture_fitness] {status}")
        for c in report["checks"]:
            mark = "OK" if c["passed"] else "NG"
            print(f"  [{mark}] {c['id']}: {c['detail']}")
            if c["id"] == "F-4" and "extras" in c:
                print("    prod_top:")
                for row in c["extras"].get("prod_top", [])[:8]:
                    print(f"      {row['lines']:>5}  {row['path']}")
                print("    test_top:")
                for row in c["extras"].get("test_top", [])[:5]:
                    print(f"      {row['lines']:>5}  {row['path']}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
