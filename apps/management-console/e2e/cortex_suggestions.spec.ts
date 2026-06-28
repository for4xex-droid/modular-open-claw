/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

test.describe('Cortex Dynamic Suggestions', () => {
  test.beforeEach(async ({ page }) => {
    // Authenticate and bypass onboarding
    await bypassAuth(page);

    // Intercept Cortex Suggestions API
    await page.route('**/api/v1/cortex/suggestions', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          "What is Cortex?",
          "How do I use Aiome?",
          "Explain the Biotope"
        ]),
      });
    });

    page.on('console', msg => console.log('BROWSER LOG:', msg.text()));
    page.on('pageerror', err => console.log('BROWSER ERROR:', err));

    await page.goto('/');

    // Wait for main UI to load
    await page.waitForSelector('.app-container', { timeout: 5000 });

    // Navigate to Home V2
    const homeV2Tab = page.locator('nav.nav-group div', { hasText: 'Home v2' }).first();
    await expect(homeV2Tab).toBeVisible();
    await homeV2Tab.click();
  });

  test('Shows suggestion chips on focus and populates input', async ({ page }) => {
    const chatInput = page.locator('textarea[placeholder="Ready"]');
    await expect(chatInput).toBeVisible();

    // 1. Focus the input to trigger suggestions fetch and display
    await chatInput.click();

    // 2. Wait for suggestion chips to appear
    const suggestionsContainer = page.locator('.cortex-suggestions');
    await expect(suggestionsContainer).toBeVisible();

    const chips = suggestionsContainer.locator('.suggestion-chip');
    await expect(chips).toHaveCount(3);
    await expect(chips.nth(0)).toHaveText('What is Cortex?');

    // 3. Click the first suggestion
    await chips.nth(0).click();

    // 4. Verify the input is populated
    await expect(chatInput).toHaveValue('What is Cortex?');
  });
});
