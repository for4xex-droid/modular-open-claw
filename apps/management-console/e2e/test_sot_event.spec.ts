import { test, expect } from '@playwright/test';

test('debug sot event', async ({ page }) => {
  page.on('console', msg => console.log('BROWSER LOG:', msg.text()));
  page.on('pageerror', err => console.log('BROWSER ERROR:', err.message));

  await page.addInitScript(() => {
    window.localStorage.setItem('aiome_onboarding_done', 'true');
    window.localStorage.setItem('aiome_birth_shown', 'true');
    window.sessionStorage.setItem('aiome_secret', 'mock_valid_token_dev');
    window.localStorage.setItem('aiome_test_mode', 'true');
  });

  await page.goto('/');

  console.log('Waiting for story-flow...');
  await page.waitForSelector('.story-flow', { state: 'visible', timeout: 5000 });
  console.log('story-flow visible. Injecting event...');

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

  console.log('Waiting 1s...');
  await page.waitForTimeout(1000);
  
  const isAttached = await page.evaluate(() => !!document.querySelector('.story-flow'));
  console.log('Is story-flow attached?', isAttached);
});
