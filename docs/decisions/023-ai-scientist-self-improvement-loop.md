# ADR-023: AI-Scientist Self-Improvement Loop — 自律的スキル改良の仮説検証サイクル

> **Status**: Implemented  
> **Date**: 2026-03-26  
> **Origin**: SakanaAI/AI-Scientist パターンの Aiome への応用  
> **Impact**: DreamState / Oracle / SkillArena / TaskConductor  
> **Reference**: [SakanaAI/AI-Scientist](https://github.com/SakanaAI/AI-Scientist)

## Context

SakanaAI の AI-Scientist は、LLM が「仮説生成→実験→検証→論文執筆→自己レビュー」を完全自律で回すフレームワークである。このパターンの**エッセンス**（ソースコードそのものではなく設計思想）を取り入れることで、Aiome のエージェンティック AI がより高品質な自律行動を実現できる可能性がある。

### 解決すべき課題

1. **Dream State の出力品質**: 現在の `explorative_dream` / `reflective_dream` はランダムシード＋トレンド検索で Job を生成するが、実行後の成果物に対する**構造化された自己評価**が存在しない
2. **Oracle の単発性**: `Oracle::evaluate()` は SNS メトリクス依存の 1 回限りの評価。LLM による「多角的・反復的レビュー」がない
3. **スキル改善の非連続性**: `SkillArena` で比較対決は行われるが、「負けた理由を分析→改良版を自動生成→再対決」のループが閉じていない

### 安全性に関する制約

> [!CAUTION]
> AI-Scientist は Linux + NVIDIA GPU + Python 環境を前提とし、ライセンスは Responsible AI License 派生（商用利用に制約）。**ソースコードの直接利用・依存は行わない**。あくまで設計パターンのみを参照し、Rust + 既存 Aiome アーキテクチャ上に再実装する。

## Decision

### 1. Hypothesis-Experiment-Review ループの導入

AI-Scientist の核心ループを、`DreamState` の第4の夢モードとして追加する。

```
DreamState（既存）
├── explorative_dream    — トレンド探索夢
├── reflective_dream     — 過去の失敗への省察夢
├── communicative_dream  — Biome AI 間共鳴夢
└── 【新規】scientific_dream  — 仮説検証夢 (AI-Scientist パターン)
```

#### `scientific_dream` のフロー

```
1. 仮説生成 (Hypothesize)
   - 既存 Karma の中で「成功率の低いドメイン」を特定
   - LLM に「なぜこのドメインでの成功率が低いか」の仮説を生成させる
   - 仮説を構造化データ（HypothesisManifest）として保存

2. 実験設計 (Design)
   - 仮説に基づく「改善実験」を Job として生成
   - 既存 TaskConductor パターンで実行

3. 自己レビュー (Review) ← AI-Scientist パターン
   - 実行結果に対して Oracle にマルチレビューを依頼
   - 【新規】num_reflections パラメータで N 回の自己反省ループ
   - 【新規】レビュー結果を Karma に記録（学習記録の永続化）

4. 帰結判定 (Verdict)
   - レビュー結果が positive → スキルとして SkillForge に登録
   - レビュー結果が negative → 失敗原因を Karma に記録し、次回の仮説生成に反映
```

### 2. Oracle の Multi-Reflection 拡張

AI-Scientist の `perform_review` パターン（N 回の反復レビュー + アンサンブル）を Oracle に導入する。

```rust
/// Oracle の新メソッド（既存 evaluate() と共存）
impl Oracle {
    /// AI-Scientist スタイルのマルチレビュー
    pub async fn multi_review(
        &self,
        content: &str,
        context: &ReviewContext,
        config: ReviewConfig,
    ) -> Result<MultiReviewResult, AiomeError>;
}

struct ReviewConfig {
    /// 反省ループの回数（AI-Scientist の num_reflections に相当）
    num_reflections: u8,  // デフォルト: 3, 最大: 5
    /// 温度パラメータ（低いほど一貫性重視）
    temperature: f32,     // デフォルト: 0.1
}

struct MultiReviewResult {
    /// 1-10 スコア（AI-Scientist の Overall に相当）
    overall_score: f32,
    /// Accept / Reject / Revise
    decision: ReviewDecision,
    /// 各反省ラウンドの洞察
    reflections: Vec<String>,
    /// 弱点の構造化リスト
    weaknesses: Vec<String>,
    /// 強みの構造化リスト
    strengths: Vec<String>,
}
```

> [!IMPORTANT]
> **コスト制御**: `num_reflections` の最大値を 5 に制限する。各ループで LLM API を呼び出すため、`ResourceUsageLog` に自動記録し、`CommerceEngine::validate_activity` で予算チェックを挟む。

### 3. SkillForge へのフィードバックループ

```
SkillArena（対決）
    ↓ 敗北スキルの ID
scientific_dream（仮説生成）
    ↓ 改善仮説
TaskConductor（実験実行）
    ↓ 実験結果
Oracle::multi_review（自己レビュー）
    ↓ positive verdict
SkillForge（改良スキル登録）
    ↓
SkillArena（再対決）← ループが閉じる
```

### 4. 安全装置（ガードレール）

| ガードレール | 実装方針 |
|---|---|
| **LLM コスト暴走防止** | `num_reflections` 上限 5 + `ResourceUsageLog` 強制記録 + `validate_activity` 連動 |
| **無限ループ防止** | 同一仮説に対する実験回数の上限（`max_attempts_per_hypothesis: 3`） |
| **実行環境の隔離** | 実験 Job は既存の `RuntimeJail` (WASM Sandbox) 内で実行。ファイルシステムへの直接アクセスは不可 |
| **品質劣化の防止** | SkillForge への登録前に `SkillArena` での勝率 > 50% を必須条件化 |

## Proposed Changes

### DreamState

#### [MODIFY] [dream_state.rs](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/dream_state.rs)

- `scientific_dream` メソッドの追加
- `HypothesisManifest` 構造体の定義
- `dream()` のランダム分岐に `scientific_dream` を追加（Lv5 以上で出現）

---

### Oracle

#### [MODIFY] [oracle.rs](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/oracle.rs)

- `multi_review()` メソッドの追加
- `ReviewConfig` / `MultiReviewResult` / `ReviewDecision` 型の定義
- 既存 `evaluate()` はそのまま保持（後方互換）

---

### Contracts

#### [MODIFY] [contracts.rs](file:///Users/motista/Desktop/antigravity/aiome/libs/aiome-contracts/src/contracts.rs)

- `HypothesisManifest` / `ReviewDecision` を契約型として追加

## Open Questions

> [!WARNING]  
> 以下の項目は実装前に追加の設計判断が必要。

1. **仮説の永続化先**: `HypothesisManifest` を Karma テーブルの新カテゴリとして保存するか、専用テーブルを新設するか
2. **SkillArena 勝率の計算期間**: 直近 N 回の対決で判断するか、全期間の累計か

## Verification Plan

### Automated Tests

```bash
# DreamState の scientific_dream テスト
cargo test --package infrastructure -- dream_state::tests::test_scientific_dream

# Oracle の multi_review テスト
cargo test --package infrastructure -- oracle::tests::test_multi_review

# 統合テスト（仮説→実験→レビュー→SkillForge のフロー）
cargo test --package infrastructure -- tests::test_hypothesis_to_skill_loop
```

### Manual Verification

- Autonomous Demo を実行し、Lv5 以上の AI が `scientific_dream` を発動することを確認
- Watchtower ログで `multi_review` の reflections が記録されていることを目視確認

## Consequences

- **既存 API への影響**: `Oracle` に新メソッド追加のみで既存の `evaluate()` は不変。後方互換
- **パフォーマンス**: `multi_review` は `num_reflections` 回の LLM 呼び出しを行うが、Dream State でのみ発動するためリアルタイム制約なし
- **品質向上**: スキルの改善サイクルが自動化され、AI の出力品質が漸進的に向上する仕組みが整う
- **ライセンス**: AI-Scientist のソースコードは一切使用しない。設計パターンの参照のみであり、ライセンス上の制約は発生しない
