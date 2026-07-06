/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

test.describe('Autonomous AI Economy Demo', () => {
  test.beforeEach(async ({ page }) => {
    // Setup Auth and skip onboarding/birth
    await bypassAuth(page);

    await page.goto('/');

    // U6-7: デモはサイドバー常設から降格したため、A2UI ナビゲーションで遷移する
    // （設定画面の「デモを再生」ボタンと同じ経路）
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('a2ui-navigate', { detail: { tab: 'demo' } }));
    });
  });

  test('should navigate to the demo page, render the UI, and start the demo', async ({ page }) => {
    // Verify DemoView title
    await expect(page.getByRole('heading', { name: 'Autonomous AI Economy Demo' })).toBeVisible();

    const startButton = page.getByRole('button', { name: 'Start Demo' });
    await expect(startButton).toBeVisible();

    await page.route('**/api/v1/demo/start', async route => {
      await route.fulfill({ status: 200, json: { status: 'success', message: 'Demo started' } });
    });

    await startButton.click();

    // Look for timeline
    await expect(page.locator('.timeline')).toBeVisible();
  });
});
