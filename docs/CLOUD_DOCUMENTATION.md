# 🌌 Documentation Hub - Aiome

Welcome to the Aiome project documentation. This wiki is automatically generated.

## 🏗️ Architecture Overviews

### Apps
- [management-console](./management-console.md)
- [api-server](./api-server.md)
- [aiome-node](./aiome-node.md)
- [samsara-hub](./samsara-hub.md)
- [shadow-worker](./shadow-worker.md)
- [timesfm-sidecar](./timesfm-sidecar.md)
- [key-proxy](./key-proxy.md)
- [aiome-migrate](./aiome-migrate.md)

### Libs
- [core](./core.md)
- [napi-bridge](./napi-bridge.md)
- [shared](./shared.md)
- [avatar-engine](./avatar-engine.md)
- [aiome-commerce](./aiome-commerce.md)
- [aiome-core-contracts](./aiome-core-contracts.md)
- [aiome-contracts](./aiome-contracts.md)
- [wasm-skills](./wasm-skills.md)
- [soul](./soul.md)
- [infrastructure](./infrastructure.md)

## 🛠️ Operations

- [Backup Strategy](./operations/BACKUP.md)

## 🛡️ Iron Principles

- **Result Type Mandatory**: `unwrap()` and `expect()` are forbidden.
- **Async/Await**: Using `tokio` for non-blocking I/O.
- **Workspace Structure**: Strict dependency directions.
