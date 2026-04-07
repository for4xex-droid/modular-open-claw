import os
import re
import sys

def run_fixer():
    target_dir = "apps/management-console/src/components"
    
    # Simple direct mappings for React 'style={{ ... }}' where var() is valid
    hex_map = {
        re.compile(r'#fff(fff)?', re.IGNORECASE): 'var(--text-primary)',
        re.compile(r'#000(000)?', re.IGNORECASE): 'var(--bg-primary)',
        re.compile(r'#00f2ff22', re.IGNORECASE): 'rgba(0, 242, 255, 0.13)', # GraphView specific
        re.compile(r'#bc8cff44', re.IGNORECASE): 'rgba(188, 140, 255, 0.26)',
        re.compile(r'#00f2ff44', re.IGNORECASE): 'rgba(0, 242, 255, 0.26)',
        re.compile(r'#bc8cff22', re.IGNORECASE): 'rgba(188, 140, 255, 0.13)',
        re.compile(r'#00f2ff', re.IGNORECASE): 'var(--accent-cyan)',
        re.compile(r'#00f3ff', re.IGNORECASE): 'var(--accent-cyan)',
        re.compile(r'#bc8cff', re.IGNORECASE): 'var(--accent-purple)',
        re.compile(r'#ff4d6d', re.IGNORECASE): 'var(--accent-rose)',
        re.compile(r'#ff5252', re.IGNORECASE): 'var(--accent-rose)',
        re.compile(r'#ff4757', re.IGNORECASE): 'var(--accent-rose)',
        re.compile(r'#ff6464', re.IGNORECASE): 'var(--accent-rose)',
        re.compile(r'#b71540', re.IGNORECASE): 'var(--accent-rose)',
        re.compile(r'#00ff66', re.IGNORECASE): 'var(--accent-emerald)',
        re.compile(r'#079992', re.IGNORECASE): 'var(--accent-emerald)',
        re.compile(r'#ffab00', re.IGNORECASE): 'var(--accent-amber)',
        re.compile(r'#f0f2f5', re.IGNORECASE): 'var(--bg-secondary)',
        re.compile(r'#0[a0]0[a-f0-9][a-f0-9]{1,2}', re.IGNORECASE): 'var(--bg-dark-sidebar)',
        re.compile(r'#1a1a2e', re.IGNORECASE): 'var(--bg-dark)',
        re.compile(r'#2c3e50', re.IGNORECASE): 'var(--bg-dark-sidebar)',
        re.compile(r'#2d3436', re.IGNORECASE): 'var(--bg-dark)',
        re.compile(r'#1e3799', re.IGNORECASE): 'var(--accent-blue)',
        re.compile(r'#3c6382', re.IGNORECASE): 'var(--text-muted)',
        re.compile(r'#0a3d62', re.IGNORECASE): 'var(--bg-glass)',
        re.compile(r'#7f8c8d', re.IGNORECASE): 'var(--text-muted)',
        re.compile(r'#050505', re.IGNORECASE): 'var(--bg-black, #050505)' # Will handle later
    }

    exceptions = [] # Files we want to skip if they require special js handling
    
    replacements_made = 0
    files_modified = 0

    for root, _, files in os.walk(target_dir):
        for file in files:
            if file.endswith(('.tsx', '.ts')) and file not in exceptions:
                filepath = os.path.join(root, file)
                with open(filepath, 'r', encoding='utf-8') as f:
                    original_content = f.read()
                
                content = original_content
                
                # Special handling for vis-network files (CausalVisualizer, GraphView)
                if file in ['CausalVisualizer.tsx', 'GraphView.tsx']:
                    # In vis-network, we need the actual hex strings for the canvas.
                    # As a temporary bridge, we replace hardcoded hex with a function call to cssVar()
                    # First, inject the cssVar helper if it's not there
                    if 'const cssVar =' not in content:
                        bridge_code = """
const cssVar = (name: string, fallback: string) => {
    if (typeof document === 'undefined') return fallback;
    const val = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return val || fallback;
};
"""
                        # Insert right after imports
                        content = re.sub(r'(import .*?;?\n)(?=\n*(?:const|function|interface))', r'\1\n' + bridge_code, content, count=1)
                    
                    # Manual replacements for vis-network options where string is needed
                    content = content.replace("'#fff'", "cssVar('--text-primary', '#fff')")
                    content = content.replace("'#000'", "cssVar('--bg-primary', '#000')")
                    content = content.replace("'#2d3436'", "cssVar('--bg-dark', '#2d3436')")
                    content = content.replace("'#1e3799'", "cssVar('--accent-blue', '#1e3799')")
                    content = content.replace("'#0a3d62'", "cssVar('--bg-glass', '#0a3d62')")
                    content = content.replace("'#079992'", "cssVar('--accent-emerald', '#079992')")
                    content = content.replace("'#b71540'", "cssVar('--accent-rose', '#b71540')")
                    content = content.replace("'#2c3e50'", "cssVar('--bg-dark-sidebar', '#2c3e50')")
                    content = content.replace("'#3c6382'", "cssVar('--text-muted', '#3c6382')")
                    content = content.replace("'#7f8c8d'", "cssVar('--text-muted', '#7f8c8d')")
                    
                    # GraphView colors
                    content = content.replace("'#00f2ff22'", "'rgba(0, 242, 255, 0.13)'")
                    content = content.replace("'#bc8cff22'", "'rgba(188, 140, 255, 0.13)'")
                    content = content.replace("'#00f2ff44'", "'rgba(0, 242, 255, 0.26)'")
                    content = content.replace("'#bc8cff44'", "'rgba(188, 140, 255, 0.26)'")
                    
                    # Anything else
                    for pattern, replace_val in hex_map.items():
                        content = pattern.sub(replace_val, content)

                else:
                    # Normal React Style replacements
                    for pattern, replace_val in hex_map.items():
                        content = pattern.sub(replace_val, content)
                
                # Undo accidental replacements inside Lucide icon imports or others 
                # (Lucide uses color="#fff" optionally but usually standard replacements work fine for string props too)

                if content != original_content:
                    with open(filepath, 'w', encoding='utf-8') as f:
                        f.write(content)
                    files_modified += 1
                    # Count differences (rough estimate)
                    diffs = len(original_content) - len(content)
                    replacements_made += 1

    print(f"✅ Modified {files_modified} files to remove HEX codes.")

if __name__ == "__main__":
    run_fixer()
