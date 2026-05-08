/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';

test.describe('UI Endpoint Fixes (TDD)', () => {

  test.beforeEach(async ({ page }) => {
    // Setup Auth and skip onboarding/birth
    await page.addInitScript(() => {
      window.localStorage.setItem('aiome_onboarding_done', 'true');
      window.localStorage.setItem('aiome_birth_shown', 'true');
      window.sessionStorage.setItem('aiome_secret', 'mock_valid_token_dev');
      window.localStorage.setItem('aiome_view_mode', 'advanced');
      window.localStorage.setItem('i18nextLng', 'en-US');
      window.localStorage.setItem('aiome_test_mode', 'true');
    });
    await page.goto('/');
  });

  test('AgentConsole should use /api/stream/chat', async ({ page }) => {
    const agentTab = page.locator('.nav-item').filter({ hasText: 'Agent Console' });
    await expect(agentTab).toBeVisible();
    await agentTab.click();

    // Monitor API requests
    const chatRequestPromise = page.waitForRequest(req => 
      req.url().includes('/api/stream/chat') && req.method() === 'POST'
    );

    // Send a message
    const textarea = page.getByPlaceholder('Send a message...');
    await textarea.fill('Hello AI');
    await page.keyboard.press('Enter');

    // If this times out, it means it's still using /api/agent/chat/stream (or nothing)
    try {
      const chatRequest = await chatRequestPromise;
      expect(chatRequest.url()).toContain('/api/stream/chat');
    } catch (e) {
      throw new Error("FAIL: AgentConsole did NOT use correctly mapped /api/stream/chat endpoint.");
    }
  });

  test('VoiceStore should use absolute API_BASE for commerce/balance', async ({ page }) => {
    const voiceTab = page.locator('.nav-item').filter({ hasText: 'Voice Store' });
    await expect(voiceTab).toBeVisible();
    await voiceTab.click();

    // It should hit http://localhost:3015/api/v1/commerce/balance/agent-001
    // (Note: Currently it hits /api/v1/commerce/balance/agent-001 relative to App origin)
    const balanceRequestPromise = page.waitForRequest(req => 
      req.url().startsWith('http://localhost:3015/api/v1/commerce/balance')
    );

    try {
      const balanceRequest = await balanceRequestPromise;
      expect(balanceRequest.url()).toContain('http://localhost:3015');
      // Verify Authorization header
      const authHeader = await balanceRequest.headerValue('Authorization');
      expect(authHeader).toBe('Bearer mock_valid_token_dev');
    } catch (e) {
      throw new Error("FAIL: VoiceStore did NOT use absolute API_BASE or correct Auth headers.");
    }
  });
});
