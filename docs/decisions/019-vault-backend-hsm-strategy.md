# ADR-019: Vault Backend 抽象化と HSM 移行戦略

> **Status**: Accepted  
> **Date**: 2026-03-23  
> **Author**: AIome Sovereign Task Force  
> **Supersedes**: None  
> **References**: SECURITY_DESIGN.md §6, MASTER_BLUEPRINT Tier 4

---

## 1. 背景と問題

`AbyssVoiceVault` は現在、**全ての復号鍵をインメモリ `HashMap` にリストアして保持**しています（L25 FIXME）。

### 現状のアーキテクチャ

```
┌────────────────────────────────────────────────┐
│  AbyssVoiceVault                               │
│                                                │
│  master_key: OnceCell<MlockedVec>              │  ← VAULT_MASTER_KEY env var から導出
│  keys: Mutex<HashMap<Uuid, MlockedVec>>        │  ← 起動時に全鍵を DB から復号して保持
│                                                │
│  ┌──────────────────────────┐                  │
│  │ SQLite: vault_keys       │                  │  ← AES-256-GCM で暗号化された DEK
│  │ (asset_id, encrypted_key)│                  │
│  └──────────────────────────┘                  │
└────────────────────────────────────────────────┘
```

### 3つの限界

| # | 問題 | 影響 | 深刻度 |
|---|---|---|---|
| 1 | **全鍵インメモリ保持** | アセット数に比例してメモリ消費が増大。1万件 × 32B = 320KB + HashMap オーバーヘッド。100万件で致命的 | 🟠 |
| 2 | **Master Key が SPOF** | 環境変数の1つの hex 文字列に全セキュリティが依存。ローテーション時に全 DEK の再暗号化が必要 | 🔴 |
| 3 | **バックエンド切替不可** | `AbyssVoiceVault` が SQLite に直結。外部 Vault サービスや HSM への切り替えにはコード大規模書き換えが必要 | 🔴 |

---

## 2. 提案: 3段階の移行戦略

### Phase A: VaultBackend トレイトの導入（Phase 28 で即実施可能）

**目標**: バックエンドの抽象化により、将来の HSM 移行をゼロ・コスト・スイッチにする。

```rust
// libs/aiome-contracts/src/vault_backend.rs [NEW]

#[async_trait]
pub trait VaultBackend: Send + Sync {
    /// 指定されたアセットの DEK を取得する（復号済み）
    async fn get_dek(&self, asset_id: Uuid) -> Result<Zeroizing<Vec<u8>>, AiomeError>;
    
    /// 新しいアセットの DEK を保存する
    async fn store_dek(&self, asset_id: Uuid, dek: &[u8]) -> Result<(), AiomeError>;
    
    /// Master Key のローテーション（全 DEK の再暗号化）
    async fn rotate_master_key(&self, new_master: &[u8]) -> Result<usize, AiomeError>;
    
    /// バックエンドのヘルスチェック
    async fn health_check(&self) -> Result<(), AiomeError>;
}
```

**AbyssVoiceVault の変更**:

```rust
pub struct AbyssVoiceVault {
    backend: Arc<dyn VaultBackend>,   // ← 差し替え可能なバックエンド
    cache: Mutex<LruCache<Uuid, MlockedVec>>,  // ← 全保持 → LRU キャッシュ
    registry: Arc<RegistryManager>,
}
```

### Phase B: LRU キャッシュ化（Phase 29 と同時）

**目標**: 全鍵インメモリ保持を廃止し、オンデマンド取得 + LRU キャッシュに移行。

```
┌───────────────────────────────────────────┐
│  AbyssVoiceVault                          │
│                                           │
│  cache: LruCache<Uuid, MlockedVec> (256)  │  ← 最大256件の MRU 鍵のみ保持
│  backend: Arc<dyn VaultBackend>           │  ← キャッシュミス時にバックエンドへ問い合わせ
│                                           │
│  fetch_decryption_key():                  │
│    1. cache.get(asset_id)?                │  ← O(1) ヒット
│    2. cache miss → backend.get_dek()      │  ← DB or HSM へ問い合わせ
│    3. cache.put(asset_id, dek)            │  ← キャッシュに追加
│    4. 溢れた鍵は MlockedVec::drop() で    │
│       自動 zeroize + munlock              │
└───────────────────────────────────────────┘
```

**メリット**:
- メモリ消費: O(N) → O(1)（固定上限）
- セキュリティ: 攻撃者がメモリダンプを取得しても最大 256 件分の鍵しか漏洩しない

### Phase C: 外部 Vault / HSM 統合（Phase 35+ / Tier 4）

**目標**: Master Key を物理的にプロセス外に隔離する。

#### 選択肢の比較

| 選択肢 | コスト | セキュリティ | 運用性 | 推奨度 |
|---|---|---|---|---|
| **A. HashiCorp Vault** (OSS) | 無料（Self-Host） | ★★★★☆ | 運用負荷中 | 🟢 Phase C-1 |
| **B. AWS KMS / GCP Cloud KMS** | $1/key/月 | ★★★★★ | 運用負荷低 | 🟢 Phase C-2 |
| **C. YubiHSM 2 (物理)** | $650/台 | ★★★★★ | 運用負荷高 | 🟡 Phase C-3 |
| **D. Apple Secure Enclave** | 無料（macOSのみ） | ★★★★☆ | macOS限定 | 🟡 開発用 |
| **E. PKCS#11 汎用** | ハードウェア依存 | ★★★★★ | 運用負荷高 | 🟡 エンタープライズ |

#### 推奨パス

```
Phase 28 (今すぐ)     Phase 29         Phase 35+
     │                    │                │
     ▼                    ▼                ▼
  VaultBackend       LRU Cache化      外部 Vault 統合
  トレイト導入       HashMap 全保持廃止  Master Key 物理隔離
  SqliteVaultBackend  オンデマンド取得   HashiCorp or KMS
```

1. **Phase C-1: HashiCorp Vault** — Self-Host が可能で、OSS の Transit Engine で Master Key をプロセス外に分離。`VaultBackendHashiCorp` 実装で対応。
2. **Phase C-2: Cloud KMS** — クラウドデプロイ時に KMS Envelope Encryption を使用。`VaultBackendKms` 実装。
3. **Phase C-3: YubiHSM** — 物理ハードウェアでの鍵保護。エンタープライズ顧客向けオプション。

---

## 3. 実装計画

### Phase A の詳細（即時実施可能）

| # | タスク | ファイル | 工数 |
|---|---|---|---|
| A-1 | `VaultBackend` トレイト定義 | `aiome-contracts/src/vault_backend.rs` [NEW] | 1h |
| A-2 | `SqliteVaultBackend` 実装 | `infrastructure/src/security/sqlite_vault_backend.rs` [NEW] | 2h |
| A-3 | `AbyssVoiceVault` リファクタリング | `infrastructure/src/security/abyss_voice_vault.rs` [MODIFY] | 2h |
| A-4 | `VoiceCoreDrm` の更新 | `infrastructure/src/security.rs` [MODIFY] | 30m |
| A-5 | テスト（`api_integration_tests.rs` 含む）| 各ファイル + `api_integration_tests.rs:867` | 1.5h |
| A-6 | `SECURITY_DESIGN.md` §6.5 更新 | `docs/architecture/SECURITY_DESIGN.md` | 15m |

### Phase B の詳細（Phase 29 PostgreSQL 移行と同時）

| # | タスク | 工数 |
|---|---|---|
| B-1 | `lru` クレート導入 + LRU キャッシュ実装 | 2h |
| B-2 | `restore_keys_from_db` 廃止 → オンデマンド `get_dek` | 1h |
| B-3 | キャッシュヒット率のメトリクス導入 | 1h |
| B-4 | `VAULT_CACHE_SIZE` 環境変数のサポート追加 | 30m |

---

## 4. セキュリティ上の考慮事項

### Master Key ローテーション

現在の設計では Master Key のローテーションには**全 DEK の再暗号化**が必要です。
`VaultBackend::rotate_master_key()` をトレイトに抽出するに加え、将来的には **Key Wrapping 階層** を導入：

```
Root Key (HSM内、エクスポート不可)
  └─ Master Key (Transit Engine で暗号化)
       └─ DEK (Master Key で暗号化、SQLite/PostgreSQL に保存)
```

これにより、Root Key のローテーションは Master Key の再暗号化のみで済み、DEK には影響しません。

### LRU キャッシュのセキュリティ

- キャッシュ内の全エントリは `MlockedVec`（mlock + zeroize）を使用
- キャッシュからの追い出し時に `Drop` による自動ゼロ消去 + `munlock`
- キャッシュサイズ上限により、メモリダンプ攻撃時の被害範囲を限定

---

## 5. 決定事項

1. **Phase A（VaultBackend 抽象化）を Phase 28 で実施する**
2. **Phase B（LRU キャッシュ化）を Phase 29 の PostgreSQL 移行と並行する**
3. **Phase C（HSM/外部 Vault）は Phase 35+ (Tier 4) で HashiCorp Vault を第一候補とする**
4. **Apple Secure Enclave は開発環境用のオプショナル実装として提供する**

---
*最終更新: 2026-03-23*
