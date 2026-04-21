/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::security::sqlite_vault_backend::get_global_master_key;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use aiome_core_contracts::error::AiomeError;
use hkdf::Hkdf;
use rand::Rng;
use sha2::Sha256;
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

/// Derive a key specifically for encrypting DB settings using HKDF.
fn derive_settings_key() -> Result<Zeroizing<Vec<u8>>, AiomeError> {
    let master = get_global_master_key()?;
    let hk = Hkdf::<Sha256>::new(None, master.as_slice());
    let mut okm = vec![0u8; 32];
    hk.expand(b"aiome-settings-encryption", &mut okm)
        .map_err(|_| AiomeError::SecurityViolation {
            reason: "HKDF derivation failed".into(),
        })?;
    Ok(Zeroizing::new(okm))
}

/// Encrypt a setting value using the derived settings key.
pub fn encrypt_setting(plaintext: &str) -> Result<String, AiomeError> {
    let key = derive_settings_key()?;
    let encrypted = encrypt_aes256gcm(plaintext.as_bytes(), &key)?;
    Ok(hex::encode(encrypted))
}

/// Decrypt a setting value using the derived settings key.
pub fn decrypt_setting(ciphertext_hex: &str) -> Result<String, AiomeError> {
    let key = derive_settings_key()?;
    let encrypted = hex::decode(ciphertext_hex).map_err(|_| AiomeError::SecurityViolation {
        reason: "Invalid hex for setting ciphertext".into(),
    })?;
    let decrypted = decrypt_aes256gcm(&encrypted, &key)?;
    String::from_utf8(decrypted).map_err(|_| AiomeError::SecurityViolation {
        reason: "Decrypted setting is not valid UTF-8".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_aes256gcm() {
        let key = Zeroizing::new(rand::thread_rng().gen::<[u8; 32]>().to_vec());
        let data = b"Hello, Voice DRM!";

        let encrypted = encrypt_aes256gcm(data, &key).unwrap(); // allow-anti-pattern

        // Nonce is 12 bytes, Auth Tag is 16 bytes.
        assert_eq!(encrypted.len(), data.len() + 12 + 16);

        // Decrypt
        let decrypted = decrypt_aes256gcm(&encrypted, &key).unwrap(); // allow-anti-pattern
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encrypt_nonce_uniqueness() {
        // 同じ鍵、同じデータでもNonceが異なるため暗号文が異なること（§SEC-1 Nonce運用仕様）
        let key = Zeroizing::new(rand::thread_rng().gen::<[u8; 32]>().to_vec());
        let data = b"Secret Voice Model";

        let encrypted1 = encrypt_aes256gcm(data, &key).unwrap(); // allow-anti-pattern
        let encrypted2 = encrypt_aes256gcm(data, &key).unwrap(); // allow-anti-pattern

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

    #[test]
    fn test_encrypt_decrypt_setting_roundtrip() {
        // Initialize GLOBAL_MASTER_KEY for this test
        use crate::security::mlock::MlockedVec;

        // Force-init the global key (idempotent if already set)
        let test_key = vec![0xABu8; 32];
        let _ =
            crate::security::sqlite_vault_backend::GLOBAL_MASTER_KEY.set(MlockedVec::new(test_key));

        let plaintext = "sk-proj-abc123XYZ_super_secret_key";
        let encrypted = encrypt_setting(plaintext).expect("encrypt_setting should succeed");

        // Verify: encrypted is valid hex and longer than 56 chars (28 bytes minimum)
        assert!(encrypted.len() >= 56, "Ciphertext hex must be >= 56 chars");
        assert!(
            encrypted.chars().all(|c| c.is_ascii_hexdigit()),
            "Ciphertext must be valid hex"
        );

        // Verify: decryption recovers original plaintext
        let decrypted = decrypt_setting(&encrypted).expect("decrypt_setting should succeed");
        assert_eq!(
            decrypted, plaintext,
            "Roundtrip must recover original plaintext"
        );
    }

    #[test]
    fn test_encrypt_setting_nonce_uniqueness() {
        let plaintext = "same-api-key-twice";
        // Two encryptions of the same plaintext should produce different ciphertexts
        if let (Ok(enc1), Ok(enc2)) = (encrypt_setting(plaintext), encrypt_setting(plaintext)) {
            assert_ne!(
                enc1, enc2,
                "Different nonces should produce different ciphertexts"
            );
        }
    }

    #[test]
    fn test_decrypt_setting_invalid_hex() {
        let result = decrypt_setting("not-valid-hex!!!");
        assert!(result.is_err(), "Invalid hex should return error");
    }

    #[test]
    fn test_decrypt_setting_tampered_ciphertext() {
        if let Ok(encrypted) = encrypt_setting("secret-value") {
            // Tamper with one byte in the middle of the ciphertext
            let mut tampered = hex::decode(&encrypted).expect("hex decode");
            if tampered.len() > 20 {
                tampered[20] ^= 0xFF;
            }
            let tampered_hex = hex::encode(tampered);
            let result = decrypt_setting(&tampered_hex);
            assert!(
                result.is_err(),
                "Tampered ciphertext must fail authentication"
            );
        }
    }
}
