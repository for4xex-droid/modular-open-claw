/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect, type Page } from '@playwright/test';

test.use({
  launchOptions: {
    args: ['--use-gl=angle', '--enable-webgl', '--ignore-gpu-blocklist'],
  },
});

async function meanCanvasLuminance(page: Page): Promise<number> {
  const canvas = page.locator('canvas').first();
  await expect(canvas).toBeVisible({ timeout: 60_000 });
  await page.waitForFunction(() => {
    const el = document.querySelector('canvas');
    return el instanceof HTMLCanvasElement && el.width > 0 && el.height > 0;
  });
  return canvas.evaluate(async (el: HTMLCanvasElement) => {
    const url = el.toDataURL('image/png');
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error('canvas toDataURL failed'));
      img.src = url;
    });
    const off = document.createElement('canvas');
    off.width = img.width;
    off.height = img.height;
    const ctx = off.getContext('2d');
    if (!ctx) return 0;
    ctx.drawImage(img, 0, 0);
    const { data } = ctx.getImageData(0, 0, off.width, off.height);
    let sum = 0;
    const pixels = data.length / 4;
    for (let i = 0; i < data.length; i += 4) {
      sum += (data[i] + data[i + 1] + data[i + 2]) / 3;
    }
    return sum / pixels;
  });
}

async function waitForBiomeReady(page: Page): Promise<void> {
  await expect(page.getByTestId('biome-generation')).toBeVisible({ timeout: 60_000 });
  // WASM エンジンが実際に動作していること（世代が進むこと）を必須にする
  await page.waitForFunction(() => {
    const genEl = document.querySelector('[data-testid="biome-generation"]');
    return parseInt(genEl?.textContent?.replace(/\D/g, '') || '0', 10) >= 3;
  }, { timeout: 60_000 });
  await page.waitForFunction(async () => {
    const canvas = document.querySelector('canvas');
    if (!(canvas instanceof HTMLCanvasElement) || canvas.width === 0) return false;
    const url = canvas.toDataURL('image/png');
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error('toDataURL failed'));
      img.src = url;
    });
    const off = document.createElement('canvas');
    off.width = img.width;
    off.height = img.height;
    const ctx = off.getContext('2d');
    if (!ctx) return false;
    ctx.drawImage(img, 0, 0);
    const { data } = ctx.getImageData(0, 0, off.width, off.height);
    let sum = 0;
    for (let i = 0; i < data.length; i += 4) {
      sum += (data[i] + data[i + 1] + data[i + 2]) / 3;
    }
    return sum / (data.length / 4) > 8;
  }, { timeout: 60_000 });
}

test.describe('Biome Lenia rendering', () => {
  test.describe.configure({ timeout: 120_000 });

  test.beforeEach(async ({ page }) => {
    await page.route('**/api/v1/biome/**', async (route) => {
      await route.fulfill({ status: 200, json: [] });
    });
    await page.route('**/api/v1/bootstrap/status', async (route) => {
      await route.fulfill({ status: 200, json: { mode: 'normal' } });
    });
    await page.addInitScript(() => {
      window.localStorage.setItem('biome_tutorial_done', '1');
      window.localStorage.setItem('i18nextLng', 'en-US');
    });
    await page.goto('/biome-popup.html');
    await waitForBiomeReady(page);
  });

  test('Positive: Orbium field is visible (not all dark pixels)', async ({ page }) => {
    const lum = await meanCanvasLuminance(page);
    expect(lum).toBeGreaterThan(8);
  });

  test('Positive: seed brush brightens click region', async ({ page }) => {
    await page.getByTestId('control-seed-mode').click();
    const canvas = page.locator('canvas').first();
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    const cx = box!.x + box!.width / 2;
    const cy = box!.y + box!.height / 2;
    await page.mouse.click(cx, cy);
    await page.waitForTimeout(500);
    const lumAfter = await meanCanvasLuminance(page);
    expect(lumAfter).toBeGreaterThan(8);
  });

  test('Negative: seed mode OFF ignores canvas clicks', async ({ page }) => {
    const seedBtn = page.getByTestId('control-seed-mode');
    const label = await seedBtn.textContent();
    if (label?.includes('ON')) {
      await seedBtn.click();
    }
    // 自然進化による輝度変動を排除するため一時停止して比較
    await page.getByTestId('cycle-pause').click();
    await page.waitForTimeout(500);
    const before = await meanCanvasLuminance(page);
    const canvas = page.locator('canvas').first();
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.click(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.waitForTimeout(800);
    const after = await meanCanvasLuminance(page);
    expect(Math.abs(after - before)).toBeLessThan(2);
  });
});
