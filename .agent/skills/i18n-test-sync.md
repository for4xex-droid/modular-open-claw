---
name: i18n-test-sync
description: UI 文字列を t() 化する・翻訳 JSON を触る・Jest テスト期待値を直すときに読む。Rust バックエンドのみの変更なら不要。
---

# i18n とテスト同期規約

management-console の UI 文字列国際化（`src/i18n/`）と Jest テストの整合を保つ規約です。

## 発動条件

- コンポーネントのハードコード文字列を `t('key')` へ移行
- `src/i18n/ja.json` / `src/i18n/en.json` のキー追加・削除
- `App.test.tsx` 等で `getByText` / `findByText` が失敗

## 手順

1. **翻訳 JSON 双方向同期**: 新キーは `ja.json` と `en.json` の両方へ同時追加。`src/i18n/i18n.test.ts` がキー parity を検査する。
2. **テスト期待値はキー文字列**: `App.test.tsx` の i18n モックは `t: (k) => k` のため、期待値は訳文ではなくキー（例: `'session.expired'`, `'nav.biotope'`）。
3. **条件ガードの退行確認**: i18n リファクタ後、変更ファイルで `isAuth` / `isAuthenticated` / 認可分岐が誤削除されていないか `rg` で確認。

## 良い例 / 悪い例

### 良い例 — キー追加 + テスト追随

```tsx
// コンポーネント
<p>{t('session.expired')}</p>

// App.test.tsx（モックはキーをそのまま返す）
await screen.findByText('session.expired');
```

```json
// ja.json / en.json 両方に同キーを追加
"session": { "expired": "…" }
```

### 悪い例 — 訳文でアサート

```tsx
// ❌ JSDOM では t() がキーを返すため常に失敗
expect(screen.getByText('Session expired')).toBeInTheDocument();
```

> 出典: memory/2026-06-14.md Lessons「t() 移行時、JSDOM アサーションは i18n モックのキー返却に依存 — テスト並行追随必須」、CHANGELOG「App.test.tsx で 'session.expired' キー期待へ修正」

## 完了条件

- `cd apps/management-console && npm test -- --run` が全 GREEN
- `npm test -- --run src/i18n/i18n.test.ts` で ja/en キー parity が PASS
- **Negative Test**: 追加したキーを `ja.json` から一時削除し、`i18n.test.ts` が RED になることを確認してから復元
- **Revert**: キー復元後に再実行し、再び全 GREEN であること
