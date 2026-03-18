use std::any::Any;
use std::future::Future;
use std::pin::Pin;

use crate::error::SoulError;
use crate::model::{AgentSoul, Experience};
use crate::defense::DefenseAction;

/// ドメインアダプター（各アプリケーションが実装）
pub trait SoulDomainAdapter: Send + Sync {
    /// 生イベント → Experience 変換
    fn to_experience(&self, raw: &dyn Any) -> Experience;

    /// 蒸留時の LLM プロンプト（ドメイン固有）
    fn distillation_system_prompt(&self) -> &str;

    /// DefenseAction をドメイン固有の副作用に変換
    fn execute_defense<'a>(&'a self, action: &'a DefenseAction) -> Pin<Box<dyn Future<Output = Result<(), SoulError>> + Send + 'a>>;

    /// 予測モデルの予測値を算出（ドメイン固有のメトリクス）
    fn predict_outcome(&self, soul: &AgentSoul, context: &Experience) -> f64;
}
