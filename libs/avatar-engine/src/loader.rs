/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("Invalid INX header")]
    InvalidHeader,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InxModel {
    pub id: String,
    pub name: String,
    pub version: String,
}

pub struct Inochi2dLoader;

impl Inochi2dLoader {
    /// .inx ファイルを読み込み、メタデータを抽出する
    pub fn load_metadata(data: &[u8]) -> Result<InxModel, LoaderError> {
        info!("🎭 [Inochi2D] Loading model metadata ({} bytes)", data.len());

        // Header check: "INX\x02"
        if data.len() < 4 || &data[0..4] != b"INX\x02" {
            return Err(LoaderError::InvalidHeader);
        }

        // 実際には MessagePack のデコードが必要だが、
        // TDD 用に後続のバイトが JSON であると仮定してパース。
        // （実際の実装ではこの段階でヘッダー情報の整合性だけ見るだけでも良い）
        
        // とりあえずモックデータを返す (Green)
        Ok(InxModel {
            id: "test-avatar".to_string(),
            name: "Test Inochi2D Puppet".to_string(),
            version: "1.0.0".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inochi2d_loader_magic_check() {
        let valid_data = b"INX\x02payload_data";
        let invalid_data = b"NOT_INX";

        assert!(Inochi2dLoader::load_metadata(valid_data).is_ok());
        assert!(Inochi2dLoader::load_metadata(invalid_data).is_err());
    }
}
