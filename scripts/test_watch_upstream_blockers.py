#!/usr/bin/env python3
"""Unit tests for watch_upstream_blockers (Gate α). No network."""

from __future__ import annotations

import io
import os
import sys
import unittest
from contextlib import redirect_stdout
from unittest.mock import MagicMock, patch

sys.path.append(os.path.dirname(os.path.abspath(__file__)))

try:
    import watch_upstream_blockers as w
except ImportError:
    w = None


def _mock_urlopen(payload: bytes):
    mock_response = MagicMock()
    mock_response.read.return_value = payload
    mock_response.__enter__.return_value = mock_response
    return mock_response


class TestWatchUpstreamBlockers(unittest.TestCase):
    def test_module_exists(self):
        self.assertIsNotNone(w, "watch_upstream_blockers.py does not exist")

    def test_parse_version_strips_prerelease(self):
        self.assertEqual(w.parse_version("1.30.0"), [1, 30, 0])
        self.assertEqual(w.parse_version("1.22.0-rc.1"), [1, 22, 0])
        self.assertEqual(w.parse_version("2"), [2, 0, 0])

    def test_version_reached(self):
        self.assertFalse(w.version_reached("0.12.5", "0.13.0"))
        self.assertTrue(w.version_reached("0.13.0", "0.13.0"))
        self.assertTrue(w.version_reached("1.30.0", "1.22.0"))

    @patch("watch_upstream_blockers.urllib.request.urlopen")
    def test_check_crate_version_blocked(self, mock_urlopen):
        mock_urlopen.return_value = _mock_urlopen(
            b'{"crate": {"max_stable_version": "0.12.5"}}'
        )
        result = w.check_crate_target("serenity", "0.13.0")
        self.assertFalse(result["reached"])
        self.assertEqual(result["current_version"], "0.12.5")

    @patch("watch_upstream_blockers.urllib.request.urlopen")
    def test_check_crate_version_released(self, mock_urlopen):
        mock_urlopen.return_value = _mock_urlopen(
            b'{"crate": {"max_stable_version": "0.13.1"}}'
        )
        result = w.check_crate_target("serenity", "0.13.0")
        self.assertTrue(result["reached"])
        self.assertEqual(result["current_version"], "0.13.1")

    def test_format_status_line_includes_op(self):
        line = w.format_status_line(
            {
                "crate": "extism",
                "version": "1.22.0",
                "op": "OP-032",
                "issue": "Issue C",
                "gate": True,
            },
            "1.30.0",
            True,
        )
        self.assertIn("OP-032", line)
        self.assertIn("Issue C", line)
        self.assertIn("UNBLOCKED", line)
        self.assertIn("[GATE]", line)

    def test_targets_map_to_open_ids(self):
        by_crate = {t["crate"]: t for t in w.TARGETS}
        self.assertEqual(by_crate["serenity"]["op"], "OP-030")
        self.assertEqual(by_crate["extism"]["op"], "OP-032")
        self.assertEqual(by_crate["tauri"]["op"], "OP-033")
        self.assertTrue(all(t.get("gate") for t in w.TARGETS))
        self.assertFalse(w.WASM_INFO["gate"])

    @patch("watch_upstream_blockers.check_crate_target")
    def test_run_watch_exit_0_when_all_gate_blocked(self, mock_check):
        mock_check.side_effect = lambda crate, need: {
            "reached": False,
            "current_version": "0.0.1",
        }

        buf = io.StringIO()
        with redirect_stdout(buf):
            code = w.run_watch(w.TARGETS)
        self.assertEqual(code, 0)
        out = buf.getvalue()
        self.assertIn("OP-030", out)
        self.assertIn("OP-032", out)
        self.assertIn("OP-033", out)
        self.assertIn("still blocked", out.lower())

    @patch("watch_upstream_blockers.check_crate_target")
    def test_run_watch_exit_1_when_extism_unblocked(self, mock_check):
        def side_effect(crate, need):
            cur = {
                "serenity": "0.12.5",
                "extism": "1.30.0",
                "tauri": "2.11.5",
                "wasmtime": "47.0.1",
            }[crate]
            return {"reached": w.version_reached(cur, need), "current_version": cur}

        mock_check.side_effect = side_effect
        buf = io.StringIO()
        with redirect_stdout(buf):
            code = w.run_watch(w.TARGETS)
        self.assertEqual(code, 1)
        out = buf.getvalue()
        self.assertIn("OP-032", out)
        self.assertIn("GATE α UNBLOCKED", out)
        self.assertIn("Do not delete deny.toml ignores on α alone", out)

    @patch("watch_upstream_blockers.check_crate_target")
    def test_wasmtime_info_alone_does_not_exit_1(self, mock_check):
        """Even if wasmtime info shows reached, gate targets blocked → exit 0."""

        def side_effect(crate, need):
            if crate == "wasmtime":
                return {"reached": True, "current_version": "47.0.1"}
            return {"reached": False, "current_version": "0.1.0"}

        mock_check.side_effect = side_effect
        buf = io.StringIO()
        with redirect_stdout(buf):
            code = w.run_watch(w.TARGETS)
        self.assertEqual(code, 0)
        self.assertIn("[INFO]", buf.getvalue())


if __name__ == "__main__":
    unittest.main()
