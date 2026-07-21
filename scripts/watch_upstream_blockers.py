#!/usr/bin/env python3
"""
Gate α (Upstream Reachable) watcher for OPEN OP-030 / OP-032 / OP-033.

- Exit 0: all *gate* targets still below threshold
- Exit 1: at least one gate target reached (ACTION REQUIRED)
- Do not pipe into `tail` when checking $?; use the process exit directly
- Not wired into ci.yml / HEARTBEAT.md (see foolproof §8)

wasmtime is printed for operators (45-cleared hint) but does not drive exit codes.
"""

from __future__ import annotations

import json
import sys
import urllib.error
import urllib.request
from typing import Any

USER_AGENT = "Aiome-Upstream-Watcher/1.1"

# Gate α targets — `gate=True` contributes to exit 1 when reached.
# Issue letters match .cargo/audit.toml comments; OP-* match OPEN.md.
TARGETS: list[dict[str, Any]] = [
    {
        "crate": "serenity",
        "version": "0.13.0",
        "op": "OP-030",
        "issue": "Issue A",
        "label": "Discord / serenity TLS path",
        "gate": True,
    },
    {
        "crate": "extism",
        "version": "1.22.0",
        "op": "OP-032",
        "issue": "Issue C",
        "label": "Extism → wasmtime tree",
        "gate": True,
    },
    {
        "crate": "tauri",
        "version": "3.0.0",
        "op": "OP-033",
        "issue": "Issue D",
        "label": "Tauri GTK4/unic path",
        "gate": True,
    },
]

# Informational only (does not set exit 1). Helps Gate β 45-cleared planning.
WASM_INFO = {
    "crate": "wasmtime",
    "version": "45.0.0",
    "op": "OP-068/032",
    "issue": "info",
    "label": "wasmtime/wasi 45+ clears RUSTSEC-2026-0188 (44.x insufficient)",
    "gate": False,
}


def parse_version(v: str) -> list[int]:
    """Parse leading X.Y.Z from a crates.io version string (pre-release suffix ignored)."""
    core = v.strip().split("+")[0].split("-")[0]
    parts: list[int] = []
    for x in core.split("."):
        if not x.isdigit():
            break
        parts.append(int(x))
    while len(parts) < 3:
        parts.append(0)
    return parts[:3]


def version_reached(current: str, target: str) -> bool:
    return parse_version(current) >= parse_version(target)


def check_crate_target(crate_name: str, target_version: str) -> dict[str, Any]:
    """
    Check if the specified crate has reached or exceeded the target version.
    Returns dict with 'reached' (bool) and 'current_version' (str).
    """
    url = f"https://crates.io/api/v1/crates/{crate_name}"
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})

    with urllib.request.urlopen(req, timeout=30) as response:
        data = json.loads(response.read().decode("utf-8"))

    max_version = data.get("crate", {}).get("max_stable_version") or "0.0.0"
    return {
        "reached": version_reached(max_version, target_version),
        "current_version": max_version,
    }


def format_status_line(target: dict[str, Any], current: str, reached: bool) -> str:
    op = target["op"]
    issue = target["issue"]
    crate = target["crate"]
    need = target["version"]
    role = "GATE" if target.get("gate", True) else "INFO"
    flag = "UNBLOCKED" if reached else "BLOCKED"
    return (
        f"  [{role}] {op} / {issue}: {crate} "
        f"current={current} need>={need} → {flag}"
    )


def run_watch(targets: list[dict[str, Any]] | None = None) -> int:
    """
    Run Gate α checks. Returns process exit code (0=all gate blocked, 1=any gate unblocked).
    """
    targets = targets if targets is not None else list(TARGETS)
    unblocked: list[dict[str, Any]] = []
    errors: list[str] = []

    print("Aiome Upstream Watcher (Gate α) — foolproof §8")
    print("Gate targets drive exit code; INFO lines do not.\n")

    for target in targets:
        crate = target["crate"]
        need = target["version"]
        print(f"Checking {crate} (waiting for >= {need})...")
        try:
            result = check_crate_target(crate, need)
            print(format_status_line(target, result["current_version"], result["reached"]))
            if result["reached"] and target.get("gate", True):
                unblocked.append(target)
                print(f"  [!] ACTION: {target['op']} Gate α satisfied — run Gate β before ignore removal.")
            elif not result["reached"]:
                print("  [ ] still below threshold.")
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, KeyError) as e:
            msg = f"{crate}: {e}"
            errors.append(msg)
            print(f"  Error checking {crate}: {e}")

    # Informational wasmtime line (never contributes to exit 1).
    print(f"\nChecking {WASM_INFO['crate']} (info only, need>={WASM_INFO['version']} for 0188)...")
    try:
        w = check_crate_target(WASM_INFO["crate"], WASM_INFO["version"])
        print(format_status_line(WASM_INFO, w["current_version"], w["reached"]))
        if w["reached"]:
            print("  [i] crates.io wasmtime is 45+; still confirm lock tree (Gate β) before deleting 0188.")
        else:
            print("  [i] 0188 (wasi FilePerms) still needs wasi/wasmtime 45+ in the *lockfile*.")
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, KeyError) as e:
        print(f"  Error checking wasmtime (info): {e}")

    if errors and not unblocked:
        print("\nWatcher completed with fetch errors and no gate unblocks (exit 0).")
        print("Re-run without piping to verify network.")

    if unblocked:
        print("\n" + "=" * 50)
        print("ACTION REQUIRED: GATE α UNBLOCKED")
        print("=" * 50)
        for t in unblocked:
            print(
                f"- {t['op']} ({t['issue']}): {t['crate']} >= {t['version']} — "
                f"{t['label']}. Next: Gate β (tree/deny/audit), then implement if permitted."
            )
        print("\nDo not delete deny.toml ignores on α alone.")
        return 1

    print("\nAll Gate α targets are still blocked.")
    return 0


def main() -> None:
    sys.exit(run_watch())


if __name__ == "__main__":
    main()
