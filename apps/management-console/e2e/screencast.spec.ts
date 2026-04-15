/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';

// Enable video recording for this specific screencast "test"
test.use({ 
  video: 'on',
  launchOptions: { slowMo: 400 } // Slow down to make it viewable for humans
});

test('Aiome 90-second Screencast', async ({ page }) => {
  // Setup: Clear storage to simulate first-time visit
  await page.addInitScript(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });

  // Scene 1: Welcome & Onboarding
  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1 })).toContainText(/Aiome|Welcome/i);
  
  // Enter initial info if onboarding modal exists
  const nameInput = page.getByPlaceholder(/Call me/i);
  if (await nameInput.isVisible()) {
    await nameInput.fill('Admin');
    await page.getByRole('button', { name: /Start/i }).click();
  }

  // Scene 2: Agent Console (Chat / System Feed)
  await page.waitForSelector('.app-container');
  const chatInput = page.getByPlaceholder(/Talk to Artem/i);
  await expect(chatInput).toBeVisible();
  
  // Send a message
  await chatInput.fill('Please write a blog post about autonomous AI agents.');
  await page.keyboard.press('Enter');

  // Wait for some streaming response
  await page.waitForTimeout(3000);

  // Scene 3: Cortex / TrendView Navigation
  const targetTab = page.locator('.nav-item, nav.nav-group div').filter({ hasText: /Cortex|Trend/i }).first();
  await targetTab.click();
  await expect(page.locator('.cortex-container, .trend-view-container')).toBeVisible();
  await page.waitForTimeout(2000);

  // Scene 4: SystemBirth / Observability Navigation
  const observabilityTab = page.locator('.nav-item').filter({ hasText: /SystemBirth/i }).first();
  await observabilityTab.click();
  await expect(page.locator('.system-birth, .soul-status')).toBeVisible();
  await page.waitForTimeout(2000);

  // Scene 5: Finish
  await page.waitForTimeout(1000);
});
