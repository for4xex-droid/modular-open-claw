/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use hkdf::Hkdf;
use sha2::Sha256;

/// Biome プロトコルで使用する共有鍵を導出する (HKDF RFC 5869)
pub fn derive_biome_key(hub_secret: &str) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, hub_secret.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"aiome-biome-p2p-v1", &mut okm)
        .expect("HKDF expand should not fail for 32-byte key");
    okm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_biome_key_consistency() {
        let secret = "test-secret-123";
        let key1 = derive_biome_key(secret);
        let key2 = derive_biome_key(secret);
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);
    }

    #[test]
    fn test_derive_biome_key_uniqueness() {
        let key1 = derive_biome_key("secret-a");
        let key2 = derive_biome_key("secret-b");
        assert_ne!(key1, key2);
    }
}
