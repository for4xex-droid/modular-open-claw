use aiome_contracts::commerce::CommerceEngine;
use anyhow::Result;
use std::sync::Arc;

pub struct CommerceEngineFactory;

impl CommerceEngineFactory {
    /// Dynamically creates the correct CommerceEngine instance depending on the
    /// provided API key and the build environment (debug vs release).
    pub async fn create(
        api_key: Option<String>,
        webhook_secret: String,
        pool: sqlx::SqlitePool,
    ) -> Result<Arc<dyn CommerceEngine>> {
        if let Some(key) = api_key {
            // If we have a key, we always use the Stripe engine
            Ok(Arc::new(crate::stripe::StripeCommerceEngine::new(
                key,
                webhook_secret,
                pool,
            )))
        } else {
            // When building in debug mode without a key, fallback to mock
            #[cfg(debug_assertions)]
            {
                tracing::warn!("⚠️ [CommerceFactory] STRIPE_API_KEY not set. Using MockCommerceEngine for development.");
                Ok(Arc::new(crate::mock::MockCommerceEngine::new()))
            }

            // In release mode, the API key is MANDATORY. Fail-closed.
            #[cfg(not(debug_assertions))]
            {
                Err(anyhow::anyhow!(
                    "🚨 [FATAL SECURITY ERROR] STRIPE_API_KEY must be set in production!"
                ))
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_commerce_factory_real_stripe_when_key_present() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let api_key = "sk_test_mock_123".to_string();
        let webhook_secret = "whsec_mock".to_string();

        let engine = CommerceEngineFactory::create(Some(api_key), webhook_secret, pool).await;

        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_commerce_factory_mock_when_no_key_in_debug() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let engine = CommerceEngineFactory::create(None, "".to_string(), pool).await;

        #[cfg(debug_assertions)]
        {
            assert!(
                engine.is_ok(),
                "Should return MockCommerceEngine in debug mode"
            );
        }

        #[cfg(not(debug_assertions))]
        {
            assert!(engine.is_err(), "Should fail in release mode if no API key");
            if let Err(e) = engine {
                assert!(e
                    .to_string()
                    .contains("STRIPE_API_KEY must be set in production"));
            }
        }
    }
}
