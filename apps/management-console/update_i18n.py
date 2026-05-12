import json
import os

EN_FILE = 'src/i18n/en.json'
JA_FILE = 'src/i18n/ja.json'

def load_json(path):
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)

def save_json(path, data):
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write('\n')

en = load_json(EN_FILE)
ja = load_json(JA_FILE)

# nav replacements
nav_map_en = {
    "synergyHub": "Dashboard",
    "biotope": "System Overview",
    "chronicle": "Activity Log",
    "resonanceMap": "Knowledge Graph",
    "artifactVault": "Files",
    "biomeLab": "Multi-Agent Lab",
    "immuneSystem": "Security",
    "agentConsole": "AI Chat",
    "cortex": "Knowledge Base",
    "skillVault": "Skills",
    "voiceStore": "Voice"
}
nav_map_ja = {
    "synergyHub": "ダッシュボード",
    "biotope": "システム概要",
    "chronicle": "アクティビティログ",
    "resonanceMap": "ナレッジグラフ",
    "artifactVault": "ファイル",
    "biomeLab": "マルチエージェントラボ",
    "immuneSystem": "セキュリティ",
    "agentConsole": "AIチャット",
    "cortex": "ナレッジベース",
    "skillVault": "スキル",
    "voiceStore": "音声"
}

for k, v in nav_map_en.items():
    if k in en['nav']['section']:
        en['nav']['section'][k] = v
    elif k in en['nav']:
        en['nav'][k] = v

for k, v in nav_map_ja.items():
    if 'section' in ja.get('nav', {}):
        if k in ja['nav']['section']:
            ja['nav']['section'][k] = v
    if k in ja.get('nav', {}):
        ja['nav'][k] = v

# page replacements
page_map_en = {
    "chronicle": "Activity Log",
    "skillVault": "Skills",
    "biotope": "System Overview",
    "immuneSystem": "Security Monitor",
    "agentConsole": "AI Chat Console",
    "cortex": "Knowledge Base",
    "artifactVault": "Files Vault",
    "biomeLab": "Multi-Agent Lab",
    "voiceStore": "Voice Manager",
    "resonanceMap": "Knowledge Graph"
}
page_map_ja = {
    "chronicle": "アクティビティログ",
    "skillVault": "スキル",
    "biotope": "システム概要",
    "immuneSystem": "セキュリティ",
    "agentConsole": "AIチャットコンソール",
    "cortex": "ナレッジベース",
    "artifactVault": "ファイル",
    "biomeLab": "マルチエージェントラボ",
    "voiceStore": "音声管理",
    "resonanceMap": "ナレッジグラフ"
}
for k, v in page_map_en.items():
    if k in en.get('page', {}):
        en['page'][k] = v
for k, v in page_map_ja.items():
    if k in ja.get('page', {}):
        ja['page'][k] = v

# event replacements
event_map_en = {
    "karmaAssimilated": "Experience Gained",
    "aegisSentinel": "Security Alert",
    "societyOfThought": "Thinking Process"
}
event_map_ja = {
    "karmaAssimilated": "経験値獲得",
    "aegisSentinel": "セキュリティアラート",
    "societyOfThought": "思考プロセス"
}
for k, v in event_map_en.items():
    if k in en.get('event', {}):
        en['event'][k] = v
for k, v in event_map_ja.items():
    if k in ja.get('event', {}):
        ja['event'][k] = v

# agent replacements
agent_map_en = {
    "title": "AI Chat",
    "ready": "Ready",
    "synapticMemory": "Memory"
}
agent_map_ja = {
    "title": "AIチャット",
    "ready": "準備完了",
    "synapticMemory": "メモリ"
}
for k, v in agent_map_en.items():
    if k in en.get('agent', {}):
        en['agent'][k] = v
for k, v in agent_map_ja.items():
    if k in ja.get('agent', {}):
        ja['agent'][k] = v

# biotope replacements
biotope_map_en = {
    "ascension": "Level {{n}}",
    "resonance": "ENGAGEMENT",
    "neuralFatigue": "SYSTEM FATIGUE",
    "chroniclePulse": "ACTIVITY PULSE",
    "monitoringActivity": "Monitoring system activity...",
    "synergyHeartbeat": "SYSTEM HEARTBEAT"
}
biotope_map_ja = {
    "ascension": "レベル {{n}}",
    "resonance": "エンゲージメント",
    "neuralFatigue": "システム疲労",
    "chroniclePulse": "アクティビティパルス",
    "monitoringActivity": "システムアクティビティを監視中...",
    "synergyHeartbeat": "システムハートビート"
}
for k, v in biotope_map_en.items():
    if k in en.get('biotope', {}):
        en['biotope'][k] = v
for k, v in biotope_map_ja.items():
    if k in ja.get('biotope', {}):
        ja['biotope'][k] = v

# immune replacements
immune_map_en = {
    "title": "Security Monitor"
}
immune_map_ja = {
    "title": "セキュリティモニター"
}
for k, v in immune_map_en.items():
    if k in en.get('immune', {}):
        en['immune'][k] = v
for k, v in immune_map_ja.items():
    if k in ja.get('immune', {}):
        ja['immune'][k] = v

# timeline replacements
timeline_map_en = {
    "title": "Activity Log",
    "noEntries": "No log entries yet.",
    "loading": "Loading log...",
    "syncing": "Synchronizing logs..."
}
timeline_map_ja = {
    "title": "アクティビティログ",
    "noEntries": "ログがありません。",
    "loading": "ログを読み込み中...",
    "syncing": "ログを同期中..."
}
for k, v in timeline_map_en.items():
    if k in en.get('timeline', {}):
        en['timeline'][k] = v
for k, v in timeline_map_ja.items():
    if k in ja.get('timeline', {}):
        ja['timeline'][k] = v

# artifact replacements
artifact_map_en = {
    "decrypting": "Loading files..."
}
artifact_map_ja = {
    "decrypting": "ファイルを読み込み中..."
}
for k, v in artifact_map_en.items():
    if k in en.get('artifact', {}):
        en['artifact'][k] = v
for k, v in artifact_map_ja.items():
    if k in ja.get('artifact', {}):
        ja['artifact'][k] = v

# diagnostics replacements
diagnostics_map_en = {
    "title": "Audit Log"
}
diagnostics_map_ja = {
    "title": "監査ログ"
}
for k, v in diagnostics_map_en.items():
    if k in en.get('diagnostics', {}):
        en['diagnostics'][k] = v
for k, v in diagnostics_map_ja.items():
    if k in ja.get('diagnostics', {}):
        ja['diagnostics'][k] = v

# expression replacements
expression_map_en = {
    "currentInsight": "Current Insight",
    "analyzingKarma": "Loading analytics..."
}
expression_map_ja = {
    "currentInsight": "インサイト",
    "analyzingKarma": "分析を読み込み中..."
}
for k, v in expression_map_en.items():
    if k in en.get('expression', {}):
        en['expression'][k] = v
for k, v in expression_map_ja.items():
    if k in ja.get('expression', {}):
        ja['expression'][k] = v

# graph replacements
graph_map_en = {
    "title": "Knowledge Graph",
    "karma": "MEMORY"
}
graph_map_ja = {
    "title": "ナレッジグラフ",
    "karma": "メモリ"
}
for k, v in graph_map_en.items():
    if k in en.get('graph', {}):
        en['graph'][k] = v
for k, v in graph_map_ja.items():
    if k in ja.get('graph', {}):
        ja['graph'][k] = v

# treasure replacements
treasure_map_en = {
    "title": "AI Workspace",
    "subtitle": "AI's Workspace",
    "tuning": "Loading...",
    "loading": "Loading...",
    "resonance": "Engagement +5!"
}
treasure_map_ja = {
    "title": "AIワークスペース",
    "subtitle": "AIのワークスペース",
    "tuning": "読み込み中...",
    "loading": "読み込み中...",
    "resonance": "エンゲージメント +5!"
}
for k, v in treasure_map_en.items():
    if k in en.get('treasure', {}):
        en['treasure'][k] = v
for k, v in treasure_map_ja.items():
    if k in ja.get('treasure', {}):
        ja['treasure'][k] = v

# system replacements
system_map_en = {
    "calibrating": "Calibrating system...",
    "genesisProtocol": "System Active"
}
system_map_ja = {
    "calibrating": "システムを調整中...",
    "genesisProtocol": "システム稼働中"
}
for k, v in system_map_en.items():
    if k in en.get('system', {}):
        en['system'][k] = v
for k, v in system_map_ja.items():
    if k in ja.get('system', {}):
        ja['system'][k] = v

# sot replacements
sot_map_en = {
    "active": "Thinking Process Active"
}
sot_map_ja = {
    "active": "思考プロセス実行中"
}
for k, v in sot_map_en.items():
    if k in en.get('sot', {}):
        en['sot'][k] = v
for k, v in sot_map_ja.items():
    if k in ja.get('sot', {}):
        ja['sot'][k] = v

save_json(EN_FILE, en)
save_json(JA_FILE, ja)

print("i18n updated successfully.")
