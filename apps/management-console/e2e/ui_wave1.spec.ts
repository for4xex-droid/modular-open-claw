/*
 * Aiome - UI Wave 1 (TDD RED Phase)
 */
import { test, expect } from '@playwright/test';

test.describe('UI Wave 1 Features (TDD)', () => {
  test.beforeEach(async ({ page }) => {
    // Setup Mock Auth and clear state
    await page.addInitScript(() => {
      window.localStorage.setItem('aiome_onboarding_done', 'true');
      window.localStorage.setItem('aiome_birth_shown', 'true');
      window.sessionStorage.setItem('aiome_secret', 'mock_valid_token_dev');
    });

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
    // CharacterPanel is usually visible on the main page or world tab, assuming it's loaded.
    const characterPanel = page.locator('.character-panel, [data-testid="character-panel"]');
    
    // As per TDD, we expect these elements to exist but they don't yet.
    // Check for Soul Status Badge
    const soulBadge = page.locator('text=Lvl 42 | Awake');
    await expect(soulBadge).toBeVisible({ timeout: 4000 }); 

    // Check for eKYC Verified Badge
    const ekycBadge = page.locator('text=✓ Verified');
    await expect(ekycBadge).toBeVisible({ timeout: 4000 }); 
  });

  test('W1-3: StoryFlow should render SoT progress events', async ({ page }) => {
    // Navigate to World/StoryFlow if needed, but World is usually default
    const worldTab = page.locator('.nav-item').filter({ hasText: 'World' });
    if (await worldTab.isVisible()) {
      await worldTab.click();
    }

    // Mock SSE event injection for SoT progress
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
        detail: {
          type: 'sot_progress',
          message: 'Society of Thought deliberation: evaluating branch A vs B.',
          timestamp: new Date().toISOString()
        }
      }));
    });

    // We expect the new SoT styled event to appear in the StoryFlow feed
    const sotMessage = page.locator('text=Society of Thought deliberation: evaluating branch A vs B.');
    await expect(sotMessage).toBeVisible({ timeout: 4000 }); 
  });
});
