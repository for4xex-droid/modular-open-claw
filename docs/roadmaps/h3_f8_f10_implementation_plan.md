# 実装計画書 H3: F-8 Multi-Tenant Agency / F-9 開放経済圏 / F-10 Voice Interface

**作成日**: 2026-07-03（偵察による実在確認済み。行番号は当日時点 — 着手時はアンカー文字列で再特定）
**実行順**: F-10（独立・最小）→ F-8 → F-9（F-9 は Safety-Critical 最深部。全項目人間レビュー必須）
**前提**: F-8 は OP-012（PostgreSQL 本番検証）と F-1/F-2 完了後。F-9 は Phase 4（Biome Reputation）・OP-011・F-3・F-4 完了後。

---

## 0. 共通の安全網

機能ごとにブランチ（`feature/f10-voice` 等）を切り、ベースラインを記録:

```bash
cargo check --workspace --tests
cargo test -p infrastructure -p api-server -p aiome-node 2>&1 | grep "test result"
cargo clippy --workspace --all-targets -- -D warnings
cd apps/management-console && npx jest 2>&1 | tail -4
```

共通ルール: 1項目=1コミット / 完了条件未達なら中断・報告 / assert 緩和禁止 / アンカーで再特定。

---

# PART A: F-10 Voice Interface

## 現状理解（偵察で確定した重要事実）

- **STT エンジンは実装・DI 済み**: `WhisperTranscriptionAdapter`（`whisper_transcription.rs`、`RuntimeJail` 経由で `insanely-fast-whisper` CLI 実行）。`AIOME_STT_ENABLED`（デフォルト false）で有効化。`AppState.transcription_engine` に注入済み。MCP `transcribe` ツールあり（**入力はファイルパスのみ**）。
- **STT の REST エンドポイントは存在しない**。音声アップロードの参照実装は `POST /api/v1/voice/upload`（multipart、**WAV マジックバイト検証済み** — アンカー: `voice/upload`）。
- TTS は完備: `POST /api/v1/voice/synthesize?stream=true` で SSE（`audio`/`viseme` イベント）→ フロント `useTtsSse` → `useVisemeSync` → VRM 表情、のリップシンク経路が稼働中。
- チャット送信: `useAgentChat.ts` の `sendMessage`（アンカー: `/api/stream/chat`）。**音声の差し込み位置は prompt 確定前**。
- フロントに `getUserMedia`/`MediaRecorder` の使用実績ゼロ。
- `insanely-fast-whisper` は `tauri.conf.json` の externalBin に**未登録**（CLI はユーザー環境の PATH 前提）。externalBin 変更は Safety-Critical（T-003 チェック必須）だが、**本計画では externalBin に追加しない**（PATH 実行の現状を維持。同梱化は別判断）。

## 作業項目

### V-1: STT REST エンドポイント（api-server）

- **対象**: `routes/voice.rs` に `transcribe_handler` 追加、`router.rs`、`api.rs`
- **変更**: `POST /api/v1/voice/transcribe`（multipart、WAV のみ）。実装は既存 `voice/upload` の multipart 受理＋マジックバイト検証をコピーし、`tempfile`（既存依存を確認。なければ `resolver.resolve("tmp")` 配下に UUID 名で書く）へ保存 → `state.transcription_engine.transcribe(path)` → **応答後に一時ファイルを必ず削除**（`defer` 的に `let _ = std::fs::remove_file(...)` を成功・失敗両経路で実行 — 受け入れ基準3「平文残留なし」）。`AIOME_STT_ENABLED=false` 時は 503 と有効化手順のメッセージ。応答: `{ "text": ..., "segments": [...] }`。サイズ上限 25MB。
- **統合テスト**: `test_voice_transcribe_auth`（401）/ `test_voice_transcribe_rejects_non_wav`（400）/ `test_voice_transcribe_disabled_returns_503` / `test_voice_transcribe_cleans_temp_file`（モックエンジンで成功後に tmp が空）。実 Whisper 実行はテストしない（`TranscriptionEngine` trait のモックを `api_integration_tests/common.rs` の慣例で注入）。
- **完了条件**: 4本 PASS、Negative 配線確認、clippy。
- **依存**: 項目0

### V-2: プッシュトゥトーク UI（management-console）

- **対象**: `src/hooks/useVoiceInput.ts`（新規）、`src/components/AgentConsole.tsx`（マイクボタン1個の追加）、i18n
- **変更**:
  1. フック: `navigator.mediaDevices.getUserMedia({audio:true})` + `MediaRecorder`。**録音形式は webm/opus になるため WAV 変換が必要** — `AudioContext.decodeAudioData` → 16bit PCM WAV エンコード（依存追加禁止、~40行の純関数 `encodeWav()` を `src/lib/wavEncoder.ts` に実装。テスト付き）。停止時に `POST /api/v1/voice/transcribe` へ送信し text を返す。
  2. AgentConsole: 入力欄横にマイクボタン（押下中録音、離すと送信）。結果 text を `setInput(text)` に入れる（**自動送信しない** — 誤認識の確認機会を残す）。マイク拒否・503 は Toast 表示。
  3. 既存 `autoTts`（応答の自動読み上げ + Viseme リップシンク）が ON なら、これで「話す→聞く→アバターが答える」が完成する（新規実装不要 — 既存経路を使う）。
- **テスト**: `wavEncoder.test.ts`（既知 PCM 入力の RIFF ヘッダ検証）/ `useVoiceInput` はフック単体で MediaRecorder を jest.mock。AgentConsole 既存テストが存在すれば GREEN 維持。
- **完了条件**: jest PASS、tsc 新規エラーなし。
- **依存**: V-1

### V-3: ローカル完結の手動 E2E チェックリストとドキュメント同期

- **対象**: `docs/guides/VOICE_INTERFACE.md`（新規）、CHANGELOG/OPEN/roadmap
- **変更**: (a) `AIOME_STT_ENABLED=true` + `insanely-fast-whisper` のインストール手順、(b) 手動チェックリスト: ネットワーク遮断状態で 発話→転写→応答→TTS＋リップシンク が完結すること、応答開始 3 秒以内（M シリーズ基準）の計測欄。
- **完了条件**: docs-sync-check PASS。
- **依存**: V-2

## F-10 やらないこと

1. `tauri.conf.json` の externalBin に触らない（サイドカー同梱は別判断。触る場合は T-003 と人間レビュー必須）。
2. Whisper エンジンの再実装・別 STT 追加をしない。
3. ウェイクワード・常時待受をしない（プッシュトゥトークのみ）。
4. OpenAI TTS の Viseme 対応改修をしない（Mock/ローカル TTS の既存対応を使う）。

---

# PART B: F-8 Multi-Tenant Agency Mode

## 現状理解

- セル分離は**プロセス境界で実装済み**: `CELL_ID` 環境変数（必須、`[a-zA-Z0-9_-]{1,64}`）→ `workspace/{CELL_ID}/aiome.db` 等の物理分離。preflight が DB パス逸脱を fatal 化。**つまり「1セル=1 api-server プロセス」であり、テナント管理とはプロセスのライフサイクル管理である**。
- 管理系 API は ban 3本のみ（`routes/admin.rs`）。セルのライフサイクル API は存在しない。
- メータリング素材は実在: `prompt_evaluation_log`（cost_usd 含む）＋ `get_job_count_since` / `get_pending_job_count`。ただし**すべてセル内 DB に閉じる**ため、横断集計には「各セルの API を叩く」構造が正しい（DB 直結の横断はセル分離の破壊）。
- `AiaaOnboardingWizard.tsx` は UI 骨格のみ（Blueprint 保存 API 未接続、checkout のみ実装）。

## アーキテクチャ決定（確定。実行者は変更しない）

セル管理は**新規の独立バイナリ `apps/cell-manager`**（親プロセス）として実装する。api-server 本体には「自セルの使用量を返す API」だけを足す。理由: api-server 内にセル起動機能を持たせると、セルが他セルのプロセスを制御できてしまい Cell Isolation を破壊するため。

## 作業項目

### C-1: セル内使用量 API（api-server）

- **対象**: `routes/metering.rs`（新規）+ 配線 + OpenAPI
- **変更**: `GET /api/v1/metering/usage?period=30d`（Admin/System ロール必須 — `audit.rs` の RBAC パターン踏襲）→ `{ cell_id, jobs_completed, jobs_pending, llm_calls, llm_cost_usd }`。実装は `get_job_count_since` + `prompt_evaluation_log` 集計（F-2 L-1 の `outcome_ledger.rs` があれば再利用、なければ同等 SQL）。
- **完了条件**: 統合テスト2本（401 / 集計一致）、Negative 配線確認、clippy。
- **依存**: 項目0

### C-2: `cell-manager` バイナリ（新規クレート）

- **対象**: `apps/cell-manager/`（workspace members 追加）
- **変更**: 機能は最小4つ: `create <cell_id>`（`CELL_ID`/`PORT`/`AIOME_DATA_DIR` を割り当てて api-server を子プロセス起動、設定は `cells.json` に永続化）/ `stop <cell_id>` / `list` / `usage`（各セルの C-1 API を admin トークンで叩いて集約表示）。HTTP サーバーは**持たない**（CLI のみ。管理ダッシュボードの HTTP 化は次期）。子プロセスの起動は Tauri sidecar 起動（`src-tauri/src/lib.rs` の `CELL_ID` 注入パターン、アンカー: `desktop-0`）を参照。
- **テスト**: `cells.json` の CRUD ユニットテスト、`create` の環境変数組み立てテスト（実プロセス起動は手動チェックリスト）。
- **完了条件**: `cargo test -p cell-manager` PASS、`cargo check --workspace`、手動: 2セル起動 → 相互のトークンで 403（クロスセル遮断、受け入れ基準1）を確認しチェックリストに記録。
- **依存**: C-1

### C-3: クロスセル遮断の自動テスト

- **対象**: `apps/api-server/tests/cell_isolation_tests.rs`（新規）
- **変更**: 同一テストプロセス内で `CELL_ID` の異なる2つのテストサーバー（`create_test_server` を CELL_ID パラメタ化 — `common.rs` の変更は Mock 同期4点セットに従う）を立て、セル A のトークンでセル B の全代表 API（workflows/settings/jobs/metering）が 401/403 になることを検証。JWT 署名鍵がセルごとに異なることが遮断の根拠（`JWT_PRIVATE_KEY_B64` 未共有）— これをテストの前提コメントに明記。
- **完了条件**: 新テスト PASS。**Negative Test**: 両セルに同一署名鍵を与えるとテストが FAIL する（=遮断が鍵分離に依存している事実の検証）ことを確認し、復元。
- **依存**: C-2

### C-4: ヘッドレス統合（セル作成→Playbook→初回ジョブ）とダッシュボード最小版

- **対象**: `cell-manager` に `provision <cell_id> --playbook seo-operations` サブコマンド、`AiaaOnboardingWizard.tsx` の Step 2 実体化
- **変更**: provision = create → setup/init API 呼び出し（管理者パスワード自動生成・表示）→ `POST /api/v1/playbooks/:id/install`（F-1 実装済み）→ workflow execute。Wizard は provision 結果（セル URL・初期パスワード）の表示までを実装（checkout 連携は現状維持）。
- **完了条件**: provision の統合テスト（テストサーバー相手）PASS（受け入れ基準2）、jest GREEN。
- **依存**: C-3、F-1（実装済み）

### C-5: ドキュメント同期

- `docs/guides/AGENCY_MODE.md`（新規: 構成図・provision 手順・課金メータの読み方）、CHANGELOG/OPEN/roadmap。障害注入（1セル OOM が他セルに波及しない）は手動チェックリスト（受け入れ基準4）。
- **依存**: C-4

## F-8 やらないこと

1. **DB 直結のセル横断集計をしない**（分離の破壊。集約は各セル API 経由のみ）。
2. api-server にセル起動・停止機能を持たせない。
3. 課金の自動決済（Stripe 連携）をしない（メータの数字提供まで。請求は事業者の手動）。
4. OP-012（PostgreSQL 検証）未消化での本番宣言をしない。

---

# PART C: F-9 開放経済圏（**全項目 Safety-Critical・人間レビュー必須**）

## 現状理解

- **Gig 状態機械は実装済み**: `GigEngine` trait（publish_intent/submit_bid/accept_bid/deliver/verify_and_settle）と `UniversalGigEngine`（`libs/aiome-commerce/src/gig.rs`）。エスクローは `'Locked'`→release/refund。**ただし `GigOrderStatus` の `Bidding`/`InProgress`/`Verified`/`Disputed`/`Cancelled` は未使用**（実装は Open→Accepted→Delivered→Completed/Rejected の短縮経路）。
- aiome-node の MCP に `gig/publish`・`gig/capabilities` あり。**`gig/status` は Agent Card に記載されているが未実装**（tools/list にない）。
- samsara-hub に Gig メッセージ型は**ない**。拡張パターンは確立済み: `contracts.rs` の `HubMessage` に variant 追加 → ハンドラで `state.tx.send` → `ws.rs` で配信。
- OXP Trust: HMAC 証明書 + ヘッダ `X-OxiLean-Proof-Certificate`、Nurture 側 `require_oxp_certificate`（score≥900、鮮度 -60..300s）。
- KarmaForge の `evaluate_trust_score` は `commercial/` 側に実装済みだが **HTTP API はない**。

## 作業項目（全項目、コミット後に人間レビュー承認を得てから次へ）

### G-1: `gig/status` MCP ツールの実装（欠落補完）

- **対象**: `apps/aiome-node/src/mcp_server.rs`（アンカー: `gig/publish` のハンドラ）
- **変更**: tools/list に `gig/status` を追加し、`handle_tool_call` で intent_id を受けて `gig_intents` の status を返す。Agent Card（`agent_card.rs`）との整合を回復。
- **完了条件**: `cargo test -p aiome-node` に status roundtrip テスト追加・PASS。
- **依存**: 項目0

### G-2: samsara-hub への Gig 中継メッセージ

- **対象**: `libs/aiome-core-contracts/src/contracts.rs`（`HubMessage` に `GigRelay(GigRelayEnvelope)` 追加。アンカー: `ZeroMetadataCommuneRelay`）、`apps/samsara-hub/src/handlers/gig.rs`（新規）、`main.rs` ルート追加
- **変更**: `GigRelayEnvelope { from_node_id, to_node_id, payload_kind: "intent"|"bid"|"delivery"|"settlement", payload: serde_json::Value, signature_b64 }`。hub は**署名検証（Ed25519、`samsara-hub/src/auth.rs` の `verify_ed25519_signature` 使用）だけ行い、中身の解釈はしない**（中継のみ）。`POST /api/v1/gig/relay` → 対象ノードへ `state.tx.send`。無署名は 401（受け入れ基準4）。
- **完了条件**: hub ユニットテスト3本（正常中継 / 無署名 401 / 改ざん 401）PASS。
- **依存**: G-1

### G-3: aiome-node 側の受信ハンドラと GigEngine 結線

- **対象**: `apps/aiome-node/src/`（WS 購読 → `GigRelayEnvelope` を `UniversalGigEngine` の対応メソッドへディスパッチ）
- **変更**: `payload_kind` ごとに `publish_intent`（相手の発注を自ノードに登録）/`submit_bid`/`deliver`/`verify_and_settle` を呼ぶ。**すべての経済ミューテーション前に** (a) OXP 証明書検証（`oxilean.rs` の `verify`）、(b) 信頼スコア閾値チェック（G-4 の API）。閾値未満は自動拒否（受け入れ基準3）。
- **完了条件**: モック hub でのディスパッチテスト、閾値未満拒否テスト PASS。
- **依存**: G-2, G-4

### G-4: 信頼スコア参照 API（Phase 4 KarmaForge の開放）

- **対象**: samsara-hub `GET /api/v1/reputation/:node_id`（`node_reputation` テーブルは既存 — federation push_handler が更新済み）
- **変更**: reputation 値を返す読み取り専用 API（federation 認証必須）。`commercial/` の KarmaForge には**触らない**（`node_reputation` が samsara-hub 側の正本）。
- **完了条件**: 統合テスト2本 PASS。
- **依存**: G-2

### G-5: タイムアウト自動返金と2ノード E2E

- **対象**: `UniversalGigEngine` に期限超過スイープ（`gig_intents` の deadline 超過 `'Accepted'`/`'Delivered'` を `escrow_refund` → `'Cancelled'`。定期実行は `McpDiscoveryTask` の interval パターン踏襲）、`libs/infrastructure/tests/gig_federation_e2e.rs`（新規）
- **変更**: E2E: 2ノード＋hub のプロセス内テストで、発注→受託→納品→検収→エスクロー解放の全経路を Mock Commerce で完走（受け入れ基準1）。期限超過の自動返金（受け入れ基準2、**Negative Test**）。
- **完了条件**: E2E 2本 PASS、`cargo test --workspace` ベースライン同数以上。
- **依存**: G-3
- **注意**: `escrow_refund` は経済処理（Safety-Critical）。実装は既存 `verify_and_settle` の fail 経路（アンカー: `escrow_refund`）の呼び出し形を厳密に踏襲し、新しい金額計算を書かない。

### G-6: ドキュメント同期

- SECURITY_DESIGN.md（中継の署名検証・閾値拒否）、AIOME_NURTURE_SYNERGY.md（シーケンス図更新 — AGENTS.md ルール11）、CHANGELOG/OPEN/roadmap。
- **依存**: G-5

## F-9 やらないこと

1. **`commercial/` 配下と `routes/commerce.rs` の変更**（読み取り・呼び出しのみ。変更が必要になったら中断・報告）。
2. 未使用の `GigOrderStatus`（Bidding/InProgress/Verified/Disputed）の実装追加（紛争解決 Court は次期。短縮経路のまま接続する）。
3. 法定通貨建の自律決済は非対象（OP-011 は無償 KC マーケット S2S のみ ✅ 2026-07-22。有償チャージは凍結）。
4. hub にエスクロー・決済ロジックを置かない（hub は署名検証つき中継に徹する）。
5. 独自トークン・ブロックチェーンの導入。

---

## 実行者への指示文（このままコピペ）

```
docs/roadmaps/h3_f8_f10_implementation_plan.md を実行してください。
1. 実行順は F-10（V-1→V-3）→ F-8（C-1→C-5）→ F-9（G-1→G-6）。F-8 は OP-012 と F-1/F-2、F-9 は Phase 4・OP-011・F-3・F-4 の前提を確認し、未達なら該当 PART に着手せず報告する。
2. 1項目=1コミット。完了条件を満たしてからコミット。満たせなければ中断・報告。
3. F-9 は全項目 Safety-Critical。各コミット後に人間レビューを明示的に要求し、承認まで次へ進まない。
4. 「やらないこと」を先に読み全項目で遵守。アンカー文字列で対象を再特定。「踏襲」と指定された箇所は該当コードを読んでから書く。
5. 完了後、コミットハッシュ一覧・最終テスト結果・手動チェックリストの記録を報告。git push はしない。
```

## ロールバック

各項目独立コミット。revert は各 PART 内で逆順。`HubMessage` への variant 追加（G-2）は後方互換（未知 variant はデシリアライズ失敗で無視される設計かを実装時に確認し、異なれば中断・報告）。
