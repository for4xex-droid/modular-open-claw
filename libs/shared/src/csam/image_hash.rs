/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use thiserror::Error;
use visual_hash::{HashAlg, HasherConfig};

/// CSAM画像ハッシュ生成に関連するエラー
#[derive(Error, Debug)]
pub enum CsamError {
    /// 画像処理中に発生したエラー
    #[error("Image processing error: {0}")]
    ImageError(#[from] image::ImageError),
    /// ハッシュ生成自体が失敗したエラー
    #[error("Hash generation failed")]
    HashError,
}

impl From<CsamError> for aiome_core_contracts::error::AiomeError {
    fn from(e: CsamError) -> Self {
        aiome_core_contracts::error::AiomeError::Infrastructure {
            reason: format!("[CSAM] {}", e),
        }
    }
}

/// 知覚ハッシュ生成器 (PhotoDNA 互換アプローチ)
pub struct ImageHasher {
    hasher: visual_hash::Hasher,
}

impl Default for ImageHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageHasher {
    /// `ImageHasher` の新しいインスタンスを作成する
    pub fn new() -> Self {
        // DCT (Discrete Cosine Transform) を使用してノイズ耐性を高める
        let hasher = HasherConfig::new()
            .hash_alg(HashAlg::Gradient) // グラフィックの差分に強いアルゴリズム
            .hash_size(16, 16) // 256bit ハッシュ (16x16)
            .to_hasher();

        Self { hasher }
    }

    /// バイト列からハッシュを計算
    pub fn compute_hash(&self, data: &[u8]) -> Result<String, CsamError> {
        let img = image::load_from_memory(data)?;
        let hash = self.hasher.hash_image(&img);
        Ok(hash.to_base64())
    }

    /// 既知の有害ハッシュリストとの照合
    pub async fn check_blacklist(
        pool: &crate::db::DatabasePool,
        hash_base64: &str,
    ) -> Result<bool, CsamError> {
        let q = format!(
            "SELECT COUNT(*) FROM csam_blacklist WHERE image_hash = {}",
            pool.ph(0)
        );
        let count: Option<i64> = match pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind(hash_base64)
                .fetch_optional(p)
                .await
                .map_err(|e| {
                    tracing::error!("CSAM Blacklist SQLite error: {}", e);
                    CsamError::HashError
                })?,
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind(hash_base64)
                .fetch_optional(p)
                .await
                .map_err(|e| {
                    tracing::error!("CSAM Blacklist Postgres error: {}", e);
                    CsamError::HashError
                })?,
        };
        Ok(count.unwrap_or(0) > 0)
    }

    /// 類似度（ハミング距離）の計算 (0.0 - 1.0)
    pub fn calculate_similarity(&self, hash_a: &str, hash_b: &str) -> f64 {
        use visual_hash::ImageHash;
        let a = ImageHash::<Vec<u8>>::from_base64(hash_a).ok();
        let b = ImageHash::<Vec<u8>>::from_base64(hash_b).ok();

        if let (Some(ha), Some(hb)) = (a, b) {
            let bits = (ha.as_bytes().len() * 8) as f64;
            if bits <= 0.0 {
                return 0.0;
            }
            let dist = ha.dist(&hb);
            1.0 - (dist as f64 / bits)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::error::AiomeError;

    #[test]
    fn csam_error_into_aiome_maps_hash_error() {
        let err: AiomeError = CsamError::HashError.into();
        assert!(matches!(
            err,
            AiomeError::Infrastructure { reason }
                if reason.contains("CSAM") || reason.contains("Hash")
        ));
    }

    #[test]
    fn test_hash_consistency() {
        let hasher = ImageHasher::new();
        // image クレートを使用してテスト画像を生成
        let mut img = image::ImageBuffer::new(10, 10);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgb([x as u8 * 10, y as u8 * 10, 0]);
        }
        let dynamic_img = image::DynamicImage::ImageRgb8(img);

        // ハッシュ計算 (DynamicImage を直接渡す)
        let hash1 = hasher.hasher.hash_image(&dynamic_img).to_base64();
        let hash2 = hasher.hasher.hash_image(&dynamic_img).to_base64();
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_is_blacklisted_db() {
        let pool = crate::db::DatabasePool::new_sqlite(":memory:")
            .await
            .unwrap();
        crate::sql_exec!(
            &pool,
            "CREATE TABLE csam_blacklist (image_hash TEXT PRIMARY KEY)"
        )
        .unwrap();
        crate::sql_exec!(
            &pool,
            "INSERT INTO csam_blacklist (image_hash) VALUES ('malicious123')"
        )
        .unwrap();

        let is_bad = ImageHasher::check_blacklist(&pool, "malicious123")
            .await
            .unwrap();
        assert!(is_bad);

        let is_bad = ImageHasher::check_blacklist(&pool, "safe_hash")
            .await
            .unwrap();
        assert!(!is_bad);
    }
}
