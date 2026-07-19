# Management Console 配布・ソース正本計画（v1.0 FINAL）

- **ステータス**: **P1–P3 ✅ 2026-07-20**（P4 本番同期は都度 Human 許可）
- **目的**: MC の価値を git 静的ゴミではなく配信面に載せる。誤 Path A・`rm -rf`・ゾンビ `index.html` を構造的に排除する
- **継承**: billing closeout の「static コミット除外 / 本番 rsync」、`HUMAN_PUBLIC_BETA_RUNBOOK` NT-1 distroless、`scripts/sync_production_sources.sh`
- **/perfect-plan**: 2026-07-20 検証 → Open Question を推奨案でロック → 本ファイルが実行正本

## 0. Human 決定（ロック済み）

| # | 問い | 決定 |
|---|------|------|
| Q-A | 本番 rsync 実行者 | **Human が許可 → Agent が手順実行可**（開始トリガーと結果確認は Human） |
| Q-B | bind-mount 撤去 | **今四半期は維持**（ホットパッチ利便）。撤去は FE 頻度低下後の **別 ADR** |
| Q-C | 追跡 `static/index.html` | **短期: スタブ差し替え** → スクリプト/ガイド定着後に **untrack**（gitignore 全無視へ） |

## 1. コードベース事実（推測禁止）

| 事実 | 根拠 |
|------|------|
| Web UI ソース SSOT | `apps/management-console`（Vite。`public/` → `dist/` に avatar/vrm 等） |
| 配信 | `ServeDir` + `frontend_static_path` 既定 `apps/api-server/static`（`libs/shared/src/config.rs` / `FRONTEND_STATIC_PATH`） |
| イメージ同梱 | `docker/distroless.Dockerfile` L81: `dist` → `/app/apps/api-server/static` |
| **本番でイメージ内 static が負ける** | `docker-compose.production.yml` L148: bind-mount `./apps/api-server/static:...:ro` |
| git | `.gitignore`: `/apps/api-server/static/*` + `!index.html` 例外 |
| HEAD の index | 旧「Aiome \| Dashboard」CDN モノリス（製品 UI ではない） |
| ソース→ホスト同期（既存） | `scripts/sync_production_sources.sh`（allowlist。`apps/management-console` / `apps/api-server` 含む。compose 既定スキップ） |
| MC static 専用スクリプト | **未存在** → 新規はこれだけ（車輪の再発明禁止） |
| 運用実績 | `docs/operations/stripe-production-setup.md` §7.D「rsync apps/api-server/static」 |
| CI | `.github/workflows/ci.yml` が `npm run build`（artifact の本番 rsync 自動化は無し） |
| 独立 console サービス | 本番 compose に無し（`docker/console.Dockerfile` は本計画外） |

```text
apps/management-console  ──npm run build──►  dist/
                                              │
                    ┌─────────────────────────┼─────────────────────────┐
                    ▼                         ▼                         ▼
         distroless COPY              ホスト static/              git (原則載せない)
         (イメージ内)                 ★ bind-mount で実配信         index のみ例外
                    │                         ▲
                    └──── 現行 compose では ──┘ に負ける
```

## 2. 成功条件（DoD）

| ID | 条件 |
|----|------|
| D1 | FE の git 差分は原則 `apps/management-console/**`（＋本計画のスクリプト/docs/スタブ） |
| D2 | `static/assets/**` を git に載せない |
| D3 | 本番: Vite SPA の `index` + 参照 `assets/main-*.js` が 200（closeout R2 と同型） |
| D4 | `checkout/`・`biome-popup.html`・`avatar/` または `vrm/` が欠落しない |
| D5 | 同期前 bak があり Negative 後に復帰可 |
| D6 | 文書・スクリプトが「Path A 単独 ≠ UI 更新」を明示 |

## 3. やること / やらないこと

### 3.1 やること（実装フェーズ）

| Phase | 成果物 | 既存との関係 |
|-------|--------|----------------|
| **P0** | 本計画を OPEN / CHANGELOG から参照 | 手順の複製はしない |
| **P1** | **新規** `scripts/sync_mc_static.sh` | `sync_production_sources.sh` は触らない（ソース同期のまま）。本スクリプトは **`dist/` → static 宛先** のみ |
| **P2** | **新規** `docs/guides/MC_STATIC_DEPLOY.md`（薄い運用ガイド） | ランブック/stripe-setup/billing closeout へ **リンクのみ**（手順コピペ禁止） |
| **P3** | 追跡 `apps/api-server/static/index.html` を **スタブ**に置換 | 製品 UI・CDN Dashboard を置かない。Vite 本物 shell もコミットしない（assets 無しで壊れる） |
| **P4** | （FE 変更時）Human 許可後、Agent が P1 スクリプトで反映 | 本番ホスト操作は承認付き |
| **P5** | 後続: untrack index + gitignore 全無視 / bind-mount 撤去 ADR | 本 FINAL のスコープ外ゲート |

### 3.2 やらないこと

- `static/assets` の git 追跡化
- `rm -rf apps/api-server/static/*` をランブック正本に残すこと
- Path A（distroless rebuild）単独を「FE リリース完了」と呼ぶこと
- `sync_production_sources.sh` の二重実装・置き換え
- OP-011 / Tauri 配布 / CSP・Cache-Control 大規模改修を本計画に同梱（品質バックログ §6）
- HEAD 旧 Dashboard の復活

## 4. 配布経路（運用正本）

### Path B — ホスト static 同期（**UI 更新の必須経路**）

```text
1. Human: 「MC static を同期してよい」と明示
2. Agent/Human:
   a. （リモートソースが古い場合）DEST=... ./scripts/sync_production_sources.sh
      ※ api-server/*** は作業ツリーの static も運び得る → 先にローカルで P1 するか、ホスト上で build+P1
   b. ./scripts/sync_mc_static.sh
      - ローカル DEST 既定: apps/api-server/static
      - リモート例: DEST=root@HOST:/app/aiome/apps/api-server/static
   c. Smoke（スクリプト内 + Human ブラウザ）
3. Human: 結果確認（白画面なし / 主要導線）
```

**推奨ビルド場所**: 再現性のため「ホストまたは CI 相当環境で `npm ci && npm run build` → その `dist` を DEST へ」。開発 Mac の dirty dist を本番にそのまま載せない。

### Path A — distroless rebuild（**UI のためではない**）

- 目的: api-server バイナリ / 依存 / ラベル更新
- bind-mount 維持中は **A だけでは UI は変わらない**
- FE+Rust 同時リリース時は **B のあと（または前後で）A**（イメージとホストの内容ドリフトを減らす保険）

### ローカル検証

- dirty `static/` は **コミットしない**（放置可）
- 検証用: `sync_mc_static.sh`（ローカル DEST）後に api-server の ServeDir で確認
- `git restore apps/api-server/static/index.html` はスタブ導入後は「スタブに戻る」意味になる

## 5. P1 スクリプト仕様（実装契約）

`scripts/sync_mc_static.sh`（新規・bash・`set -euo pipefail`）

| 項目 | 仕様 |
|------|------|
| 入力 | `SKIP_BUILD=0`（既定）で `apps/management-console` にて `npm ci && npm run build`。`SKIP_BUILD=1` は既存 `dist/` 使用 |
| DEST | 既定 `apps/api-server/static`。リモート可 |
| バックアップ | `DEST.bak-YYYYMMDD-HHMMSS`（DEST がローカルで存在する場合）。リモートは可能なら同ホストに bak |
| 同期 | `rsync -a --delete --human-readable` **from** `apps/management-console/dist/` **to** `DEST/`（portable flags。`sync_production_sources.sh` と同系） |
| Smoke（必須） | (1) `index.html` が `/assets/` または `type="module"` を含む (2) `checkout` ディレクトリ存在 (3) `biome-popup.html` 存在 (4) `avatar` または `vrm` 存在 |
| 失敗 | 非0終了。bak パスを stderr に出す |
| 禁止 | リポジトリルートの無差別 `rm -rf` |

Verification Protocol（実装時）:
1. Positive: 正常 dist → smoke PASS  
2. Negative: 空ディレクトリを偽 dist にして拒否または smoke FAIL  
3. Revert: bak から復帰できることを確認

## 6. P3 スタブ仕様（実装契約）

追跡ファイル `apps/api-server/static/index.html` のみ置換。

必須文言の意図（文言は実装時に日本語/英語どちらでも可）:
- これは製品 UI ではない
- 配信は `scripts/sync_mc_static.sh` または distroless ビルド + **ホスト static（bind-mount）**
- `apps/management-console` がソース正本
- CDN Dashboard / Vite hashed 本物 shell を置かない（後者は assets 欠落下で壊れる）

スタブは最小 HTML（外部 script CDN なし）。ServeDir で開いても「ビルドが必要」と分かる程度。

## 7. ドキュメント配置（重複排除）

| ファイル | 役割 |
|----------|------|
| **本ファイル** | 実行正本・決定・DoD・非スコープ |
| `docs/guides/MC_STATIC_DEPLOY.md` | コピペ短い手順 + 本計画へのリンク |
| `HUMAN_PUBLIC_BETA_RUNBOOK.md` | NT-1 に「UI 更新は MC_STATIC_DEPLOY / Path B」1段落＋リンク（手順複製しない） |
| `billing_closeout_plan.md` | 既存「コミット除外」行を本計画へ参照追加（履歴は消さない） |
| `stripe-production-setup.md` | §7.D の rsync 行にガイドリンク |

## 8. 品質バックログ（本 FINAL に同梱しない）

優先度のみ記録。着手は別指示。

| ID | 内容 | 依存 |
|----|------|------|
| Q2 | hashed assets 長期キャッシュ / `index.html` no-cache（Caddy or ServeDir） | 設計 |
| Q3 | CSP（HTML meta vs `router.rs` ヘッダ）一本化 | Safety 明示承認 |
| Q5 | bind-mount 撤去 ADR → イメージ SSOT | Human + 今四半期は見送り（§0） |
| Q6 | index untrack + gitignore `static/**` | P1–P3 定着後 |
| Q7 | CI `dist` artifact → 本番 rsync | CI ゲート |

## 9. 実行順

```text
P0 台帳参照（OPEN/CHANGELOG）
 → P1 sync_mc_static.sh + 検証プロトコル
 → P2 MC_STATIC_DEPLOY.md + 既存 docs へリンク
 → P3 index.html スタブ（製品 UI を git に載せない）
 →（FE 変更時）P4 Human 許可 → Agent Path B
 → 後日ゲート: Q6 untrack / Q5 ADR
```

## 10. 実装着手条件

次のいずれか明示で実装開始:
- 「P1–P3 を実装しろ」
- 「MC static 計画を実装しろ」

本番ホストへの同期（P4）は毎回 **Human の許可文**が必要（§0 Q-A）。
