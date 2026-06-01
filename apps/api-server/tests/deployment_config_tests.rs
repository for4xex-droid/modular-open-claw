/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use std::fs;
use std::path::Path;

#[test]
fn test_caddyfile_template_is_valid_and_has_required_directives() {
    // CARGO_MANIFEST_DIR を利用した、実行ディレクトリに依存しない安全なパス解決
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let paths = vec![
        Path::new(&manifest_dir).join("docker/caddy/Caddyfile"),
        Path::new(&manifest_dir).join("../../docker/caddy/Caddyfile"),
        Path::new("docker/caddy/Caddyfile").to_path_buf(),
        Path::new("../../docker/caddy/Caddyfile").to_path_buf(),
    ];

    let mut content = None;
    for p in paths {
        if p.exists() {
            content = Some(fs::read_to_string(p).expect("Failed to read Caddyfile"));
            break;
        }
    }

    let caddyfile = content.expect("Caddyfile template not found in searched paths");

    // 必須インフラ・セキュリティディレクティブの徹底検証 (Aegis Shield verification)
    let mandatory_directives = vec![
        "reverse_proxy",             // リバースプロキシ
        ":3015",                     // api-server の内部ポート
        "header_up",                 // ホストヘッダー中継 (SSRF / Proxy 保護)
        "Strict-Transport-Security", // HSTS 強制 HTTPS 設定
        "X-Frame-Options",           // クリックジャッキング対策
        "X-Content-Type-Options",    // MIME スニッフィング防御
        "Referrer-Policy",           // リファラー情報制御
        "Permissions-Policy",        // 不要デバイスアクセス遮断 (SSRF緩和)
        "Content-Security-Policy",   // XSS保護
    ];

    for dir in mandatory_directives {
        assert!(
            caddyfile.contains(dir),
            "Caddyfile template is missing critical configuration: '{}'",
            dir
        );
    }
}
