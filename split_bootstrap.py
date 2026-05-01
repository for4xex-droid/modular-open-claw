import re

with open("apps/api-server/src/bootstrap.rs", "r") as f:
    lines = f.readlines()

out_lines = []
for line in lines:
    out_lines.append(line)

print("Total lines:", len(out_lines))
