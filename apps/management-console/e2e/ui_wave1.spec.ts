/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
/*
 * Aiome - UI Wave 1 (TDD RED Phase)
 */
import { test, expect } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

test.describe('UI Wave 1 Features (TDD)', () => {
  test.beforeEach(async ({ page }) => {
    // Setup Mock Auth and clear state
    await bypassAuth(page);

    // Mock the Soul Status API
    await page.route('**/api/v1/soul/status', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          level: 42,
          state: 'Awake',
          lastSync: new Date().toISOString()
        })
      });
    });

    // Mock the eKYC API
    await page.route('**/api/v1/ekyc/status', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          verified: true,
          method: 'DID_KEY'
        })
      });
    });

    await page.goto('/');
  });

  test('W1-1 & W1-2: CharacterPanel should display Soul Status and eKYC Badge', async ({ page }) => {
    
    // As per TDD, we expect these elements to exist but they don't yet.
    // Check for Soul Status Badge
    const soulBadge = page.locator('text=Lvl 42 | Awake');
    await expect(soulBadge).toBeVisible({ timeout: 4000 }); 

    // Check for eKYC Verified Badge
    const ekycBadge = page.locator('text=✓ Verified');
    await expect(ekycBadge).toBeVisible({ timeout: 4000 }); 
  });

  test('W1-3: StoryFlow should render SoT progress events', async ({ page }) => {
    // Ensure the main flow area is visible before injecting events
    await page.waitForSelector('.story-flow', { state: 'visible', timeout: 5000 });

    // Mock SSE event injection for SoT progress
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
        detail: {
          type: 'sot_progress',
          data: {
             type: 'SessionStart',
             data: { session_id: 'test-session-123', config: {}, trigger: 'Manual' }
          },
          timestamp: new Date().toISOString()
        }
      }));
    });

    // We expect the new SoT styled event to appear in the StoryFlow feed
    const sotMessage = page.getByText(/Deliberation session started/i);
    await expect(sotMessage).toBeVisible({ timeout: 4000 }); 
  });
});
