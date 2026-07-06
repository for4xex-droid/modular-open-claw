/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/** Tabs where the resident Diorama avatar may appear (allowlist). */
export const DIORAMA_VISIBLE_TABS = new Set(['biome', 'dashboard']);

export function isDioramaVisible(activeTab: string): boolean {
  return DIORAMA_VISIBLE_TABS.has(activeTab);
}
