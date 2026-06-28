/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

test('debug sot event', async ({ page }) => {
  page.on('console', msg => console.log('BROWSER LOG:', msg.text()));
  page.on('pageerror', err => console.log('BROWSER ERROR:', err.message));

  await bypassAuth(page);

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
