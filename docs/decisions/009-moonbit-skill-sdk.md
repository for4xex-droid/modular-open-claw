# ADR 009: MoonBit の採用範囲と将来計画

- **Status**: Proposed (Phase 10+ 検討事項)
- **Date**: 2026-03-20
- **Context**: WASM スキル開発の DX 向上とエコシステム拡大

## 背景

Aiome のコアは Rust で構築されており、WASM スキルも現在 Rust → `wasm32-wasi` でコンパイルされている。
しかし、Rust のビルド時間（数十秒）とスキルバイナリサイズ（数百KB〜数MB）は、TDD Forge の高速サイクルや Federation 経由のスキル共有において摩擦となっている。

MoonBit は WASM ファーストで設計された言語であり、極小バイナリ（数KB〜数十KB）と高速ビルド（1秒以下）を特徴とする。

## 決定

### ✅ 採用する領域: Skill SDK

MoonBit を **サードパーティ向けの Skill SDK 言語** として位置づける。

- `aiome-skill-sdk-moonbit` パッケージの公開
- TDD Forge のビルド時間を 10倍以上短縮
- Federation 経由のスキル転送量を 1/10 に圧縮
- コミュニティ貢献のハードルを大幅に低下

### ❌ 採用しない領域: コアエンジン

以下の理由により、コア（api-server, infrastructure, soul）への採用は行わない。

1. **メモリ安全性**: Rust の所有権システム・`unsafe` 禁止ポリシーを代替不可能
2. **物理メモリ制御**: `mlockall` / `zeroize` (Abyss Vault) は GC 言語では実装不可
3. **エコシステム成熟度**: `cargo audit` 相当のサプライチェーン監査ツールが未整備
4. **移行コスト**: 既存 10万行超の Rust コードベースの書き直しは非合理

## 実装タイムライン

| Phase | 内容 |
|---|---|
| Phase 10+ | MoonBit SDK の PoC 作成、TDD Forge との統合検証 |
| Phase 11+ | SDK 公開、コミュニティスキルの MoonBit サポート |

## 参考

- [MoonBit 公式](https://www.moonbitlang.com/)
- ADR 007 (Preserve Intent) — スキル SDK でも同ポリシーを適用
