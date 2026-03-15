use async_trait::async_trait;
use std::sync::Arc;
use crate::commerce::CommerceEngine;
use axum::Router;

#[async_trait]
pub trait AiomePlugin: Send + Sync {
    /// プラグインの一意識別名
    fn name(&self) -> &str;

    /// プラグインのバージョン
    fn version(&self) -> &str;

    /// プラグインが提供するAPIルートを返す。提供しない場合は None。
    fn routes(&self) -> Option<Router>;

    /// プラグインが提供する（SecurityPolicyに登録すべき）ツール名リスト
    fn registered_tools(&self) -> Vec<String>;

    /// プラグインが動作するために必須の環境変数名リスト
    fn required_env_vars(&self) -> Vec<String>;

    /// 経済エンジン・インターフェースを返す。提供しない場合は None。
    fn commerce_engine(&self) -> Option<Arc<dyn CommerceEngine>>;
}
