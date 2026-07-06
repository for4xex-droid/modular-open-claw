/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/**
 * Whitelist of activeTab values used in App.tsx — must stay in sync with App.tsx routes.
 *
 * U6-5/U6-7 注記:
 * - 'audit' / 'prompt-stats' はサイドバーから消えたが activeTab としては有効で、
 *   統合後の「アクティビティ」画面（karma）の該当タブにルーティングされる。
 * - 'demo' もサイドバー非表示だが、設定画面・ホームからの遷移先として有効。
 */
export const A2UI_NAV_TABS = [
    'home-v2',
    'agency',
    'dashboard',
    'demo',
    'biome',
    'karma',
    'graph',
    'causal',
    'seo-pulse',
    'buzz-approval',
    'artifacts',
    'audit',
    'expressions',
    'commune',
    'store',
    'nurture',
    'workflow-builder',
    'status-page',
    'ban-dashboard',
    'immune',
    'agent',
    'cortex',
    'vault',
    'mcp-dashboard',
    'prompt-stats',
    'lora',
    'settings',
] as const;

export type A2uiNavTab = (typeof A2UI_NAV_TABS)[number];

export function isValidA2uiNavTab(tab: string): tab is A2uiNavTab {
    return (A2UI_NAV_TABS as readonly string[]).includes(tab);
}
