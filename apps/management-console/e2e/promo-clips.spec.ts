import { test, expect, Page } from '@playwright/test';
import { bypassAuth } from './helpers/auth-bypass';
import * as fs from 'fs';
import * as path from 'path';

// 動画録画の設定を強制（実行時フラグに関わらず確実に録画する）
test.use({
  video: {
    mode: 'on',
    size: { width: 1440, height: 900 }
  },
  viewport: { width: 1440, height: 900 },
  launchOptions: {
    slowMo: 300 // アニメーションとインタラクションをゆっくり見せる
  }
});

// タブをクリックして内容ロードを待つ共通関数
async function navigateToTab(page: Page, label: RegExp) {
  const tab = page.locator('.nav-item').filter({ hasText: label }).first();
  await expect(tab).toBeVisible({ timeout: 5000 });
  await tab.click();
  await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
  await page.waitForTimeout(1500); // アニメーション settle 待ち
}

test.describe('Aiome Promo Clips', () => {
  test.setTimeout(120000);

  // 動画アセット永続化フック
  // WHY: Playwright は test-results/ を毎テスト起動時に全削除するため、
  //      部分実行（-g "Clip 3"）時に他の収録済み動画が不可逆に消失する。
  //      本フックはテスト成功時にのみ永続フォルダへコピーし、この消失を防止する。
  test.afterEach(async ({ page, context }, testInfo) => {
    const video = page.video();
    if (video && testInfo.status === 'passed') {
      try {
        const safeTitle = testInfo.title.replace(/[^a-zA-Z0-9]/g, '_');
        const destDir = path.resolve('../../docs/assets/promo-clips');
        fs.mkdirSync(destDir, { recursive: true });
        const destPath = path.join(destDir, `${safeTitle}.webm`);

        // Playwright の afterEach は page/context をフック終了後に閉じるが、
        // video.saveAs() はコンテキスト終了を待機するためデッドロックする。
        // 先に明示クローズすることで録画を即時完了させ、安全にコピーする。
        await page.close();
        await context.close();

        await video.saveAs(destPath);
        console.log(`[PROMO ASSET SECURED] ${destPath}`);
      } catch (err) {
        // バックアップ失敗はテスト結果に影響させない（副次機能のため）
        console.error(`[PROMO ASSET WARNING] Video backup failed for "${testInfo.title}":`, err);
      }
    }
  });

  test('Clip 1: Setup Wizard', async ({ page }) => {
    // SetupWizard を表示するため bootstrap/status を 'setup' で返す
    await page.route('**/api/v1/bootstrap/status', async (route) => {
      await route.fulfill({ status: 200, json: { mode: 'setup' } });
    });
    // Finalize 時の reload を阻止
    await page.route('**/api/v1/setup/init', async (route) => {
      await route.fulfill({ status: 200, json: { access_token: 'mock_token_for_promo' } });
    });
    // ページロード時の dialog を阻止 (reload阻止)
    page.on('dialog', async dialog => await dialog.dismiss());

    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(2000); // ロードアニメーション

    // Step 0 → 1: Start Setup クリック
    const startBtn = page.getByRole('button', { name: /Start Setup/i });
    if (await startBtn.isVisible()) {
      await startBtn.click();
      await page.waitForTimeout(2000);
    }

    // Step 1 → 2: TOS 同意
    const checkbox = page.locator('input[type="checkbox"]').first();
    if (await checkbox.isVisible()) {
      await checkbox.check();
      await page.waitForTimeout(1000);
      const nextBtn1 = page.getByRole('button', { name: /Next|次へ/i });
      await nextBtn1.click();
      await page.waitForTimeout(1500);
    }

    // Step 2 → 3: AI名入力
    const nameInput = page.locator('input[type="text"]').first();
    if (await nameInput.isVisible()) {
      await nameInput.fill('');
      await page.keyboard.type('Artemis', { delay: 80 });
      await page.waitForTimeout(1000);
      const nextBtn2 = page.getByRole('button', { name: /Next|次へ/i });
      await nextBtn2.click();
      await page.waitForTimeout(1500);
    }

    // Step 3 → 4: ViewMode 選択
    const nextBtn3 = page.getByRole('button', { name: /Next|次へ/i });
    if (await nextBtn3.isVisible()) {
      await page.waitForTimeout(2000);
      await nextBtn3.click();
      await page.waitForTimeout(1500);
    }

    // Step 4: Credentials 入力（Finalize は押さない）
    const pwInput = page.locator('input[type="password"]').first();
    if (await pwInput.isVisible()) {
      await page.keyboard.type('example_admin@domain.com');
      await page.keyboard.press('Tab');
      await page.keyboard.type('SuperSecretPassword123!', { delay: 50 });
      await page.keyboard.press('Tab');
      await page.keyboard.type('SuperSecretPassword123!', { delay: 50 });
    }
    await page.waitForTimeout(3000);
  });

  test('Clip 2: Home Dashboard', async ({ page }) => {
    await bypassAuth(page);
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(3000); // アバター + StoryFlow のアニメーション settle
    // スクロールして下部コンテンツも表示
    await page.mouse.wheel(0, 200);
    await page.waitForTimeout(2000);
    await page.mouse.wheel(0, 200);
    await page.waitForTimeout(3000);
  });

  test('Clip 3: Agent Console', async ({ page }) => {
    await bypassAuth(page);
    // チャットストリーミング応答をモック
    await page.route('**/api/stream/chat*', async (route) => {
      await route.fulfill({
        status: 200,
        headers: { 'Content-Type': 'text/event-stream' },
        body: [
          'event: text',
          'data: I\'ll analyze the quarterly revenue trends and suggest optimization strategies based on system performance.\r\rHere are 3 key optimization areas:\r1. **Dynamic Pricing** — Adjust pricing in real-time based on customer demand and CPU usage.\r2. **Churn Prevention** — Identify at-risk users and trigger automated follow-ups.\r3. **Cross-sell Optimization** — Leverage product affinity and model synergy.',
          '',
          'event: done',
          'data: stream finished',
          ''
        ].join('\n')
      });
    });
    await page.route('**/api/artifacts*', async (route) => {
      await route.fulfill({ status: 200, json: [] });
    });
    await page.route('**/api/v1/audit/ledger*', async (route) => {
      await route.fulfill({ status: 200, json: { entries: [] } });
    });

    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /AI Chat/i);

    const input = page.locator('textarea').first();
    if (await input.isVisible()) {
      await input.click();
      await page.keyboard.type('Analyze the quarterly revenue trends and suggest optimization strategies.', { delay: 50 });
      await page.waitForTimeout(1000);
      await page.keyboard.press('Enter');
    }
    
    // AIの応答バブルが画面上に正しくレンダリングされていることを検証 (TDD RED)
    const assistantBubble = page.locator('.app-container').getByText(/Dynamic Pricing|Churn Prevention/i).first();
    await expect(assistantBubble).toBeVisible({ timeout: 15000 });
    
    await page.waitForTimeout(15000); // ストリーミング応答の全展開を待つ
  });

  test('Clip 4: Cortex', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/v1/cortex/wiki*', async (route) => {
      await route.fulfill({ status: 200, json: [{ id: 'doc1', title: 'Architecture Protocol', content: '# Architecture Protocol\n\nThis defines the core behavior...' }] });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /Knowledge Base/i);
    await page.waitForTimeout(5000);
  });

  test('Clip 5: Knowledge Graph', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/synergy/graph*', async (route) => {
      await route.fulfill({ status: 200, json: { nodes: [{ id: 1, label: 'Core' }, { id: 2, label: 'Node A' }], edges: [{ from: 1, to: 2 }] } });
    });
    await page.route('**/api/artifacts?limit=50', async (route) => {
      await route.fulfill({ status: 200, json: [] });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /Knowledge Graph/i);
    await page.waitForTimeout(8000);
  });

  test('Clip 6: Causal Reasoning', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/v1/trajectory/*', async (route) => {
      await route.fulfill({ status: 200, json: { nodes: [], edges: [] } });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /Causal Trace/i);
    // フォーム画面の表示
    await page.waitForTimeout(5000);
  });

  test('Clip 7: Immune System', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/synergy/rules*', async (route) => {
      await route.fulfill({ status: 200, json: [{ id: 'rule1', name: 'SQL Injection Block', enabled: true }] });
    });
    await page.route('**/api/v1/audit/quarantine*', async (route) => {
      await route.fulfill({ status: 200, json: [] });
    });
    await page.route('**/api/v1/watchtower*', async (route) => {
      await route.fulfill({ status: 200, json: { status: 'active', score: 98 } });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /Security/i);
    await page.mouse.wheel(0, 300);
    await page.waitForTimeout(5000);
  });

  test('Clip 8: Skill Vault', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/skills*', async (route) => {
      await route.fulfill({ status: 200, json: [{ id: 'sk1', name: 'Web Search', description: 'Search the internet' }] });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /Skills/i);
    await page.waitForTimeout(5000);
  });

  test('Clip 9: MCP Integration', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/skills/mcp/config*', async (route) => {
      await route.fulfill({ status: 200, json: { servers: { 'local-tools': { command: 'node' } } } });
    });
    await page.route('**/api/skills*', async (route) => {
      await route.fulfill({ status: 200, json: [] });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /MCP Servers/i);
    await page.waitForTimeout(5000);
  });

  test('Clip 10: LoRA Autotuner', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/v1/lora/status/*', async (route) => {
      await route.fulfill({ status: 200, json: { status: 'idle' } });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /LoRA Autotuner/i);
    await page.waitForTimeout(5000);
  });

  test('Clip 11: SEO Pulse', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/v1/quality-gate/history?limit=10', async (route) => {
      await route.fulfill({ status: 200, json: [{ id: 1, score: 95 }] });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /SEO Pulse/i);
    await page.waitForTimeout(5000);
  });

  test('Clip 12: Voice Store', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/v1/commerce/balance/*', async (route) => {
      await route.fulfill({ status: 200, json: { points: 1500 } });
    });
    await page.route('**/api/v1/voice/list?scope=public', async (route) => {
      await route.fulfill({ status: 200, json: [{ id: 'v1', name: 'Echo', price: 100 }] });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /Voice/i);
    await page.waitForTimeout(5000);
  });

  test('Clip 13: Nurture Economy', async ({ page }) => {
    await bypassAuth(page);
    await page.route('**/api/v1/commerce/points/*', async (route) => {
      await route.fulfill({ status: 200, json: { points: 5000 } });
    });
    await page.route('**/api/v1/commerce/history/*', async (route) => {
      await route.fulfill({ status: 200, json: [] });
    });
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    await navigateToTab(page, /Economy/i);
    await page.waitForTimeout(5000);
  });

  test('Clip 14: Landing Page', async ({ page }) => {
    await page.goto('http://localhost:5174');
    await page.waitForTimeout(3000); // Hero 停留

    // LP セクションは mouse.wheel() で均等スクロール
    for (let i = 0; i < 8; i++) {
      await page.mouse.wheel(0, 350);
      await page.waitForTimeout(3000); // 各セクションで停留
    }
    await page.waitForTimeout(3000); // Footer/CTA で停留
  });
});
