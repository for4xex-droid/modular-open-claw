/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use commerce_protocol::error::NurtureError;
use rand::RngCore;

/// DRM 暗号化パッケージ
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DrmPackage {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

pub struct DrmEngine;

impl DrmEngine {
    /// アセットを ChaCha20-Poly1305 で暗号化する
    pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<DrmPackage, NurtureError> {
        let cipher = ChaCha20Poly1305::new(key.into());
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| NurtureError::Infrastructure(format!("DRM 暗号化失敗: {}", e)))?;

        Ok(DrmPackage {
            nonce: nonce_bytes,
            ciphertext,
        })
    }

    /// 暗号化されたアセットを復号する
    pub fn decrypt(package: &DrmPackage, key: &[u8; 32]) -> Result<Vec<u8>, NurtureError> {
        let cipher = ChaCha20Poly1305::new(key.into());
        let nonce = Nonce::from_slice(&package.nonce);

        let plaintext = cipher
            .decrypt(nonce, package.ciphertext.as_slice())
            .map_err(|e| NurtureError::Infrastructure(format!("DRM 復号失敗: {}", e)))?;

        Ok(plaintext)
    }

    /// 暗号化用のランダムキーを生成する
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drm_roundtrip() {
        let original_data = b"NURTURE project secret asset data";
        let key = DrmEngine::generate_key();

        // Encrypt
        let package = DrmEngine::encrypt(original_data, &key).expect("Encryption failed");
        assert_ne!(original_data.to_vec(), package.ciphertext);

        // Decrypt
        let decrypted = DrmEngine::decrypt(&package, &key).expect("Decryption failed");
        assert_eq!(original_data.to_vec(), decrypted);
    }

    #[test]
    fn test_drm_wrong_key() {
        let original_data = b"secret";
        let key1 = DrmEngine::generate_key();
        let key2 = DrmEngine::generate_key();

        let package = DrmEngine::encrypt(original_data, &key1).expect("Encryption failed");
        let result = DrmEngine::decrypt(&package, &key2);
        assert!(result.is_err());
    }
}
