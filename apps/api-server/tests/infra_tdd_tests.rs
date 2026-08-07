/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn alpha_td_1_and_6_dockerfile_security_and_version() {
    let path = workspace_root().join("Dockerfile");
    let content = fs::read_to_string(path).unwrap_or_default();

    // D-1: Rust version mismatch
    assert!(
        content.contains("RUST_VERSION=1.85"),
        "TDD RED: Missing RUST_VERSION=1.85"
    );
    assert!(
        content.contains("bookworm"),
        "TDD RED: Missing bookworm base image"
    );

    // F-1: Dockerfile USER non-root
    assert!(
        content.contains("USER aiome"),
        "TDD RED: Missing non-root USER aiome declaration"
    );
}

#[test]
fn alpha_td_2_port_unification() {
    let compose_path = workspace_root().join("docker-compose.quickstart.yml");
    let compose_content = fs::read_to_string(compose_path).unwrap_or_default();
    assert!(
        compose_content.contains("\"3015:3015\""),
        "TDD RED: docker-compose does not map 3015"
    );

    let pw_path = workspace_root().join("apps/management-console/playwright.config.ts");
    let pw_content = fs::read_to_string(pw_path).unwrap_or_default();
    assert!(
        pw_content.contains("localhost:3015"),
        "TDD RED: playwright doesn't target 3015"
    );
    assert!(
        !pw_content.contains("localhost:1420"),
        "TDD RED: playwright still targets 1420"
    );

    let docker_path = workspace_root().join("Dockerfile");
    let docker_content = fs::read_to_string(docker_path).unwrap_or_default();
    assert!(
        docker_content.contains("ENV PORT=3015"),
        "TDD RED: Dockerfile doesn't expose 3015"
    );
    assert!(
        !docker_content.contains("ENV PORT=1420"),
        "TDD RED: Dockerfile still exposes 1420"
    );
}

#[test]
fn alpha_td_3_gitignore_vault() {
    let path = workspace_root().join(".gitignore");
    let content = fs::read_to_string(path).unwrap_or_default();
    assert!(
        content.contains(".abyss_vault"),
        "TDD RED: Missing .abyss_vault in .gitignore"
    );
}

#[test]
fn alpha_td_4_distroless_frontend() {
    let path = workspace_root().join("docker/distroless.Dockerfile");
    let content = fs::read_to_string(path).unwrap_or_default();
    assert!(
        content.contains("npm run build"),
        "TDD RED: distroless missing frontend builder stage"
    );
}

#[test]
fn alpha_td_5_i18n_parity() {
    let en_path = workspace_root().join("apps/management-console/src/i18n/en.json");
    let ja_path = workspace_root().join("apps/management-console/src/i18n/ja.json");

    let en_content = fs::read_to_string(en_path).unwrap_or_default();
    let ja_content = fs::read_to_string(ja_path).unwrap_or_default();

    // JA に欠落しているキーの検証
    assert!(
        ja_content.contains("\"loadMore\""),
        "TDD RED: JA missing timeline.loadMore"
    );
    assert!(
        ja_content.contains("\"loading\""),
        "TDD RED: JA missing timeline.loading"
    );
    assert!(
        ja_content.contains("\"noEntries\""),
        "TDD RED: JA missing timeline.noEntries"
    );

    // EN に欠落しているキーの検証
    assert!(
        en_content.contains("\"noArtifacts\""),
        "TDD RED: EN missing artifact.noArtifacts"
    );
    assert!(
        en_content.contains("\"evolutionStep\""),
        "TDD RED: EN missing timeline.evolutionStep"
    );
    assert!(
        en_content.contains("\"federatedMemory\""),
        "TDD RED: EN missing timeline.federatedMemory"
    );
    assert!(
        en_content.contains("\"localMemory\""),
        "TDD RED: EN missing timeline.localMemory"
    );
}

#[test]
fn td_f2_changelog_unreleased_count() {
    let path = workspace_root().join("CHANGELOG.md");
    let content = fs::read_to_string(path).unwrap_or_default();
    // 見出し形式のみを数える（本文中の言及は除外）
    let unreleased_count = content.matches("## [Unreleased]").count();

    assert_eq!(
        unreleased_count, 1,
        "TDD RED: CHANGELOG.md has {} '## [Unreleased]' sections, expected 1",
        unreleased_count
    );
}

#[test]
fn td_grafana_error_rate_panel() {
    let path =
        workspace_root().join("docker/grafana/provisioning/dashboards/aiome_voice_metrics.json");
    let content = fs::read_to_string(path).unwrap_or_default();
    assert!(
        content.contains("\"title\": \"TTS Error Rate\"")
            || content.contains("\"title\": \"Request Failure Rate\""),
        "TDD RED: Grafana dashboard missing Error Rate panel"
    );
}

#[test]
fn test_pro_gating_requirements() {
    let error_rs = workspace_root().join("libs/aiome-contracts/src/error.rs");
    let error_content = fs::read_to_string(error_rs).unwrap_or_default();
    assert!(
        error_content.contains("PaymentRequired"),
        "TDD RED: AiomeError is missing PaymentRequired variant"
    );

    let auth_rs = workspace_root().join("apps/api-server/src/auth.rs");
    let auth_content = fs::read_to_string(auth_rs).unwrap_or_default();
    assert!(
        auth_content.contains("pub struct ProAuthenticated"),
        "TDD RED: ProAuthenticated struct is missing in auth.rs"
    );

    // 各ハンドラの ProAuthenticated ゲート適用の検証
    let lora_market_rs = workspace_root().join("apps/api-server/src/routes/lora_market.rs");
    let lora_content = fs::read_to_string(lora_market_rs).unwrap_or_default();
    assert!(
        lora_content.contains("auth: crate::auth::ProAuthenticated"),
        "TDD RED: publish_listing or purchase_listing is not protected by ProAuthenticated"
    );
}
