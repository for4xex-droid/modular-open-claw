import re

path = 'libs/infrastructure/src/task_orchestrator/mod.rs'
with open(path, 'r') as f:
    text = f.read()

# Find all occurrences of TaskDispatcher::new
lines = text.split('\n')
for start_idx in range(len(lines)):
    if 'TaskDispatcher::new(' in lines[start_idx] and 'pub fn new' not in lines[start_idx]:
        # find matching paren
        idx = start_idx
        nested = 0
        found = False
        while idx < len(lines):
            line = lines[idx]
            if '(' in line:
                nested += line.count('(')
            if ')' in line:
                nested -= line.count(')')
                if nested <= 0:
                    lines.insert(idx, "            None, // hook_manager")
                    found = True
                    break
            idx += 1
            if found: break

with open(path, 'w') as f:
    f.write('\n'.join(lines))
