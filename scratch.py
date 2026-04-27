import re

# Update CHANGELOG.md
with open("CHANGELOG.md", "r") as f:
    content = f.read()

unreleased_idx = content.find("## [Unreleased]")
if unreleased_idx != -1:
    insert_pos = content.find("\n", unreleased_idx) + 1
    new_entries = """### Changed
- `tts_worker.rs`: Applied Interface Segregation Principle (ISP) to decouple from `JobQueue` God Trait, introducing `TtsQueue` for robust `mockall::automock` testing.
- `aiome-core-contracts`: Removed `WP_API_TOKEN` plaintext injection logic; fully migrated to AbyssVault Key Proxy for `WordPressAdapter` to eliminate memory extraction vulnerabilities.
"""
    content = content[:insert_pos] + new_entries + content[insert_pos:]
    with open("CHANGELOG.md", "w") as f:
        f.write(content)

# Update RIPPLE_MAP.md
with open(".context/RIPPLE_MAP.md", "r") as f:
    content = f.read()

# Replace "Abyss Vault化は「将来フェーズへのTODO」としてマーク済。" with "Abyss Vault化完了。"
content = content.replace("Abyss Vault化は「将来フェーズへのTODO」としてマーク済。", "Abyss Vault化を完了し、直接取得フォールバックを撤廃済。")

with open(".context/RIPPLE_MAP.md", "w") as f:
    f.write(content)

print("Updated docs")
