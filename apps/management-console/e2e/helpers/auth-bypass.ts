import { Page } from '@playwright/test';

/**
 * E2E テスト用の認証バイパス。
 *
 * 1. /api/v1/bootstrap/status をモック → bootMode = 'Normal' → SetupWizard スキップ
 * 2. JWT を sessionStorage に設定 → isAuthenticated() = true → LoginScreen スキップ
 * 3. i18n を英語に固定 → nav ラベルが英語で安定
 * 4. 表示モードを advanced に固定 → 全タブを表示
 */
export async function bypassAuth(page: Page) {
  await page.route('**/api/v1/bootstrap/status', async (route) => {
    await route.fulfill({ status: 200, json: { mode: 'normal' } });
  });

  await page.addInitScript(() => {
    window.sessionStorage.setItem(
      'aiome_secret',
      'eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJlMmUtdGVzdCIsInJvbGUiOiJhZG1pbiJ9.mock'
    );
    window.localStorage.setItem('i18nextLng', 'en-US');
    window.localStorage.setItem('aiome_view_mode', 'advanced');
  });
}
