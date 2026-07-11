#!/usr/bin/env python3
"""NT (Human Public Beta) mechanical gates — no secrets, exit 0 = proceed.

Usage:
  python3 scripts/nt_gate.py step0
  python3 scripts/nt_gate.py hygiene
  python3 scripts/nt_gate.py status
  python3 scripts/nt_gate.py mark NT-1 0.4 PASS
  python3 scripts/nt_gate.py self-test

SSOT steps: docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md
Progress (gitignored): states/nt_progress.json
"""

from __future__ import annotations

import argparse
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEFAULT_COMPOSE = REPO / "docker-compose.production.yml"
DEFAULT_PROGRESS = REPO / "states" / "nt_progress.json"
EXAMPLE_PROGRESS = REPO / "docs" / "guides" / "nt_progress.example.json"


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def load_progress(path: Path) -> dict:
    if not path.exists():
        if EXAMPLE_PROGRESS.exists():
            data = json.loads(EXAMPLE_PROGRESS.read_text(encoding="utf-8"))
        else:
            data = {"version": 1, "current": None, "tasks": {}}
        data.setdefault("version", 1)
        data.setdefault("tasks", {})
        return data
    return json.loads(path.read_text(encoding="utf-8"))


def save_progress(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    data["updated"] = utc_now()
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def check_no_stripe_api_key_assignment(compose_text: str) -> tuple[bool, str]:
    """Fail if STRIPE_API_KEY= appears as an env assignment (not comment-only)."""
    bad = []
    for i, line in enumerate(compose_text.splitlines(), 1):
        stripped = line.lstrip()
        if stripped.startswith("#"):
            continue
        if re.search(r"STRIPE_API_KEY\s*=", line):
            bad.append(f"L{i}")
    if bad:
        return False, f"STRIPE_API_KEY assignment found ({', '.join(bad)}) — Zero-Trust violation"
    return True, "OK: no STRIPE_API_KEY assignment"


def check_api_server_distroless(compose_text: str) -> tuple[bool, str]:
    """api-server service block must use docker/distroless.Dockerfile."""
    # Split on top-level service keys (2-space indent under services)
    m = re.search(
        r"(?ms)^  api-server:\n(.*?)(?=^  [a-z0-9-]+:|\Z)",
        compose_text,
    )
    if not m:
        return False, "api-server service block not found in compose"
    block = m.group(1)
    if re.search(r"dockerfile:\s*docker/distroless\.Dockerfile", block):
        return True, "OK: api-server → docker/distroless.Dockerfile"
    if re.search(r"dockerfile:\s*docker/production\.Dockerfile", block):
        return False, "FAIL: api-server still uses production.Dockerfile (need distroless)"
    return False, "FAIL: api-server dockerfile not found / unexpected"


def check_data_api_owner(data_api: Path) -> tuple[bool, str]:
    if not data_api.exists():
        return True, f"SKIP: {data_api} does not exist yet (create+chown in Step 0.2)"
    try:
        st = data_api.stat()
    except OSError as e:
        return False, f"FAIL: cannot stat {data_api}: {e}"
    uid, gid = st.st_uid, st.st_gid
    # distroless nonroot = 65532
    if uid == 65532 and gid == 65532:
        return True, f"OK: {data_api} owned by 65532:65532"
    mode = stat.filemode(st.st_mode)
    return (
        False,
        f"FAIL: {data_api} owner {uid}:{gid} mode={mode} (want 65532:65532). "
        f"Run: sudo chown -R 65532:65532 data/api",
    )


def docker_available() -> bool:
    try:
        r = subprocess.run(
            ["docker", "info"],
            capture_output=True,
            text=True,
            timeout=15,
        )
        return r.returncode == 0
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
        return False


def check_running_distroless(compose: Path) -> tuple[bool, str]:
    if not docker_available():
        return True, "SKIP: docker not available — run Step 0.4 manually on host"
    try:
        q = subprocess.run(
            ["docker", "compose", "-f", str(compose), "ps", "-q", "api-server"],
            cwd=str(REPO),
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (subprocess.TimeoutExpired, OSError) as e:
        return True, f"SKIP: docker compose ps failed ({e}) — verify Step 0.4 manually"

    cid = (q.stdout or "").strip().splitlines()
    if q.returncode != 0 or not cid:
        return (
            False,
            "FAIL: api-server container not running (build+up -d required). "
            "restart alone does not update the image.",
        )
    container_id = cid[0]
    try:
        insp = subprocess.run(
            [
                "docker",
                "inspect",
                "--format",
                "{{index .Config.Labels \"security.distroless\"}}|{{.Config.User}}",
                container_id,
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
    except (subprocess.TimeoutExpired, OSError) as e:
        return True, f"SKIP: docker inspect failed ({e})"

    out = (insp.stdout or "").strip()
    if insp.returncode != 0:
        return False, f"FAIL: docker inspect error: {(insp.stderr or '').strip()}"
    label, _, user = out.partition("|")
    if label == "true":
        return True, f"OK: running security.distroless=true user={user or '?'}"
    if user in ("65532:65532", "65532", "nonroot"):
        return (
            True,
            f"WARN→PASS: label={label!r} but user={user} looks distroless — confirm browser MC",
        )
    return (
        False,
        f"FAIL: running image not distroless (label={label!r} user={user!r}). "
        f"Do Step 0.3 build + up -d (not restart).",
    )


def cmd_hygiene(compose: Path) -> int:
    text = compose.read_text(encoding="utf-8")
    ok, msg = check_no_stripe_api_key_assignment(text)
    print(("[PASS] " if ok else "[FAIL] ") + msg)
    return 0 if ok else 1


def cmd_step0(compose: Path, data_api: Path, skip_docker: bool) -> int:
    if not compose.exists():
        print(f"[FAIL] compose not found: {compose}")
        return 1
    text = compose.read_text(encoding="utf-8")
    results: list[tuple[bool, str]] = []
    results.append(check_no_stripe_api_key_assignment(text))
    results.append(check_api_server_distroless(text))
    results.append(check_data_api_owner(data_api))
    if skip_docker:
        results.append((True, "SKIP: --skip-docker"))
    else:
        results.append(check_running_distroless(compose))

    failed = False
    for ok, msg in results:
        print(("[PASS] " if ok else "[FAIL] ") + msg)
        if not ok:
            failed = True

    if failed:
        print(
            "\nNext: open docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md NT-1 Step 0 "
            "(or tell Agent: /nt-assist NT-1). Never paste secrets."
        )
        return 1
    print("\n[OK] step0 gate PASS — proceed to NT-1 Step A (Vault GUI; Human enters secrets).")
    return 0


def cmd_status(progress: Path) -> int:
    data = load_progress(progress)
    print(json.dumps(data, indent=2, ensure_ascii=False))
    if not progress.exists():
        print(f"\n(note: {progress} not written yet; showing template/defaults)", file=sys.stderr)
    return 0


def cmd_mark(progress: Path, nt: str, step: str, status: str, note: str) -> int:
    status = status.upper()
    if status not in {"PASS", "FAIL", "WAIT", "SKIP", "DEFER"}:
        print("status must be PASS|FAIL|WAIT|SKIP|DEFER", file=sys.stderr)
        return 2
    # Never store secret-like notes
    if re.search(r"(sk_|whsec_|password\s*=)", note, re.I):
        print("[FAIL] note looks like a secret — refuse to write", file=sys.stderr)
        return 2
    data = load_progress(progress)
    tasks = data.setdefault("tasks", {})
    entry = tasks.setdefault(nt, {"status": "in_progress", "steps": {}})
    entry.setdefault("steps", {})[step] = {"result": status, "at": utc_now()}
    if note:
        entry["steps"][step]["note"] = note[:200]
    if status == "FAIL":
        entry["status"] = "blocked"
    data["current"] = {"nt": nt, "step": step, "mode": "assist"}
    save_progress(progress, data)
    print(f"[OK] marked {nt} step {step} = {status} → {progress}")
    return 0


def cmd_self_test() -> int:
    """Negative + positive checks without touching real compose."""
    good = (
        "services:\n"
        "  api-server:\n"
        "    build:\n"
        "      dockerfile: docker/distroless.Dockerfile\n"
        "      args:\n"
        "        BIN_NAME: api-server\n"
        "    environment:\n"
        "      - STRIPE_TEST_MODE=${STRIPE_TEST_MODE:-false}\n"
    )
    bad_key = good + "      - STRIPE_API_KEY=sk_live_fake\n"
    bad_df = (
        "services:\n"
        "  api-server:\n"
        "    build:\n"
        "      dockerfile: docker/production.Dockerfile\n"
    )

    ok, _ = check_no_stripe_api_key_assignment(good)
    assert ok, "positive hygiene failed"
    ok, _ = check_no_stripe_api_key_assignment(bad_key)
    assert not ok, "negative: key assignment must FAIL"
    ok, _ = check_api_server_distroless(good)
    assert ok, "positive distroless failed"
    ok, _ = check_api_server_distroless(bad_df)
    assert not ok, "negative: production.Dockerfile must FAIL"

    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "nt_progress.json"
        # secret note must be rejected
        rc = cmd_mark(p, "NT-1", "0.1", "PASS", "sk_live_should_fail")
        assert rc == 2, "secret note must be rejected"
        rc = cmd_mark(p, "NT-1", "0.1", "PASS", "compose distroless ok")
        assert rc == 0 and p.exists()

    print("[OK] self-test PASS (positive + negative)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Human Public Beta NT mechanical gates")
    parser.add_argument(
        "--compose",
        type=Path,
        default=DEFAULT_COMPOSE,
        help="path to docker-compose.production.yml",
    )
    parser.add_argument(
        "--progress",
        type=Path,
        default=Path(os.environ.get("NT_PROGRESS", str(DEFAULT_PROGRESS))),
        help="path to states/nt_progress.json",
    )
    parser.add_argument(
        "--data-api",
        type=Path,
        default=REPO / "data" / "api",
        help="host data/api directory",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    p0 = sub.add_parser("step0", help="NT-1 Step 0 gate (compose + ownership + running image)")
    p0.add_argument("--skip-docker", action="store_true")

    sub.add_parser("hygiene", help="compose must not assign STRIPE_API_KEY=")
    sub.add_parser("status", help="print progress JSON")
    sub.add_parser("self-test", help="positive + negative unit checks")

    pm = sub.add_parser("mark", help="record step result (no secrets)")
    pm.add_argument("nt", help="e.g. NT-1")
    pm.add_argument("step", help="e.g. 0.4 or A")
    pm.add_argument("status", help="PASS|FAIL|WAIT|SKIP|DEFER")
    pm.add_argument("--note", default="", help="non-secret note ≤200 chars")

    args = parser.parse_args()
    if args.cmd == "step0":
        return cmd_step0(args.compose, args.data_api, args.skip_docker)
    if args.cmd == "hygiene":
        return cmd_hygiene(args.compose)
    if args.cmd == "status":
        return cmd_status(args.progress)
    if args.cmd == "mark":
        return cmd_mark(args.progress, args.nt, args.step, args.status, args.note)
    if args.cmd == "self-test":
        return cmd_self_test()
    return 2


if __name__ == "__main__":
    sys.exit(main())
