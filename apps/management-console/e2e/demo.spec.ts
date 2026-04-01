/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';

test.describe('Autonomous AI Economy Demo', () => {
  test('should navigate to the demo page, render the UI, and start the demo', async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('aiome_onboarding_done', 'true');
      window.localStorage.setItem('aiome_birth_shown', 'true');
      window.sessionStorage.setItem('aiome_secret', 'mock_token');
    });

    await page.goto('/');

    // Click on "Synergy Demo" in the sidebar (assuming it gets added)
    const demoTab = page.locator('.nav-item').filter({ hasText: 'Synergy Demo' });
    
    // We expect this to fail (RED) since it doesn't exist yet
    await expect(demoTab).toBeVisible();
    await demoTab.click();

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
