import re

def parse_env_keys(filepath):
    keys = set()
    with open(filepath, 'r') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            # Match KEY=VALUE or export KEY=VALUE
            match = re.match(r'^(?:export\s+)?([A-Za-z0-9_]+)\s*=', line)
            if match:
                keys.add(match.group(1))
    return keys

env_keys = parse_env_keys('.env')
example_keys = parse_env_keys('.env.example')

missing_in_example = env_keys - example_keys
print("Missing in .env.example:")
for k in sorted(missing_in_example):
    print(k)
