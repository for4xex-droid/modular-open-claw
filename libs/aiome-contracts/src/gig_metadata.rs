use async_trait::async_trait;

#[async_trait]
pub trait GigMetadataUpdater: Send + Sync {
    async fn mark_as_verified(&self, skill_name: &str, oxp: u32) -> Result<(), String>;
}
