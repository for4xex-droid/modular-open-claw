# ADR 040: Aegis Prover — Kani Rust Verifier Sandbox Architecture

## Status
Proposed

## Context
The Aegis Sentinel autonomous immune system generates patches for WASM skill panics via LLM (`AegisProver::generate_patch`). These patches must be formally verified before hot-swapping into production. Currently, `verify_with_kani()` is a stub returning `true` (see `libs/infrastructure/src/aegis/prover.rs:38`).

[Kani Rust Verifier](https://github.com/model-checking/kani) is a model-checking tool that can prove the absence of certain classes of bugs (arithmetic overflows, out-of-bounds access, assertion failures) in Rust code. Integrating Kani into the Aegis pipeline requires executing `cargo kani` on untrusted, LLM-generated code, which introduces significant security and resource risks.

This ADR defines the sandbox architecture for safely executing Kani verification within the existing Aiome security infrastructure (`BastionGuard`, `SafeCommandBuilder`, `PathSandbox`).

## Decision

### 1. Podman Rootless Container Execution

Kani verification will execute inside a **Podman rootless container** to provide OS-level isolation:

```
┌─────────────────────────────────────────────────────┐
│  Aiome API Server (Host)                            │
│                                                     │
│  AegisProver::verify_with_kani(patch_code)          │
│       │                                             │
│       ▼                                             │
│  SafeCommandBuilder::new("podman")                  │
│    .arg("run")                                      │
│    .arg("--rm")                                     │
│    .arg("--network=none")        ← ネットワーク遮断 │
│    .arg("--memory=2g")           ← メモリ制限       │
│    .arg("--cpus=1")              ← CPU 制限         │
│    .arg("--read-only")           ← RO ファイルシステム│
│    .arg("-v /tmp/kani-XXXX:/work:Z") ← 一時マウント │
│    .arg("aiome/kani-verifier:latest")               │
│    .arg("cargo kani --harness verify_patch")        │
│    .profile(SandboxProfile::Strict)                 │
│    .build_internal()                                │
│       │                                             │
│       ▼                                             │
│  ZombieKiller::run_with_timeout()                   │
│    timeout = KANI_PROOF_TIMEOUT_SECS                │
└─────────────────────────────────────────────────────┘
```

**コンテナイメージ (`aiome/kani-verifier`)**:
- Base: `rust:1.82-slim`
- 追加: `cargo-kani` (Kani Rust Verifier)
- Dockerfile を `docker/kani-verifier.Dockerfile` に配置
- CI で定期的にビルド・タグ付けする

### 2. BastionGuard によるパス検証

ホストとコンテナ間のファイル共有には一時ディレクトリを使用する。`PathSandbox` によりパストラバーサルを防止する:

```rust
// 1. 一時ディレクトリを作成
let tmp_dir = tempfile::Builder::new()
    .prefix("kani-")
    .tempdir_in(&workspace_root)?;

// 2. PathSandbox でパス検証
let sandbox = PathSandbox::new(tmp_dir.path())?;
let patch_path = tmp_dir.path().join("src/lib.rs");
sandbox.validate_path(&patch_path)?;  // ← パストラバーサル防止

// 3. パッチコードを一時ディレクトリに書き込み
tokio::fs::write(&patch_path, patch_code).await?;

// 4. コンテナに一時ディレクトリをバインドマウント
// (上記のコンテナ実行フローに合流)
```

**制約事項**:
- 一時ディレクトリは `workspace_root` 配下に作成 (サンドボックス外へのエスケープ防止)
- コンテナ側は `:Z` フラグで SELinux ラベルを再設定 (Podman rootless 環境で必須)
- 検証完了後に `tmp_dir` は自動ドロップ (RAII)

### 3. SafeCommandBuilder による引数ホワイトリスト

`SafeCommandBuilder` を通じて `podman` を起動する。`podman` は既に `SecurityConfig::allowed_binaries` に含まれている (`security.rs:63`)。

**Kani 固有の引数ホワイトリスト**:

| 引数 | 目的 | 許可 |
|---|---|---|
| `--harness <name>` | 検証対象のハーネス関数を指定 | ✅ (英数字 + `_` のみ) |
| `--unwind <N>` | ループ展開の上限 | ✅ (数値 ≤ 100) |
| `--restrict-vtable-size` | vtable サイズ制限 | ✅ |
| `--output-format terse` | 出力形式 | ✅ |
| `--tests` | テスト実行 | ❌ (テスト経由の脱獄防止) |
| `--features` | 任意の feature flag | ❌ (依存関係の操作防止) |
| `-- --cfg` | 任意の cfg | ❌ (コンパイル条件の改竄防止) |

```rust
/// Kani 検証に許可される引数のバリデーション
fn validate_kani_args(args: &[String]) -> Result<(), AiomeError> {
    let allowed_flags = ["--harness", "--unwind", "--restrict-vtable-size", "--output-format"];
    for arg in args {
        if arg.starts_with("--") {
            let flag = arg.split('=').next().unwrap_or(arg);
            if !allowed_flags.contains(&flag) {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Kani argument '{}' is not whitelisted", flag),
                });
            }
        }
    }
    Ok(())
}
```

### 4. タイムアウト制御

OxiLean (ADR-038) で確立した 4-Layer Defense の **L3 (Timeout)** パターンを踏襲する:

```rust
/// Kani 検証のタイムアウト秒数 (デフォルト: 300秒)
const DEFAULT_KANI_TIMEOUT_SECS: u64 = 300;

fn kani_timeout() -> Duration {
    let secs = std::env::var("KANI_PROOF_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_KANI_TIMEOUT_SECS);
    Duration::from_secs(secs)
}
```

- `KANI_PROOF_TIMEOUT_SECS` 環境変数で外部設定可能 (12-Factor 準拠)
- OxiLean の `OXILEAN_PROOF_TIMEOUT_SECS` とは独立 (Kani はモデル検査のため、定理証明より長い実行時間が必要)
- タイムアウト時は `ZombieKiller::run_with_timeout` がプロセスを強制終了し、ゾンビプロセスを防止

### 5. 検証失敗時の IncidentStatus 遷移

Kani が検証失敗を報告した場合、インシデントを `WontFix` に自動遷移させ、無限パッチ→検証ループを防止する:

```
┌───────┐    LLM パッチ生成    ┌─────────────────┐
│ Open  │ ──────────────────→ │ PatchGenerated  │
└───────┘                     └────────┬────────┘
                                       │
                              Kani 検証開始
                                       │
                                       ▼
                              ┌─────────────────┐
                              │ KaniVerifying    │
                              └────────┬────────┘
                                       │
                          ┌────────────┴────────────┐
                          │                         │
                     検証成功                    検証失敗
                          │                    (retry_count >= MAX_RETRIES)
                          ▼                         │
                  ┌──────────────┐                  ▼
                  │ KaniSuccess  │          ┌──────────────┐
                  └──────┬───────┘          │   WontFix    │
                         │                  └──────────────┘
                    HotSwap 適用
                         │
                         ▼
                  ┌──────────────┐
                  │  HotSwapped  │
                  └──────┬───────┘
                         │
                    E2E 検証成功
                         │
                         ▼
                  ┌──────────────┐
                  │   Resolved   │
                  └──────────────┘
```

**リトライ上限**:
```rust
const MAX_KANI_RETRIES: u32 = 3;
```

- 検証失敗時、`retry_count < MAX_KANI_RETRIES` であれば LLM に失敗理由を含めて再度パッチ生成を依頼
- `retry_count >= MAX_KANI_RETRIES` で `IncidentStatus::WontFix` に遷移
- `WontFix` への遷移は `IncidentRepository::update_status()` を通じて永続化
- Prometheus メトリクス `aegis_kani_wontfix_total` をインクリメント

## Consequences

### Positive
- **完全な故障隔離**: Podman rootless + `--network=none` + `--read-only` により、悪意あるパッチコードがホストシステムに影響を与えることは不可能
- **既存インフラの活用**: `SafeCommandBuilder`, `BastionGuard`, `ZombieKiller` を再利用し、新たなセキュリティプリミティブの導入を回避
- **自律修復の安全限界**: `WontFix` 遷移により、Aegis が修復不能なバグに対して無限リソースを消費することを防止
- **OxiLean との統合**: OxiLean (定理証明) と Kani (モデル検査) の二重検証により、パッチの安全性を多角的に保証

### Negative
- **コンテナイメージの管理負荷**: `aiome/kani-verifier` イメージのビルド・更新パイプラインが必要
- **ローカル開発環境の要件**: Podman のインストールが前提条件に追加される (Stub モードでの開発は引き続き可能)
- **検証時間**: Kani のモデル検査は OxiLean の定理証明より長時間 (デフォルト 300秒) かかるため、インシデント対応のレイテンシが増加

## Related
- ADR-038: OxiLean Kernel Integration (4-Layer Defense Architecture)
- ADR-039: Aegis Sentinel Incident Repository
- `libs/infrastructure/src/aegis/prover.rs` — 実装対象
- `libs/infrastructure/src/security.rs` — `BastionGuard`, `SafeCommandBuilder`
- `libs/infrastructure/src/aegis/types.rs` — `IncidentStatus` enum
