/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';

test('Home V2 (Beta) Layout Loads Correctly', async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem('aiome_onboarding_done', 'true');
    window.localStorage.setItem('aiome_birth_shown', 'true');
    window.sessionStorage.setItem('aiome_secret', 'mock_token');
    window.localStorage.setItem('i18nextLng', 'en-US');
    window.localStorage.setItem('aiome_view_mode', 'advanced');
  });

  await page.goto('/');

  // Wait for main UI to load
  await page.waitForSelector('.app-container');

  // Click the new Home V2 tab
  const homeV2Tab = page.locator('nav.nav-group div', { hasText: 'Home v2(Beta)' });
  await expect(homeV2Tab).toBeVisible();
  await homeV2Tab.click();

  // Ensure the main new container mounts
  const homeContainer = page.locator('.home-v2-container');
  await expect(homeContainer).toBeVisible();

  // Character Panel should exist
  const charPanel = page.locator('.character-panel');
  await expect(charPanel).toBeVisible();
  
  // Checking for some character stats elements
  await expect(charPanel.getByText('Level')).toBeVisible();

  // Story Flow panel should exist
  const storyFlow = page.locator('.story-flow');
  await expect(storyFlow).toBeVisible();

  // Should have at least one FlowCard or the placeholder
  await expect(storyFlow.locator('.flow-card').first().or(storyFlow.locator('.artemis-status'))).toBeVisible();
});

test('Interactive Avatar Viewer Modal opens and closes', async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem('aiome_onboarding_done', 'true');
    window.localStorage.setItem('aiome_birth_shown', 'true');
    window.sessionStorage.setItem('aiome_secret', 'mock_token');
    window.localStorage.setItem('i18nextLng', 'en-US');
    window.localStorage.setItem('aiome_view_mode', 'advanced');
  });

  await page.goto('/');

  const homeV2Tab = page.locator('nav.nav-group div', { hasText: 'Home v2(Beta)' });
  await expect(homeV2Tab).toBeVisible();
  await homeV2Tab.click();

  const charPanel = page.locator('.character-panel');
  await expect(charPanel).toBeVisible();

  // Avatar billboard should be present and clickable
  const avatarBillboard = charPanel.locator('.avatar-billboard-container');
  await expect(avatarBillboard).toBeVisible();
  await avatarBillboard.click();

  // The modal should appear
  const viewerModal = page.locator('.avatar-viewer-modal');
  await expect(viewerModal).toBeVisible();
  
  const closeButton = viewerModal.locator('button.close-viewer-btn');
  await expect(closeButton).toBeVisible();

  // Close the modal
  await closeButton.click();
  await expect(viewerModal).not.toBeVisible();
});
