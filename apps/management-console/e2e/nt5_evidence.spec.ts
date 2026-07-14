/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 *
 * NT-5 / OP-063: 証拠ビジュアル自動撮影（ローカル実機・実ログイン）。
 *
 * 前提:
 *   - MC が起動済み（既定 http://127.0.0.1:1420）
 *   - export NT5_ADMIN_PASSWORD=...（未設定なら即 FAIL・直書き禁止）
 *
 * 実行:
 *   npx playwright test --config=playwright.nt5.config.ts
 *
 * 任意:
 *   NT5_BASE_URL / NT5_EVIDENCE_DATE=YYYY-MM-DD / NT5_SKIP_GIF=1
 */
import { test, expect, Page } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';
import { execFileSync } from 'child_process';
import { fileURLToPath } from 'url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const EVIDENCE_DATE = process.env.NT5_EVIDENCE_DATE || '2026-07-14';
const OUT_DIR = path.resolve(HERE, '../../../docs/assets/evidence', EVIDENCE_DATE);
const SKIP_GIF = process.env.NT5_SKIP_GIF === '1';

function requireAdminPassword(): string {
  const pw = process.env.NT5_ADMIN_PASSWORD;
  if (!pw || pw.trim().length === 0) {
    throw new Error(
      'NT5_ADMIN_PASSWORD is required (do not hardcode secrets in this file). Example: export NT5_ADMIN_PASSWORD=...'
    );
  }
  return pw;
}

async function ensureCockpit(page: Page) {
  await page.addInitScript(() => {
    window.localStorage.setItem('aiome_view_mode', 'cockpit');
    window.localStorage.setItem('i18nextLng', 'ja');
  });
}

async function loginIfNeeded(page: Page, password: string) {
  await page.goto('/');
  const pw = page.locator('#login-password');
  if (await pw.isVisible({ timeout: 5000 }).catch(() => false)) {
    await pw.fill(password);
    await page.getByRole('button', { name: /Login|ログイン/i }).click();
  }
  await expect(page.locator('.app-container')).toBeVisible({ timeout: 30000 });
  await expect(page.locator('#login-password')).toHaveCount(0);
}

async function goTab(page: Page, tab: string) {
  const nav = page.getByTestId(`nav-${tab}`);
  if (await nav.isVisible({ timeout: 8000 }).catch(() => false)) {
    await nav.click();
  } else {
    await page.evaluate((t) => {
      window.dispatchEvent(new CustomEvent('a2ui-navigate', { detail: { tab: t } }));
    }, tab);
  }
  await expect(page.locator('.app-container')).toBeVisible({ timeout: 15000 });
}

async function shot(page: Page, file: string) {
  fs.mkdirSync(OUT_DIR, { recursive: true });
  const dest = path.join(OUT_DIR, file);
  await page.screenshot({ path: dest, fullPage: false });
  expect(fs.statSync(dest).size, `${file} too small`).toBeGreaterThan(1000);
  console.log(`[NT5] wrote ${dest}`);
}

function resolveFfmpeg(): string | null {
  if (process.env.FFMPEG_PATH && fs.existsSync(process.env.FFMPEG_PATH)) {
    return process.env.FFMPEG_PATH;
  }
  const macBundled = path.resolve(HERE, '../node_modules/playwright-core/third_party/ffmpeg/ffmpeg-mac');
  if (fs.existsSync(macBundled)) return macBundled;

  const cacheRoots = [
    process.env.PLAYWRIGHT_BROWSERS_PATH,
    path.join(process.env.HOME || '', 'Library/Caches/ms-playwright'),
  ].filter(Boolean) as string[];
  for (const root of cacheRoots) {
    try {
      const found = execFileSync(
        'bash',
        ['-lc', `find "${root}" -type f \\( -name ffmpeg-mac -o -name ffmpeg \\) 2>/dev/null | head -1`],
        { encoding: 'utf8' }
      ).trim();
      if (found && fs.existsSync(found)) return found;
    } catch {
      /* continue */
    }
  }
  try {
    const found = execFileSync('bash', ['-lc', 'command -v ffmpeg'], { encoding: 'utf8' }).trim();
    return found || null;
  } catch {
    return null;
  }
}

test.describe.configure({ mode: 'serial' });

test.beforeAll(() => {
  // ブラウザ起動前に秘密未設定を落とす（fixture 起動後でも test 本体より先）
  requireAdminPassword();
});

test('NT-5 evidence capture', async ({ page }) => {
  test.setTimeout(180_000);
  const password = requireAdminPassword();
  fs.mkdirSync(OUT_DIR, { recursive: true });
  await ensureCockpit(page);
  await loginIfNeeded(page, password);

  const gifPath = path.join(OUT_DIR, '01-quickstart.gif');
  if (!SKIP_GIF && !fs.existsSync(gifPath)) {
    // Setup 済み環境では SetupWizard 相当として Home→Agent→Home を録画（ランブック「同等フロー」）
    const videoDir = path.join(OUT_DIR, '_video');
    fs.mkdirSync(videoDir, { recursive: true });
    const browser = page.context().browser();
    if (!browser) throw new Error('browser missing');

    const recCtx = await browser.newContext({
      viewport: { width: 1920, height: 1080 },
      colorScheme: 'dark',
      recordVideo: { dir: videoDir, size: { width: 1920, height: 1080 } },
    });
    await recCtx.addInitScript(() => {
      window.localStorage.setItem('aiome_view_mode', 'cockpit');
      window.localStorage.setItem('i18nextLng', 'ja');
    });
    const rec = await recCtx.newPage();
    await loginIfNeeded(rec, password);
    await goTab(rec, 'home-v2');
    await goTab(rec, 'agent');
    await goTab(rec, 'home-v2');
    await rec.close();
    await recCtx.close();

    const webms = fs.readdirSync(videoDir).filter((f) => f.endsWith('.webm'));
    if (webms.length === 0) throw new Error('no webm recorded for GIF');
    const webmPath = path.join(videoDir, webms[0]);
    const ffmpegBin = resolveFfmpeg();
    if (!ffmpegBin) {
      // Playwright 同梱 ffmpeg は gif muxer 無しのことが多い → webm を残して明示 FAIL
      console.warn(`[NT5] ffmpeg unavailable; left webm at ${webmPath}`);
      throw new Error('ffmpeg required to produce 01-quickstart.gif (set FFMPEG_PATH or install ffmpeg with gif support)');
    }
    try {
      execFileSync(
        ffmpegBin,
        ['-y', '-i', webmPath, '-vf', 'fps=8,scale=1920:-1:flags=lanczos', '-loop', '0', gifPath],
        { stdio: 'inherit' }
      );
    } catch {
      // gif 非対応ビルド向け: PNG 連番は別ツール。ここでは失敗を隠さない。
      throw new Error(
        `ffmpeg could not write GIF (try a full Homebrew ffmpeg). webm kept under ${videoDir}`
      );
    }
    fs.rmSync(videoDir, { recursive: true, force: true });
    console.log(`[NT5] wrote ${gifPath} via ${ffmpegBin}`);
  } else if (SKIP_GIF) {
    console.log('[NT5] NT5_SKIP_GIF=1 — skipping GIF');
  } else {
    console.log(`[NT5] reuse existing ${gifPath}`);
  }

  await goTab(page, 'karma');
  await page.getByTestId('activity-tab-audit').click();
  await expect(page.getByTestId('activity-tab-audit')).toHaveAttribute('aria-selected', 'true');
  await shot(page, '02-audit.png');

  await goTab(page, 'buzz-approval');
  await shot(page, '03-buzz-approval.png');

  await goTab(page, 'nurture');
  await shot(page, '04-nurture-economy.png');

  await goTab(page, 'workflow-builder');
  await shot(page, '05-workflow-builder.png');

  await goTab(page, 'agent');
  await shot(page, '06-agent-diorama.png');

  await goTab(page, 'karma');
  await page.getByTestId('activity-tab-usage').click();
  await expect(page.getByTestId('activity-tab-usage')).toHaveAttribute('aria-selected', 'true');
  await shot(page, '07-prompt-stats.png');

  const expected = [
    ...(SKIP_GIF ? [] : ['01-quickstart.gif']),
    '02-audit.png',
    '03-buzz-approval.png',
    '04-nurture-economy.png',
    '05-workflow-builder.png',
    '06-agent-diorama.png',
    '07-prompt-stats.png',
  ];
  for (const f of expected) {
    const p = path.join(OUT_DIR, f);
    expect(fs.existsSync(p), `missing ${f}`).toBeTruthy();
    expect(fs.statSync(p).size, `${f} too small`).toBeGreaterThan(1000);
  }
});
