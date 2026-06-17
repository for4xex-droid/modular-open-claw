#!/usr/bin/env python3
import argparse
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

# サイドカーバイナリのリスト
CORE_BINARIES = ["api-server", "key-proxy", "nurture-api"]
ALL_BINARIES = CORE_BINARIES + ["obscura"]

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


def generate_placeholders(binaries_dir: str | Path, triple: str, target_binaries: list[str] | None = None) -> None:
    """ダミープレースホルダーを生成する。

    target_binaries が指定された場合、そのバイナリのみを生成する。
    未指定の場合は ALL_BINARIES 全体を生成する。
    """
    binaries_dir = Path(binaries_dir)
    binaries_dir.mkdir(parents=True, exist_ok=True)
    is_windows = sys.platform == "win32" or "windows-msvc" in triple
    binaries = target_binaries if target_binaries is not None else ALL_BINARIES

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


def check_binaries(binaries_dir, triple, check_all=False):
    """バイナリの妥当性を検証する。

    ダミーや欠損を検出した場合は ValueError を発生させる。
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

    print(f"Validation passed for triple {triple} (check_all={check_all})")


def run_build(binaries_dir, triple):
    """Rustサイドカーバイナリのビルドとコピーを行う。"""
    binaries_dir = Path(binaries_dir)
    binaries_dir.mkdir(parents=True, exist_ok=True)
    is_windows = sys.platform == "win32" or "windows-msvc" in triple
    ext = ".exe" if is_windows else ""

    print("Building Rust sidecars...")

    # 1. api-server
    print("Building api-server...")
    subprocess.run(["cargo", "build", "--release", "-p", "api-server"], check=True)
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

    # 3. nurture-api (desktop featureのみでAWS SDKを除外)
    print("Building nurture-api (desktop configurations)...")
    subprocess.run(
        ["cargo", "build", "--release", "-p", "nurture-api", "--no-default-features", "--features", "desktop"],
        check=True
    )
    src_nurture_api = Path("target/release") / f"nurture-api{ext}"
    dst_nurture_api = binaries_dir / get_sidecar_filename("nurture-api", triple, is_windows)
    shutil.copy2(src_nurture_api, dst_nurture_api)
    print(f"Copied nurture-api -> {dst_nurture_api.name}")

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


def main():
    parser = argparse.ArgumentParser(description="Tauri Sidecar Manager & Guardian")
    parser.add_argument(
        "--setup-placeholders",
        action="store_true",
        help="Generate dummy placeholder shell scripts or batch files"
    )
    parser.add_argument(
        "--build",
        action="store_true",
        help="Build production-ready sidecars and place them"
    )
    parser.add_argument(
        "--check-core",
        action="store_true",
        help="Perform stage 1 check: verifying the three Rust core binaries"
    )
    parser.add_argument(
        "--check-all",
        action="store_true",
        help="Perform stage 2 check: verifying all 4 binaries including obscura (for release CI)"
    )
    parser.add_argument(
        "--binaries-dir",
        type=str,
        default="apps/management-console/src-tauri/binaries",
        help="Path to Tauri sidecar binaries directory"
    )

    args = parser.parse_args()

    triple = detect_target_triple()
    print(f"Detected target triple: {triple}")

    binaries_dir = Path(args.binaries_dir)

    try:
        if args.setup_placeholders:
            generate_placeholders(binaries_dir, triple)
        elif args.build:
            run_build(binaries_dir, triple)
        elif args.check_core:
            check_binaries(binaries_dir, triple, check_all=False)
        elif args.check_all:
            check_binaries(binaries_dir, triple, check_all=True)
        else:
            parser.print_help()
            sys.exit(1)
    except ValueError as e:
        sys.exit(1)
    except Exception as e:
        print(f"Unexpected error: {e}", file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()
