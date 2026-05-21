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

    const demoTab = page.locator('.nav-item').filter({ hasText: 'Synergy Demo' });
    
    // We expect this to fail (RED) since it doesn't exist yet
    await expect(demoTab).toBeVisible();
    await demoTab.click();
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
