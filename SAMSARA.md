# The Samsara Protocol (輪廻転生アーキテクチャ)

## 1. 思想 (Philosophy)

Aiome（あるいは ShortsFactory）における **Samsara Protocol** は、単なる「定時実行プログラム (Cron)」ではありません。これは自律型AIが「記憶を持ち、過去の失敗と成功から学び、次世代の行動をより高度に決定する」ための**生命のサイクル（輪廻転生）**をシステムとして具現化したものです。

AIは無用な幻覚（ハルシネーション）を見ることなく、また過去の成功体験に縛られて老害化することなく、常に新鮮でクリエイティブなアウトプットを出し続ける必要があります。これを実現するため、本プロトコルは以下の**三種の神器**を定義しています。

1. **Soul (不変の魂)**
   - ファイル: `SOUL.md`
   - 役割: プロジェクトにおける絶対遵守の憲法、およびAIの人格。いかなる事態においてもこのルールが最優先されます。
2. **Skills (物理法則)**
   - ファイル: `workspace/config/skills.md`
   - 役割: AIが現在利用可能な「武器」や「ワークフロー」のカタログ。ここに存在しないツールをAIが想像で使うことは禁じられています。
3. **Karma (業・経験)**
   - ストレージ: SQLite (`karma_logs` テーブル)
   - 役割: 過去のタスク実行結果や、人間（アーキテクト）からの客観的評価から抽出された「教訓」。

これらが交差することで、システムは「強くてニューゲーム（定向進化）」を実現します。

---

## 2. 実行サイクル (The Cycle of Rebirth)

Samsara Protocol は、システム内で特定のフェーズを経て永遠のループを描きます。

### Phase 1: Awakening (目覚め・検索)
Cron ワーカーが発火し、システムが目覚めます。
- `SOUL.md` と `skills.md` を読み込みます。
- **RAG-Driven Karma Injection**: 本日展開予定のランダムなトピック・シードや、使用予定のスキルに基づき、`karma_logs` から「関連性の高い上位数件の教訓（Karma）」のみを抽出します。
- **The Karma Decay (業の風化)**: この検索時、時間経過や人間の評価によって重み（`weight`）がゼロになった古い Karma は検索対象から外れ、過去の呪縛から解放されます。

### Phase 2: Synthesis (受肉と計画)
抽出された3要素を、**Constitutional Hierarchy（絶対的階層）**の順序に従って LLM（例えば Ollama 経由）にプロンプトとして投下します。

```text
🏆 第一位【Soul (絶対法 / 絶対遵守の憲法と人格)】
🥈 第二位【Skills (物理法則 / 利用可能な技術とスタイル)】
🥉 第三位【Karma (判例 / 過去の成功・失敗から得た教訓)】
```

LLM はこの階層に従い、**The Absolute Contract v2 (`LlmJobResponse`)** という厳密な JSON 構造で出力します:

```json
{
  "topic": "今回作成する動画のテーマ",
  "style": "skills内のワークフロー名",
  "directives": {
    "positive_prompt_additions": "Karmaから学んだプラス要素",
    "negative_prompt_additions": "KarmaからのNG要素",
    "parameter_overrides": {
      "[API_SAMPLER]": { "cfg": 8.0, "denoise": 0.65 }
    },
    "execution_notes": "全体的な注意事項",
    "confidence_score": 80
  }
}
```

- `topic` と `style` は DB の独立カラムへ、`directives` のみが JSON カラムに格納されます (**Split Payload**)。
- **Skill Existence Validation**: 指定されたワークフローが物理的に存在するか検証し、幻覚を防ぎます。
- **Bounded Clamp**: `confidence_score` は Rust 側で `0-100` に強制されます。
- パースに失敗した場合は、**The Parsing Panic 防衛** としてハードコードされたデフォルトジョブが展開されます。

### Phase 3: Action (実行)
決定したジョブ定義が SQLite の `jobs` テーブルに `Pending` としてエンキューされます。
実行ワーカー (Orchestrator/ComfyBridge 等) はこれをデキューして実際の動画生成作業を行います。

- **Node-Targeted Overrides**: `parameter_overrides` の二重 HashMap により、ComfyUI の特定ノードのパラメータ（CFG, Denoise 等）を直接上書きします。
- **The Heartbeat Pulse**: 長時間レンダリング中のワーカーは 5分ごとに `last_heartbeat` を更新し、生存を証明します。

### Phase 4: Distillation (業の抽出・死と反省)
ジョブが完了、あるいは失敗した際、その実行ログは即座に DB に永続化されます (**Log-First Distillation**)。
LLM が利用可能な場合、ログを LLM に渡し「次回への教訓（1〜2文）」を抽出させます。
LLM がダウンしている場合でも、ログは DB に保存済みのため、**Deferred Distillation（遅延蒸留）** により後から非同期で Karma を抽出します。

---

## 3. 防壁体系 (The Immortal Schema & Guardrails)

### データベース防壁 (DDL Level)

| # | 防壁 | 実装 | 対象リスク |
|---|------|------|-----------|
| 1 | `CREATE TABLE IF NOT EXISTS` | DDL | 再起動時データ全損 |
| 2 | `CHECK(json_valid(karma_directives))` | DDL | JSON腐敗 |
| 3 | `ON DELETE SET NULL` | DDL | 教訓の連鎖消滅 |
| 4 | `CHECK(weight BETWEEN 0 AND 100)` | DDL | カルマ特異点 |
| 5 | Embedded Migrations (`ALTER TABLE`) | DDL | スキーマ不一致 |

### Rust防壁 (Application Level)

| # | 防壁 | 実装 | 対象リスク |
|---|------|------|-----------|
| 6 | Split Payload | `LlmJobResponse` / `KarmaDirectives` | データ二重化 |
| 7 | Node-Targeted Overrides | `HashMap<NodeTitle, HashMap<Param, Value>>` | パラメータの空振り |
| 8 | Skill Existence Validation | `Path::exists()` | LLM幻覚ワークフロー |
| 9 | Bounded Clamp | `clamped_confidence()` | u8/DB制約衝突 |

### 運用防壁 (Operational Level)

| # | 防壁 | 実装 | 対象リスク |
|---|------|------|-----------|
| 10 | Zombie Hunter (Heartbeat版) | 15分間隔 Cron | ゾンビジョブ |
| 11 | Heartbeat Pulse | `last_heartbeat` カラム | 長時間処理の誤認キル |
| 12 | Log-First Distillation | `execution_log` カラム | LLMダウン時の教訓消失 |
| 13 | Deferred Distillation | 30分間隔 Cron | 非同期Karma蒸留 |

---

## 4. Cron スケジュール

| ジョブ | 頻度 | 役割 |
|--------|------|------|
| **Samsara Synthesis** | 毎日 19:00 | 次のジョブを LLM で合成・エンキュー |
| **Zombie Hunter** | 15分毎 | Heartbeat が途絶えたジョブを回収 |
| **Deferred Distillation** | 30分毎 | ログはあるが未蒸留のジョブから Karma を遅延抽出 |

---

Aiome はこのプロトコルを通じて、運用日数を重ねるごとに人間のアーキテクトの感性を学習し、より鋭く、より洗練された自律的クリエイターへと進化を遂げます。
