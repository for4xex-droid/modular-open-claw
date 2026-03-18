use std::future::Future;
use std::pin::Pin;

use crate::error::SoulError;
use crate::model::AgentSoul;
use crate::instinct::Instinct;

/// 転生のコアロジック（ドメイン非依存）
pub trait SamsaraEngine: Send + Sync {
    /// 経験を蒸留し、本能に変換する
    fn distill<'a>(&'a self, soul: &'a AgentSoul) -> Pin<Box<dyn Future<Output = Result<Instinct, SoulError>> + Send + 'a>>;

    /// 転生を実行（旧魂を消費し、新魂を生成）
    fn rebirth<'a>(&'a self, soul: AgentSoul) -> Pin<Box<dyn Future<Output = Result<AgentSoul, SoulError>> + Send + 'a>>;

    /// 衝撃判定（PredictiveModelの予測誤差が閾値超過等）
    fn is_shock(&self, soul: &AgentSoul) -> bool {
        soul.predictive_model.domains.values()
            .any(|dm| dm.last_surprise > self.shock_threshold())
    }

    fn shock_threshold(&self) -> f64 {
        0.8
    }
}
