/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

// Enable video recording for this specific screencast "test"
test.use({ 
  video: 'on',
  launchOptions: { slowMo: 400 } // Slow down to make it viewable for humans
});

test('Aiome 90-second Screencast', async ({ page }) => {
  // Setup: Clear storage to simulate first-time visit, but we'll bypass onboarding for the screencast
  await bypassAuth(page);

  // Scene 1: Dashboard
  await page.goto('/');
  await page.waitForSelector('.app-container');
  await expect(page.locator('.status-badge').first()).toBeVisible();
  
  // Scene 2: Agent Console (Chat / System Feed)
  // Find the Agent Console tab and click it
  const agentTab = page.locator('.nav-item').filter({ hasText: /Agent Console|エージェント/i }).first();
  if (await agentTab.isVisible()) {
    await agentTab.click();
  }

  // Find the chat input
  const chatInput = page.locator('textarea[placeholder*="Send a message"], textarea[placeholder*="メッセージ"]');
  if (await chatInput.isVisible()) {
    await chatInput.fill('Please write a blog post about autonomous AI agents.');
    await page.keyboard.press('Enter');
  }

  // Wait for some streaming response
  await page.waitForTimeout(3000);

  // Scene 3: Cortex Navigation
  const targetTab = page.locator('.nav-item').filter({ hasText: /Cortex|コーテックス/i }).first();
  if (await targetTab.isVisible()) {
    await targetTab.click();
    await expect(page.locator('.cortex-container')).toBeVisible();
    await page.waitForTimeout(2000);
  }

  // Scene 4: Immune System Navigation
  const observabilityTab = page.locator('.nav-item').filter({ hasText: /Immune|免疫/i }).first();
  if (await observabilityTab.isVisible()) {
    await observabilityTab.click();
    await expect(page.locator('.immune-system-container, .immune-system')).toBeVisible();
    await page.waitForTimeout(2000);
  }

  // Scene 5: Finish
  await page.waitForTimeout(1000);
});
