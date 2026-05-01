/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core_contracts::commerce::CommerceEngine;
use anyhow::Result;
use secrecy::SecretString;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    Stripe,
    Polar,
    Mock,
}

#[derive(Debug, Clone)]
pub struct CommerceConfig {
    pub provider: ProviderType,
    pub api_key: Option<SecretString>,
    pub webhook_secret: SecretString,
    pub base_url: Option<String>,
}

pub struct CommerceEngineFactory;

impl CommerceEngineFactory {
    /// Dynamically creates the correct CommerceEngine instance depending on the
    /// provided config and the build environment (debug vs release).
    pub async fn create(
        config: CommerceConfig,
        pool: sqlx::SqlitePool,
        nurture_url: Option<String>,
        nurture_secret: Option<String>,
        oxp_score_provider: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
    ) -> Result<Arc<dyn CommerceEngine>> {
        match config.provider {
            ProviderType::Stripe => {
                let key = config.api_key.ok_or_else(|| {
                    anyhow::anyhow!("STRIPE_API_KEY must be set for Stripe provider")
                })?;
                let mut engine = crate::stripe::StripeCommerceEngine::new(
                    key,
                    config.webhook_secret,
                    pool,
                    nurture_url,
                    nurture_secret,
                );
                if let Some(p) = oxp_score_provider {
                    engine = engine.with_oxp_score_provider(p);
                }
                Ok(Arc::new(engine))
            }
            ProviderType::Polar => {
                let key = config.api_key.ok_or_else(|| {
                    anyhow::anyhow!("POLAR_API_KEY must be set for Polar provider")
                })?;
                Ok(Arc::new(crate::polar::PolarCommerceEngine::new(
                    key,
                    config.webhook_secret,
                    config.base_url,
                )))
            }
            ProviderType::Mock => {
                #[cfg(debug_assertions)]
                {
                    tracing::warn!(
                        "⚠️ [CommerceFactory] Using MockCommerceEngine for local/OSS economy."
                    );
                    Ok(Arc::new(crate::mock::MockCommerceEngine::new()))
                }
                #[cfg(not(debug_assertions))]
                {
                    Err(anyhow::anyhow!(
                        "Mock provider is not allowed in production"
                    ))
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_commerce_factory_real_stripe() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap(); // allow-anti-pattern

        let config = CommerceConfig {
            provider: ProviderType::Stripe,
            api_key: Some(SecretString::from("sk_test_mock_123".to_string())),
            webhook_secret: SecretString::from("whsec_mock".to_string()),
            base_url: None,
        };

        let engine = CommerceEngineFactory::create(config, pool, None, None, None).await;

        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_commerce_factory_real_polar() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap(); // allow-anti-pattern

        let config = CommerceConfig {
            provider: ProviderType::Polar,
            api_key: Some(SecretString::from("polar_test_mock_123".to_string())),
            webhook_secret: SecretString::from("whsec_mock".to_string()),
            base_url: None,
        };

        let engine = CommerceEngineFactory::create(config, pool, None, None, None).await;

        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_commerce_factory_mock() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap(); // allow-anti-pattern

        let config = CommerceConfig {
            provider: ProviderType::Mock,
            api_key: None,
            webhook_secret: SecretString::from("".to_string()),
            base_url: None,
        };

        let engine = CommerceEngineFactory::create(config, pool, None, None, None).await;

        #[cfg(debug_assertions)]
        {
            assert!(
                engine.is_ok(),
                "Should return MockCommerceEngine in debug mode"
            );
        }

        #[cfg(not(debug_assertions))]
        {
            assert!(engine.is_err(), "Should fail in release mode for mock");
        }
    }
}
