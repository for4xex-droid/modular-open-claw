use crate::error::AiomeError;
use async_trait::async_trait;
use uuid::Uuid;

/// ボイスキー・保管庫トレイト
/// 
/// ボイスアセットの復号キー（AES等）を安全に管理するためのインターフェース。
/// Abyss Vault (Security Proxy) への認証済みリクエストをラップする。
#[async_trait]
pub trait VoiceKeyVault: Send + Sync {
    /// 特定のアセットの復号キーを取得する
    /// 
    /// `agent_id`: リクエスト元のエージェント ID
    /// `asset_id`: 復号したいボイスアセットの ID
    async fn fetch_decryption_key(&self, agent_id: Uuid, asset_id: Uuid) -> Result<Vec<u8>, AiomeError>;

    /// アセットの所有権（ライセンス）を検証する
    /// 
    /// 物理的な復号を行う前に、トランザクション台帳（Ledger）と照合する。
    async fn verify_license(&self, agent_id: Uuid, asset_id: Uuid) -> Result<bool, AiomeError>;

    /// 新しいアセットのキーを Vault に登録する（クリエイターによるアップロード時）
    /// 
    /// キーは物理的に隔離されたストレージに保存される。
    async fn register_asset_key(&self, asset_id: Uuid, key: Vec<u8>) -> Result<(), AiomeError>;
}
