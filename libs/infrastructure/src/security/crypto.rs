/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::security::sqlite_vault_backend::get_global_master_key;
use aiome_core_contracts::error::AiomeError;
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use rand::Rng;
use sha2::Sha256;
use zeroize::Zeroizing;

/// マスターパスワードとソルトから Argon2id で鍵を導出する
pub fn derive_master_key_argon2id(
    password: &str,
    salt: &[u8],
) -> Result<Zeroizing<Vec<u8>>, AiomeError> {
    let params = Params::new(19456, 2, 1, None).map_err(|e| AiomeError::SecurityViolation {
        reason: format!("Argon2id parameter initialization failed: {}", e),
    })?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut okm = vec![0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut okm)
        .map_err(|e| AiomeError::SecurityViolation {
            reason: format!("Argon2id derivation failed: {}", e),
        })?;
    Ok(Zeroizing::new(okm))
}

/// XChaCha20Poly1305 でデータを暗号化する
/// - 鍵は 32 bytes (256 bits) であること
/// - 返り値は `[nonce(24 bytes) || ciphertext]` となる
pub fn encrypt_xchacha20poly1305(
    data: &[u8],
    key: &Zeroizing<Vec<u8>>,
) -> Result<Vec<u8>, AiomeError> {
    if key.len() != 32 {
        return Err(AiomeError::SecurityViolation {
            reason: "XChaCha20Poly1305 requires a 32-byte key".into(),
        });
    }

    let cipher =
        XChaCha20Poly1305::new_from_slice(key).map_err(|e| AiomeError::SecurityViolation {
            reason: format!("Failed to initialize cipher: {:?}", e),
        })?;

    let nonce_bytes: [u8; 24] = rand::thread_rng().gen();
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| AiomeError::SecurityViolation {
            reason: format!("Encryption failed: {:?}", e),
        })?;

    let mut result = Vec::with_capacity(24 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// XChaCha20Poly1305 でデータを復号する
/// - `encrypted_data` は `[nonce(24 bytes) || ciphertext]` の形式であること
pub fn decrypt_xchacha20poly1305(
    encrypted_data: &[u8],
    key: &Zeroizing<Vec<u8>>,
) -> Result<Vec<u8>, AiomeError> {
    if key.len() != 32 {
        return Err(AiomeError::SecurityViolation {
            reason: "XChaCha20Poly1305 requires a 32-byte key".into(),
        });
    }

    if encrypted_data.len() < 24 {
        return Err(AiomeError::SecurityViolation {
            reason: "Encrypted data is too short to contain a nonce".into(),
        });
    }

    let cipher =
        XChaCha20Poly1305::new_from_slice(key).map_err(|e| AiomeError::SecurityViolation {
            reason: format!("Failed to initialize cipher: {:?}", e),
        })?;

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(24);
    let nonce = XNonce::from_slice(nonce_bytes);

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
    let encrypted = encrypt_xchacha20poly1305(plaintext.as_bytes(), &key)?;
    Ok(hex::encode(encrypted))
}

/// Decrypt a setting value using the derived settings key.
pub fn decrypt_setting(ciphertext_hex: &str) -> Result<String, AiomeError> {
    let key = derive_settings_key()?;
    let encrypted = hex::decode(ciphertext_hex).map_err(|_| AiomeError::SecurityViolation {
        reason: "Invalid hex for setting ciphertext".into(),
    })?;
    let decrypted = decrypt_xchacha20poly1305(&encrypted, &key)?;
    String::from_utf8(decrypted).map_err(|_| AiomeError::SecurityViolation {
        reason: "Decrypted setting is not valid UTF-8".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argon2id_derivation() {
        let password = "my_super_secret_master_password";
        let salt = b"some_random_salt_for_tests";
        let key1 = derive_master_key_argon2id(password, salt).unwrap(); // allow-anti-pattern
        let key2 = derive_master_key_argon2id(password, salt).unwrap(); // allow-anti-pattern

        assert_eq!(key1.len(), 32);
        assert_eq!(
            *key1, *key2,
            "Same password and salt should derive same key"
        );

        let salt2 = b"different_salt_for_tests";
        let key3 = derive_master_key_argon2id(password, salt2).unwrap(); // allow-anti-pattern
        assert_ne!(*key1, *key3, "Different salt should derive different key");
    }

    #[test]
    fn test_encrypt_decrypt_xchacha20poly1305() {
        let key = Zeroizing::new(rand::thread_rng().gen::<[u8; 32]>().to_vec());
        let data = b"Hello, Voice DRM via XChaCha20!";

        let encrypted = encrypt_xchacha20poly1305(data, &key).unwrap(); // allow-anti-pattern

        // Nonce is 24 bytes, Auth Tag is 16 bytes.
        assert_eq!(encrypted.len(), data.len() + 24 + 16);

        // Decrypt
        let decrypted = decrypt_xchacha20poly1305(&encrypted, &key).unwrap(); // allow-anti-pattern
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encrypt_nonce_uniqueness() {
        let key = Zeroizing::new(rand::thread_rng().gen::<[u8; 32]>().to_vec());
        let data = b"Secret Voice Model";

        let encrypted1 = encrypt_xchacha20poly1305(data, &key).unwrap(); // allow-anti-pattern
        let encrypted2 = encrypt_xchacha20poly1305(data, &key).unwrap(); // allow-anti-pattern

        assert_ne!(
            encrypted1, encrypted2,
            "Encrypted payloads should differ due to unique nonces"
        );

        // Nonce should be 24 bytes long
        assert_ne!(
            &encrypted1[..24],
            &encrypted2[..24],
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
        let encrypted = encrypt_setting(plaintext).expect("encrypt_setting should succeed"); // allow-anti-pattern

        assert!(
            encrypted.len() >= 80,
            "Ciphertext hex must be >= 80 chars (24+16+...)"
        );
        assert!(
            encrypted.chars().all(|c| c.is_ascii_hexdigit()),
            "Ciphertext must be valid hex"
        );

        // Verify: decryption recovers original plaintext
        let decrypted = decrypt_setting(&encrypted).expect("decrypt_setting should succeed"); // allow-anti-pattern
        assert_eq!(
            decrypted, plaintext,
            "Roundtrip must recover original plaintext"
        );
    }

    #[test]
    fn test_encrypt_setting_nonce_uniqueness() {
        use crate::security::mlock::MlockedVec;
        let test_key = vec![0xABu8; 32];
        let _ =
            crate::security::sqlite_vault_backend::GLOBAL_MASTER_KEY.set(MlockedVec::new(test_key));

        let plaintext = "same-api-key-twice";
        if let (Ok(enc1), Ok(enc2)) = (encrypt_setting(plaintext), encrypt_setting(plaintext)) {
            assert_ne!(
                enc1, enc2,
                "Different nonces should produce different ciphertexts"
            );
        }
    }

    #[test]
    fn test_decrypt_setting_invalid_hex() {
        use crate::security::mlock::MlockedVec;
        let test_key = vec![0xABu8; 32];
        let _ =
            crate::security::sqlite_vault_backend::GLOBAL_MASTER_KEY.set(MlockedVec::new(test_key));

        let result = decrypt_setting("not-valid-hex!!!");
        assert!(result.is_err(), "Invalid hex should return error");
    }

    #[test]
    fn test_decrypt_setting_tampered_ciphertext() {
        use crate::security::mlock::MlockedVec;
        let test_key = vec![0xABu8; 32];
        let _ =
            crate::security::sqlite_vault_backend::GLOBAL_MASTER_KEY.set(MlockedVec::new(test_key));

        if let Ok(encrypted) = encrypt_setting("secret-value") {
            let mut tampered = hex::decode(&encrypted).expect("hex decode"); // allow-anti-pattern
            if tampered.len() > 30 {
                tampered[30] ^= 0xFF;
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
