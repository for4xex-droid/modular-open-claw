/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { DIORAMA_VISIBLE_TABS, isDioramaVisible } from './dioramaVisibleTabs';

describe('dioramaVisibleTabs', () => {
  it('shows avatar only on atmosphere tabs (biome, dashboard)', () => {
    expect(isDioramaVisible('biome')).toBe(true);
    expect(isDioramaVisible('dashboard')).toBe(true);
  });

  it('hides avatar on data-dense and tool tabs', () => {
    expect(isDioramaVisible('nurture')).toBe(false);
    expect(isDioramaVisible('store')).toBe(false);
    expect(isDioramaVisible('settings')).toBe(false);
    expect(isDioramaVisible('karma')).toBe(false);
    expect(isDioramaVisible('vault')).toBe(false);
    expect(isDioramaVisible('lora')).toBe(false);
    expect(isDioramaVisible('agent')).toBe(false);
    expect(isDioramaVisible('workflow-builder')).toBe(false);
  });

  it('allowlist contains exactly biome and dashboard', () => {
    expect([...DIORAMA_VISIBLE_TABS].sort()).toEqual(['biome', 'dashboard']);
  });
});
