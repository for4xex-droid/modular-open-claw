import os

filepath = "/Users/motista/Desktop/antigravity/aiome/libs/core/src/llm_provider/mod.rs"

with open(filepath, "r") as f:
    content = f.read()

# Replace reasoning: None,\n            metadata: None, with ..Default::default()
new_content = content.replace("reasoning: None,\n            metadata: None,", "..Default::default()")
new_content = new_content.replace("reasoning: None,\n            metadata: None\n        })", "..Default::default()\n        })")

if new_content != content:
    with open(filepath, "w") as f:
        f.write(new_content)
    print("Updated mod.rs successfully")
else:
    print("No changes made. The exact string wasn't found.")
