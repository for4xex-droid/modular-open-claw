#!/usr/bin/env python3
import json
import argparse
import sys
from pathlib import Path
from collections import deque

class ImpactAnalyzer:
    def __init__(self, edges):
        self.edges = edges
        # Build immediate reverse dependency map (who depends on X)
        self.reverse_deps = {}
        for edge in edges:
            to_node = edge["to"]
            from_node = edge["from"]
            if to_node not in self.reverse_deps:
                self.reverse_deps[to_node] = []
            self.reverse_deps[to_node].append(from_node)

    def is_test_file(self, filename):
        fname = filename.lower()
        return "test" in fname or "spec" in fname

    def query(self, symbol, exclude_tests=False):
        affected = {} # node -> minimum depth
        visited = set()
        
        # Queue stores tuples of (current_symbol, current_depth)
        queue = deque([(symbol, 0)])
        
        while queue:
            current, depth = queue.popleft()
            
            if current in visited:
                continue
            visited.add(current)
            
            # If it's not the initial symbol, record its depth
            if depth > 0:
                if current not in affected or affected[current] > depth:
                    affected[current] = depth
            
            # Find dependents
            dependents = self.reverse_deps.get(current, [])
            for dep in dependents:
                if exclude_tests and self.is_test_file(dep):
                    continue
                queue.append((dep, depth + 1))
                
        # Format result
        res = []
        for file, d in affected.items():
            res.append({"file": file, "depth": d})
            
        # Sort by depth, then filename
        res.sort(key=lambda x: (x["depth"], x["file"]))
        return res

def main():
    parser = argparse.ArgumentParser(description="Query the actual impact blast-radius of a change based on AST metadata.")
    parser.add_argument("symbol", help="The struct/trait/module or UI component symbol to analyze.")
    parser.add_argument("--exclude-tests", action="store_true", help="Exclude test files from the report.")
    args = parser.parse_args()
    
    project_root = Path(__file__).parent.parent
    graph_path = project_root / ".context" / "impact_graph.json"
    
    if not graph_path.exists():
        print(f"Error: Impact graph not found at {graph_path}. Please run nurture_auditor.py first.")
        sys.exit(1)
        
    try:
        with open(graph_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except Exception as e:
        print(f"Error: Could not parse impact graph JSON: {e}")
        sys.exit(1)
        
    analyzer = ImpactAnalyzer(data.get("edges", []))
    impacts = analyzer.query(args.symbol, exclude_tests=args.exclude_tests)
    
    if not impacts:
        print(f"✅ No dependencies found for `{args.symbol}`. It appears to be a leaf node or unused.")
    else:
        print(f"💥 Impact Radius for `{args.symbol}`:")
        for res in impacts:
            score = "WILL BREAK" if res["depth"] == 1 else f"LIKELY AFFECTED (Depth: {res['depth']})"
            print(f"  - [{score}] {res['file']}")

if __name__ == '__main__':
    main()
