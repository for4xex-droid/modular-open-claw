use async_trait::async_trait;
use std::sync::Arc;
use crate::commerce::CommerceEngine;

/// プラグインが提供するルート。Axum に依存しない不透明なハンドル。
pub type OpaqueRouter = Box<dyn std::any::Any + Send + Sync>;

#[async_trait]
pub trait AiomePlugin: Send + Sync {
    /// プラグインの一意識別名
    fn name(&self) -> &str;

    /// プラグインのバージョン
    fn version(&self) -> &str;

    /// プラグインが提供するAPIルートを返す。
    /// 実体は axum::Router だが、型安全性を高めるために OpaqueRouter でラップする。
    fn routes(&self) -> Option<OpaqueRouter>;

    /// プラグインが提供するツール名リスト
    fn registered_tools(&self) -> Vec<String>;

    /// プラグインが動作するために必須の環境変数名リスト
    fn required_env_vars(&self) -> Vec<String>;

    /// 経済エンジン・インターフェースを返す
    fn commerce_engine(&self) -> Option<Arc<dyn CommerceEngine>>;
}
