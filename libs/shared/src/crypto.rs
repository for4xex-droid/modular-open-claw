/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use hkdf::Hkdf;
use sha2::Sha256;

/// Commune プロトコルで使用する共有鍵を導出する (HKDF RFC 5869)
pub fn derive_commune_key(hub_secret: &str) -> Result<[u8; 32], String> {
    let hk = Hkdf::<Sha256>::new(None, hub_secret.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"aiome-commune-p2p-v1", &mut okm)
        .map_err(|_| "HKDF expand failed for 32-byte key".to_string())?;
    Ok(okm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_commune_key_consistency() {
        let secret = "test-secret-123";
        let key1 = derive_commune_key(secret).unwrap();
        let key2 = derive_commune_key(secret).unwrap();
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);
    }

    #[test]
    fn test_derive_commune_key_uniqueness() {
        let key1 = derive_commune_key("secret-a").unwrap();
        let key2 = derive_commune_key("secret-b").unwrap();
        assert_ne!(key1, key2);
    }
}
