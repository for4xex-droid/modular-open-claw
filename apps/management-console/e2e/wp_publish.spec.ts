/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';

test.describe('WordPress E2E Integration', () => {
  test('AI should autonomous publish article to wp-test environment', async ({ page }) => {
    // This test requires a live WP instance and a running LLM — skip in bare CI.
    test.skip(!process.env.WP_API_URL, 'WP_API_URL is not set.');

    await page.goto('/');

    // Ensure we are connected
    const chatInput = page.getByPlaceholder(/Talk to Artem/i);
    await expect(chatInput).toBeVisible();

    // Command the agent to publish to WordPress
    await chatInput.fill('Please publish a short test article about "The Future of AI" to WordPress.');
    await page.keyboard.press('Enter');

    // LLM + WP round-trip can be slow.
    test.setTimeout(90000);

    // Wait for any assistant bubble that contains a URL (proof of successful publish).
    const successBubble = page.locator('.message.assistant').filter({ hasText: /http/i });
    await expect(successBubble.first()).toBeVisible({ timeout: 60000 });
  });

  // Direct backend validation for the publish endpoint to be sure the E2E infrastructure works
  test('key-proxy /api/v1/wp/publish should successfully post to WordPress', async ({ request }) => {
    const vaultSecret = process.env.VAULT_SECRET || 'test_key_proxy_secret';
    const keyProxyUrl = process.env.KEY_PROXY_URL || 'http://127.0.0.1:3017';

    test.skip(!process.env.WP_API_URL, 'WP_API_URL is not set.');

    const response = await request.post(`${keyProxyUrl}/api/v1/wp/publish`, {
      headers: {
        'Authorization': `Bearer ${vaultSecret}`,
      },
      data: {
        caller_id: 'e2e-test',
        title: 'Direct E2E Test Title',
        content: 'This is a test post from Playwright direct E2E.',
        status: 'draft' // Always draft to avoid polluting the live site.
      }
    });

    expect(response.status()).toBe(200);
    const body = await response.json();
    expect(body.link).toBeDefined();
    expect(body.link).toContain('http');
  });
});
