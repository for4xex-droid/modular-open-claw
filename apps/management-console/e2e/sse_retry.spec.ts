/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';

test.describe('SSE Resilience and Retry Guard', () => {
  test('should limit SSE retries and apply exponential backoff on 401 error', async ({ page }) => {
    let connectionAttempts = 0;
    const attemptTimestamps: number[] = [];

    // Intercept SSE requests and return 401
    await page.route('**/api/stream/vitality', async (route) => {
      connectionAttempts++;
      attemptTimestamps.push(Date.now());
      await route.fulfill({
        status: 401,
        body: 'Unauthorized'
      });
    });

    await page.addInitScript(() => {
      window.sessionStorage.setItem('aiome_secret', 'invalid_token');
    });

    await page.goto('/');

    // Wait for some time to allow retries
    await page.waitForTimeout(15000); 

    // RED EXPECTATION: 
    // Currently (before fix), it might retry many times rapidly or hang.
    // We want it to stop after 5 attempts and show "Connection Lost" status.
    
    console.log(`Connection attempts: ${connectionAttempts}`);
    
    // Check UI status
    const statusText = page.locator('.status-badge');
    await expect(statusText).toContainText('Connection Lost');

    // Verify limit (max 5 + initial 1 = 6 attempts total)
    expect(connectionAttempts).toBeLessThanOrEqual(6);

    // Verify backoff (delta between attempts should grow)
    if (attemptTimestamps.length >= 3) {
      const delta1 = attemptTimestamps[1] - attemptTimestamps[0];
      const delta2 = attemptTimestamps[2] - attemptTimestamps[1];
      expect(delta2).toBeGreaterThan(delta1);
    }
  });
});
