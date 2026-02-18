# 🌌 CODE Wiki - Antigravity

Welcome to the Antigravity project documentation. This wiki is automatically generated.

## 🏗️ Architecture & Constitution

- **[Lex AI Constitution](./ARCHITECTURE_LAW.md)**: AI 都市建築基準法。アクターの境界、契約、統治を規定。
- **[Apps](./api-server.md)**: `api-server` (Dashboard), `shorts-factory` (Industrial Core).
- **[Libs](./core.md)**: `core` (Traits/Contracts), `shared` (Utils/Health), `infrastructure` (Tools).

## 🛡️ Iron Principles

- **Result Type Mandatory**: `unwrap()` and `expect()` are forbidden outside tests.
- **Lex AI Compliance**: Actors MUST use `Jail`, `Contracts`, and run under a `Supervisor`.
- **Resource Discipline**: Every component must be `HealthMonitor` friendly and use `Secret<T>` for sensitive data.
- **Fail-Safe Design**: Default to `DENY`. Security violations trigger immediate isolation.
- **Async/Await**: Powered by `tokio` for high-performance non-blocking operations.
