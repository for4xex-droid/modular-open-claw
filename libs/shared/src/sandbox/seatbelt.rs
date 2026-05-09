/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! macOS Seatbelt ポリシー生成およびコマンドラッパー

/// Sandboxの実行プロファイル
#[derive(Default)]
pub struct SeatbeltProfile {
    /// ネットワークアクセスを許可するか
    pub allow_network: bool,
    /// ファイルシステムへの書き込みを許可するか
    pub allow_fs_write: bool,
    /// Loraトレーニング用の特殊プロファイルか
    pub is_lora_training: bool,
}

/// プロファイル設定からSeatbeltのプロファイル文字列を生成する
pub fn generate_profile_str(profile: &SeatbeltProfile) -> String {
    if profile.is_lora_training {
        return "(version 1)\n                         (allow default)\n                         (allow network-outbound (remote tcp \"*:443\"))\n                         (allow network-outbound (remote tcp \"*:80\"))\n                         (deny file-write* (regex #\"^/etc\"))\n                         (deny file-write* (regex #\"^/var\"))"
            .to_string();
    }

    let mut p = String::from("(version 1)\n                         (allow default)");
    if !profile.allow_network {
        p.push_str("\n                         (deny network*)");
    }
    if !profile.allow_fs_write {
        p.push_str("\n                         (deny file-write*)");
    }
    p
}

/// 指定したバイナリとプロファイルからsandbox-execの実行コマンドと引数を生成する
pub fn create_seatbelt_command_args(
    binary: &str,
    profile: &SeatbeltProfile,
) -> (String, Vec<String>) {
    let profile_str = generate_profile_str(profile);
    (
        "sandbox-exec".to_string(),
        vec!["-p".to_string(), profile_str, binary.to_string()],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_profile_str_strict() {
        let profile = SeatbeltProfile::default();
        let s = generate_profile_str(&profile);
        assert!(s.contains("(version 1)"));
        assert!(s.contains("(deny network*)"));
        assert!(s.contains("(deny file-write*)"));
    }

    #[test]
    fn test_create_seatbelt_command_args() {
        let profile = SeatbeltProfile::default();
        let (cmd, args) = create_seatbelt_command_args("echo", &profile);
        assert_eq!(cmd, "sandbox-exec");
        assert_eq!(args[0], "-p");
        assert!(args[1].contains("(version 1)"));
        assert_eq!(args[2], "echo");
    }

    #[test]
    fn test_generate_profile_str_lora_training() {
        let profile = SeatbeltProfile {
            is_lora_training: true,
            ..Default::default()
        };
        let s = generate_profile_str(&profile);
        assert!(s.contains("(version 1)"));
        assert!(s.contains("network-outbound"));
        assert!(s.contains("deny file-write*"));
        // LoRA は全ネットワーク deny ではなく、特定ポートのみ許可
        assert!(!s.contains("(deny network*)"));
    }

    #[test]
    fn test_generate_profile_str_network_only() {
        // ネットワーク許可・ファイル書き込み拒否 (PythonForge 相当)
        let profile = SeatbeltProfile {
            allow_network: false,
            allow_fs_write: true,
            ..Default::default()
        };
        let s = generate_profile_str(&profile);
        assert!(s.contains("(deny network*)"));
        assert!(!s.contains("(deny file-write*)"));
    }

    #[test]
    fn test_generate_profile_str_fully_permissive() {
        let profile = SeatbeltProfile {
            allow_network: true,
            allow_fs_write: true,
            ..Default::default()
        };
        let s = generate_profile_str(&profile);
        assert!(!s.contains("(deny network*)"));
        assert!(!s.contains("(deny file-write*)"));
    }
}
