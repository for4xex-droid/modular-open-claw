# 進化的アーキテクチャ整合計画（v2.2）

> **作成**: 2026-07-22（v1.0→v2.1）→ **v2.2**（同日・第3次実コード検証）  
> **根拠**: brain `refactoring_value_analysis.md` × 実コード × OPEN/ADR 突合  
> **タスク正本**: [`OPEN.md`](../../OPEN.md)  
> **ステータス**: **Wave E（OP-090–093）実装完了**（2026-07-22）。Upstream/Human は別レーン継続  
> **検証軸**: Grok。探索委譲は Composer 2.5 のみ  
> **第3次検証**: 構造欠陥 **0**（F-1〜F3 PASS / Provider 本番0 / Registry 未配線 / reason_code 未導入 / OPEN 同期 OK）

---

## 0. 30 秒サマリ

| 項目 | v2.1 の固定 |
|------|-------------|
| 大規模リファクタ | **禁止** |
| 優先 | L0 Human → L1 Upstream α → L2 OP-068 → **L3 Wave E** |
| 機能性の核 | **CapabilityProvider 実装0を解消**し Tool 発見面に載せる＋拒否を機械可読に |
| 計測 | OP-090 = 薄い Harness（F-1〜F-4）。enforcer **再実装禁止** |

### 0.1 再検証スタンプ（2026-07-22）

| 主張 | 結果 |
|------|------|
| F-1〜F-3（shared/contracts/soul → infra 禁止） | **現行 PASS** |
| `CapabilityRegistry::new` 本番 | **0**（レジストリファイル外） |
| `impl CapabilityProvider` 本番 | **0**（テスト Mock のみ） |
| ToolDiscovery 配線 | `core_services` → **TaskDispatcher**（AppState フィールドではない） |
| Router | `apps/api-server/src/tool_call_router.rs`・`reason_code` **未導入**・人間向け BLOCK 文字列＋tracing 既存 |
| fitness スクリプト | **未作成** |
| `lib.rs` セクション見出し | **なし**（`pub mod` 97） |

---

## 0.2 v2.0 → v2.1

| # | 抜け | 修正 |
|---|------|------|
| 1 | 「エンジンに Provider を生やす」と読める | **アダプタ1個**（Tool カタログ→CapabilityProvider）を第一実装に固定 |
| 2 | ToolDiscovery を AppState 前提に読める | 注入先は **TaskDispatcher**（`core_services.rs` ≈740） |
| 3 | OP-093 が ImmuneAlert と二重化し得る | 既存 `Err("🚨 [… BLOCK]")` / tracing / `record_evolution_event` を**拡張**し、並列イベント経路は作らない |
| 4 | Provider 実装0が未記載 | §8 に明示（機能性ギャップの本体） |

---

## 1. v1.0 → v2.0 で潰した欠陥（実コード）

| # | v1.0 の問題 | 実コード事実 | v2.0 の修正 |
|---|-------------|--------------|------------|
| 1 | ToolCallRouter を `infrastructure` と記載 | **`apps/api-server/src/tool_call_router.rs`**（≈993 行） | アンカー修正 |
| 2 | CapabilityRegistry を「認可の土台」扱い | API は `register` / `get_capabilities_summary` / `get_capability_detail` のみ。**本番呼び出し 0**（自ファイルテストのみ） | Token 認可は見送り。**配線（Progressive Disclosure）を OP-092 に変更** |
| 3 | OP-090 が deep-scan 再実行を本丸に | deep-scan=CC-2/3/4/6・未ドキュメント。pattern-enforcer=AP。**依存方向・行数しきい値は未カバー** | F-1〜F-4 のみ新規。F-5 は既存スクリプト**委譲オプション** |
| 4 | 「84 modules」残骸リスク | `lib.rs` **213 行**・`pub mod` **≈97**。個別 `///` はあるが**セクション見出しなし** | 規模は実測。OP-091=セクション+ADR |
| 5 | F-4 が全 `.rs` ≥800 | Top は **tests**（dream_state/tests 1620 等）が占める | **本番コードと `#[cfg(test)]`/tests を分離報告** |
| 6 | CapabilityToken が Immune と二重化 | Router は既に `evaluate_security` + 課金 Fail-Closed（OP-075/024） | **新認可層は作らない**。観測性を OP-093 に分離 |

---

## 2. 優先順位ラダー（不変）

```
L0 Human / 凍結     OP-064, OP-040(動画), ADR-055 実行（明示時）
L1 Upstream Gate α  OP-030→031 / OP-033→034
L2 OP-068           deny ignore → 0
L3 Wave E           OP-090 → OP-091 → OP-092 → OP-093
L4 ポストリリース   value_10x / ADR-026 等（本計画外）
```

- OP-090/091: α 待ちと**並列可**（計測・文書）  
- OP-092/093: Upstream 実装 PR と**混在禁止**  
- α 到達時は L1 割り込み優先  

---

## 3. 突合（除外・重複禁止）

| 対象 | 扱い |
|------|------|
| actix / lunatic / 全面 ES・CQRS | **除外** |
| OP-050 skills 分割 / OP-075 Immune / OP-024 課金 FC | **完了済・再発明禁止** |
| ADR-031 JobQueue ISP | **延期維持**。091 で触らない |
| ADR-018 apps/watchtower | **廃案**（OP-052） |
| ADR-020 CapabilityProvider | **OP-092 が実装側を回収**（レジストリは既にある） |
| ADR-024 ベクトル索引 | **別レーン**（092 完了後に再評価） |
| Hexagonal / contracts 再設計 | **除外**（`CapabilityProvider` は contracts に既存） |
| DomainEvent 全面 | **除外** |

---

## 4. 機能性進化の目標像

現状の穴は「きれいさ」より **能力が繋がっていない**こと:

1. **Capability カタログが死んでいる** → エージェントが段階的に能力を発見できない（ADR-020 未完）  
2. **拒否がブラックボックス** → Immune/課金拒否の理由が運用・自己改善に回りにくい  
3. **アーキテクチャ適応度が測れない** → 改善の前後比較が感覚頼み  

v2 Wave E はこの3点を加算で塞ぐ。

---

## 5. 実コードアンカー

| 用途 | パス |
|------|------|
| Capability（未配線） | `libs/infrastructure/src/capability_registry.rs` |
| CapabilityProvider trait | `libs/aiome-core-contracts/src/traits.rs`（ADR-020） |
| Tool 発見 | `libs/infrastructure/src/skills/discovery.rs` + `apps/api-server/src/bootstrap/core_services.rs`（≈660） |
| Tool 実行 | `apps/api-server/src/tool_call_router.rs`（`evaluate_security` / `execute_skill`） |
| Infra 入口 | `libs/infrastructure/src/lib.rs` |
| 既存検査 | `scripts/deep-scan.sh` / `scripts/pattern-enforcer.sh` / `.aiome/anti-patterns.yml` |
| 影響分析 | `scripts/impact_query.py` |
| エージェント成長 | `libs/infrastructure/src/score_tracker.rs`（**システム fitness とは別**） |

### F-4 ベースライン実測（2026-07-22・本番寄り Top）

| 行数 | ファイル | 区分 |
|------|----------|------|
| 1597 | `infrastructure/.../workflow/mod.rs` | prod |
| 1330 | `society_of_thought.rs` | prod |
| 1101 | `lora_marketplace.rs` | prod |
| 1010 | `lora_training.rs` | prod |
| 1002 | `api-server/.../core_services.rs` | prod |
| 993 | `api-server/.../tool_call_router.rs` | prod |
| 962 | `context_engine.rs` | prod |
| 1620/1523 | `dream_state/tests.rs` / `job_queue/tests.rs` | **test（F-4 別枠）** |

---

## 6. OP-090 — Architecture Fitness Harness（薄い計測）

### 目的
**未カバーの適応度だけ**を測る。既存ツールのラッパー祭りにしない。

### 成果物
`scripts/architecture_fitness.py`（推奨）または `fitness_check.sh`  
出力: JSON/テキストレポート1つ。

### チェック

| ID | 内容 | 実装ヒント | 既存との差 |
|----|------|------------|------------|
| F-1 | `shared` が `infrastructure`/`api-server` を依存しない | 対象 `Cargo.toml` の dependencies キー検査 | deep-scan 非対象 |
| F-2 | `aiome-core-contracts` が `infrastructure` を依存しない | 同上 | 同上 |
| F-3 | `soul` が `infrastructure` を依存しない | ADR-003 | 同上 |
| F-4 | prod `.rs` 行数 Top N と ≥800 警告。tests は別セクション | `path` に `/tests` or `tests.rs` を分離 | **新規** |
| F-5 | （任意）`pattern-enforcer.sh` / `deep-scan.sh --ci` を**呼ぶだけ** | subprocess | 再実装禁止 |

### 禁止
- anti-patterns.yml の複製  
- impact_query / RIPPLE の再発明  
- score_tracker の流用（別ドメイン）  

### DoD
- [x] 現行 tree で F-1〜F-3 PASS  
- [x] Negative: 一時ツリーへ `infrastructure` 依存注入で F-1 FAIL → 復元（live `shared/Cargo.toml` 非破壊）  
- [x] F-4 が上表と矛盾しない（±ファイル移動は許容）  
- [x] 初回はローカル実行のみ。CI 必須化は明示許可後 → **2026-07-24** `architecture-fitness` job（unit + F-1..F-3。F-5 非委譲）

### 工数
1–2 日  

---

## 7. OP-091 — Infrastructure 論理境界（低コスト可視化）

### 目的
`lib.rs` を Bounded Context 見出しで読みやすくし、F-4 候補を ADR に固定。**物理分割しない**。

### 成果物
1. `lib.rs` に固定セクション（例）: Security / Economy / Soul-adapters / Skills / Observability / Cortex / Channels / Workflow-JobQueue / Platform  
2. ADR: `docs/decisions/0xx-infrastructure-logical-boundaries.md`（番号は実装時確定）  
3. 分割候補表 = F-4 prod Top（実装しない）  

### 禁止
クレート分割一括 / skills 再分割 / ADR-031 ISP / Commerce DomainEvent 全面  

### DoD
- [ ] `cargo check -p infrastructure` GREEN  
- [ ] ADR にセクション定義 + F-4 表  
- [ ] 新規 `pub mod` はセクション必須、を ADR にレビュー規約として記載  

### 工数
1–2 日（090 の直後）  

---

## 8. OP-092 — Capability Progressive Disclosure（機能性の本丸）

### 目的
**CapabilityProvider 実装0 / Registry 未配線を解消し、エージェントが Tool 能力を段階発見できる状態にする**（ADR-020）。

### 事実（設計拘束）
- 本番 `impl CapabilityProvider` は **Mock 以外ゼロ** → エンジンへ一括 impl は禁止（車輪・爆発）  
- `DefaultToolDiscoveryEngine` は **`TaskDispatcher::new(..., Some(tool_discovery), ...)`** に注入（AppState に無し）  
- Registry API は summary/detail のみ（認可ではない）  

### 第一実装（固定）
1. **`ToolCatalogCapabilityProvider`**（仮名）を `infrastructure` に追加  
   - `CapabilityProvider` を impl  
   - **カタログ取得は新ロジックを書かない**: 既存  
     `WasmSkillManager::list_skills` / `list_skills_with_metadata`（`skills/mod.rs`）を呼ぶ  
   - MCP 併記が必要なら既存 `DefaultToolDiscoveryEngine::discover_tools` の結果を**要約マージ**（再スキャン実装禁止）  
   - `capability_name` = `"tool_catalog"`  
   - schema/detail = メタデータ JSON（件数上限＋省略）  
2. `core_services.rs` で `CapabilityRegistry::new()` → `register(...)`  
   - `WasmSkillManager` は bootstrap / AppState と**同一インスタンス**を渡す  
3. 消費側（どちらか一方で可・両方は過剰）:  
   - **A**: TaskDispatcher に `Option<Arc<CapabilityRegistry>>` を足し、計画/ツール選定前に `get_capabilities_summary()`  
   - **B**: `discover_tools` 結果の先頭に summary を合成するヘルパ1関数  
4. 詳細は `get_capability_detail("tool_catalog")` のみ  

### 第2波（本計画の外・092 完了後に限り起票可）
- 追加 Provider（例: GenerativeEngine / Cortex）— **1 PR 1 Provider**  
- ADR-024 ベクトル索引 — 024 レーンのまま  

### 非目標
- CapabilityToken / 新認可  
- GenerativeEngine 等への Provider 乱立（第2波以降・別 OP）  
- ADR-024 ベクトル索引  

### DoD
- [ ] 起動後 Registry providers.len() ≥ 1  
- [ ] Positive: TaskDispatcher / discover 経路から summary が取得できる（単体 or 結合）  
- [ ] Negative: 未知名 detail = None、カタログ空でもパニックしない  
- [ ] `CapabilityRegistry::new` が bootstrap から呼ばれる（グラフ上 lib 外）  
- [ ] auth / commerce / Vault 非変更  

### 工数
3–5 日  

---

## 9. OP-093 — Tool 拒否の観測性（機能性の第二矢）

### 目的
拒否を**機械可読**にし、運用・自己改善が集計できるようにする。認可ルールは増やさない。

### 既存（再利用・二重化禁止）
`evaluate_security` は既に:
- `🚨 [GUARDRAIL BLOCK]` / `[SENTINEL BLOCK]` / `[SECURITY BLOCK]` 文字列  
- `tracing::warn!` / `error!`  
- Sentinel 時 `record_evolution_event(..., "ImmuneAlert", ...)`  

### 成果物
1. 拒否分岐に **`reason_code`** を付与（例: `guardrail` / `sentinel` / `immune_db_error` / `mcp_suspended` / `moe_culling`）  
2. `tracing` に `reason_code=%code` を追加（人間向けメッセージは維持可）  
3. 既存 `ImmuneAlert` を置き換えない。必要なら event payload に code を**足すだけ**  
4. テスト: `test_tool_call_router_immune_db_error_fail_closed` 等で code 安定を断言  

### 禁止
- 新 Token / 新 Immune エンジン  
- 別チャンネルへの二重 audit  
- プロンプト全文のログ  

### DoD
- [ ] Positive: immune_db_error で `reason_code=immune_db_error`  
- [ ] Negative: `Ok(())` 経路で拒否 code が出ない  
- [ ] 既存 Fail-Closed テストが GREEN  

### 工数
2–3 日（092 後推奨）  

---

## 10. 実行順

| 順 | ID | 並列 |
|----|-----|------|
| 1 | OP-090 | α 待ちと可 |
| 2 | OP-091 | 090 直後 |
| 3 | OP-092 | Upstream PR と分離 |
| 4 | OP-093 | 092 後推奨 |

---

## 11. Verification Protocol（各 OP）

1. Positive 2. Negative（注入→検知→**復元**） 3. OPEN/CHANGELOG/RIPPLE  

---

## 12. 改訂ルール

- 数値は実測（本計画 §5）。脳内「84」禁止  
- Upstream 専用 roadmap を増やさない  
- 「大規模リファクタ」提案は §3 で除外判定してから起票  
