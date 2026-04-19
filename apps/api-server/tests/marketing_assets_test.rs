/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use std::fs;
use std::path::PathBuf;

/// Same implementation as `infra_tdd_tests::workspace_root()` — kept local to
/// avoid introducing a shared test-utility crate for two files.
fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest_dir)
        .parent()
        .expect("api-server parent (apps/)")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn test_readme_contains_zero_dollar_cta() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    // Check Japanese README
    let readme_jp = root.join("README.md");
    let content_jp = fs::read_to_string(&readme_jp)?;
    assert!(
        content_jp.contains("$0"),
        "README.md MUST contain a '$0 / month' CTA for self-hosting according to Postiz Playbook."
    );
    assert!(
        content_jp.contains("Docker"),
        "README.md MUST emphasize Docker deployment."
    );

    // Check English README
    let readme_en = root.join("README_en.md");
    let content_en = fs::read_to_string(&readme_en)?;
    assert!(
        content_en.contains("$0"),
        "README_en.md MUST contain a '$0 / month' CTA for self-hosting."
    );
    assert!(
        content_en.contains("Docker"),
        "README_en.md MUST emphasize Docker deployment."
    );
    Ok(())
}

#[test]
fn test_readme_contains_youtube_tutorial() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    let readme_jp = root.join("README.md");
    let content_jp = fs::read_to_string(&readme_jp)?;
    let has_youtube_jp = content_jp.contains("youtube.com")
        || content_jp.contains("youtu.be")
        || content_jp.contains("(Coming Soon)");
    assert!(
        has_youtube_jp,
        "README.md MUST contain a YouTube tutorial link (Postiz Playbook tactic P3) or (Coming Soon)."
    );

    let readme_en = root.join("README_en.md");
    let content_en = fs::read_to_string(&readme_en)?;
    let has_youtube_en = content_en.contains("youtube.com")
        || content_en.contains("youtu.be")
        || content_en.contains("(Coming Soon)");
    assert!(
        has_youtube_en,
        "README_en.md MUST contain a YouTube tutorial link or (Coming Soon)."
    );
    Ok(())
}

/// This test is `#[ignore]`d by default. Run with `cargo test -- --ignored`
/// during release-preflight to ensure no placeholder links ship to production.
#[test]
fn test_no_pending_placeholders_in_readme() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    let content_jp = fs::read_to_string(root.join("README.md"))?;
    let content_en = fs::read_to_string(root.join("README_en.md"))?;

    assert!(
        !content_jp.contains("PENDING_") && !content_jp.contains("COMING_SOON"),
        "README.md contains a PENDING_ or COMING_SOON placeholder. Replace with a real URL before release."
    );
    assert!(
        !content_en.contains("PENDING_") && !content_en.contains("COMING_SOON"),
        "README_en.md contains a PENDING_ or COMING_SOON placeholder. Replace with a real URL before release."
    );
    Ok(())
}
