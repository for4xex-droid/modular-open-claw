/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

test.describe('UI Endpoint Fixes (TDD)', () => {

  test.beforeEach(async ({ page }) => {
    // Setup Auth and skip onboarding/birth
    await bypassAuth(page);
    await page.goto('/');
  });

  test('AgentConsole should use /api/stream/chat', async ({ page }) => {
    const agentTab = page.locator('.nav-item').filter({ hasText: 'AI Chat' });
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
      throw new Error("FAIL: AgentConsole did NOT use correctly mapped /api/stream/chat endpoint.", { cause: e });
    }
  });

  test('VoiceStore should use absolute API_BASE for commerce/balance', async ({ page }) => {
    const voiceTab = page.locator('.nav-item').filter({ hasText: 'Voice' });
    await expect(voiceTab).toBeVisible();
    // It should hit http://localhost:3015/api/v1/commerce/balance/agent-001
    // It should hit the commerce/balance endpoint using absolute or relative paths correctly
    const balanceRequestPromise = page.waitForRequest(req => 
      req.url().includes('/api/v1/commerce/balance')
    );

    await voiceTab.click();

    try {
      const balanceRequest = await balanceRequestPromise;
      // We know it includes /commerce/balance, verify the Auth header
      // Verify Authorization header
      const authHeader = await balanceRequest.headerValue('Authorization');
      expect(authHeader).toBe('Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZ2VudC0wMDEiLCJla3ljX3ZlcmlmaWVkIjp0cnVlfQ.mock_signature');
    } catch (e) {
      throw new Error("FAIL: VoiceStore did NOT use absolute API_BASE or correct Auth headers.", { cause: e });
    }
  });
});
