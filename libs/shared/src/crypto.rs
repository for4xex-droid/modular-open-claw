/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use sha2::{Digest, Sha256};

/// Biome プロトコルで使用する共有鍵を導出する (HKDF-like)
pub fn derive_biome_key(hub_secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"aiome-biome-p2p");
    hasher.update(hub_secret.as_bytes());
    let derived = hasher.finalize();

    let mut key = [0u8; 32];
    key.copy_from_slice(&derived);
    key
}
