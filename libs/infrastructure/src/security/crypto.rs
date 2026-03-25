/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use aiome_contracts::error::AiomeError;
use rand::Rng;
use zeroize::Zeroizing;

/// AES-256-GCM でデータを暗号化する
/// - 鍵は 32 bytes (256 bits) であること
/// - 返り値は `[nonce(12 bytes) || ciphertext]` となる
pub fn encrypt_aes256gcm(data: &[u8], key: &Zeroizing<Vec<u8>>) -> Result<Vec<u8>, AiomeError> {
    if key.len() != 32 {
        return Err(AiomeError::SecurityViolation {
            reason: "AES-256-GCM requires a 32-byte key".into(),
        });
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| AiomeError::SecurityViolation {
        reason: format!("Failed to initialize cipher: {:?}", e),
    })?;

    let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| AiomeError::SecurityViolation {
            reason: format!("Encryption failed: {:?}", e),
        })?;

    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// AES-256-GCM でデータを復号する
/// - `encrypted_data` は `[nonce(12 bytes) || ciphertext]` の形式であること
pub fn decrypt_aes256gcm(
    encrypted_data: &[u8],
    key: &Zeroizing<Vec<u8>>,
) -> Result<Vec<u8>, AiomeError> {
    if key.len() != 32 {
        return Err(AiomeError::SecurityViolation {
            reason: "AES-256-GCM requires a 32-byte key".into(),
        });
    }

    if encrypted_data.len() < 12 {
        return Err(AiomeError::SecurityViolation {
            reason: "Encrypted data is too short to contain a nonce".into(),
        });
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| AiomeError::SecurityViolation {
        reason: format!("Failed to initialize cipher: {:?}", e),
    })?;

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AiomeError::SecurityViolation {
            reason: format!("Decryption failed: {:?}", e),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_aes256gcm() {
        let key = Zeroizing::new(rand::thread_rng().gen::<[u8; 32]>().to_vec());
        let data = b"Hello, Voice DRM!";

        let encrypted = encrypt_aes256gcm(data, &key).unwrap();

        // Nonce is 12 bytes, Auth Tag is 16 bytes.
        assert_eq!(encrypted.len(), data.len() + 12 + 16);

        // Decrypt
        let decrypted = decrypt_aes256gcm(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encrypt_nonce_uniqueness() {
        // 同じ鍵、同じデータでもNonceが異なるため暗号文が異なること（§SEC-1 Nonce運用仕様）
        let key = Zeroizing::new(rand::thread_rng().gen::<[u8; 32]>().to_vec());
        let data = b"Secret Voice Model";

        let encrypted1 = encrypt_aes256gcm(data, &key).unwrap();
        let encrypted2 = encrypt_aes256gcm(data, &key).unwrap();

        assert_ne!(
            encrypted1, encrypted2,
            "Encrypted payloads should differ due to unique nonces"
        );

        // None should be 12 bytes long
        assert_ne!(
            &encrypted1[..12],
            &encrypted2[..12],
            "Nonces should be unique"
        );
    }
}
