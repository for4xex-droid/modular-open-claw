# OP-099 コードレビュー指摘 修正計画書（Fix Plan）

- **Status**: 実装完了（2026-08-04）
- **Source**: OP-099 Intelligent LLM Router / OP-096-097 Egress Defense の `/code-review` 指摘事項
- **原則**: 本書は「読めばそのまま実装できる」レベルの仕様書である。現行コードの引用行番号は 2026-08-04 時点。実装時に数行ずれている可能性があるため、**引用コードの内容一致**を正とすること。
- **検証**: 各 FIX に検証コマンドを明記。最終的に `cargo fmt --check && cargo check --workspace --tests && cargo test --workspace` が PASS すること。

---

## 0. 修正一覧とフェーズ

| FIX | レビューID | 優先度 | 対象ファイル | 内容 |
|---|---|---|---|---|
| FIX-1 | C-1 | Critical | `humanizer_filter.rs` | `complete_with_cache` override 追加（format/metadata 落ち解消） |
| FIX-2 | C-2 | Critical | `background.rs`, `llm_providers.rs` | `pin_local` フラグで local provider のクラウド昇格を遮断 |
| FIX-6 | C-3 | Critical | `core_services.rs` | Caching を EntropyGate 外側へ移動 + rules モード限定 DI |
| FIX-3 | H-1 | High | `dynamic.rs`, `background.rs` | route_* ログを request.metadata 優先に |
| FIX-7 | H-2 | High | `utils.rs`, `semantic_cache.rs`, `caching_provider.rs` | キャッシュキー衝突・漏洩対策（framing + 全リクエストキー + セマンティック照合停止） |
| FIX-8 | M-1 | Medium | `route_rules.rs` | 外部 `route_tier` metadata の信頼を廃止 |
| FIX-4 | M-3 | Medium | `config.rs` | `LLM_ROUTE_BUDGET_DEGRADE` パース堅牢化 |
| FIX-9 | H-3 | High | `bastion_guard.rs` | `network_target_host` の Fail-Closed 化 |
| FIX-10 | H-4+M-4 | High | `aiome-contracts/src/security.rs` | `host_permitted` 制御文字拒否 + 大小/FQDN 正規化 |
| FIX-11 | M-2+M-5 | Medium | migration SQL, ADR-057 | 注記追加のみ（コード変更なし） |
| FIX-5 | — | 検証 | 新規 `llm_chat_stack_test.rs` | チャット DI スタック結合テスト |

**実施順序（依存関係順）:**

1. **Phase R1（Critical・チャット経路）**: FIX-1 → FIX-6 → FIX-2
2. **Phase R2（High・キャッシュ/観測）**: FIX-7 → FIX-3
3. **Phase R3（Medium・ルータ堅牢化）**: FIX-8, FIX-4
4. **Phase R4（Egress 硬化・独立実施可 = OP-100 として切り出し可）**: FIX-9, FIX-10, FIX-11
5. **Phase R5（結合テスト + ドキュメント同期）**: FIX-5 → §7 ドキュメント同期

FIX-7 は FIX-6 に依存（`embedding_provider=None` 前提）。FIX-5 は FIX-1/6/7 完了後に書くこと。

---

## 1. Phase R1: Critical（チャット経路）

### FIX-1 (C-1): HumanizerFilter に `complete_with_cache` を追加

**問題**: `HumanizerFilter` が `complete_with_cache` を override していないため、トレイト既定実装（`libs/aiome-contracts/src/llm.rs` L114-125）が `self.complete(prompt, system)` へフォールバックし、チャット経路で `format` / `metadata` が内側チェーン（EntropyGate/IR）に届かない。IR のルーティングと sticky tier が事実上無効化される。

**ファイル**: `libs/infrastructure/src/llm/humanizer_filter.rs`

**手順 1**: L11 の import を変更。

```rust
// 変更前
use aiome_core::llm_provider::{LlmProvider, LlmResponse};
// 変更後
use aiome_core::llm_provider::{LlmProvider, LlmRequest, LlmResponse};
```

**手順 2**: `impl LlmProvider for HumanizerFilter` 内、既存 `complete`（L113-133）の直後・`stream_complete` の前に追加。

```rust
    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let mut response = self.inner.complete_with_cache(request).await?;

        let original_len = response.content.len();
        response.content = self.apply_rules(&response.content);
        let new_len = response.content.len();

        if original_len != new_len {
            info!(
                "📝 [HumanizerFilter] Applied AI-writing filters. Length: {} -> {}",
                original_len, new_len
            );
        }

        Ok(response)
    }
```

**注意**:
- `content` のみ変更。`metadata` / `logprobs` / `reasoning` / `stop_reason` には触らない。
- JSON らしき content のスキップは既存の `apply_rules` 内 `is_likely_json` がそのまま効く。

**テスト（同ファイル `mod tests` に追加）**:
- Positive: format=Some("json") 付き `LlmRequest` を `complete_with_cache` に渡し、inner（Mock）に format が届くこと（inner を format 記録用のカスタム mock にする。`fallback_router.rs` の `test_complete_with_cache_propagates_format_to_primary` の mock が手本）。
- Positive: content にフィルタ対象語を含む応答が変換されること。

**検証**: `cargo test -p infrastructure --lib humanizer_filter`

---

### FIX-6 (C-3): CachingLlmProvider を EntropyGate 外側へ + rules モード限定 DI

**問題**: 現行チェーン `HF → EG → Caching → IR` では (a) EG 検証**前**の応答がキャッシュに書かれ、EG リトライ中もヒットしてしまう（低品質応答のキャッシュ汚染）、(b) legacy モードでもキャッシュ層が挿入され従来挙動と非等価。

**設計判断（確定）**: 新チェーンは `HF → [Caching (rules のみ)] → EG → IR`。キャッシュには **EG 検証済み応答のみ**が書かれる。キャッシュヒット時は EG をスキップするが、書き込み時に検証済みのため許容（本判断を ADR-058 に追記、§7 参照）。

**ファイル**: `apps/api-server/src/bootstrap/core_services.rs` L387-426

**現行コード（L400-426）**:

```rust
    let semantic_cache = Arc::new(infrastructure::llm::semantic_cache::SemanticCache::new(
        Arc::new(
            infrastructure::llm::semantic_cache::SqlSemanticCacheRepository::new(db_pool.clone()),
        ),
        Some(embed_provider.clone()),
    ));

    let caching_base = Arc::new(
        infrastructure::llm::caching_provider::CachingLlmProvider::new(
            intelligent_base,
            semantic_cache,
            3600,
        ),
    ) as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;

    let entropy_gate_provider = Arc::new(infrastructure::llm::entropy_gate::EntropyGate::new(
        caching_base,
        2.0,
        3,
    ))
        as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;

    let router_provider = Arc::new(infrastructure::llm::humanizer_filter::HumanizerFilter::new(
        entropy_gate_provider,
        infrastructure::llm::humanizer_rules::default_rules_ja(),
        infrastructure::llm::writing_context::WritingContext::Default,
    )) as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;
```

**置換後**（`intelligent_base`（L387-398）は現状維持。L400-426 を以下に置換）:

```rust
    // OP-099 fix (C-3): Caching は EntropyGate の外側・rules モード限定。
    // キャッシュには EG 検証済み応答のみが書かれる（ADR-058 追記参照）。
    let entropy_gate_provider = Arc::new(infrastructure::llm::entropy_gate::EntropyGate::new(
        intelligent_base,
        2.0,
        3,
    ))
        as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;

    let chat_core: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync> =
        if config.llm_route_mode == shared::config::LlmRouteMode::Rules {
            let semantic_cache = Arc::new(infrastructure::llm::semantic_cache::SemanticCache::new(
                Arc::new(
                    infrastructure::llm::semantic_cache::SqlSemanticCacheRepository::new(
                        db_pool.clone(),
                    ),
                ),
                None, // FIX-7: セマンティック照合無効・完全一致キーのみ
            ));
            Arc::new(
                infrastructure::llm::caching_provider::CachingLlmProvider::new(
                    entropy_gate_provider,
                    semantic_cache,
                    3600,
                ),
            ) as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>
        } else {
            entropy_gate_provider
        };

    let router_provider = Arc::new(infrastructure::llm::humanizer_filter::HumanizerFilter::new(
        chat_core,
        infrastructure::llm::humanizer_rules::default_rules_ja(),
        infrastructure::llm::writing_context::WritingContext::Default,
    )) as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;
```

**注意**:
- `embed_provider` がこの箇所で未使用になる。同関数内の他利用箇所を確認し、完全未使用になる場合のみ該当 clone を削除（Preserve Intent 原則。`embed_provider` 自体は他サービスで使用されているはず）。
- `caching_provider.rs` L20-21 の doc コメント「EntropyGate 内側」を「EntropyGate 外側・HumanizerFilter 内側」に更新。
- legacy モードではキャッシュ層が完全に消え、OP-099 以前と等価なチェーンに戻る。

**検証**: `cargo check -p api-server` + FIX-5 の T3/T4。

---

### FIX-2 (C-2): BackgroundLlmProvider に `pin_local` を追加しクラウド昇格を遮断

**問題**: `build_local_background_provider` で構築される「ローカル」provider も、DB 設定 `bg_llm_provider` が gemini/openai 等ならクラウドに接続する。特に `enforce_cost_limit: false` の budget-degrade 用インスタンスがクラウド化すると**コスト制限なしの課金 API 呼び出し**になる。

**ファイル**: `libs/infrastructure/src/llm/background.rs`, `apps/api-server/src/bootstrap/llm_providers.rs`

`BackgroundLlmProvider` の struct リテラルは全リポジトリで **2 箇所のみ**（`llm_providers.rs` L19, L55。grep で再確認済み）。

**手順 1**: `background.rs` の struct（L19-42）末尾、`enforce_cost_limit` の直後にフィールド追加。

```rust
    /// When true, ignore settings/env provider selection and always use local Ollama
    /// (`fallback_host` + `fallback_model`). Blocks cloud promotion on this instance.
    pub pin_local: bool,
```

**手順 2**: provider 解決 3 箇所すべてに `pin_local` 分岐を追加。

対象:
| # | メソッド | 位置 |
|---|---|---|
| 1 | `complete` | L91-161（`get_setting_value("bg_llm_provider")` から match まで） |
| 2 | `complete_with_cache` | L245-316（同型） |
| 3 | `EmbeddingProvider::embed` の `_` 腕 | L406-421 |

**箇所 1・2 の変更方針**（latency 計測を壊さない形。既存の settings 読み〜match ブロック全体を `else` に入れる）:

```rust
        let start_time = std::time::Instant::now();

        let (provider_type, model, res) = if self.pin_local {
            // settings/env/API キーを一切読まない（resolve_bg_api_key 非呼び出し）
            let model = self.fallback_model.clone();
            let provider = aiome_core::llm_provider::OllamaProvider::new(
                self.fallback_host.clone(),
                model.clone(),
            );
            ("ollama".to_string(), model, provider.complete(prompt, system).await)
        } else {
            // ★ 既存 L91-161 の settings 読み・resolve_bg_api_key・provider match を
            //   そのままここへ移動し、(provider_type, model, res) を返す形に整形
        };
```

箇所 2 は `provider.complete(prompt, system)` の代わりに `provider.complete_with_cache(request.clone())` を呼ぶ（既存コードの呼び方に合わせる）。**既存の `start_time` / `log_provider` / `log_model` / Post Hooks / `log_evaluation` の流れは変えない**こと（変数束縛位置の調整のみ）。

**箇所 3（embed `_` 腕）の置換**:

```rust
            _ => {
                let (host, model) = if self.pin_local {
                    (self.fallback_host.clone(), self.fallback_model.clone())
                } else {
                    let host = self
                        .ops
                        .get_setting_value("ollama_host")
                        .await?
                        .unwrap_or_else(|| self.fallback_host.clone());
                    let model = self
                        .ops
                        .get_setting_value("bg_llm_model")
                        .await?
                        .or_else(|| std::env::var("BG_LLM_MODEL").ok())
                        .unwrap_or_else(|| self.fallback_model.clone());
                    (host, model)
                };
                aiome_core::llm_provider::OllamaProvider::new(host, model)
                    .embed(text, is_query)
                    .await
            }
```

**手順 3**: `llm_providers.rs` の 2 リテラルにフィールド追加。

- `build_local_background_provider`（L19-31）: `pin_local: true,`
- `bg_instance`（L55-67）: `pin_local: false,`

**注意**:
- `resolve_bg_api_key` は `pin_local` 経路では**絶対に呼ばない**（else 内に閉じ込める）。
- `gemini_embed_fallback`（L453 以降、`EMBEDDING_PROVIDER=ruri/gemini` 経路）は本 FIX の対象外。
- `Debug` impl は `finish_non_exhaustive()` のため追加不要。

**テスト（`background.rs` の `mod tests`、なければ新設）**:
- **Negative（必須）**: `pin_local: true` + `get_setting_value("bg_llm_provider")` が `"gemini"` を返す mock ops で `complete` を呼び、Ollama 経路（= `fallback_host` への接続試行エラー等）になること。クラウド provider の生成に入らないことを検証する（mock ops 側で `bg_llm_provider` 読み取りが発生しないことを記録するのが最も確実）。
- Positive: `pin_local: false` では従来どおり settings が読まれること。

**検証**: `cargo test -p infrastructure --lib background`

---

## 2. Phase R2: High（キャッシュ / 観測）

### FIX-7 (H-2): キャッシュキーの衝突・漏洩対策

**問題**: (a) `compute_prompt_hash` が区切りなし連結のため `("ab", "c")` と `("a", "bc")` が衝突。(b) キャッシュキーが最終 user メッセージ + system のみで、会話履歴・temperature・format を無視 → 別文脈の応答が混線。(c) セマンティック（embedding）照合により意味的に近い別会話の応答が返る漏洩リスク。

**設計判断（確定）**:
- ハッシュは長さプレフィクス framing に変更。
- チャットキャッシュのキーは **`LlmRequest` 全体**（全 messages + format + temperature + max_tokens）から算出。`metadata` は **キーに含めない**（route_* が入るため EG リトライで不安定化する）。`stop_sequences` / `LlmMessage.cache` もキー外。
- セマンティック照合は当面**無効**（FIX-6 で DI を `None` に）。スコープ付きセマンティック照合は将来 OP。
- 旧ハッシュ値とは不連続になる（既存キャッシュのウォーム分は失効、eval ログの `prompt_hash` も新旧不連続）。**許容**とし CHANGELOG に明記。

#### FIX-7-A: `libs/infrastructure/src/llm/utils.rs`

**現行（L11-19）**:

```rust
pub fn compute_prompt_hash(prompt: &str, system: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    if let Some(sys) = system {
        hasher.update(sys.as_bytes());
    }
    hex::encode(hasher.finalize())
}
```

**置換 + 追加**（import に `use aiome_core_contracts::llm::LlmRequest;` を追加。既存 re-export 経由 `aiome_core::llm_provider::LlmRequest` でも可、ファイル内の既存スタイルに合わせる）:

```rust
/// Prompt + optional system の SHA-256（長さプレフィクス framing）。
/// eval logger / SemanticCache::get|set 共通。
pub fn compute_prompt_hash(prompt: &str, system: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    update_framed(&mut hasher, prompt);
    if let Some(sys) = system {
        update_framed(&mut hasher, sys);
    }
    hex::encode(hasher.finalize())
}

const CACHE_KEY_SOME_TAG: u8 = 0x00;
const CACHE_KEY_NONE_TAG: u8 = 0xFF;

fn update_framed(hasher: &mut Sha256, part: &str) {
    hasher.update((part.len() as u64).to_le_bytes());
    hasher.update(part.as_bytes());
}

fn update_opt_str(hasher: &mut Sha256, value: Option<&str>) {
    match value {
        Some(s) => {
            hasher.update([CACHE_KEY_SOME_TAG]);
            update_framed(hasher, s);
        }
        None => hasher.update([CACHE_KEY_NONE_TAG]),
    }
}

/// LlmRequest 全体（metadata / stop_sequences 除外）からチャットキャッシュキーを算出。
pub fn compute_request_cache_key(request: &LlmRequest) -> String {
    let mut hasher = Sha256::new();
    for m in &request.messages {
        update_framed(&mut hasher, &m.role);
        update_framed(&mut hasher, &m.content);
    }
    update_opt_str(&mut hasher, request.format.as_deref());
    match request.temperature {
        // 注意: LlmRequest.temperature は f32（f64 ではない）
        Some(t) => {
            hasher.update([CACHE_KEY_SOME_TAG]);
            hasher.update(t.to_bits().to_le_bytes());
        }
        None => hasher.update([CACHE_KEY_NONE_TAG]),
    }
    match request.max_tokens {
        Some(v) => {
            hasher.update([CACHE_KEY_SOME_TAG]);
            hasher.update(v.to_le_bytes());
        }
        None => hasher.update([CACHE_KEY_NONE_TAG]),
    }
    hex::encode(hasher.finalize())
}
```

※ `max_tokens` の実型（`Option<u32>` / `Option<i32>` 等）は `libs/aiome-contracts/src/llm.rs` L68-81 を実装時に確認し、`to_le_bytes()` はその型のまま使う。

**テスト（`utils.rs` の `mod tests` に追加）**:

```rust
#[test]
fn test_prompt_hash_no_boundary_collision() {
    // 区切り衝突の Negative: ("ab", Some("c")) != ("a", Some("bc"))
    assert_ne!(
        compute_prompt_hash("ab", Some("c")),
        compute_prompt_hash("a", Some("bc"))
    );
    assert_ne!(compute_prompt_hash("abc", None), compute_prompt_hash("ab", Some("c")));
}

#[test]
fn test_request_cache_key_sensitive_to_history_and_params() {
    // messages 数・format・temperature が異なればキーも異なること（各1ケース）
    // LlmRequest を組み立てて assert_ne! で比較
}
```

#### FIX-7-B: `libs/infrastructure/src/llm/semantic_cache.rs` — key API 追加

既存 `get(prompt, system)` / `set(prompt, system, ...)` は変更しない（呼び出し元は `caching_provider.rs` と自身のテストのみ、と rg 確認済み）。`impl SemanticCache` に以下を追加:

```rust
    /// 事前計算済みキーでの完全一致取得（セマンティック照合なし）
    pub async fn get_by_key(&self, key: &str) -> Result<Option<LlmResponse>, AiomeError> {
        if let Some(content) = self.repo.get_by_hash(key).await? {
            debug!("🎯 [SemanticCache] Exact Hit (by key)! Hash: {}", key);
            return Ok(Some(LlmResponse {
                content,
                stop_reason: StopReason::EndTurn,
                ..Default::default()
            }));
        }
        Ok(None)
    }

    /// 事前計算済みキーでの保存（embedding なし）
    pub async fn set_by_key(
        &self,
        key: &str,
        response: &LlmResponse,
        provider_name: &str,
        model_name: &str,
        ttl_seconds: i64,
    ) -> Result<(), AiomeError> {
        self.repo
            .set(key, &response.content, provider_name, model_name, ttl_seconds, None)
            .await?;
        debug!("💾 [SemanticCache] Stored (by key) hash: {}", key);
        Ok(())
    }
```

（`repo.set` の署名 `(hash, response, provider_name, model_name, ttl_seconds, embedding: Option<Vec<u8>>)` は L26-34 で確認済み。）

#### FIX-7-C: `libs/infrastructure/src/llm/caching_provider.rs`

- `extract_prompt_parts` を削除し、`complete_with_cache` は `compute_request_cache_key(&request)` + `get_by_key`/`set_by_key` を使用。
- `complete(prompt, system)` は `LlmRequest` を組み立てて `complete_with_cache` に**委譲**し、キー体系を単一化:

```rust
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<LlmResponse, AiomeError> {
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(LlmMessage { role: "system".to_string(), content: sys.to_string(), cache: false });
        }
        messages.push(LlmMessage { role: "user".to_string(), content: prompt.to_string(), cache: false });
        self.complete_with_cache(LlmRequest { messages, ..Default::default() }).await
    }

    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        if Self::should_bypass_cache(&request) {
            return self.inner.complete_with_cache(request).await;
        }
        let key = compute_request_cache_key(&request);
        if let Some(cached) = self.cache.get_by_key(&key).await? {
            return Ok(Self::mark_cache_hit(cached));
        }
        let response = self.inner.complete_with_cache(request).await?;
        let provider_name = self.inner.name();
        if let Err(e) = self
            .cache
            .set_by_key(&key, &response, provider_name, provider_name, self.ttl_seconds)
            .await
        {
            tracing::warn!("SemanticCache write failed (non-fatal): {}", e);
        }
        Ok(response)
    }
```

（`mark_cache_hit` / `should_bypass_cache`（format=="json" バイパス）/ `stream_complete` 素通し / `name` / `test_connection` は現行維持。import に `LlmMessage` と `compute_request_cache_key` を追加、`compute_prompt_hash` 系 import は不要になれば削除。）

**テスト**:
- 既存 `test_caching_provider_hit` は委譲後もそのまま有効（PASS を確認）。
- **Negative 追加**: 同一の最終 user メッセージでも会話履歴が異なる 2 つの `LlmRequest` でキャッシュが混線しないこと（1 つ目を set 後、2 つ目が MISS になる）。

**検証**: `cargo test -p infrastructure --lib utils && cargo test -p infrastructure --lib semantic_cache && cargo test -p infrastructure --lib caching_provider`

---

### FIX-3 (H-1): route_* ログを request.metadata 優先に変更

**問題**: IR は route metadata を **request** に注入するが、内側 provider は response を新規構築するため response.metadata に route_* が乗らないことがあり、eval ログの route_* が NULL になる。

**対象 4 箇所（`route_fields_from_metadata` 呼び出し）**:

| # | ファイル:行 | メソッド | 対応 |
|---|---|---|---|
| 1 | `dynamic.rs` L176-177 | `complete` | **現状維持**（request は local 構築で metadata=None のため） |
| 2 | `dynamic.rs` L434-435 | `complete_with_cache` | **置換** |
| 3 | `background.rs` L186-187 | `complete` | **現状維持** |
| 4 | `background.rs` L349-350 | `complete_with_cache` | **置換** |

**#2・#4 の置換コード（同一。helper 新設はせず呼び出し側で `or` 合成）**:

```rust
            // OP-099 fix (H-1): IR は request 側に route_* を注入するため request 優先
            let (req_tier, req_reason, req_mode) =
                super::cost::route_fields_from_metadata(request.metadata.as_ref());
            let (resp_tier, resp_reason, resp_mode) =
                super::cost::route_fields_from_metadata(response.metadata.as_ref());
            let (route_tier, route_reason, route_mode) = (
                req_tier.or(resp_tier),
                req_reason.or(resp_reason),
                req_mode.or(resp_mode),
            );
```

**注意**: `complete_with_cache` 内で `request` が move 済み（inner へ渡す等）の場合は、inner 呼び出し**前**に `let request_meta = request.metadata.clone();` で退避し `route_fields_from_metadata(request_meta.as_ref())` とする。FIX-2 で `request.clone()` を inner に渡す形になっていれば元の `request` が残るのでそのまま使える。実装時にビルドエラーで判断。

**テスト**: eval ログ書き込みは統合的なため、FIX-5 の結合テスト実施後に手動確認（`prompt_evaluation_log` の route_tier 非 NULL）で代替。ユニットでは `req.or(resp)` 合成の性質上、既存 `route_fields_from_metadata` テストで足りる。

**検証**: `cargo check -p infrastructure` + §6 手動検証。

---

## 3. Phase R3: Medium（ルータ堅牢化）

### FIX-8 (M-1): 外部 `route_tier` metadata の信頼を廃止

**問題**: `decide_route` が外部から渡された `route_tier` metadata を無条件に信頼し、呼び出し元が Fast を強制できる（コスト・品質バイパス経路）。信頼するのは内部生成の `route_tier_locked` のみとする。

**ファイル**: `libs/infrastructure/src/llm/route_rules.rs`

**現行（L36-47）**:

```rust
    if let Some(meta) = metadata {
        if let Some(locked) = meta.get(ROUTE_TIER_LOCKED_KEY) {
            return parse_tier_override(locked, "tier_locked", "Sticky tier from prior evaluation");
        }
        if let Some(tier_raw) = meta.get(ROUTE_TIER_KEY) {
            return parse_tier_override(
                tier_raw,
                "metadata_override",
                "Explicit route_tier metadata",
            );
        }
    }
```

**置換後**（`ROUTE_TIER_KEY` 分岐を削除）:

```rust
    if let Some(meta) = metadata {
        if let Some(locked) = meta.get(ROUTE_TIER_LOCKED_KEY) {
            return parse_tier_override(locked, "tier_locked", "Sticky tier from prior evaluation");
        }
    }
```

**テスト**: 既存 `test_metadata_override_fast`（L129-135）を Negative に書き換え:

```rust
    #[test]
    fn test_metadata_override_fast() {
        // route_tier metadata では上書きできない（長文は default_smart 維持）
        let mut meta = HashMap::new();
        meta.insert(ROUTE_TIER_KEY.to_string(), "fast".to_string());
        let long = "x".repeat(600);
        let d = decide_route(&long, None, Some(&meta), &CFG);
        assert_eq!(d.tier, TaskTier::Smart);
        assert_eq!(d.reason_code, "default_smart");
    }
```

**注意**（rg 確認済み）:
- `intelligent_router.rs` の `ROUTE_TIER_KEY` 使用は決定後の注入・enrichment・アサーションのみで、入力 override に依存するテストは**ない** → 修正不要。
- `ROUTE_TIER_KEY` 定数と `inject_route_metadata` での出力（観測用）は残す。

**検証**: `cargo test -p infrastructure --lib route_rules && cargo test -p infrastructure --lib intelligent_router`

---

### FIX-4 (M-3): `LLM_ROUTE_BUDGET_DEGRADE` パース堅牢化

**ファイル**: `libs/shared/src/config.rs` L435-437

**現行**:

```rust
            llm_route_budget_degrade: env::var("LLM_ROUTE_BUDGET_DEGRADE")
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(true),
```

**置換後**（`LlmRouteMode` パース（L29-38）の warn + 既定スタイルに合わせる）:

```rust
            llm_route_budget_degrade: match env::var("LLM_ROUTE_BUDGET_DEGRADE") {
                Ok(v) => match v.trim().to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => true,
                    "false" | "0" | "no" | "off" => false,
                    other => {
                        tracing::warn!(
                            "Invalid LLM_ROUTE_BUDGET_DEGRADE '{}'. Defaulting to true. Valid: true/false, 1/0, yes/no, on/off.",
                            other
                        );
                        true
                    }
                },
                Err(_) => true,
            },
```

**検証**: `cargo check -p shared`（env 依存のためユニットテストは不要。`"FALSE"` が false になることが目視で自明）。

---

## 4. Phase R4: Egress 硬化（OP-100 として独立実施可）

### FIX-9 (H-3): `network_target_host` の Fail-Closed 化

**問題**: URL パース失敗時・非対象 scheme 時に `Some(trimmed)` で生文字列を返すため、`ftp://evil.com/.example.com` のような文字列が suffix 一致で allow される余地がある。

**ファイル**: `libs/infrastructure/src/security/bastion_guard.rs` L302-317

**置換後（関数全体）**:

```rust
/// Resolve a `check_network` target to a host string (bare domain or URL host).
/// Fail-Closed: 非 http(s)/ws(s) scheme、および host として不正な文字を含む
/// bare 文字列は None（deny）を返す。
fn network_target_host(target: &str) -> Option<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(parsed) = url::Url::parse(trimmed) {
        return match parsed.scheme() {
            "http" | "https" | "ws" | "wss" => parsed
                .host_str()
                .filter(|h| !h.is_empty())
                .map(|h| h.to_string()),
            _ => None, // ftp / file / data 等は Fail-Closed
        };
    }
    // bare host: URL 構成文字・制御文字を含む場合は deny
    if trimmed
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '/' | '@' | ':' | '#' | '?'))
    {
        return None;
    }
    Some(trimmed.to_lowercase())
}
```

**既存テスト影響**（`libs/infrastructure/src/security/tests.rs`）:
- `test_bastion_check_network_bare_host_and_url_ok`（L466 付近）: 影響なし（lowercase 後も一致）。
- `test_bastion_check_network_target_resolution_edges`（L503 付近）: `ftp://example.com` は引き続き `is_err()`（deny 到達経路が DomainBlocked → Invalid target に変わる可能性あり。アサーションが `is_err()` なら修正不要、エラー種別を見ているなら追随）。

**追加テスト（Negative 必須）**:

```rust
#[test]
fn test_bastion_check_network_fail_closed_hostile_targets() {
    let manifest = PermissionManifest {
        allow_network: true,
        allowed_domains: vec!["example.com".into()],
        ..Default::default()
    };
    let guard = BastionGuard::new(manifest);

    assert!(guard.check_network("ftp://evil.com/.example.com").is_err()); // 非対象 scheme
    assert!(guard.check_network("allowed.com@evil.com").is_err()); // userinfo 風
    assert!(guard.check_network("example.com:443").is_err()); // bare + port は deny（仕様）
    assert!(guard.check_network("example.com/path").is_err());
    assert!(guard.check_network("example.com?x=1").is_err());
    assert!(guard.check_network("example.com#frag").is_err());
}
```

**検証**: `cargo test -p infrastructure --lib security`

---

### FIX-10 (H-4 + M-4): `host_permitted` の制御文字拒否 + 正規化

**問題**: (a) NUL 等の制御文字入り host が素通しで下流の比較を撹乱しうる。(b) 大文字 host（`EVIL.COM`）や FQDN 末尾ドット（`example.com.`）が正規化されず、allow すべきものが deny / 判定不整合になる。

**ファイル**: `libs/aiome-contracts/src/security.rs` L23-52（doc コメント含め置換）

**置換後（関数全体 + doc）**:

```rust
/// Host allowlist check for [`PermissionManifest::allowed_domains`] (OP-096 / ADR-057).
///
/// Rules (Code Mode `aiome.fetch` base + harden):
/// - empty list / empty host (after trim) → deny
/// - host with control chars or internal whitespace → deny
/// - host is lowercased; a single trailing `.` (FQDN) is stripped
/// - allow entries are lowercased after trim; empty / leading-`.` / trailing-`.` junk ignored
/// - `*` → allow any non-empty normalized host
/// - exact host match (case-insensitive via normalization)
/// - subdomain suffix (`host.ends_with("." + domain)`) only when `domain` contains `.`
///   (prevents `allowed_domains=["com"]` from matching `evil.com`)
pub fn host_permitted(host: &str, allowed_domains: &[String]) -> bool {
    let host = host.trim();
    if host.is_empty() || allowed_domains.is_empty() {
        return false;
    }
    if host.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    let mut host = host.to_lowercase();
    if host.ends_with('.') {
        host.pop();
        if host.is_empty() {
            return false;
        }
    }

    for domain in allowed_domains {
        let domain = domain.trim();
        if domain.is_empty() || domain.starts_with('.') || domain.ends_with('.') {
            continue;
        }
        let domain = domain.to_lowercase();
        if domain == "*" || domain == host {
            return true;
        }
        if domain.contains('.') && host.ends_with(&format!(".{}", domain)) {
            return true;
        }
    }
    false
}
```

**テスト**（`libs/aiome-contracts/tests/security_tests.rs`）:
- 既存 `test_host_permitted_case_sensitive`（L77-81 付近）は新仕様と矛盾するため、下記 case-insensitive Positive に**置換**。

```rust
#[test]
fn test_host_permitted_rejects_control_and_internal_whitespace() {
    let domains = vec!["example.com".to_string()];
    assert!(!host_permitted("exam\0ple.com", &domains));
    assert!(!host_permitted("exam ple.com", &domains));
}

#[test]
fn test_host_permitted_case_insensitive_allow() {
    let domains = vec!["evil.com".to_string()];
    assert!(host_permitted("EVIL.COM", &domains));
    assert!(host_permitted("Evil.Com", &domains));
}

#[test]
fn test_host_permitted_trailing_dot_fqdn_normalized() {
    let domains = vec!["example.com".to_string()];
    assert!(host_permitted("example.com.", &domains));
    assert!(host_permitted("api.example.com.", &domains));
}

#[test]
fn test_host_permitted_trailing_dot_does_not_widen_suffix() {
    assert!(!host_permitted(
        "evil.com.",
        &[".com".to_string(), "com.".to_string(), "com".to_string()]
    ));
}
```

**検証**: `cargo test -p aiome-contracts`

**波及注意**: `host_permitted` は BastionGuard / constraint_checker / cleanroom 等から共用されている。case-insensitive 化は「allow が広がる」方向の変更（`Example.COM` が許可される）であり、セキュリティ上は正規化として妥当だが、ADR-057 の「Case-sensitive」記述の更新が必要（§7）。

---

### FIX-11 (M-2 + M-5): migration 注記 + ADR 追記（コード変更なし）

**FIX-11-A**: `libs/infrastructure/migrations/sqlite/20260801000000_prompt_eval_route_fields.sql` の冒頭にコメント追加（既存 3 行の ALTER は不変。**適用済み migration のためチェックサム変更に注意** — sqlx は既適用ファイルの変更を検出してエラーにする。**開発 DB が既にこの migration を適用済みの場合はコメント追加を見送り、本注記を runbook 側（`docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md`）にのみ記載する**こと):

```sql
-- prompt_evaluation_log: route observability columns (ADR-058).
-- sqlx migrator applies this file inside a transaction and records success in
-- `_sqlx_migrations`; partial application does not occur under normal operation.
-- Manual application: run all three ALTERs to completion or roll back the batch.
-- SQLite ADD COLUMN has no IF NOT EXISTS — re-running fails if columns exist
-- (the postgres sibling uses IF NOT EXISTS).
```

**FIX-11-B**: `docs/decisions/057-manifest-host-permit-fail-closed.md` の `## Consequences` 内、Follow-up 行（L27 付近）の直後に追記:

```markdown
- **OP-097 clarification**: `constraint_checker` の `DomainBlocked` は旧来の exact-host 一致より広い。`host_permitted` 委任により subdomain suffix（allow entry に `.` を含む場合）および `*` ワイルドカードが有効。旧 exact-only を前提にした呼び出し側は挙動差に注意すること。
```

あわせて FIX-10 実施時は同 ADR の「Case-sensitive rules」記述を「normalized (case-insensitive, FQDN trailing-dot stripped)」に更新。

---

## 5. Phase R5: 結合テスト（FIX-5）

**新規ファイル**: `libs/infrastructure/tests/llm_chat_stack_test.rs`

**目的**: 本番 DI チェーン `HF → [Caching(rules)] → EG → IR` を実際に組み、モード別のルーティングとキャッシュ動作を検証する（FIX-1/6/7 完了後に実装）。

**前提（確認済み）**:
- `infrastructure::db` / `job_queue` / `llm` は `pub mod`（lib.rs L45/L219/L235）。
- `MockLlmProvider { response, should_fail }` は `aiome_core::llm_provider`（debug_assertions で export、integration test から利用可）。
- `MockJQ` は `pub(crate)` のため使用不可 → テスト内に `ZeroCostOps` を定義。
- `IntelligentRouter::new` 引数順: `(mode, budget_degrade, short_prompt_chars, fast, fast_degraded, smart, cost_ops, default_cost_limit_usd)`。
- `SettingsOps` の必須メソッドは `intelligent_router.rs` の `TrippedCostOps` 実装を正とする（下記と差分があればそちらに合わせる）。

**ファイル全文**:

```rust
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::unwrap_used)]

use aiome_core::llm_provider::{LlmMessage, LlmProvider, LlmRequest, MockLlmProvider};
use aiome_core_contracts::error::AiomeError;
use async_trait::async_trait;
use infrastructure::job_queue::CostOps;
use infrastructure::llm::caching_provider::CachingLlmProvider;
use infrastructure::llm::entropy_gate::EntropyGate;
use infrastructure::llm::humanizer_filter::HumanizerFilter;
use infrastructure::llm::humanizer_rules::default_rules_ja;
use infrastructure::llm::intelligent_router::IntelligentRouter;
use infrastructure::llm::semantic_cache::{SemanticCache, SqlSemanticCacheRepository};
use infrastructure::llm::writing_context::WritingContext;
use shared::config::LlmRouteMode;
use std::sync::Arc;

#[derive(Debug)]
struct ZeroCostOps;

#[async_trait]
impl CostOps for ZeroCostOps {
    async fn aggregate_cost_hours(&self, _hours: i64) -> Result<f64, AiomeError> {
        Ok(0.0)
    }
    async fn aggregate_cost_days(&self, _days: i64) -> Result<f64, AiomeError> {
        Ok(0.0)
    }
    async fn aggregate_cost_by_job(&self, _job_id: &str) -> Result<f64, AiomeError> {
        Ok(0.0)
    }
}

#[async_trait]
impl aiome_core_contracts::traits::SettingsOps for ZeroCostOps {
    async fn do_get_setting(&self, _key: &str) -> Result<Option<String>, AiomeError> {
        Ok(None)
    }
    async fn do_set_setting(&self, _k: &str, _v: &str, _c: &str, _s: bool) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn do_get_all_settings(
        &self,
    ) -> Result<Vec<aiome_core_contracts::contracts::SystemSetting>, AiomeError> {
        Ok(vec![])
    }
    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        Ok(false)
    }
    async fn set_auto_expression_enabled(&self, _e: bool) -> Result<(), AiomeError> {
        Ok(())
    }
}

fn mock_chain(label: &str) -> Arc<dyn LlmProvider + Send + Sync> {
    Arc::new(MockLlmProvider {
        response: label.to_string(),
        should_fail: false,
    })
}

/// 本番 DI（FIX-6 後）と同一の順序: HF → [Caching(rules のみ)] → EG → IR
async fn build_stack(mode: LlmRouteMode) -> Arc<dyn LlmProvider + Send + Sync> {
    let pool = infrastructure::db::DatabasePool::new_sqlite("sqlite::memory:")
        .await
        .unwrap();
    let ts = Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
    );
    let jq = Arc::new(
        infrastructure::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
            .await
            .unwrap(),
    );
    infrastructure::job_queue::migrations::DbInitializer::init_db(&*jq)
        .await
        .unwrap();

    let router = IntelligentRouter::new(
        mode,
        false,
        512,
        mock_chain("fast"),
        mock_chain("fast_degraded"),
        mock_chain("smart"),
        Arc::new(ZeroCostOps),
        10.0,
    );
    let gate: Arc<dyn LlmProvider + Send + Sync> =
        Arc::new(EntropyGate::new(Arc::new(router), 2.0, 3));

    let core: Arc<dyn LlmProvider + Send + Sync> = if mode == LlmRouteMode::Rules {
        let repo = Arc::new(SqlSemanticCacheRepository::new(pool.clone()));
        let cache = Arc::new(SemanticCache::new(repo, None));
        Arc::new(CachingLlmProvider::new(gate, cache, 3600))
    } else {
        gate
    };

    Arc::new(HumanizerFilter::new(
        core,
        default_rules_ja(),
        WritingContext::Default,
    ))
}

fn user_request(prompt: &str, format: Option<&str>) -> LlmRequest {
    LlmRequest {
        messages: vec![LlmMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            cache: false,
        }],
        temperature: None,
        max_tokens: None,
        stop_sequences: None,
        format: format.map(|s| s.to_string()),
        metadata: None,
    }
}

/// T1: rules + 短文 → Fast
#[tokio::test]
async fn t1_rules_short_prompt_routes_fast() {
    let stack = build_stack(LlmRouteMode::Rules).await;
    let resp = stack
        .complete_with_cache(user_request("hello", None))
        .await
        .unwrap();
    assert_eq!(resp.content, "fast");
}

/// T2: rules + format=json → Smart（かつキャッシュバイパス）
#[tokio::test]
async fn t2_rules_json_format_routes_smart() {
    let stack = build_stack(LlmRouteMode::Rules).await;
    let resp = stack
        .complete_with_cache(user_request("hello", Some("json")))
        .await
        .unwrap();
    assert_eq!(resp.content, "smart");
}

/// T3: legacy → 常に Smart（キャッシュ層なし）
#[tokio::test]
async fn t3_legacy_always_smart() {
    let stack = build_stack(LlmRouteMode::Legacy).await;
    let resp = stack
        .complete_with_cache(user_request("hello", None))
        .await
        .unwrap();
    assert_eq!(resp.content, "smart");
}

/// T4: rules + 同一リクエスト 2 回目はキャッシュヒット（cache_hit metadata）
#[tokio::test]
async fn t4_rules_second_call_hits_cache() {
    let stack = build_stack(LlmRouteMode::Rules).await;
    let first = stack
        .complete_with_cache(user_request("hello cache", None))
        .await
        .unwrap();
    assert!(first
        .metadata
        .as_ref()
        .and_then(|m| m.get("cache_hit"))
        .is_none());
    let second = stack
        .complete_with_cache(user_request("hello cache", None))
        .await
        .unwrap();
    assert_eq!(
        second.metadata.as_ref().and_then(|m| m.get("cache_hit")).map(String::as_str),
        Some("true")
    );
}
```

**注意**:
- Mock 応答 `"fast"` / `"smart"` は Humanizer ルールに引っかからない前提（引っかかる場合はラベル文字列を変える）。
- MockLlmProvider の応答に logprobs がなければ EG は素通り（EG のリトライは発火しない）。
- `MockLlmProvider` に追加フィールドがある場合は `..Default::default()` で埋める。

**検証**: `cargo test -p infrastructure --test llm_chat_stack_test`

---

## 6. 最終検証プロトコル（Verification Protocol 準拠）

1. **フォーマット/コンパイル**: `cargo fmt && cargo check --workspace --tests`
2. **全テスト**: `cargo test --workspace`
3. **Positive**: FIX-5 の T1-T4 が PASS。
4. **Negative（必須）**:
   - FIX-8 の書き換え済み `test_metadata_override_fast`（外部 route_tier で Fast 強制できない）
   - FIX-9 の `test_bastion_check_network_fail_closed_hostile_targets`
   - FIX-10 の制御文字/空白 deny テスト
   - FIX-7 の boundary collision テスト・履歴混線 MISS テスト
   - FIX-2 の pin_local 下で settings 非参照テスト
5. **Revert 確認**: Negative 用に注入した細工がテストコード内で完結していること（本番コードに残置しない）。

---

## 7. ドキュメント同期チェックリスト（実装完了時に実施）

| 対象 | 内容 |
|---|---|
| `CHANGELOG.md` [Unreleased] | C-1/C-2/C-3/H-1/H-2 修正、キャッシュキー体系変更による既存セマンティックキャッシュの実質失効、legacy モードのキャッシュ層撤去、外部 route_tier override 廃止 |
| `docs/decisions/058-intelligent-llm-routing.md` | キャッシュ配置の変更（EG 内側 → EG 外側・rules 限定・セマンティック照合停止）と理由（EG 検証済み応答のみキャッシュ）を追記 |
| `docs/decisions/057-manifest-host-permit-fail-closed.md` | FIX-11-B の追記 + Case-sensitive → normalized への記述更新 |
| `docs/architecture/LLM_PROVIDER_ARCHITECTURE.md` | チェーン図を `HF → [Caching(rules)] → EG → IR` に更新、pin_local の説明追加 |
| `docs/roadmaps/intelligent_llm_router_plan.md` | v1.3 の「EG 内側」記述に本計画で上書きされた旨の注記 |
| `.context/RIPPLE_MAP.md` | `pin_local` / `compute_request_cache_key` / `get_by_key` / `set_by_key` の追加を反映 |
| `OPEN.md` | 本計画の消化状況を反映。Phase R4 を OP-100 として切り出す場合はチケット追加。将来 OP: スコープ付きセマンティックキャッシュ再有効化 |
| `docs/architecture/SECURITY_DESIGN.md` / `SECURITY_WHITEPAPER.md` | FIX-9/10 の Fail-Closed・正規化仕様を反映（Phase R4 実施時） |

---

## 8. 明示的に対応しない項目（レビュー指摘のうち）

- **セマンティック照合の恒久無効化ではない**: 完全一致のみへ縮退は暫定措置。会話/チャネルスコープ付きの再有効化は別 OP として `OPEN.md` に記録。
- **Postgres migration の注記**: sqlite 側のみ対応（postgres は `IF NOT EXISTS` 済みでリスクなし）。
- **`stop_sequences` / `LlmMessage.cache` のキー参入**: 現状の利用実態では不要と判断。将来これらがチャット経路で可変になった場合に再検討。
