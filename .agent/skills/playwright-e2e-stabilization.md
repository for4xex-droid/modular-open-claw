---
name: playwright-e2e-stabilization
description: management-console の Playwright E2E が Flaky・ハングする場合に読む。認証/JWT・初期データ・Suspense・Tokio 枯渇の切り分け手順。単発の UI 文言修正のみなら不要。
---

# Playwright E2E 安定化規約

Playwright E2E（`apps/management-console/e2e/`、`playwright.config.ts`）の Flakiness とハングを防ぐための規約です。

## 発動条件

- `cd apps/management-console && npx playwright test` が不安定・タイムアウト・無限待機
- 新規 spec（例: `e2e/home_v2.spec.ts`, `e2e/cortex_view.spec.ts`）追加時
- lazy コンポーネント追加後に E2E が白画面・パニック

## 手順

1. **認証**: `e2e/helpers/auth-bypass.ts` の `bypassAuth(page)` を必ず先に呼ぶ。自前 JWT 文字列を spec 内に直書きしない。
2. **初期データ/オンボーディング**: `bypassAuth` と同様に `page.route('**/api/v1/bootstrap/status')` と `localStorage`（`i18nextLng`, `aiome_view_mode`）で SetupWizard・表示モードを固定する。
3. **Suspense**: `App.tsx` の lazy ルートと同様、遅延ロードは `<React.Suspense>` でラップする。
4. **ハング調査**: 20 分以上進まない場合は Tokio ワーカ枯渇（過去: `cortex_synth` 同期 await ループ）を疑い、`pkill -f playwright` / ゾンビ Node・Tauri プロセスを掃除してから再実行。

## 良い例 / 悪い例

### 良い例 — 共通ヘルパー + 3 パート JWT

```ts
import { bypassAuth } from './helpers/auth-bypass';

test.beforeEach(async ({ page }) => {
  await bypassAuth(page); // header.payload.signature 形式を sessionStorage へ注入
  await page.goto('/');
});
```

### 悪い例 — 不正 JWT による「幻覚的グリーン」

```ts
// ❌ ドット区切り2パートのみ → isAuthenticated() が誤って true になり得る
await page.addInitScript(() => {
  window.sessionStorage.setItem('aiome_secret', 'not-a-real-jwt');
});
```

> 出典: memory/2026-05-08.md Lessons「初期データフラグ注入は Flakiness を劇的に低減。Suspense 未ラップでパニック」、memory/archive/2026-04-07-handover.md「E2E 20時間ハング（cortex_synth の Tokio ワーカ枯渇）」、CHANGELOG「幻覚的グリーン根絶 — JWT 3パート構造へ統一」

## 完了条件

- `cd apps/management-console && npx playwright test` が全 GREEN
- **Negative Test**: `auth-bypass.ts` の JWT を一時的に `'broken'` に差し替え、認証必須 spec が RED になることを確認してから元に戻す
- **Revert**: JWT 復元後に再実行し、再び全 GREEN であること
