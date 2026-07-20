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
            "obscura-aarch64-apple-darwin",
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

        # OP-088 P3: 公式プレースホルダーに nurture-api は含めない
        self.assertFalse(
            (binaries_dir / "nurture-api-aarch64-apple-darwin").exists(),
            "Official placeholders must not include nurture-api",
        )

    def test_generate_placeholders_with_nurture_sidecar(self):
        self._has_module()
        binaries_dir = Path(self.temp_dir) / "binaries"
        binaries_dir.mkdir()
        with patch("sys.platform", "darwin"):
            desktop_sidecar_manager.generate_placeholders(
                binaries_dir, "aarch64-apple-darwin", with_nurture_sidecar=True
            )
        self.assertTrue((binaries_dir / "nurture-api-aarch64-apple-darwin").exists())

    def test_generate_placeholders_windows(self):
        self._has_module()
        binaries_dir = Path(self.temp_dir) / "binaries"
        binaries_dir.mkdir()

        with patch("sys.platform", "win32"):
            desktop_sidecar_manager.generate_placeholders(binaries_dir, "x86_64-pc-windows-msvc")

        expected_files = [
            "api-server-x86_64-pc-windows-msvc.exe",
            "key-proxy-x86_64-pc-windows-msvc.exe",
            "obscura-x86_64-pc-windows-msvc.exe",
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

        # 2. 公式2つが実バイナリ、obscuraがダミーの場合
        for name in ["api-server", "key-proxy"]:
            with open(binaries_dir / f"{name}-{triple}", "wb") as f:
                f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 200000)

        # check_core (check_all=False) はパスするはず
        try:
            desktop_sidecar_manager.check_binaries(binaries_dir, triple, check_all=False)
        except ValueError as e:
            self.fail(f"check_binaries(check_all=False) failed unexpectedly: {e}")

        # check_all (check_all=True) は obscura がダミーなので失敗するはず
        with self.assertRaises(ValueError):
            desktop_sidecar_manager.check_binaries(
                binaries_dir, triple, check_all=True, forbid_nurture_sidecar=True
            )

        # 3. 公式 + obscura が実バイナリ（nurture-api 無し）→ check-all PASS
        with open(binaries_dir / f"obscura-{triple}", "wb") as f:
            f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 200000)

        try:
            desktop_sidecar_manager.check_binaries(
                binaries_dir, triple, check_all=True, forbid_nurture_sidecar=True
            )
        except ValueError as e:
            self.fail(f"check_binaries(check_all=True) failed unexpectedly: {e}")

        # 4. 実 nurture-api が混入 → 公式 check-all は Fail-Closed
        with open(binaries_dir / f"nurture-api-{triple}", "wb") as f:
            f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 200000)
        with self.assertRaises(ValueError):
            desktop_sidecar_manager.check_binaries(
                binaries_dir, triple, check_all=True, forbid_nurture_sidecar=True
            )

    def test_official_binaries_exclude_nurture_api(self):
        self._has_module()
        self.assertNotIn("nurture-api", desktop_sidecar_manager.OFFICIAL_BINARIES)
        self.assertNotIn("nurture-api", desktop_sidecar_manager.CORE_BINARIES)
        self.assertNotIn("nurture-api", desktop_sidecar_manager.ALL_BINARIES)

    def test_check_core_forbid_nurture_sidecar(self):
        """CI 同等: check-core + forbid で実 nurture-api を拒否する。"""
        self._has_module()
        binaries_dir = Path(self.temp_dir) / "binaries_ci"
        binaries_dir.mkdir()
        triple = "aarch64-apple-darwin"
        for name in ["api-server", "key-proxy"]:
            with open(binaries_dir / f"{name}-{triple}", "wb") as f:
                f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 200000)
        desktop_sidecar_manager.check_binaries(
            binaries_dir, triple, check_all=False, forbid_nurture_sidecar=True
        )
        with open(binaries_dir / f"nurture-api-{triple}", "wb") as f:
            f.write(b"\xfe\xed\xfa\xcf" + b"\x00" * 200000)
        with self.assertRaises(ValueError):
            desktop_sidecar_manager.check_binaries(
                binaries_dir, triple, check_all=False, forbid_nurture_sidecar=True
            )

    def test_tauri_conf_excludes_nurture_api_and_port_3020(self):
        """P3-3 / T-003: externalBin・CSP から nurture-api / :3020 を除去済みであること。"""
        root = Path(__file__).resolve().parents[1]
        conf_path = root / "apps/management-console/src-tauri/tauri.conf.json"
        caps_path = root / "apps/management-console/src-tauri/capabilities/default.json"
        conf = conf_path.read_text(encoding="utf-8")
        caps = caps_path.read_text(encoding="utf-8")
        self.assertNotIn("nurture-api", conf)
        self.assertNotIn(":3020", conf)
        self.assertNotIn("nurture-api", caps)


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

    def test_channel_features_economy_vs_oss(self):
        """OP-089: Economy links nurture; OSS has no nurture feature."""
        self._has_module()
        self.assertEqual(
            desktop_sidecar_manager.api_server_cargo_features("economy"), ["nurture"]
        )
        self.assertEqual(desktop_sidecar_manager.api_server_cargo_features("oss"), [])
        self.assertEqual(
            desktop_sidecar_manager.normalize_channel(None),
            desktop_sidecar_manager.CHANNEL_ECONOMY,
        )
        with self.assertRaises(ValueError):
            desktop_sidecar_manager.normalize_channel("cloud")

    def test_write_channel_manifest(self):
        self._has_module()
        binaries_dir = Path(self.temp_dir) / "bins"
        binaries_dir.mkdir()
        path = desktop_sidecar_manager.write_channel_manifest(
            binaries_dir, "oss", "aarch64-apple-darwin"
        )
        data = __import__("json").loads(path.read_text(encoding="utf-8"))
        self.assertEqual(data["channel"], "oss")
        self.assertEqual(data["api_server_features"], [])
        self.assertEqual(data["asset_name_prefix"], "AiomeOS-OSS")
        self.assertFalse(data["nurture_sidecar_allowed"])

    def test_oss_channel_rejects_nurture_sidecar(self):
        self._has_module()
        with self.assertRaises(ValueError) as ctx:
            desktop_sidecar_manager.run_build(
                Path(self.temp_dir),
                "aarch64-apple-darwin",
                with_nurture_sidecar=True,
                channel="oss",
            )
        self.assertIn("OSS", str(ctx.exception))

    def test_verify_channel_link_economy_positive(self):
        """Positive: Economy tree must include nurture-api."""
        self._has_module()
        desktop_sidecar_manager.verify_api_server_channel_link("economy")

    def test_verify_channel_link_oss_positive(self):
        """Positive: OSS tree must not include nurture-api."""
        self._has_module()
        desktop_sidecar_manager.verify_api_server_channel_link("oss")

    def test_verify_channel_link_economy_negative_when_tree_misses(self):
        """Negative: Economy fails closed if cargo tree lacks nurture-api."""
        self._has_module()
        fake = MagicMock()
        fake.returncode = 1
        fake.stdout = ""
        fake.stderr = "package ID specification `nurture-api` did not match"
        with patch("desktop_sidecar_manager.subprocess.run", return_value=fake):
            with self.assertRaises(ValueError) as ctx:
                desktop_sidecar_manager.verify_api_server_channel_link("economy")
        self.assertIn("Economy", str(ctx.exception))

    def test_verify_channel_link_oss_negative_when_tree_has_nurture(self):
        """Negative: OSS fails closed if nurture-api appears without feature."""
        self._has_module()
        fake = MagicMock()
        fake.returncode = 0
        fake.stdout = "nurture-api v0.1.0\n"
        fake.stderr = ""
        with patch("desktop_sidecar_manager.subprocess.run", return_value=fake):
            with self.assertRaises(ValueError) as ctx:
                desktop_sidecar_manager.verify_api_server_channel_link("oss")
        self.assertIn("OSS", str(ctx.exception))


if __name__ == "__main__":
    unittest.main()
