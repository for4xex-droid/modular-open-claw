/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! ファイル検証モジュール
//!
//! Magic Bytes (ファイルシグネチャ) を検査し、拡張子の偽装（例: `.jpg` に偽装した `.php` や `.sh`）
//! を防止するためのセキュリティ検証を提供する。

use aiome_core_contracts::error::AiomeError;
use std::path::Path;

/// サポートされているファイルタイプとその Magic Bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// JPEG 画像 (FF D8 FF)
    Jpeg,
    /// PNG 画像 (89 50 4E 47 0D 0A 1A 0A)
    Png,
    /// GIF 画像 (GIF87a / GIF89a)
    Gif,
    /// PDF ドキュメント (%PDF-)
    Pdf,
    /// WAV 音声 (RIFF ... WAVE)
    Wav,
    /// WebP 画像 (RIFF ... WEBP)
    Webp,
    /// INX (Inochi2D) - 実態は ZIP アーカイブ (PK\x03\x04)
    Inx,
}

impl FileType {
    /// 拡張子から予測される FileType を取得
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "gif" => Some(Self::Gif),
            "pdf" => Some(Self::Pdf),
            "wav" => Some(Self::Wav),
            "webp" => Some(Self::Webp),
            "inx" => Some(Self::Inx),
            _ => None,
        }
    }

    /// 対象のシグネチャが一致しているか判定
    pub fn matches_magic_bytes(&self, bytes: &[u8]) -> bool {
        if bytes.is_empty() {
            return false;
        }

        match self {
            Self::Jpeg => bytes.starts_with(&[0xFF, 0xD8, 0xFF]) && bytes.ends_with(&[0xFF, 0xD9]),
            Self::Png => {
                bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
                    // 終端シグネチャ (IENDチャンク + CRC32) を確認して、PHPなどの追記型ポリグロットをブロック
                    && bytes.ends_with(&[0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82])
            }
            Self::Gif => {
                (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"))
                    && bytes.ends_with(&[0x3B]) // Trailer character ';'
            }
            Self::Pdf => {
                // PDF は %%EOF で終わるが、末尾に改行 (\n, \r\nなど) が許容されることが多いため少し許容範囲を持たせる
                if !bytes.starts_with(b"%PDF-") {
                    return false;
                }
                // 最後の32バイト以内に %%EOF が存在するか検索
                let end_chunk = if bytes.len() > 32 {
                    &bytes[bytes.len() - 32..]
                } else {
                    bytes
                };
                end_chunk.windows(5).any(|w| w == b"%%EOF")
            }
            Self::Wav => {
                bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE"
            }
            Self::Webp => {
                bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP"
            }
            Self::Inx => {
                // Inochi2D internal format signature (INX\x02)
                bytes.starts_with(b"INX\x02")
            }
        }
    }
}

/// ファイルのバイト列と主張する拡張子が一致するか検証する
pub fn validate_magic_bytes(claimed_extension: &str, bytes: &[u8]) -> Result<(), AiomeError> {
    let expected_type = FileType::from_extension(claimed_extension).ok_or_else(|| {
        AiomeError::SecurityViolation {
            reason: format!("Unsupported file extension: {}", claimed_extension),
        }
    })?;

    if !expected_type.matches_magic_bytes(bytes) {
        return Err(AiomeError::SecurityViolation {
            reason: format!("Magic bytes mismatch for extension: {}", claimed_extension),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_png() {
        // Start with PNG magic bytes, end with IEND chunk (49 45 4E 44 AE 42 60 82)
        let mut png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        png_bytes.extend_from_slice(&[0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82]);
        let result = validate_magic_bytes("png", &png_bytes);
        assert!(result.is_ok(), "Valid PNG bytes should pass verification");
    }

    #[test]
    fn test_spoofed_png_with_php() {
        // Starts with PNG magic bytes, but ends with PHP instead of IEND (polyglot attack)
        let mut malicious_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        malicious_bytes.extend_from_slice(b"<?php echo 'hack'; ?>");
        let result = validate_magic_bytes("png", &malicious_bytes);
        assert!(
            result.is_err(),
            "Spoofed PNG containing PHP should be blocked"
        );
    }

    #[test]
    fn test_spoofed_jpeg_with_sh() {
        // Starts with JPEG, ends with script payload
        let mut malicious_bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
        malicious_bytes.extend_from_slice(b"#!/bin/sh\necho pwned");
        let result = validate_magic_bytes("jpg", &malicious_bytes);
        assert!(
            result.is_err(),
            "Spoofed JPG containing script should be blocked"
        );
    }

    #[test]
    fn test_valid_jpeg() {
        // Valid JPEG ends with FF D9
        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0xFF, 0xD9];
        let result = validate_magic_bytes("jpg", &jpeg_bytes);
        assert!(result.is_ok(), "Valid JPEG bytes should pass verification");
    }

    #[test]
    fn test_unknown_extension_rejected() {
        let some_bytes = [0x00, 0x01];
        let result = validate_magic_bytes("xyz", &some_bytes);
        assert!(
            result.is_err(),
            "Unsupported or unknown extensions should be rejected"
        );
    }
}
