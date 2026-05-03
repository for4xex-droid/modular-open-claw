/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
pub mod federation;
pub mod ws;
pub fn verify_bearer(auth_header: &str, secret: &secrecy::SecretString) -> bool {
    use secrecy::ExposeSecret;
    use subtle::ConstantTimeEq;
    let expected = format!("Bearer {}", secret.expose_secret());
    // SEC: Always perform constant-time comparison regardless of length to prevent timing leaks
    let max_len = std::cmp::max(auth_header.len(), expected.len());
    let mut a = vec![0u8; max_len];
    let mut b = vec![0u8; max_len];
    a[..auth_header.len()].copy_from_slice(auth_header.as_bytes());
    b[..expected.len()].copy_from_slice(expected.as_bytes());
    auth_header.len() == expected.len() && bool::from(a.ct_eq(&b))
}
