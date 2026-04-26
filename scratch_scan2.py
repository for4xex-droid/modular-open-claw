import os
import re

dirs = ['/Users/motista/Desktop/antigravity/aiome', '/Users/motista/Desktop/antigravity/Project-Nurture']
pattern = re.compile(r'(HACK|TEMPORARY|XXX|stub)', re.IGNORECASE)

results = []
for d in dirs:
    for root, _, files in os.walk(d):
        if '.git' in root or 'node_modules' in root or 'target' in root:
            continue
        for f in files:
            if not f.endswith('.rs') and not f.endswith('.ts') and not f.endswith('.tsx') and not f.endswith('.md'):
                continue
            path = os.path.join(root, f)
            try:
                with open(path, 'r') as fp:
                    for i, line in enumerate(fp):
                        if pattern.search(line):
                            clean_line = line.strip()
                            if 'perfect' in path.lower() or 'roadmap' in path.lower() or 'memory' in path.lower() or 'changelog' in path.lower() or 'scratch' in path.lower():
                                continue
                            if 'test' in path.lower(): # skip tests where mocks are normal
                                continue
                            results.append(f"{path}:{i+1}: {clean_line}")
            except Exception:
                pass

with open('full_hack_scan.txt', 'w') as out:
    for r in results:
        out.write(r + '\n')
print(f"Found {len(results)} matches.")
