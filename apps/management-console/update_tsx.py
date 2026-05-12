import os

def replace_in_file(filepath, replacements):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    new_content = content
    for old_str, new_str in replacements:
        new_content = new_content.replace(old_str, new_str)
        
    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

base_dir = 'src/components'

updates = {
    'Timeline.tsx': [
        ("!isKarma ? 'var(--accent-amber)' : (isLocal ? 'var(--accent-cyan)' : 'var(--accent-purple)')", "!isKarma ? 'var(--accent-amber)' : (isLocal ? 'var(--accent-cyan)' : 'var(--accent-purple)')"),
        ("{isKarma ? `${(e.karma_type || 'UNKNOWN').toUpperCase()} | JOB #${e.job_id || '?'}` : (e.event_type || 'SYSTEM').toUpperCase()}", "{isKarma ? `${(e.karma_type || 'UNKNOWN').toUpperCase()} | JOB #${e.job_id || '?'}` : (e.event_type || 'SYSTEM').toUpperCase()}"),
        ("t('timeline.localMemory')", "t('timeline.localMemory')"),
    ],
    'home/StoryFlow.tsx': [
        ("'KARMA'", "'MEMORY'"),
        ("'SOCIETY OF THOUGHT'", "'THINKING PROCESS'"),
        ("'IMMUNE ALERT'", "'SECURITY ALERT'"),
        ("'Deliberation in progress'", "'Thinking in progress'"),
        ("chat.relevantKarma.includes('見つかりませんでした') ? 'OUT-OF-DOMAIN' : 'MEMORY RETRIEVED'", "chat.relevantKarma.includes('見つかりませんでした') ? 'OUT-OF-DOMAIN' : 'MEMORY RETRIEVED'"),
    ],
    'AgentConsole.tsx': [
        ("'GENESIS NEURAL CONSOLE'", "t('agent.title', { defaultValue: 'AI Chat' })"),
        ("'Synaptic Memory'", "t('agent.synapticMemory', { defaultValue: 'Memory' })"),
    ],
    'DemoView.tsx': [
        ("'Settlement & Karma'", "t('demo.steps.settlementKarma', { defaultValue: 'Settlement & Experience' })"),
        ("{ step: 7, title: t('demo.steps.settlementKarma'), icon: <Clock size={18}/> },", "{ step: 7, title: t('demo.steps.settlementKarma'), icon: <Clock size={18}/> },"),
        ("Evolution Pulse (Karma)", "Evolution Pulse (Experience)"),
        ("Resonance Buffer", "Engagement Buffer"),
        ("Harvesting Karma", "Harvesting Experience"),
    ],
    'commerce/NurtureDashboard.tsx': [
        ("Karma Balance", "ポイント残高"),
        ("Karma points", "Experience points"),
    ],
    'cortex/ForecastView.tsx': [
        ("Karma Trend", "Experience Trend"),
    ],
    'TreasureBox.tsx': [
        ("Resonance increase", "Engagement increase"),
        ("Resonance Effect", "Engagement Effect"),
    ],
    'GraphView.tsx': [
        ("Karma Nodes", "Memory Nodes"),
        ("Karma refs", "Memory refs"),
        ("'KARMA'", "t('graph.karma', { defaultValue: 'MEMORY' })"),
    ],
    'ImmuneSystem.tsx': [
        ("'Sentinel Immune System'", "t('immune.title', { defaultValue: 'Security Monitor' })"),
    ],
    'home/CharacterPanel.tsx': [
        ("'Resonance'", "t('biotope.resonance', { defaultValue: 'ENGAGEMENT' })"),
    ],
    'home/HomePage.tsx': [
        ("label='クロニクル'", "label='ログ'"),
        ("label='アーティファクト'", "label='ファイル'"),
        ("label='バイオトープ'", "label='概要'"),
    ],
}

for root, dirs, files in os.walk(base_dir):
    for file in files:
        if file.endswith('.tsx'):
            rel_path = os.path.relpath(os.path.join(root, file), base_dir)
            if rel_path in updates:
                replace_in_file(os.path.join(root, file), updates[rel_path])

print("TSX files updated.")
