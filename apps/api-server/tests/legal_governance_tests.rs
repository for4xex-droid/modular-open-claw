/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use std::fs;
use std::path::Path;

#[test]
fn test_tos_contains_mandatory_legal_clauses() {
    let paths = vec![
        Path::new("docs/legal/TERMS_OF_SERVICE.md").to_path_buf(),
        Path::new("../../docs/legal/TERMS_OF_SERVICE.md").to_path_buf(),
    ];

    let mut content = None;
    for p in paths {
        if p.exists() {
            content = Some(fs::read_to_string(p).expect("Failed to read TERMS_OF_SERVICE.md"));
            break;
        }
    }

    let tos = content.expect("TERMS_OF_SERVICE.md not found in searched paths");

    // 必須リーガルキーワードの検査
    let mandatory_keywords = vec![
        "BSL-1.1",
        "返金",     // 返金不可 (non-refundable)
        "自己修復", // 自己修復 (Self-Healing) に伴う完全免責
        "免責",     // 免責事項
        "eKYC",     // Stripe Identity による本人確認
    ];

    for kw in mandatory_keywords {
        assert!(
            tos.contains(kw),
            "ToS is missing critical legal keyword: '{}'",
            kw
        );
    }
}

#[test]
fn test_privacy_policy_contains_mandatory_clauses() {
    let paths = vec![
        Path::new("docs/legal/PRIVACY_POLICY.md").to_path_buf(),
        Path::new("../../docs/legal/PRIVACY_POLICY.md").to_path_buf(),
    ];

    let mut content = None;
    for p in paths {
        if p.exists() {
            content = Some(fs::read_to_string(p).expect("Failed to read PRIVACY_POLICY.md"));
            break;
        }
    }

    let policy = content.expect("PRIVACY_POLICY.md not found");

    // プライバシーポリシーの必須キーワード
    let mandatory_keywords = vec![
        "ローカルファースト",
        "免責",
        "忘れられる権利", // GDPR準拠のデータ物理削除
    ];

    for kw in mandatory_keywords {
        assert!(
            policy.contains(kw),
            "Privacy Policy is missing critical keyword: '{}'",
            kw
        );
    }
}
