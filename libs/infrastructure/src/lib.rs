//! # Infrastructure — I/O実装層
//!
//! `core` で定義されたトレイトの具体実装を提供する。
//! ComfyUI, FFmpeg, SQLite 等の外部サービスとの通信を担当。

pub mod comfy_bridge;
pub mod concept_manager;
pub mod factory_log;
pub mod media_forge;
pub mod trend_sonar;
pub mod voice_actor;
