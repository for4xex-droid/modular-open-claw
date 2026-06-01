/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use std::fs;
use std::path::Path;

/// 法務ドキュメントを検索・読み込むヘルパー関数。
/// `CARGO_MANIFEST_DIR` を基点にした安全なパス解決を優先し、
/// CI 環境でのカレントディレクトリ依存を排除する。
fn read_legal_doc(filename: &str) -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let paths = vec![
        Path::new(&manifest_dir).join(format!("docs/legal/{}", filename)),
        Path::new(&manifest_dir).join(format!("../../docs/legal/{}", filename)),
        Path::new("docs/legal").join(filename),
        Path::new("../../docs/legal").join(filename),
    ];

    for p in &paths {
        if p.exists() {
            return fs::read_to_string(p)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));
        }
    }

    panic!(
        "{} not found in searched paths: {:?}",
        filename,
        paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_tos_contains_mandatory_legal_clauses() {
    let tos = read_legal_doc("TERMS_OF_SERVICE.md");

    // 必須リーガルキーワードの検査
    let mandatory_keywords = vec![
        ("BSL-1.1", "ライセンス条項"),
        ("返金", "返金不可 (non-refundable) ポリシー"),
        ("自己修復", "Self-Healing に伴う完全免責"),
        ("免責", "免責事項"),
        ("eKYC", "Stripe Identity による本人確認"),
    ];

    for (kw, description) in mandatory_keywords {
        assert!(
            tos.contains(kw),
            "ToS is missing critical legal keyword: '{}' ({})",
            kw,
            description
        );
    }
}

#[test]
fn test_privacy_policy_contains_mandatory_clauses() {
    let policy = read_legal_doc("PRIVACY_POLICY.md");

    // プライバシーポリシーの必須キーワード
    let mandatory_keywords = vec![
        ("ローカルファースト", "Local-First 設計原則"),
        ("免責", "免責事項"),
        ("忘れられる権利", "GDPR 準拠のデータ物理削除"),
    ];

    for (kw, description) in mandatory_keywords {
        assert!(
            policy.contains(kw),
            "Privacy Policy is missing critical keyword: '{}' ({})",
            kw,
            description
        );
    }
}

#[test]
fn test_tokushoho_contains_mandatory_clauses() {
    let tokushoho = read_legal_doc("TOKUSHOHO.md");

    // 特定商取引法に基づく表記の必須キーワード
    let mandatory_keywords = vec![
        ("特定商取引法", "特定商取引法に基づく表記"),
        ("motivationstudio", "販売業者名"),
        ("運営責任者", "運営責任者"),
        ("所在地", "住所"),
        ("メールアドレス", "問い合わせ先メールアドレス"),
        ("販売価格", "対価"),
        ("支払方法", "お支払い方法"),
        ("引渡時期", "商品の引き渡し時期"),
        ("返品", "返品・キャンセルの特約"),
    ];

    for (kw, description) in mandatory_keywords {
        assert!(
            tokushoho.contains(kw) || tokushoho.to_lowercase().contains(kw),
            "TOKUSHOHO.md is missing critical keyword: '{}' ({})",
            kw,
            description
        );
    }
}
