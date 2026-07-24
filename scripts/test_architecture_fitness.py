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
        # dev-dependencies are scanned (layer edges); F-1 only fails on forbidden names.
        self.assertIn("tokio", keys)

    def test_dep_keys_named_table_form(self):
        """Negative: [dependencies.infrastructure] must not bypass F-1 parsing."""
        text = """
[package]
name = "x"

[dependencies.infrastructure]
path = "../infrastructure"

[target.'cfg(unix)'.dependencies.api-server]
path = "../../apps/api-server"
"""
        keys = af._dep_keys_from_cargo_toml(text)
        self.assertIn("infrastructure", keys)
        self.assertIn("api-server", keys)

    def test_dep_keys_package_rename_form(self):
        """Negative: alias + package= must not bypass F-1 parsing."""
        text = """
[package]
name = "x"

[dependencies]
infra_alias = { package = "infrastructure", path = "../infrastructure" }

[dependencies.api_alias]
package = "api-server"
path = "../../apps/api-server"
"""
        keys = af._dep_keys_from_cargo_toml(text)
        self.assertIn("infrastructure", keys)
        self.assertIn("api-server", keys)
        self.assertIn("infra_alias", keys)

    def test_dep_keys_workspace_quoted_and_dev_build(self):
        """Negative: workspace/quoted/dev/build forms must not bypass F-1."""
        text = """
[package]
name = "x"

[dependencies]
infrastructure.workspace = true
"api-server" = { path = "../../apps/api-server" }

[dev-dependencies]
infrastructure = { path = "../infrastructure" }

[build-dependencies.infrastructure]
path = "../infrastructure"
"""
        keys = af._dep_keys_from_cargo_toml(text)
        self.assertIn("infrastructure", keys)
        self.assertIn("api-server", keys)

    def test_dep_keys_single_quote_package_and_workspace_rename(self):
        """Negative: single-quoted package= and workspace alias rename must fail closed."""
        leaf = """
[dependencies]
x = { package = 'infrastructure', path = '../infrastructure' }
infra = { workspace = true }
"""
        ws = {
            "infra": "infrastructure",
            "serde": "serde",
        }
        keys = af._dep_keys_from_cargo_toml(leaf, workspace_aliases=ws)
        self.assertIn("infrastructure", keys)

        # Non-inherited alias must not pull workspace package rename.
        leaf2 = """
[dependencies]
infra = { path = "./local-infra" }
"""
        keys2 = af._dep_keys_from_cargo_toml(leaf2, workspace_aliases=ws)
        self.assertIn("infra", keys2)
        self.assertNotIn("infrastructure", keys2)

    def test_live_repo_f1_f3_pass(self):
        report = af.run_fitness(af.REPO_ROOT)
        by_id = {c["id"]: c for c in report["checks"]}
        self.assertTrue(by_id["F-1"]["passed"], by_id["F-1"]["detail"])
        self.assertTrue(by_id["F-2"]["passed"], by_id["F-2"]["detail"])
        self.assertTrue(by_id["F-3"]["passed"], by_id["F-3"]["detail"])
        self.assertTrue(report["passed"])
        self.assertIn("F-4", by_id)
        self.assertTrue(by_id["F-4"]["extras"]["prod_top"])

    def test_negative_f1_injected_dependency_fails_isolated(self):
        """Negative: poisoned shared Cargo.toml in a temp tree → F-1 FAIL (no live repo write)."""
        clean_shared = """[package]
name = "shared"
version = "0.0.0"

[dependencies]
serde = "1"
"""
        contracts = """[package]
name = "aiome-core-contracts"
version = "0.0.0"

[dependencies]
serde = "1"
"""
        soul = """[package]
name = "soul"
version = "0.0.0"

[dependencies]
serde = "1"
"""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "libs" / "shared").mkdir(parents=True)
            (root / "libs" / "aiome-core-contracts").mkdir(parents=True)
            (root / "libs" / "soul").mkdir(parents=True)
            (root / "libs" / "shared" / "Cargo.toml").write_text(clean_shared, encoding="utf-8")
            (root / "libs" / "aiome-core-contracts" / "Cargo.toml").write_text(
                contracts, encoding="utf-8"
            )
            (root / "libs" / "soul" / "Cargo.toml").write_text(soul, encoding="utf-8")

            baseline = af.run_fitness(root)
            self.assertTrue(baseline["passed"], baseline)

            poisoned = clean_shared.replace(
                "[dependencies]\n",
                "[dependencies]\ninfrastructure = { path = \"../infrastructure\" }\n",
                1,
            )
            (root / "libs" / "shared" / "Cargo.toml").write_text(poisoned, encoding="utf-8")
            report = af.run_fitness(root)
            f1 = next(c for c in report["checks"] if c["id"] == "F-1")
            self.assertFalse(f1["passed"], "injected infrastructure dep must fail F-1")
            self.assertFalse(report["passed"])

            # Restore inside the fixture and confirm green again
            (root / "libs" / "shared" / "Cargo.toml").write_text(clean_shared, encoding="utf-8")
            restored = af.run_fitness(root)
            self.assertTrue(restored["passed"], restored)

    def test_negative_f1_workspace_rename_fails_isolated(self):
        """Negative: workspace alias → package=infrastructure must fail F-1 via run_fitness."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "Cargo.toml").write_text(
                """[workspace]
members = []

[workspace.dependencies]
infra = { package = "infrastructure", path = "libs/infrastructure" }
""",
                encoding="utf-8",
            )
            for rel in (
                "libs/shared",
                "libs/aiome-core-contracts",
                "libs/soul",
            ):
                (root / rel).mkdir(parents=True)
                (root / rel / "Cargo.toml").write_text(
                    """[package]
name = "x"
version = "0.0.0"

[dependencies]
serde = "1"
""",
                    encoding="utf-8",
                )
            (root / "libs" / "shared" / "Cargo.toml").write_text(
                """[package]
name = "shared"
version = "0.0.0"

[dependencies]
infra = { workspace = true }
""",
                encoding="utf-8",
            )
            report = af.run_fitness(root)
            f1 = next(c for c in report["checks"] if c["id"] == "F-1")
            self.assertFalse(
                f1["passed"],
                f"workspace rename must fail F-1: {f1}",
            )
            self.assertFalse(report["passed"])

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
            integ = (
                root
                / "apps"
                / "api-server"
                / "src"
                / "api_integration_tests"
            )
            integ.mkdir(parents=True)
            (integ / "commerce.rs").write_text("\n" * 1400, encoding="utf-8")
            result = af.check_file_sizes(root, warn_at=800, top_n=5)
            prod_paths = [r["path"] for r in result.extras["prod_top"]]
            test_paths = [r["path"] for r in result.extras["test_top"]]
            self.assertTrue(any(p.endswith("big.rs") for p in prod_paths))
            self.assertTrue(any("foo_tests.rs" in p for p in test_paths))
            self.assertFalse(any("foo_tests.rs" in p for p in prod_paths))
            self.assertTrue(
                any("api_integration_tests/commerce.rs" in p for p in test_paths),
                test_paths,
            )
            self.assertFalse(
                any("api_integration_tests" in p for p in prod_paths),
                prod_paths,
            )


if __name__ == "__main__":
    unittest.main()
