import re

with open("libs/core/src/expression/tts_worker.rs", "r") as f:
    content = f.read()

# 1. Add `use async_trait::async_trait;`
if "use async_trait::async_trait;" not in content:
    content = content.replace("use crate::error::AiomeError;", "use crate::error::AiomeError;\nuse async_trait::async_trait;")

# 2. Change `queue: &dyn JobQueue,` to `queue: &dyn TtsQueue,`
content = content.replace("queue: &dyn JobQueue,", "queue: &dyn TtsQueue,")

with open("libs/core/src/expression/tts_worker.rs", "w") as f:
    f.write(content)

print("Fixed imports and types")
