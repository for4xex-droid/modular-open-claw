//! # Core — ドメインロジック層
//!
//! ShortsFactory のビジネスロジックを定義する。
//! 具体的なI/O実装は `infrastructure` クレートに委譲する（依存性逆転の原則）。

pub mod error;
pub mod traits;
pub mod contracts;
