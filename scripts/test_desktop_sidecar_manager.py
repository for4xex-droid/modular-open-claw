import unittest
import tempfile
import os
import shutil
import sys
from pathlib import Path
from unittest.mock import patch, MagicMock

# スクリプトディレクトリをインポートパスに追加
sys.path.insert(0, str(Path(__file__).parent.absolute()))

# ターゲットモジュールのインポート（未作成のため、インポートエラー時はテストが失敗するようにダミーを定義）
try:
    import desktop_sidecar_manager
except ImportError:
    desktop_sidecar_manager = None


class TestDesktopSidecarManager(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.temp_dir)

    def _has_module(self):
        if desktop_sidecar_manager is None:
            self.fail("desktop_sidecar_manager module could not be imported (TDD RED)")

    def test_detect_target_triple_from_rustc(self):
        self._has_module()
        # rustc の出力がある場合のモックテスト
        mock_output = (
            "rustc 1.75.0 (82e587853 2023-12-21)\n"
            "binary: rustc\n"
            "commit-hash: 82e587853b6936a81b49ae841bafab92b0e6e760\n"
            "commit-date: 2023-12-21\n"
            "host: aarch64-apple-darwin\n"
            "release: 1.75.0\n"
            "LLVM version: 17.0.6\n"
        )
        with patch("subprocess.check_output") as mock_run:
            mock_run.return_value = mock_output.encode("utf-8")
            triple = desktop_sidecar_manager.detect_target_triple()
            self.assertEqual(triple, "aarch64-apple-darwin")

    def test_detect_target_triple_fallback(self):
        self._has_module()
        # rustc が無い場合のフォールバックテスト
        with patch("subprocess.check_output", side_effect=FileNotFoundError):
            with patch("sys.platform", "darwin"), patch("platform.machine", return_value="arm64"):
                triple = desktop_sidecar_manager.detect_target_triple()
                self.assertEqual(triple, "aarch64-apple-darwin")

            with patch("sys.platform", "linux"), patch("platform.machine", return_value="x86_64"):
                triple = desktop_sidecar_manager.detect_target_triple()
                self.assertEqual(triple, "x86_64-unknown-linux-gnu")

            with patch("sys.platform", "win32"), patch("platform.machine", return_value="AMD64"):
                triple = desktop_sidecar_manager.detect_target_triple()
                self.assertEqual(triple, "x86_64-pc-windows-msvc")

    def test_generate_placeholders_unix(self):
        self._has_module()
        binaries_dir = Path(self.temp_dir) / "binaries"
        binaries_dir.mkdir()

        with patch("sys.platform", "darwin"):
            desktop_sidecar_manager.generate_placeholders(binaries_dir, "aarch64-apple-darwin")

        expected_files = [
            "api-server-aarch64-apple-darwin",
            "key-proxy-aarch64-apple-darwin",
            "nurture-api-aarch64-apple-darwin",
            "obscura-aarch64-apple-darwin"
        ]
        for name in expected_files:
            file_path = binaries_dir / name
            self.assertTrue(file_path.exists(), f"Placeholder {name} should exist")
            
            # シェルスクリプト形式であることの検証
            with open(file_path, "r") as f:
                content = f.read()
                self.assertTrue(content.startswith("#!/bin/sh"), f"{name} should start with shebang")
                self.assertIn("exit 0", content)

            # 実行権限の検証
            if sys.platform != "win32":
                self.assertTrue(os.access(file_path, os.X_OK), f"{name} should be executable")

    def test_generate_placeholders_windows(self):
        self._has_module()
        binaries_dir = Path(self.temp_dir) / "binaries"
        binaries_dir.mkdir()

        with patch("sys.platform", "win32"):
            desktop_sidecar_manager.generate_placeholders(binaries_dir, "x86_64-pc-windows-msvc")

        expected_files = [
            "api-server-x86_64-pc-windows-msvc.exe",
            "key-proxy-x86_64-pc-windows-msvc.exe",
            "nurture-api-x86_64-pc-windows-msvc.exe",
            "obscura-x86_64-pc-windows-msvc.exe"
        ]
        for name in expected_files:
            file_path = binaries_dir / name
            self.assertTrue(file_path.exists(), f"Placeholder {name} should exist")
            
            # Windowsバッチ形式であることの検証
            with open(file_path, "r") as f:
                content = f.read()
                self.assertIn("@exit /b 0", content)

    def test_is_real_binary_negative(self):
        self._has_module()
        # ダミーファイルの判定
        dummy_file = Path(self.temp_dir) / "dummy"
        with open(dummy_file, "w") as f:
            f.write("#!/bin/sh\nexit 0")
        
        is_real = desktop_sidecar_manager.is_real_binary(dummy_file)
        self.assertFalse(is_real, "Dummy scripts should not be detected as real binaries")

    def test_is_real_binary_positive_macho(self):
        self._has_module()
        # 本物のバイナリ（Mach-O 64-bit）を模倣したマジックバイトとサイズ
        macho_file = Path(self.temp_dir) / "macho"
        # 64-bit Mach-O magic bytes (feedfacf) + 十分なサイズ
        with open(macho_file, "wb") as f:
            f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 200000) # 200KB

        is_real = desktop_sidecar_manager.is_real_binary(macho_file)
        self.assertTrue(is_real, "Mach-O files with correct size should be detected as real binaries")

    def test_is_real_binary_positive_elf(self):
        self._has_module()
        # 本物のバイナリ（ELF）を模倣したマジックバイトとサイズ
        elf_file = Path(self.temp_dir) / "elf"
        with open(elf_file, "wb") as f:
            f.write(b"\x7fELF" + b"\x00" * 200000)

        is_real = desktop_sidecar_manager.is_real_binary(elf_file)
        self.assertTrue(is_real, "ELF files with correct size should be detected as real binaries")

    def test_is_real_binary_positive_pe(self):
        self._has_module()
        # 本物のバイナリ（PE）を模倣したマジックバイトとサイズ
        pe_file = Path(self.temp_dir) / "pe"
        with open(pe_file, "wb") as f:
            f.write(b"MZ" + b"\x00" * 200000)

        is_real = desktop_sidecar_manager.is_real_binary(pe_file)
        self.assertTrue(is_real, "PE files with correct size should be detected as real binaries")

    def test_check_binaries_two_stage(self):
        self._has_module()
        binaries_dir = Path(self.temp_dir) / "binaries"
        binaries_dir.mkdir()
        triple = "aarch64-apple-darwin"

        # 1. すべてダミーの場合
        desktop_sidecar_manager.generate_placeholders(binaries_dir, triple)
        
        # coreチェックは失敗するはず
        with self.assertRaises(ValueError):
            desktop_sidecar_manager.check_binaries(binaries_dir, triple, check_all=False)
        
        # allチェックも失敗するはず
        with self.assertRaises(ValueError):
            desktop_sidecar_manager.check_binaries(binaries_dir, triple, check_all=True)

        # 2. Rust側3つが実バイナリ、obscuraがダミーの場合
        # Mach-Oダミーを書き込む
        for name in ["api-server", "key-proxy", "nurture-api"]:
            with open(binaries_dir / f"{name}-{triple}", "wb") as f:
                f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 200000)

        # check_core (check_all=False) はパスするはず
        try:
            desktop_sidecar_manager.check_binaries(binaries_dir, triple, check_all=False)
        except ValueError as e:
            self.fail(f"check_binaries(check_all=False) failed unexpectedly: {e}")

        # check_all (check_all=True) は obscura がダミーなので失敗するはず
        with self.assertRaises(ValueError):
            desktop_sidecar_manager.check_binaries(binaries_dir, triple, check_all=True)

        # 3. すべて実バイナリの場合
        with open(binaries_dir / f"obscura-{triple}", "wb") as f:
            f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 200000)

        # check_all もパスするはず
        try:
            desktop_sidecar_manager.check_binaries(binaries_dir, triple, check_all=True)
        except ValueError as e:
            self.fail(f"check_binaries(check_all=True) failed unexpectedly: {e}")


    def test_is_real_binary_zero_byte_file(self):
        self._has_module()
        empty_file = Path(self.temp_dir) / "empty"
        empty_file.touch()
        self.assertFalse(desktop_sidecar_manager.is_real_binary(empty_file),
                         "Zero-byte file should not be detected as real binary")

    def test_is_real_binary_boundary_size(self):
        self._has_module()
        # 99999 bytes (just under 100KB threshold)
        boundary_file = Path(self.temp_dir) / "boundary"
        with open(boundary_file, "wb") as f:
            f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 99995)
        self.assertFalse(desktop_sidecar_manager.is_real_binary(boundary_file),
                         "99999-byte file should be rejected (under 100KB threshold)")

        # 100000 bytes (exactly at threshold)
        at_threshold_file = Path(self.temp_dir) / "at_threshold"
        with open(at_threshold_file, "wb") as f:
            f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 99996)
        self.assertTrue(desktop_sidecar_manager.is_real_binary(at_threshold_file),
                        "100000-byte Mach-O file should be accepted (at threshold)")

    def test_is_real_binary_short_header(self):
        self._has_module()
        # 1-byte file with valid size but invalid header
        short_file = Path(self.temp_dir) / "short_header"
        with open(short_file, "wb") as f:
            f.write(b"\x00" * 200000)
        self.assertFalse(desktop_sidecar_manager.is_real_binary(short_file),
                         "File with invalid magic bytes should be rejected")

    def test_is_real_binary_fat_macho(self):
        self._has_module()
        # FAT Mach-O (cafebabe)
        fat_file = Path(self.temp_dir) / "fat_macho"
        with open(fat_file, "wb") as f:
            f.write(b"\xca\xfe\xba\xbe" + b"\x00" * 200000)
        self.assertTrue(desktop_sidecar_manager.is_real_binary(fat_file),
                        "FAT Mach-O files should be detected as real binaries")

    def test_is_real_binary_nonexistent(self):
        self._has_module()
        nonexistent = Path(self.temp_dir) / "does_not_exist"
        self.assertFalse(desktop_sidecar_manager.is_real_binary(nonexistent),
                         "Nonexistent file should return False")


if __name__ == "__main__":
    unittest.main()
