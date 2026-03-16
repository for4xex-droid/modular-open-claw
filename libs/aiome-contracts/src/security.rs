/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

/// 🛡️ PermissionManifest
/// 
/// エージェントやプラグインに許可される「物理的操作」の宣言。
/// Contracts 層に定義することで、AIが勝手に権限を昇格できないように「物理ロック」をかける。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionManifest {
    pub allow_network: bool,
    pub allow_filesystem_write: bool,
    pub allow_shell_execution: bool,
    pub allowed_domains: Vec<String>,
}

impl Default for PermissionManifest {
    fn default() -> Self {
        Self {
            allow_network: false,
            allow_filesystem_write: false,
            allow_shell_execution: false,
            allowed_domains: vec![],
        }
    }
}

/// ⛓️ RuntimeJail
/// 
/// エージェントのアクションを閉じ込める「檻」。
/// 物理的な入出力を行うすべてのコンポーネントは、このインターフェースを介して
/// 検証を受ける。
pub trait RuntimeJail: Send + Sync {
    /// シェルコマンドの実行を検証し、許可されていれば実行する
    fn safe_exec(&self, cmd: &str) -> Result<String, crate::error::AiomeError>;
    
    /// ファイル書き込みを検証する
    fn check_fs_write(&self, path: &std::path::Path) -> Result<(), crate::error::AiomeError>;
    
    /// ネットワーク接続を検証する
    fn check_network(&self, url: &str) -> Result<(), crate::error::AiomeError>;
}
