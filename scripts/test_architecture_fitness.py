#!/usr/bin/env python3
"""Unit tests for architecture_fitness (OP-090)."""

from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.append(os.path.dirname(os.path.abspath(__file__)))

try:
    import architecture_fitness as af
except ImportError:
    af = None


class TestArchitectureFitness(unittest.TestCase):
    def test_module_exists(self):
        self.assertIsNotNone(af, "architecture_fitness.py does not exist")

    def test_dep_keys_parse(self):
        text = """
[package]
name = "x"

[dependencies]
serde = "1"
infrastructure = { path = "../infrastructure" }

[dev-dependencies]
tokio = "1"
"""
        keys = af._dep_keys_from_cargo_toml(text)
        self.assertIn("serde", keys)
        self.assertIn("infrastructure", keys)
        self.assertNotIn("tokio", keys)

    def test_live_repo_f1_f3_pass(self):
        report = af.run_fitness(af.REPO_ROOT)
        by_id = {c["id"]: c for c in report["checks"]}
        self.assertTrue(by_id["F-1"]["passed"], by_id["F-1"]["detail"])
        self.assertTrue(by_id["F-2"]["passed"], by_id["F-2"]["detail"])
        self.assertTrue(by_id["F-3"]["passed"], by_id["F-3"]["detail"])
        self.assertTrue(report["passed"])
        self.assertIn("F-4", by_id)
        self.assertTrue(by_id["F-4"]["extras"]["prod_top"])

    def test_negative_f1_injected_dependency_fails_then_restore(self):
        """Negative: temporarily add infrastructure dep to shared → F-1 FAIL → restore."""
        shared = af.REPO_ROOT / "libs" / "shared" / "Cargo.toml"
        original = shared.read_text(encoding="utf-8")
        marker = "\n# OP-090-NEGATIVE-INJECT\ninfrastructure = { path = \"../infrastructure\" }\n"
        try:
            if "[dependencies]" not in original:
                self.fail("shared Cargo.toml missing [dependencies]")
            poisoned = original.replace(
                "[dependencies]",
                "[dependencies]" + marker,
                1,
            )
            shared.write_text(poisoned, encoding="utf-8")
            report = af.run_fitness(af.REPO_ROOT)
            f1 = next(c for c in report["checks"] if c["id"] == "F-1")
            self.assertFalse(f1["passed"], "injected infrastructure dep must fail F-1")
            self.assertFalse(report["passed"])
        finally:
            shared.write_text(original, encoding="utf-8")
            restored = af.run_fitness(af.REPO_ROOT)
            f1 = next(c for c in restored["checks"] if c["id"] == "F-1")
            self.assertTrue(f1["passed"], "restore must make F-1 pass again")

    def test_f4_separates_tests(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "libs" / "infrastructure" / "src").mkdir(parents=True)
            (root / "libs" / "infrastructure" / "src" / "big.rs").write_text(
                "\n" * 900, encoding="utf-8"
            )
            (root / "libs" / "infrastructure" / "src" / "foo_tests.rs").write_text(
                "\n" * 1200, encoding="utf-8"
            )
            # Minimal Cargo.toml stubs so F-1..F-3 can still run if called via check_file_sizes only
            result = af.check_file_sizes(root, warn_at=800, top_n=5)
            prod_paths = [r["path"] for r in result.extras["prod_top"]]
            test_paths = [r["path"] for r in result.extras["test_top"]]
            self.assertTrue(any(p.endswith("big.rs") for p in prod_paths))
            self.assertTrue(any("foo_tests.rs" in p for p in test_paths))
            self.assertFalse(any("foo_tests.rs" in p for p in prod_paths))


if __name__ == "__main__":
    unittest.main()
