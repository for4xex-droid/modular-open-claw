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
                // B-004: MockCommerceEngine is cfg-gated (test/debug/`dev-mock` feature).
                // Release + `dev-mock` still requires AIOME_DEV_MODE=1 (defense in depth).
                #[cfg(any(test, debug_assertions, feature = "dev-mock"))]
                {
                    let is_dev = std::env::var("AIOME_DEV_MODE")
                        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                        .unwrap_or(false);
                    if cfg!(debug_assertions) || is_dev {
                        tracing::warn!(
                            "⚠️ [CommerceFactory] Using MockCommerceEngine for local/OSS economy."
                        );
                        Ok(Arc::new(crate::mock::MockCommerceEngine::new()))
                    } else {
                        Err(anyhow::anyhow!(
                            "Mock provider requires AIOME_DEV_MODE=1 (dev-mock feature alone is insufficient)"
                        ))
                    }
                }
                #[cfg(not(any(test, debug_assertions, feature = "dev-mock")))]
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
            .unwrap();

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
            .unwrap();

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
            .unwrap();

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
            // Release without AIOME_DEV_MODE must Fail-Closed (even with/without feature).
            assert!(
                engine.is_err(),
                "Should fail in release mode for mock without AIOME_DEV_MODE"
            );
        }
    }

    #[tokio::test]
    async fn test_commerce_factory_mock_with_dev_mode() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let config = CommerceConfig {
            provider: ProviderType::Mock,
            api_key: None,
            webhook_secret: SecretString::from("".to_string()),
            base_url: None,
        };

        // SAFETY: test-only env; restored below. Avoid parallel pollution via unique key...
        // (commerce tests are light; set/remove around the call)
        let prev = std::env::var("AIOME_DEV_MODE").ok();
        std::env::set_var("AIOME_DEV_MODE", "1");
        let engine = CommerceEngineFactory::create(config, pool, None, None, None).await;
        match prev {
            Some(v) => std::env::set_var("AIOME_DEV_MODE", v),
            None => std::env::remove_var("AIOME_DEV_MODE"),
        }

        #[cfg(any(debug_assertions, feature = "dev-mock"))]
        {
            assert!(
                engine.is_ok(),
                "Mock must be allowed when AIOME_DEV_MODE=1 and Mock is compiled in"
            );
        }

        #[cfg(not(any(debug_assertions, feature = "dev-mock")))]
        {
            assert!(
                engine.is_err(),
                "Release without dev-mock feature cannot construct Mock"
            );
        }
    }
}
