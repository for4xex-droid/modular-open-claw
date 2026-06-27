/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use base64::{prelude::BASE64_STANDARD, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use curve25519_dalek::edwards::CompressedEdwardsY;
use hkdf::Hkdf;
use rand::{thread_rng, RngCore};
use sha2::{Digest, Sha256, Sha512};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, StaticSecret};

/// Ed25519 公開鍵（32バイト）から X25519 公開鍵への Montgomery 変換
pub fn ed25519_pubkey_to_x25519(ed_bytes: &[u8; 32]) -> Result<X25519PublicKey, String> {
    let compressed = CompressedEdwardsY(*ed_bytes);
    let edwards_point = compressed
        .decompress()
        .ok_or_else(|| "Invalid Ed25519 public key bytes".to_string())?;
    let montgomery_point = edwards_point.to_montgomery();
    Ok(X25519PublicKey::from(montgomery_point.to_bytes()))
}

/// Ed25519 秘密鍵シード（32バイト）から固定 X25519 秘密鍵（StaticSecret）の導出
pub fn ed25519_secret_to_x25519(ed_seed: &[u8; 32]) -> StaticSecret {
    let mut hasher = Sha512::new();
    hasher.update(ed_seed);
    let hash = hasher.finalize();
    let mut x25519_seed = [0u8; 32];
    x25519_seed.copy_from_slice(&hash[..32]);
    StaticSecret::from(x25519_seed)
}

/// 1-RTT / 0-RTT エンドツーエンド暗号化
///
/// 受信側の Ed25519 公開鍵と送信元の Ed25519 秘密鍵シード（アイデンティティ）を用いて暗号化する。
/// 出力バイナリ構造： `EphemeralPublicKey (32 bytes) || Nonce (12 bytes) || Ciphertext`
pub fn encrypt_message(content: &str, recipient_pub_ed_bytes: &[u8; 32]) -> Result<String, String> {
    // 1. 相手の Ed25519 公開鍵から X25519 公開鍵を導出
    let recipient_x25519_pub = ed25519_pubkey_to_x25519(recipient_pub_ed_bytes)?;

    // 2. 一時的な Ephemeral X25519 鍵ペアを生成
    let mut rng = thread_rng();
    let ephemeral_secret = EphemeralSecret::random_from_rng(&mut rng);
    let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);

    // 3. Diffie-Hellman 共有秘密の計算
    let shared_secret = ephemeral_secret.diffie_hellman(&recipient_x25519_pub);

    // 4. HKDF-SHA256 によるセッション対称鍵の導出
    let hk = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
    let mut session_key = zeroize::Zeroizing::new([0u8; 32]);
    hk.expand(b"Commune-P2P-E2E-Encryption", &mut *session_key)
        .map_err(|e| e.to_string())?;

    // 5. ChaCha20Poly1305 による暗号化
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&*session_key));
    let mut nonce_bytes = zeroize::Zeroizing::new([0u8; 12]);
    rng.fill_bytes(&mut *nonce_bytes);
    let nonce = Nonce::from_slice(&*nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, content.as_bytes())
        .map_err(|e| format!("Encryption failed: {:?}", e))?;

    // 6. パケット構築: EphemeralPublicKey || Nonce || Ciphertext
    let mut packet = Vec::with_capacity(32 + 12 + ciphertext.len());
    packet.extend_from_slice(ephemeral_public.as_bytes());
    packet.extend_from_slice(&*nonce_bytes);
    packet.extend_from_slice(&ciphertext);

    Ok(BASE64_STANDARD.encode(packet))
}

/// 1-RTT / 0-RTT エンドツーエンド復号
///
/// 受信側（自己）の Ed25519 秘密鍵シードを用いて暗号メッセージを復号する。
pub fn decrypt_message(content_b64: &str, recipient_seed: &[u8; 32]) -> Result<String, String> {
    // 1. パケットの Base64 デコード
    let packet = BASE64_STANDARD
        .decode(content_b64)
        .map_err(|e| format!("Invalid Base64: {}", e))?;

    if packet.len() < 32 + 12 {
        return Err("Ciphertext packet too short".to_string());
    }

    // 2. パケット分解: EphemeralPublicKey, Nonce, Ciphertext
    let (ephemeral_pub_bytes, rest) = packet.split_at(32);
    let (nonce_bytes, ciphertext) = rest.split_at(12);

    let ephemeral_public = X25519PublicKey::from(
        <[u8; 32]>::try_from(ephemeral_pub_bytes)
            .map_err(|_| "Failed to parse ephemeral public key")?,
    );

    // 3. 受信側の固定 X25519 秘密鍵を導出
    let recipient_x25519_sec = ed25519_secret_to_x25519(recipient_seed);

    // 4. Diffie-Hellman 共有秘密の計算
    let shared_secret = recipient_x25519_sec.diffie_hellman(&ephemeral_public);

    // 5. HKDF-SHA256 によるセッション対称鍵の導出
    let hk = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
    let mut session_key = zeroize::Zeroizing::new([0u8; 32]);
    hk.expand(b"Commune-P2P-E2E-Encryption", &mut *session_key)
        .map_err(|e| e.to_string())?;

    // 6. ChaCha20Poly1305 による復号
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&*session_key));
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {:?}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8: {}", e))
}

/// Commune プロトコルで使用する共有鍵を導出する (HKDF RFC 5869)
pub fn derive_commune_key(hub_secret: &str) -> Result<zeroize::Zeroizing<[u8; 32]>, String> {
    let hk = Hkdf::<Sha256>::new(None, hub_secret.as_bytes());
    let mut okm = zeroize::Zeroizing::new([0u8; 32]);
    hk.expand(b"aiome-commune-p2p-v1", &mut *okm)
        .map_err(|_| "HKDF expand failed for 32-byte key".to_string())?;
    Ok(okm)
}

/// Zero-Metadata Commune Envelope の暗号化ラッパー
pub fn encrypt_commune_envelope(
    message: &aiome_core_contracts::commune::CommuneMessage,
    recipient_pub_ed_bytes: &[u8; 32],
    channel_local_id: String,
) -> Result<aiome_core_contracts::commune::ZeroMetadataCommuneEnvelope, String> {
    // 1. CommuneMessage を JSON シリアライズ
    let message_json = serde_json::to_string(message)
        .map_err(|e| format!("Failed to serialize CommuneMessage: {}", e))?;

    // 2. encrypt_message を呼び出して暗号化・Base64 化
    let encrypted_payload = encrypt_message(&message_json, recipient_pub_ed_bytes)?;

    Ok(aiome_core_contracts::commune::ZeroMetadataCommuneEnvelope {
        channel_local_id,
        encrypted_payload,
    })
}

/// Zero-Metadata Commune Envelope の復号ラッパー
pub fn decrypt_commune_envelope(
    envelope: &aiome_core_contracts::commune::ZeroMetadataCommuneEnvelope,
    recipient_seed: &[u8; 32],
) -> Result<aiome_core_contracts::commune::CommuneMessage, String> {
    // 1. decrypt_message を呼び出して復号し平文の JSON 文字列を取得
    let plaintext_json = decrypt_message(&envelope.encrypted_payload, recipient_seed)?;

    // 2. JSON から CommuneMessage へデシリアライズ
    let message =
        serde_json::from_str::<aiome_core_contracts::commune::CommuneMessage>(&plaintext_json)
            .map_err(|e| format!("Failed to deserialize CommuneMessage: {}", e))?;

    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_derive_commune_key_consistency() {
        let secret = "test-secret-123";
        let key1: zeroize::Zeroizing<[u8; 32]> = derive_commune_key(secret).unwrap();
        let key2 = derive_commune_key(secret).unwrap();
        assert_eq!(*key1, *key2);
        assert_eq!(key1.len(), 32);
    }

    #[test]
    fn test_derive_commune_key_uniqueness() {
        let key1 = derive_commune_key("secret-a").unwrap();
        let key2 = derive_commune_key("secret-b").unwrap();
        assert_ne!(*key1, *key2);
    }

    #[test]
    fn test_ed25519_to_x25519_conversion_roundtrip() {
        let mut rng = thread_rng();
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);

        // Ed25519 鍵ペア生成
        let ed_signing = SigningKey::from_bytes(&seed);
        let ed_verifying = ed_signing.verifying_key();
        let ed_pub_bytes = ed_verifying.to_bytes();

        // X25519 への Montgomery 変換
        let x25519_sec = ed25519_secret_to_x25519(&seed);
        let x25519_pub_converted = ed25519_pubkey_to_x25519(&ed_pub_bytes).unwrap();

        // X25519 秘密鍵から直接導出した公開鍵と一致するか検証
        let x25519_pub_direct = X25519PublicKey::from(&x25519_sec);
        assert_eq!(
            x25519_pub_converted.as_bytes(),
            x25519_pub_direct.as_bytes()
        );
    }

    #[test]
    fn test_e2e_encryption_decryption_success() {
        let mut rng = thread_rng();

        // 受信側の Ed25519 鍵ペア
        let mut recipient_seed = [0u8; 32];
        rng.fill_bytes(&mut recipient_seed);
        let recipient_signing = SigningKey::from_bytes(&recipient_seed);
        let recipient_pub_bytes = recipient_signing.verifying_key().to_bytes();

        let message = "This is a highly confidential P2P swarm message! 🚀";

        // 正常系: 暗号化
        let ciphertext = encrypt_message(message, &recipient_pub_bytes).unwrap();
        assert_ne!(ciphertext, message);

        // 正常系: 復号
        let decrypted = decrypt_message(&ciphertext, &recipient_seed).unwrap();
        assert_eq!(decrypted, message);
    }

    #[test]
    fn test_e2e_decryption_failure_with_wrong_key() {
        let mut rng = thread_rng();

        // 受信側の鍵
        let mut recipient_seed = [0u8; 32];
        rng.fill_bytes(&mut recipient_seed);
        let recipient_signing = SigningKey::from_bytes(&recipient_seed);
        let recipient_pub_bytes = recipient_signing.verifying_key().to_bytes();

        // 別の第三者の鍵
        let mut wrong_seed = [0u8; 32];
        rng.fill_bytes(&mut wrong_seed);

        let message = "Confidential payload";

        // 暗号化
        let ciphertext = encrypt_message(message, &recipient_pub_bytes).unwrap();

        // 異常系: 別の鍵で復号 → 失敗するはず
        let decrypt_result = decrypt_message(&ciphertext, &wrong_seed);
        assert!(decrypt_result.is_err());
    }

    #[test]
    fn test_e2e_decryption_failure_with_corrupt_packet() {
        let mut rng = thread_rng();

        let mut recipient_seed = [0u8; 32];
        rng.fill_bytes(&mut recipient_seed);
        let recipient_signing = SigningKey::from_bytes(&recipient_seed);
        let recipient_pub_bytes = recipient_signing.verifying_key().to_bytes();

        let ciphertext = encrypt_message("Secret data", &recipient_pub_bytes).unwrap();

        // 異常系: パケット改ざん (一部書き換え)
        let mut decoded = BASE64_STANDARD.decode(ciphertext).unwrap();
        if decoded.len() > 44 {
            // 暗号文部分の末尾のバイトを改ざん
            let len = decoded.len();
            decoded[len - 1] ^= 0xFF;
        }
        let tampered_b64 = BASE64_STANDARD.encode(decoded);

        let decrypt_result = decrypt_message(&tampered_b64, &recipient_seed);
        assert!(
            decrypt_result.is_err(),
            "Expected authentication tag validation failure"
        );
    }

    #[test]
    fn test_zero_metadata_envelope_encryption_roundtrip() {
        use aiome_core_contracts::commune::CommuneMessage;

        let mut rng = thread_rng();

        // 受信側の Ed25519 鍵ペア
        let mut recipient_seed = [0u8; 32];
        rng.fill_bytes(&mut recipient_seed);
        let recipient_signing = SigningKey::from_bytes(&recipient_seed);
        let recipient_pub_bytes = recipient_signing.verifying_key().to_bytes();

        // ダミーの CommuneMessage 構築
        let original_msg = CommuneMessage {
            sender_pubkey: "sender_pubkey_example".to_string(),
            recipient_pubkey: "recipient_pubkey_example".to_string(),
            topic_id: "test_topic_123".to_string(),
            content: "Hello, Zero-Metadata World! 🔒".to_string(),
            karma_root_cid: "QmXxx".to_string(),
            signature: "sig_example".to_string(),
            lamport_clock: 42,
            timestamp: "2026-06-27T00:00:00Z".to_string(),
            encryption: "none".to_string(),
            payload_type: None,
        };

        // 暗号化
        let test_channel_id = "test_channel_123_xyz".to_string();
        let envelope =
            encrypt_commune_envelope(&original_msg, &recipient_pub_bytes, test_channel_id.clone())
                .unwrap();
        assert_eq!(envelope.channel_local_id, test_channel_id);

        // 復号
        let decrypted_msg = decrypt_commune_envelope(&envelope, &recipient_seed).unwrap();
        assert_eq!(decrypted_msg.topic_id, original_msg.topic_id);
        assert_eq!(decrypted_msg.content, original_msg.content);
    }
}
