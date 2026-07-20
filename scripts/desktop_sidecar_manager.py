#!/usr/bin/env python3
"""Tauri Desktop sidecar build / placeholder / physical validation (OP-088 P3 + OP-089).

Official package sidecars: api-server + key-proxy (+ obscura for --check-all).
nurture-api is opt-in via --with-nurture-sidecar (Local escape / Economy only).

OP-089 channels:
  economy (default) — api-server --features nurture (Nurture InProcess)
  oss               — api-server with no nurture feature (commercial/ not linked)
"""
import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

# 公式 Desktop 同梱（InProcess 既定。nurture-api は含まない）
OFFICIAL_BINARIES = ["api-server", "key-proxy"]
# 開発用 Local escape（--with-nurture-sidecar）
NURTURE_SIDECAR = "nurture-api"
# リリース完全検証に含める任意バイナリ
OPTIONAL_RELEASE_BINARIES = ["obscura"]

# OP-089: 配布チャネル
CHANNEL_ECONOMY = "economy"
CHANNEL_OSS = "oss"
VALID_CHANNELS = (CHANNEL_ECONOMY, CHANNEL_OSS)

# 後方互換エイリアス（テスト・ドキュメント）
CORE_BINARIES = OFFICIAL_BINARIES
ALL_BINARIES = OFFICIAL_BINARIES + OPTIONAL_RELEASE_BINARIES


def normalize_channel(channel: str | None) -> str:
    raw = (channel or CHANNEL_ECONOMY).strip().lower()
    if raw not in VALID_CHANNELS:
        raise ValueError(
            f"invalid channel '{channel}'; use {CHANNEL_ECONOMY}|{CHANNEL_OSS}"
        )
    return raw


def api_server_cargo_features(channel: str) -> list[str]:
    """チャネル別の api-server cargo --features 引数（空 = feature なし）。"""
    ch = normalize_channel(channel)
    if ch == CHANNEL_ECONOMY:
        return ["nurture"]
    return []


def write_channel_manifest(binaries_dir: Path, channel: str, triple: str) -> Path:
    """ビルド成果物横にチャネルメタを書く（検査・配布命名の根拠）。"""
    ch = normalize_channel(channel)
    features = api_server_cargo_features(ch)
    payload = {
        "channel": ch,
        "triple": triple,
        "api_server_features": features,
        "official_binaries": list(OFFICIAL_BINARIES),
        "nurture_sidecar_allowed": False,
        "asset_name_prefix": (
            "AiomeOS-Economy" if ch == CHANNEL_ECONOMY else "AiomeOS-OSS"
        ),
    }
    path = Path(binaries_dir) / "channel-manifest.json"
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {path.name} (channel={ch})")
    return path


def verify_api_server_channel_link(channel: str) -> None:
    """Fail-Closed: Economy は nurture-api 依存必須 / OSS は禁止（cargo tree）。"""
    ch = normalize_channel(channel)
    features = api_server_cargo_features(ch)
    cmd = [
        "cargo",
        "tree",
        "-p",
        "api-server",
        "-i",
        "nurture-api",
        "--depth",
        "0",
        "--quiet",
    ]
    if features:
        cmd.extend(["--features", ",".join(features)])

    result = subprocess.run(cmd, capture_output=True, text=True)
    out = (result.stdout or "") + (result.stderr or "")
    has_nurture = result.returncode == 0 and "nurture-api" in out

    if ch == CHANNEL_ECONOMY:
        if not has_nurture:
            raise ValueError(
                "Economy channel must link nurture-api "
                f"(cargo tree failed or missing; rc={result.returncode})"
            )
        print("OK: Economy channel links nurture-api")
        return

    if has_nurture:
        raise ValueError(
            "OSS channel must NOT link nurture-api "
            "(cargo tree found nurture-api without --features nurture)"
        )
    print("OK: OSS channel does not link nurture-api")

# マジックバイトの定義
ELF_MAGIC = b"\x7fELF"
PE_MAGIC = b"MZ"
MACHO_MAGICS = [
    b"\xca\xfe\xba\xbe",  # FAT
    b"\xfe\xed\xfa\xce",  # 32-bit
    b"\xce\xfa\xed\xfe",  # 32-bit reverse
    b"\xfe\xed\xfa\xcf",  # 64-bit
    b"\xcf\xfa\xed\xfe",  # 64-bit reverse
]
MIN_BINARY_SIZE = 100000  # 100KB


def detect_target_triple():
    """ホストのターゲットトリプルを自動検出する。

    通常は `rustc -vV` を利用し、失敗した場合は platform から推測する。
    """
    try:
        output = subprocess.check_output(["rustc", "-vV"], stderr=subprocess.DEVNULL).decode("utf-8")
        for line in output.splitlines():
            if line.startswith("host:"):
                return line.split("host:")[1].strip()
    except (subprocess.SubprocessError, FileNotFoundError):
        pass

    # フォールバック検出
    sys_plat = sys.platform
    arch = platform.machine().lower()

    if sys_plat == "darwin":
        if arch in ["arm64", "aarch64"]:
            return "aarch64-apple-darwin"
        return "x86_64-apple-darwin"
    elif sys_plat == "win32":
        if arch in ["amd64", "x86_64"]:
            return "x86_64-pc-windows-msvc"
        elif arch in ["arm64", "aarch64"]:
            return "aarch64-pc-windows-msvc"
        return "x86_64-pc-windows-msvc"  # デフォルト
    else:  # Linux またはその他
        if arch in ["x86_64", "amd64"]:
            return "x86_64-unknown-linux-gnu"
        elif arch in ["aarch64", "arm64"]:
            return "aarch64-unknown-linux-gnu"
        return "x86_64-unknown-linux-gnu"  # デフォルト


def get_sidecar_filename(binary_name: str, triple: str, is_windows: bool | None = None) -> str:
    """OSごとのサイドカーバイナリ名を取得する。"""
    if is_windows is None:
        is_windows = sys.platform == "win32" or "windows-msvc" in triple

    ext = ".exe" if is_windows else ""
    return f"{binary_name}-{triple}{ext}"


def official_placeholder_binaries(with_nurture_sidecar: bool = False) -> list[str]:
    """プレースホルダー生成対象（公式 + obscura。Local 用は opt-in）。"""
    names = list(ALL_BINARIES)
    if with_nurture_sidecar:
        names = OFFICIAL_BINARIES + [NURTURE_SIDECAR] + OPTIONAL_RELEASE_BINARIES
    return names


def generate_placeholders(
    binaries_dir: str | Path,
    triple: str,
    target_binaries: list[str] | None = None,
    with_nurture_sidecar: bool = False,
) -> None:
    """ダミープレースホルダーを生成する。

    target_binaries が指定された場合、そのバイナリのみを生成する。
    未指定の場合は公式セット（+ obscura）。`--with-nurture-sidecar` で nurture-api も生成。
    """
    binaries_dir = Path(binaries_dir)
    binaries_dir.mkdir(parents=True, exist_ok=True)
    is_windows = sys.platform == "win32" or "windows-msvc" in triple
    binaries = (
        target_binaries
        if target_binaries is not None
        else official_placeholder_binaries(with_nurture_sidecar)
    )

    for name in binaries:
        filename = get_sidecar_filename(name, triple, is_windows)
        file_path = binaries_dir / filename

        if is_windows:
            # Windowsバッチファイル
            content = "@exit /b 0\n"
        else:
            # Unixシェルスクリプト
            content = "#!/bin/sh\nexit 0\n"

        with open(file_path, "w") as f:
            f.write(content)

        if not is_windows:
            os.chmod(file_path, 0o755)

        print(f"Generated placeholder sidecar: {file_path.name}")


def is_real_binary(file_path):
    """指定されたファイルが本物のバイナリかどうかを物理判定する。

    マジックバイトチェックと最小サイズチェックを行う。
    """
    file_path = Path(file_path)
    if not file_path.exists():
        return False

    # サイズチェック
    if file_path.stat().st_size < MIN_BINARY_SIZE:
        return False

    # マジックバイトチェック
    try:
        with open(file_path, "rb") as f:
            header = f.read(4)
    except IOError:
        return False

    if len(header) < 2:
        return False

    # PE (Windows)
    if header.startswith(PE_MAGIC):
        return True

    # ELF (Linux)
    if header.startswith(ELF_MAGIC):
        return True

    # Mach-O (macOS)
    for magic in MACHO_MAGICS:
        if header.startswith(magic):
            return True

    return False


def assert_nurture_sidecar_not_shipped(binaries_dir, triple):
    """公式リリース検証: 実バイナリの nurture-api が binaries/ に混入していないこと。"""
    binaries_dir = Path(binaries_dir)
    is_windows = sys.platform == "win32" or "windows-msvc" in triple
    filename = get_sidecar_filename(NURTURE_SIDECAR, triple, is_windows)
    file_path = binaries_dir / filename
    if file_path.exists() and is_real_binary(file_path):
        error_msg = (
            f"Official package must not ship real {NURTURE_SIDECAR} sidecar "
            f"(found {filename}). Rebuild without --with-nurture-sidecar."
        )
        print(f"ERROR: {error_msg}", file=sys.stderr)
        raise ValueError(error_msg)


def check_binaries(binaries_dir, triple, check_all=False, forbid_nurture_sidecar=False):
    """バイナリの妥当性を検証する。

    ダミーや欠損を検出した場合は ValueError を発生させる。
    forbid_nurture_sidecar=True（--check-all 既定）のとき実 nurture-api 混入を拒否。
    """
    binaries_dir = Path(binaries_dir)
    target_binaries = ALL_BINARIES if check_all else CORE_BINARIES
    is_windows = sys.platform == "win32" or "windows-msvc" in triple

    missing_or_dummy = []

    for name in target_binaries:
        filename = get_sidecar_filename(name, triple, is_windows)
        file_path = binaries_dir / filename

        if not file_path.exists():
            missing_or_dummy.append(f"{filename} (not found)")
        elif not is_real_binary(file_path):
            missing_or_dummy.append(f"{filename} (dummy placeholder)")

    if missing_or_dummy:
        error_msg = f"Validation failed. The following binaries are missing or dummy: {', '.join(missing_or_dummy)}"
        print(f"ERROR: {error_msg}", file=sys.stderr)
        raise ValueError(error_msg)

    if forbid_nurture_sidecar:
        assert_nurture_sidecar_not_shipped(binaries_dir, triple)

    print(f"Validation passed for triple {triple} (check_all={check_all})")


def run_build(
    binaries_dir,
    triple,
    with_nurture_sidecar: bool = False,
    channel: str = CHANNEL_ECONOMY,
):
    """Rustサイドカーバイナリのビルドとコピーを行う。"""
    channel = normalize_channel(channel)
    if with_nurture_sidecar and channel == CHANNEL_OSS:
        raise ValueError(
            "OSS channel cannot include nurture-api sidecar "
            "(use --channel economy --with-nurture-sidecar for Local escape)"
        )

    binaries_dir = Path(binaries_dir)
    binaries_dir.mkdir(parents=True, exist_ok=True)
    is_windows = sys.platform == "win32" or "windows-msvc" in triple
    ext = ".exe" if is_windows else ""
    features = api_server_cargo_features(channel)

    print(
        f"Building Rust sidecars (channel={channel}; official: api-server + key-proxy)..."
    )

    # 1. api-server — Economy: --features nurture / OSS: no nurture feature
    if features:
        feat_label = ",".join(features)
        print(f"Building api-server (--features {feat_label}; channel={channel})...")
        cargo_api = [
            "cargo",
            "build",
            "--release",
            "-p",
            "api-server",
            "--features",
            feat_label,
        ]
    else:
        print(f"Building api-server (no nurture feature; channel={channel})...")
        cargo_api = ["cargo", "build", "--release", "-p", "api-server"]
    subprocess.run(cargo_api, check=True)
    src_api_server = Path("target/release") / f"api-server{ext}"
    dst_api_server = binaries_dir / get_sidecar_filename("api-server", triple, is_windows)
    shutil.copy2(src_api_server, dst_api_server)
    print(f"Copied api-server -> {dst_api_server.name}")

    # 2. key-proxy
    print("Building key-proxy...")
    subprocess.run(["cargo", "build", "--release", "-p", "key-proxy"], check=True)
    src_key_proxy = Path("target/release") / f"key-proxy{ext}"
    dst_key_proxy = binaries_dir / get_sidecar_filename("key-proxy", triple, is_windows)
    shutil.copy2(src_key_proxy, dst_key_proxy)
    print(f"Copied key-proxy -> {dst_key_proxy.name}")

    # 3. nurture-api — 公式は除外。Economy Local escape のみ
    if with_nurture_sidecar:
        print("Building nurture-api (desktop; --with-nurture-sidecar)...")
        subprocess.run(
            [
                "cargo",
                "build",
                "--release",
                "-p",
                "nurture-api",
                "--no-default-features",
                "--features",
                "desktop",
            ],
            check=True,
        )
        src_nurture_api = Path("target/release") / f"nurture-api{ext}"
        dst_nurture_api = binaries_dir / get_sidecar_filename("nurture-api", triple, is_windows)
        shutil.copy2(src_nurture_api, dst_nurture_api)
        print(f"Copied nurture-api -> {dst_nurture_api.name}")
        print(
            "NOTE: Official tauri.conf.json does not list nurture-api in externalBin. "
            "Use a dev override or NURTURE_MODE=local only with a package that includes it."
        )
    else:
        # 旧ビルド成果物が残って公式同梱されないよう削除
        stale = binaries_dir / get_sidecar_filename(NURTURE_SIDECAR, triple, is_windows)
        if stale.exists():
            stale.unlink()
            print(f"Removed stale {stale.name} (official build excludes nurture-api)")

    # 4. obscura (システムPATHにあればコピー、なければ警告してプレースホルダーを維持)
    obscura_filename = get_sidecar_filename("obscura", triple, is_windows)
    dst_obscura = binaries_dir / obscura_filename

    obscura_path = shutil.which("obscura")
    if obscura_path:
        print(f"Found obscura in PATH: {obscura_path}")
        shutil.copy2(obscura_path, dst_obscura)
        print(f"Copied obscura -> {dst_obscura.name}")
    else:
        print("WARNING: 'obscura' binary was not found in PATH.")
        if not dst_obscura.exists():
            # obscura のみのプレースホルダーを生成（既存のビルド済みバイナリを上書きしない）
            print("Creating fallback dummy placeholder for obscura...")
            generate_placeholders(binaries_dir, triple, target_binaries=["obscura"])

    write_channel_manifest(binaries_dir, channel, triple)
    verify_api_server_channel_link(channel)


def main():
    parser = argparse.ArgumentParser(
        description="Tauri Sidecar Manager & Guardian (OP-088 P3 / OP-089 channels)"
    )
    parser.add_argument(
        "--setup-placeholders",
        action="store_true",
        help="Generate dummy placeholder shell scripts or batch files",
    )
    parser.add_argument(
        "--build",
        action="store_true",
        help="Build production-ready sidecars and place them",
    )
    parser.add_argument(
        "--channel",
        type=str,
        default=CHANNEL_ECONOMY,
        choices=list(VALID_CHANNELS),
        help="OP-089 distribution channel (default: economy)",
    )
    parser.add_argument(
        "--verify-channel-link",
        action="store_true",
        help="Fail-Closed cargo-tree check for channel nurture-api linkage (no binary build)",
    )
    parser.add_argument(
        "--with-nurture-sidecar",
        action="store_true",
        help="Also build/copy nurture-api (Local escape / Economy only; not in official package)",
    )
    parser.add_argument(
        "--check-core",
        action="store_true",
        help="Stage 1: verify official Rust sidecars (api-server, key-proxy)",
    )
    parser.add_argument(
        "--check-all",
        action="store_true",
        help="Stage 2: official + obscura, and forbid real nurture-api (release)",
    )
    parser.add_argument(
        "--forbid-nurture-sidecar",
        action="store_true",
        help="With --check-core: also Fail-Closed if a real nurture-api binary is present",
    )
    parser.add_argument(
        "--binaries-dir",
        type=str,
        default="apps/management-console/src-tauri/binaries",
        help="Path to Tauri sidecar binaries directory",
    )

    args = parser.parse_args()

    triple = detect_target_triple()
    print(f"Detected target triple: {triple}")

    binaries_dir = Path(args.binaries_dir)

    try:
        if args.verify_channel_link:
            verify_api_server_channel_link(args.channel)
        elif args.setup_placeholders:
            generate_placeholders(
                binaries_dir,
                triple,
                with_nurture_sidecar=args.with_nurture_sidecar,
            )
        elif args.build:
            run_build(
                binaries_dir,
                triple,
                with_nurture_sidecar=args.with_nurture_sidecar,
                channel=args.channel,
            )
        elif args.check_core:
            check_binaries(
                binaries_dir,
                triple,
                check_all=False,
                forbid_nurture_sidecar=args.forbid_nurture_sidecar,
            )
        elif args.check_all:
            check_binaries(binaries_dir, triple, check_all=True, forbid_nurture_sidecar=True)
        else:
            parser.print_help()
            sys.exit(1)
    except ValueError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Unexpected error: {e}", file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()
