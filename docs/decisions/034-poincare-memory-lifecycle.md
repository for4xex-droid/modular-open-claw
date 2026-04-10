# ADR-034: Poincare-based Autonomous Memory Lifecycle Management

## 📜 Status
Proposed / Accepted (Phase 4 Finalized)

## 🎯 Context & Problem Statement
Aiome のエージェントが生成する Karma（記憶）は、長期間の活動に伴い膨大な量に達し、LLM のコンテキスト制限や検索ノイズの原因となる。従来の単純な時間経過による減衰（Karma Decay）だけでは、「古くても重要な記憶」と「新しくても無価値な記憶」を見分けることができず、必要な情報の欠落や不要な情報の維持が発生していた。

## 💡 Proposed Solution
`slm` (SuperLocalMemory) の幾何学的推論機能（Poincare 分類）を活用し、記憶の「重要度（Importance）」を多角的に評価する自律的ガベージコレクション（GC）を導入する。

### 1. Poincare 重要度算出
`SlmBridge` に `calculate_importance` を実装し、以下のチャネルスコアを統合して 0.0〜1.0 の重要度を算出する：
- **Poincare Score**: 幾何学的配置に基づく概念的中心性。
- **Density**: 周辺記憶との密度。
- **Weighted Importance**: 設定可能な重み付けによる最終統合。

### 2. Watchtower 自律 GC
バックグラウンドジョブ `watchtower` の `do_karma_decay_sweep` において、従来の減衰処理の直後に Poincare GC を実行する：
- 閾値（0.3）未満の記憶を自動的にアーカイブ。
- 質的に低い記憶を動的に排除し、推論の精度と鮮度を維持。

### 3. NAPI Bridge による露出
フロントエンド UI からも重要度を可視化・利用できるよう、NAPI 経由で `karma_geodesic_importance` を提供する。

## ✅ Consequences
- **Positive**: 
  - 記憶の質に基づく動的なライフサイクル管理が可能になった。
  - LLM へのインジェクションノイズが低減し、推論効率が向上。
  - フロントエンドにおける「記憶の価値」の可視化基盤が確立。
- **Neutral**:
  - `slm trace` コマンドのオーバーヘッド（サブプロセス起動）が発生するが、バックグラウンドでのバッチ処理により許容。

## 🛠️ Verification
- `test_sqlite_job_queue_karma_decay_sweep_poincare` による自律 GC の正常動作確認。
- `napi-bridge` におけるシングルトン注入と露出の検証。
