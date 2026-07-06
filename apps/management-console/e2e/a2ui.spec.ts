/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

const CARD_SURFACE_SSE = [
  'event: a2ui',
  'data: {"type":"createSurface","surface":{"id":"e2e-card","version":"0.9","source":"e2e","components":[{"type":"card","props":{"title":"Navigation Help","content":"Jump to audit logs"},"children":[{"type":"button","props":{"label":"Open Audit","action":"navigate:audit"},"children":[]}]}]}}',
  '',
  'event: done',
  'data: ',
  '',
].join('\n');

test.describe('A2UI Generative UI (U4)', () => {
  test('positive: renders card surface from mocked chat stream', async ({ page }) => {
    await bypassAuth(page);

    await page.route('**/api/stream/chat', async (route) => {
      if (route.request().method() === 'POST') {
        await route.fulfill({
          status: 200,
          headers: { 'Content-Type': 'text/event-stream' },
          body: CARD_SURFACE_SSE,
        });
        return;
      }
      await route.continue();
    });

    await page.goto('/');
    await page.waitForSelector('.app-container');

    const agentTab = page.locator('nav.nav-group div', { hasText: 'Agent Console' }).first();
    await expect(agentTab).toBeVisible({ timeout: 10000 });
    await agentTab.click();

    const input = page.locator('textarea, input[type="text"]').filter({ hasNot: page.locator('[type="password"]') }).last();
    await input.fill('show me navigation help');
    await input.press('Enter');

    await expect(page.getByText('Navigation Help')).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('Jump to audit logs')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Open Audit' })).toBeVisible();
  });

  test('negative: rejects malformed envelope type (no A2UI surface rendered)', async ({ page }) => {
    await bypassAuth(page);

    const badEnvelopeSse = [
      'event: a2ui',
      'data: {"type":"createSurfaceEvil","surface":{"id":"bad","components":[]}}',
      '',
      'event: done',
      'data: ',
      '',
    ].join('\n');

    await page.route('**/api/stream/chat', async (route) => {
      if (route.request().method() === 'POST') {
        await route.fulfill({
          status: 200,
          headers: { 'Content-Type': 'text/event-stream' },
          body: badEnvelopeSse,
        });
        return;
      }
      await route.continue();
    });

    await page.goto('/');
    await page.waitForSelector('.app-container');

    const agentTab = page.locator('nav.nav-group div', { hasText: 'Agent Console' }).first();
    await agentTab.click();

    const input = page.locator('textarea, input[type="text"]').filter({ hasNot: page.locator('[type="password"]') }).last();
    await input.fill('inject bad envelope');
    await input.press('Enter');

    await page.waitForTimeout(2000);
    await expect(page.getByText('A2UI Surface')).toHaveCount(0);
  });
});
