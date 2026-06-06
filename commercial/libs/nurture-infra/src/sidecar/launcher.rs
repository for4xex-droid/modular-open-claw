/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

//! サイドカープロセスの起動・管理。
//! 推論エンジン等の隔離実行を担当。

use crate::sidecar::vram_arbiter::VramReservation;
use std::process::{Child, Command, Stdio};

pub struct SidecarInstance {
    pub child: Child,
    /// このサイドカーが確保している VRAM 枠。
    /// インスタンスが Drop される（終了する）と自動的に解放される。
    pub _vram_reservation: Option<VramReservation>,
}

pub struct SidecarLauncher;

impl SidecarLauncher {
    /// サイドカープロセスを起動する。必要であれば VRAM を予約する。
    pub fn spawn(
        cmd: &str,
        args: &[&str],
        reservation: Option<VramReservation>,
        signing_key: Option<String>,
    ) -> std::io::Result<SidecarInstance> {
        let mut command = Command::new(cmd);

        // 🚨 V-02 サンドボックス隔離: 環境変数をクリアし、必要なものだけを渡す
        command
            .env_clear()
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // 🚨 M-6 & M-7: 最小限の環境変数の復元と秘密鍵の注入
        let essential_vars = [
            "PATH",
            "CUDA_VISIBLE_DEVICES",
            "LD_LIBRARY_PATH",
            "HUGGINGFACE_HUB_CACHE",
            "PYTHONPATH",
            "OMP_NUM_THREADS",
        ];

        for var in essential_vars {
            if let Ok(val) = std::env::var(var) {
                command.env(var, val);
            }
        }

        // 🚨 M-6: 秘密鍵を引数ではなく環境変数で注入
        if let Some(key) = signing_key {
            command.env("AIOME_SIDECAR_SIGNING_KEY", key);
        }

        // M-7: AIOME_ プレフィックスの引き継ぎを制限 (安全なものだけ)
        let safe_prefix_vars = ["NVTE_", "AIOME_LOG_"];
        for (key, value) in std::env::vars() {
            if safe_prefix_vars.iter().any(|p| key.starts_with(p)) {
                command.env(key, value);
            }
        }

        let child = command.spawn()?;

        Ok(SidecarInstance {
            child,
            _vram_reservation: reservation,
        })
    }
}
