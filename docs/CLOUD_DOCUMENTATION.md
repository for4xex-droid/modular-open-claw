# 🌌 Documentation Hub - Aiome
Welcome to the Aiome project documentation. This wiki is automatically generated.

## 🏗️ Architecture & Constitution

- **[Lex AI Constitution](./architecture/ARCHITECTURE_LAW.md)**: AI 都市建築基準法。アクターの境界、契約、統治を規定。
- **[Security Design](./architecture/SECURITY_DESIGN.md)**: 多層防御、Abyss Vault、ゼロトラスト設計。
- **[Evolution Strategy](./architecture/EVOLUTION_STRATEGY.md)**: 自己進化と育成システムの設計思想。
- **[Layout Architecture](./architecture/LAYOUT_ARCHITECTURE.md)**: CSS Custom Properties によるレイアウト一元管理。

## 📖 User & Operations Guides

- **[Operations Manual](./guides/OPERATIONS_MANUAL.md)**: 環境構築、起動手順、監視、監視所（ruri）運用。
- **[Watchtower User Guide](./guides/WATCHTOWER_USER_GUIDE.md)**: Discord連携、コマンド一覧、育成システム。
- **[Skills Manual](./guides/SKILLS_MANUAL.md)**: AIエージェントの基本操作とツール利用ガイド。
- **[Soul Customization](./guides/CUSTOMIZING_SOUL.md)**: AIの性格、反応、人格定義の調整。
- **[Channel Setup Guide](./guides/CHANNEL_SETUP_GUIDE.md)**: Discord チャンネルと権限の設定。

## 🛡️ Specifications & Technicals

- **[Skill Forge Spec](./specs/SKILL_FORGE_SPEC.md)**: WASMスキル生成と Forge サンドボックスの仕様。
- **[Avatar System Spec](./specs/AVATAR_SYSTEM_SPEC.md)**: AIアバター生成と動的演出プロトコル。
- **[Security Whitepaper](./specs/SECURITY_WHITEPAPER.md)**: セキュリティ設計概説（顧客向け）。

## 🛡️ Iron Principles

- **Result Type Mandatory**: `unwrap()` and `expect()` are forbidden outside tests.
- **Lex AI Compliance**: Actors MUST use `Jail`, `Contracts`, and run under a `Supervisor`.
- **Resource Discipline**: Every component must be `HealthMonitor` friendly and use `Secret<T>` for sensitive data.
- **Fail-Safe Design**: Default to `DENY`. Security violations trigger immediate isolation.
- **Async/Await**: Powered by `tokio` for high-performance non-blocking operations.
