import { test, expect } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

test.describe('RTK Token Savings UI', () => {
  test('TokenSavingsIndicator should display when token_saved event is received', async ({ page }) => {
    await bypassAuth(page);

    await page.goto('/');
    
    // Ensure the app is loaded
    await page.waitForSelector('.app-container');

    // Make sure we are on the home-v2 tab (which renders StoryFlow)
    const homeV2Tab = page.locator('nav.nav-group div', { hasText: 'Home v2(Beta)' }).or(page.locator('nav.nav-group div', { hasText: 'Home' }));
    if (await homeV2Tab.isVisible()) {
        await homeV2Tab.click();
    }

    // 1. Initially, there should not be a token savings of 400
    await expect(page.getByTestId('token-saved-chars-exact')).toHaveCount(0);
    
    // 2. Dispatch a custom aiome_vitality_event for token_saved
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
        detail: {
          type: 'token_saved',
          data: { saved_chars: 400, ts: Date.now() }
        }
      }));
    });

    // 3. The UI should update to show 400 chars saved
    await expect(page.locator('.story-flow').getByTestId('token-saved-chars-exact')).toHaveText('400', { timeout: 5000 });

    // 4. Dispatch another event to check cumulative sum
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
        detail: {
          type: 'token_saved',
          data: { saved_chars: 800, ts: Date.now() + 100 } // Ensure different ts
        }
      }));
    });

    // 5. The UI should now show 1200 chars saved
    await expect(page.locator('.story-flow').getByTestId('token-saved-chars-exact')).toHaveText('1200', { timeout: 5000 });
  });
});
