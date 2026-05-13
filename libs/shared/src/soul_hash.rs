/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use std::hash::{Hash, Hasher};
use std::path::Path;

/// 内容の文字列から直接Soul Hashを計算します
pub fn compute_from_content(soul: &str, evolving_soul: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{}{}", soul, evolving_soul).hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// SOUL.mdのファイルパスから非同期にSoul Hashを計算します
pub async fn compute_from_path(soul_path: &Path) -> String {
    let soul = tokio::fs::read_to_string(soul_path)
        .await
        .unwrap_or_default();
    let evolving_soul = if let Some(parent) = soul_path.parent() {
        tokio::fs::read_to_string(parent.join("EVOLVING_SOUL.md"))
            .await
            .unwrap_or_default()
    } else {
        String::new()
    };
    compute_from_content(&soul, &evolving_soul)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_compute_from_content() {
        let soul = "I am a soul";
        let evolving = "I am evolving";
        let hash = compute_from_content(soul, evolving);

        let mut expected_hasher = std::collections::hash_map::DefaultHasher::new();
        format!("{}{}", soul, evolving).hash(&mut expected_hasher);
        let expected = format!("{:x}", expected_hasher.finish());

        assert_eq!(hash, expected);

        // Empty strings
        let hash_empty = compute_from_content("", "");
        assert!(!hash_empty.is_empty());
    }

    #[tokio::test]
    async fn test_compute_from_path() {
        let dir = tempdir().unwrap();
        let soul_path = dir.path().join("SOUL.md");
        let evolving_path = dir.path().join("EVOLVING_SOUL.md");

        let mut soul_file = std::fs::File::create(&soul_path).unwrap();
        write!(soul_file, "soul content").unwrap();

        let mut evolving_file = std::fs::File::create(&evolving_path).unwrap();
        write!(evolving_file, "evolving content").unwrap();

        let hash = compute_from_path(&soul_path).await;
        let expected = compute_from_content("soul content", "evolving content");
        assert_eq!(hash, expected);
    }

    #[tokio::test]
    async fn test_compute_from_path_missing_files() {
        let dir = tempdir().unwrap();
        let soul_path = dir.path().join("SOUL.md");

        // Neither file exists, should fallback to empty strings
        let hash = compute_from_path(&soul_path).await;
        let expected = compute_from_content("", "");
        assert_eq!(hash, expected);
    }
}
