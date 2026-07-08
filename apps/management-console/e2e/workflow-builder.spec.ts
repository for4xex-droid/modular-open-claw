/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect, Page } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';

async function mockWorkflowSaveAndExecute(
  page: Page,
  options: { executeStatus?: number; executeBody?: unknown } = {}
) {
  const executeStatus = options.executeStatus ?? 200;
  const executeBody =
    options.executeBody ??
    ({ execution_id: 'exec-e2e-1', job_ids: ['job-a', 'job-b'] } as const);

  await page.route('**/api/v1/workflows/**/execute', async (route) => {
    await route.fulfill({
      status: executeStatus,
      contentType: 'application/json',
      body:
        executeStatus === 200
          ? JSON.stringify(executeBody)
          : JSON.stringify({ message: 'Execute failed (injected)' }),
    });
  });

  await page.route('**/api/v1/workflows', async (route) => {
    if (route.request().method() === 'POST') {
      await route.fulfill({ status: 200, json: { ok: true } });
      return;
    }
    await route.continue();
  });

  await page.route('**/api/v1/workflows/*', async (route) => {
    if (route.request().method() === 'PUT') {
      await route.fulfill({ status: 200, json: { ok: true } });
      return;
    }
    await route.continue();
  });
}

test.describe('Workflow Builder (W1-6 + W2-8)', () => {
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

  test('execute shows workflow execution status panel (W2-8 smoke)', async ({ page }) => {
    await mockWorkflowSaveAndExecute(page);
    await page.getByTestId('nav-workflow-builder').click();

    const toolbar = page.locator('.workflow-toolbar');
    await toolbar.getByRole('button', { name: /^Save$|^保存$/ }).click();
    await expect(page.getByText(/saved successfully|保存/i)).toBeVisible();

    const executeBtn = toolbar.locator('button.btn-primary');
    await expect(executeBtn).toBeEnabled({ timeout: 10_000 });
    await executeBtn.click();

    await expect(page.locator('.execution-status-panel')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText('RUNNING')).toBeVisible();
    await expect(page.locator('.toolbar-success')).toContainText('exec-e2e-1');
  });

  test('negative: execute API failure does not show RUNNING panel (W2-8)', async ({ page }) => {
    await mockWorkflowSaveAndExecute(page, { executeStatus: 500 });
    await page.getByTestId('nav-workflow-builder').click();

    const toolbar = page.locator('.workflow-toolbar');
    await toolbar.getByRole('button', { name: /^Save$|^保存$/ }).click();
    await expect(page.getByText(/saved successfully|保存/i)).toBeVisible();

    const executeBtn = toolbar.locator('button.btn-primary');
    await expect(executeBtn).toBeEnabled({ timeout: 10_000 });
    await executeBtn.click();

    await expect(page.locator('.execution-status-panel')).not.toBeVisible();
    await expect(toolbar.getByText(/Error|エラー/i)).toBeVisible();
  });
});
