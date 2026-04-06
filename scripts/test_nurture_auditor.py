import unittest
import tempfile
import os

# We will test the existing (and soon failing) nurture_auditor.py
from nurture_auditor import analyze_rust_file, analyze_ts_file

# We expect these constants to exist in nurture_auditor.py as per our TDD requirement
try:
    from nurture_auditor import RUST_USE_PATTERN, RUST_MOD_PATTERN, RUST_IMPL_PATTERN, TS_IMPORT_SYMBOL_PATTERN, CSS_DEF_PATTERN, CSS_USE_PATTERN
except ImportError:
    # Dummy mock so test can at least 'run' and fail correctly
    import re
    RUST_USE_PATTERN = re.compile(r'NOT_IMPLEMENTED')
    RUST_MOD_PATTERN = re.compile(r'NOT_IMPLEMENTED')
    RUST_IMPL_PATTERN = re.compile(r'NOT_IMPLEMENTED')
    TS_IMPORT_SYMBOL_PATTERN = re.compile(r'NOT_IMPLEMENTED')
    CSS_DEF_PATTERN = re.compile(r'NOT_IMPLEMENTED')
    CSS_USE_PATTERN = re.compile(r'NOT_IMPLEMENTED')

class TestNurtureAuditorRegex(unittest.TestCase):
    def test_rust_dependencies(self):
        content = """
        use crate::models::User;
        use std::collections::{HashMap, HashSet};
        mod user_service;
        pub mod auth;
        impl VaultBackend for SqliteVaultBackend {
        """
        
        # We test the regexes directly first to decouple from file I/O
        uses = RUST_USE_PATTERN.findall(content)
        mods = RUST_MOD_PATTERN.findall(content)
        impls = RUST_IMPL_PATTERN.findall(content)
        
        # Note: Depending on regex implementation, extracting {HashMap, HashSet} might be tricky.
        # At minimum, we should extract the main path or simple uses.
        self.assertTrue(len(uses) >= 2, f"Should find use statements, found: {uses}")
        self.assertIn("user_service", mods)
        self.assertIn("auth", mods)
        self.assertTrue(any("VaultBackend" in i for i in impls), f"Should find VaultBackend in impls: {impls}")

    def test_tsx_imports(self):
        content = """
        import { Button, Modal } from '@/components/ui';
        import DefaultComponent from './something';
        import * as utils from '../utils';
        import type { UserProps } from './types';
        """
        # We want to extract the imported symbols: Button, Modal, DefaultComponent
        imports = TS_IMPORT_SYMBOL_PATTERN.findall(content)
        
        # Flatten if it's returning tuples
        if imports and isinstance(imports[0], tuple):
            imports = [item for sublist in imports for item in sublist]
            
        imports_str = ",".join(imports)
        
        self.assertIn("Button", imports_str)
        self.assertIn("Modal", imports_str)
        self.assertIn("DefaultComponent", imports_str)

    def test_css_tokens(self):
        content_css = """
        :root {
            --color-primary: #fff;
            --font-size-lg: 16px;
        }
        """
        content_usage = """
        .card {
            background-color: var(--color-primary);
            padding: var(--spacing-md);
        }
        """
        defs = CSS_DEF_PATTERN.findall(content_css)
        uses = CSS_USE_PATTERN.findall(content_usage)
        
        self.assertIn("--color-primary", defs)
        self.assertIn("--font-size-lg", defs)
        
        self.assertIn("--color-primary", uses)
        self.assertIn("--spacing-md", uses)

    def test_directory_filtering_and_graph(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td)
            
            # Create a fake node_modules directory
            nm_dir = base / "node_modules"
            nm_dir.mkdir()
            with open(nm_dir / "index.tsx", "w") as f:
                f.write("import { BadHook } from 'bad-lib';")
                
            # Create a target directory
            tgt_dir = base / "target"
            tgt_dir.mkdir()
            with open(tgt_dir / "build.rs", "w") as f:
                f.write("use fake::FakeStruct;")

            # Create a valid app directory
            app_dir = base / "apps" / "api-server"
            app_dir.mkdir(parents=True)
            with open(app_dir / "main.rs", "w") as f:
                f.write("use crate::models::User;")
                
            # We don't import generate_audit_report directly yet, we will mock or 
            # test the filter logic if extracted, or run the whole function.
            from nurture_auditor import generate_audit_report
            output_file = base / "test_report.md"
            
            # Ensure .context dir exists so it doesn't fail writing graph
            context_dir = base / ".context"
            context_dir.mkdir()
            
            generate_audit_report(base, output_file)
            
            # The report should exist
            self.assertTrue(output_file.exists())
            
            # The graph json should also exist
            impact_graph = base / ".context" / "impact_graph.json"
            self.assertTrue(impact_graph.exists(), "impact_graph.json should be created")
            
            with open(impact_graph, 'r') as f:
                import json
                graph = json.load(f)
                edges = graph.get("edges", [])
                
                # It should contain User dependency
                self.assertTrue(any("User" in e["to"] for e in edges), "Graph should contain valid dependency")
                
                # It should NOT contain BadHook or FakeStruct
                self.assertFalse(any("BadHook" in e["to"] for e in edges), "Graph should exclude node_modules")
                self.assertFalse(any("FakeStruct" in e["to"] for e in edges), "Graph should exclude target")

if __name__ == '__main__':
    from pathlib import Path
    unittest.main()

