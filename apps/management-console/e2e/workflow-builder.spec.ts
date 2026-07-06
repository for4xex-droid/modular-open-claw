/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

test.describe('Workflow Builder (W1-6)', () => {
  test.beforeEach(async ({ page }) => {
    await bypassAuth(page);
    await page.goto('/');
  });

  test('opens workflow builder and shows node palette', async ({ page }) => {
    const nav = page.getByTestId('nav-workflow-builder');
    await expect(nav).toBeVisible();
    await nav.click();

    await expect(page.locator('.workflow-palette')).toBeVisible();
    await expect(page.getByRole('button', { name: /Start Node|開始ノード/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /Validate|検証/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /Save|保存/i })).toBeVisible();
  });
});
