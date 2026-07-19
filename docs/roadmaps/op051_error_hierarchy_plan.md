# OP-051 Error Hierarchy Implementation Plan（v1.0）

- **ステータス**: **P1–P4 ✅ 2026-07-20**（OP-051 完了）
- **正本 ID**: OPEN **OP-051**
- **Decision**: [`docs/decisions/054-error-hierarchy.md`](../decisions/054-error-hierarchy.md) **Accepted 2026-07-20**
- **継承**: [`error_handling.md`](../architecture/error_handling.md) §2–§3、[`tech_debt_top5_plan.md`](tech_debt_top5_plan.md) Wave C

## 0. レーン分離

| レーン | 内容 |
|---|---|
| **実行対象** | 段階的 `From` map、契約トレイトの `AiomeError` 化、境界 anyhow の選択的 map |
| **禁止** | `anyhow` 一括置換、新規 public error enum、Safety-Critical エラー面変更 |
| **延期** | NurtureError core `From`（commercial 手動 map 維持）、OP-083-C/D・OP-011 |

## 1. 3 階層（ADR-054 要約）

| Layer | 役割 | 正本型 |
|---|---|---|
| **1 Domain / Boundary** | API・クレート境界。HTTP-safe（CWE-209） | `AiomeError`（`libs/aiome-contracts/src/error.rs`） |
| **2 Subsystem** | ドメイン固有。境界で `From` | Soul / X402 / Csam / Nurture / …（新規型禁止） |
| **3 Internal** | モジュール内部 | `anyhow` / ad-hoc → 境界前に Layer 1 へ |

## 2. 現状ギャップ（2026-07-20）

| 項目 | 状態 |
|---|---|
| `From` 済み | Soul / X402 / Csam / Process / Proportion / Loader / BudgetExhausted |
| `From` 未整備 | **NurtureError**（commercial 手動 map・延期） |
| `From` 追加済（P2） | **FactoryResetError** → `AiomeError::Infrastructure`（AppError は委譲） |
| 契約トレイト `anyhow::Result` | **P1 で解消**（上記 4 トレイト → `AiomeError`） |
| 境界 anyhow（選択） | **P4**: `AppError::internal` 不透明 + `QuarantineStore` → `AiomeError`。Layer 3 内部 anyhow は維持 |

## 3. フェーズ（crate 単位・上から実行）

| Phase | 対象 | 作業 | 検証 | 着手ゲート |
|---|---|---|---|---|
| **P1** | ✅ `aiome-contracts` / `aiome-core-contracts` | `Ekyc*` / `AuditLogger` / `X402Negotiator` → `Result<_, AiomeError>`。impl: commerce ekyc/store/x402、infrastructure audit、gift mock | contracts + commerce 70 PASS + audit_logger 2 PASS + `cargo check -p api-server` | 「OP-051 P1 を実装しろ」 ✅ |
| **P2** | ✅ `FactoryResetError` | `shared::bootstrap_detector` に `From` → `AiomeError`。`AppError` は `Self(err.into())` に委譲 | shared bootstrap 12 PASS + `error::tests::test_from_factory_reset_error` PASS | 「OP-051 P2 を実装しろ」 ✅ |
| **P3** | ✅ soul / avatar-engine / shared CSAM / security_zombie | `SoulError` の AppError 並列 map を廃止し crate `From` に委譲。`InvalidTransition` → `Validation`。Loader/Proportion/Process/Csam は境界テスト追加 | soul/avatar/shared/infra unit + `test_from_{soul,loader,proportion,process,csam}_error` PASS | 「OP-051 P3 を実装しろ」 ✅ |
| **P4** | ✅ infrastructure / api-server（公開境界のみ） | `AppError::internal` 不透明化。`QuarantineStore` → `AiomeError`。audit/cortex/expression/skill/avatar の境界リークを選択修正。BanStore/auth/webhook/Vault は触らない | quarantine 2 PASS + `test_internal_opaque_does_not_leak_db_details` PASS + `cargo check -p api-server` | 「OP-051 P4 を実装しろ」 ✅ |

## 4. やらないこと

| 禁止 | 理由 |
|---|---|
| 機械的 `anyhow` → `AiomeError` 全置換 | ADR-054 Rule 4 |
| auth / Stripe commerce 本体 / Vault・key-proxy / webhook / Tauri shell のエラー面 | Safety-Critical Zone |
| NurtureError の core `From`（P1–P4 外） | commercial 別承認 |
| skills-module 大規模 refactor を本 ADR に同乗 | ADR Out of scope |

## 5. 成功基準（全フェーズ完了時）

1. 公開契約トレイトに `anyhow::Result` が残っていない
2. api-server 境界で subsystem エラーが `AiomeError` に統一 map される
3. `cargo test --workspace` PASS
4. Negative: 内部 DB/通信詳細が HTTP レスポンスに露出しない

## 6. 台帳

- タスク ID: [`OPEN.md`](../../OPEN.md) OP-051
- Wave C: [`agentic_production_hardening_plan.md`](agentic_production_hardening_plan.md) C1
