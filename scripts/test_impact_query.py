import unittest
import tempfile
import json
import os
from pathlib import Path

# We expect these to be implemented in impact_query.py
try:
    from impact_query import ImpactAnalyzer
except ImportError:
    class ImpactAnalyzer:
        def __init__(self, edges):
            self.edges = edges
            
        def query(self, symbol, exclude_tests=False):
            return []

class TestImpactQuery(unittest.TestCase):
    def setUp(self):
        self.mock_edges = [
            {"from": "app.rs", "to": "UserModel", "kind": "use"},
            {"from": "auth.rs", "to": "UserModel", "kind": "use"},
            {"from": "api.rs", "to": "auth.rs", "kind": "use"},
            # Circular dependency
            {"from": "cycle_a.rs", "to": "cycle_b.rs", "kind": "use"},
            {"from": "cycle_b.rs", "to": "cycle_a.rs", "kind": "use"},
            # Test file
            {"from": "auth_tests.rs", "to": "auth.rs", "kind": "use"},
            {"from": "api_integration_tests.rs", "to": "app.rs", "kind": "use"},
        ]
        self.analyzer = ImpactAnalyzer(self.mock_edges)

    def test_basic_impact_depth(self):
        # A change to UserModel should affect app.rs and auth.rs at depth 1
        # and api.rs at depth 2 (since api.rs -> auth.rs -> UserModel)
        res = self.analyzer.query("UserModel", exclude_tests=False)
        
        # res should be a list of dicts: [{"file": "app.rs", "depth": 1}, ...] or similar
        affected_files = [x["file"] for x in res]
        self.assertIn("app.rs", affected_files)
        self.assertIn("auth.rs", affected_files)
        self.assertIn("api.rs", affected_files)
        
        # Check depth scoring
        api_depth = next(x["depth"] for x in res if x["file"] == "api.rs")
        auth_depth = next(x["depth"] for x in res if x["file"] == "auth.rs")
        self.assertGreater(api_depth, auth_depth, "api.rs should be deeper than auth.rs")

    def test_circular_dependency(self):
        # Should not crash or infinite loop
        res = self.analyzer.query("cycle_b.rs")
        affected = [x["file"] for x in res]
        self.assertIn("cycle_a.rs", affected)

    def test_exclude_tests(self):
        # If we query auth.rs without exclude_tests, auth_tests.rs should be included
        res_with_tests = self.analyzer.query("auth.rs", exclude_tests=False)
        self.assertTrue(any("auth_tests.rs" in x["file"] for x in res_with_tests))
        
        # If we query auth.rs WITH exclude_tests, auth_tests.rs should be excluded
        res_without_tests = self.analyzer.query("auth.rs", exclude_tests=True)
        self.assertFalse(any("auth_tests.rs" in x["file"] for x in res_without_tests))
        
if __name__ == '__main__':
    unittest.main()
