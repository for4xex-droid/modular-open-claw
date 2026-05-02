/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::writing_context::WritingContext;
use regex::Regex;
use std::sync::LazyLock;

/// ヒューマナイザールールがマッチした際のアクション定義
#[derive(Debug, Clone, PartialEq)]
pub enum HumanizerAction {
    /// 固定文字列に置換
    Replace(String),
    /// 完全に削除
    Delete,
    /// 置換はせず警告ログのみ（構造の破壊を防ぐためなど）
    LogWarning,
}

/// LLM出力からAI特有のパターンを検出して除去するためのルール
#[derive(Clone)]
pub struct HumanizerRule {
    /// ルールの識別名
    pub name: &'static str,
    /// 検出するための正規表現パターン
    pub pattern: Regex,
    /// マッチした場合のアクション
    pub action: HumanizerAction,
    /// このルールが適用されるコンテキストのリスト。空の場合は全コンテキストに適用。
    pub active_contexts: Vec<WritingContext>,
}

/// 日本語に特化したデフォルトのAIくささ除去ルールセット
pub fn default_rules_ja() -> Vec<HumanizerRule> {
    vec![
        // 1. emダッシュ・全角ダッシュの乱用
        HumanizerRule {
            name: "em_dash_replacement",
            // 文中のダッシュ（両側に文字がある場合）を全角読点または括弧に置換する（ここでは簡易的に読点とするか、単純な置換）
            // 実際は文脈によるが、最も安全な「、」やスペースへの置換、あるいは削除。
            pattern: Regex::new(r"——|—|─").expect("Invalid regex pattern"), // allow-anti-pattern
            action: HumanizerAction::Replace("、".to_string()),
            active_contexts: vec![
                WritingContext::Chat,
                WritingContext::TechLog,
                WritingContext::Default,
            ],
        },
        // 2. チャットボット残留表現
        HumanizerRule {
            name: "chatbot_artifacts",
            pattern: Regex::new(
                r"(?x)
                お役に立てれば幸いです[！!]*|
                ご不明な点がございましたら.*?お知らせください[！!]*|
                その他に[も]?お手伝いできることはありますか[？?]*|
                お気軽にお申し付けください[！!]*
            ",
            )
            .expect("Invalid regex pattern"), // allow-anti-pattern
            action: HumanizerAction::Delete,
            active_contexts: vec![], // All contexts
        },
        // 3. 追従的トーン
        HumanizerRule {
            name: "sycophantic_tone",
            pattern: Regex::new(
                r"(?x)
                素晴らしい[ご]?質問ですね[！!]*|
                おっしゃる通りです[！!]*|
                その通りです[。]*
            ",
            )
            .expect("safe unwrap"), // allow-anti-pattern
            action: HumanizerAction::Delete,
            active_contexts: vec![], // All contexts
        },
        // 4. 過剰ヘッジング
        HumanizerRule {
            name: "excessive_hedging",
            pattern: Regex::new(r"かもしれない可能性がある").expect("safe unwrap"), // allow-anti-pattern
            action: HumanizerAction::Replace("だろう".to_string()),
            active_contexts: vec![
                WritingContext::TechLog,
                WritingContext::Chat,
                WritingContext::Default,
            ],
        },
        // 5. フィラー句
        HumanizerRule {
            name: "filler_phrases",
            pattern: Regex::new(r"〜という事実により|であるため、結果として").expect("safe unwrap"), // allow-anti-pattern
            action: HumanizerAction::LogWarning, // 機械的な置換は文を壊す恐れがあるためログのみ
            active_contexts: vec![],
        },
        // 6. 意義の過剰強調
        HumanizerRule {
            name: "inflated_significance",
            pattern: Regex::new(r"の重要性を(さらに)?(強調|浮き彫りに)して(い|おり)ます").expect("Invalid regex pattern"), // allow-anti-pattern
            action: HumanizerAction::Replace("を示しています".to_string()),
            active_contexts: vec![WritingContext::TechLog, WritingContext::Default],
        },
        // 7. AI頻出接続詞の連続（簡易版: 文頭の「さらに」「加えて」が多すぎる場合）
        // ここでは単純な単語ベースルールに留める
        HumanizerRule {
            name: "ai_vocabulary",
            pattern: Regex::new(r"^(さらに|加えて)、").expect("Invalid regex pattern"), // allow-anti-pattern
            action: HumanizerAction::LogWarning,
            active_contexts: vec![],
        },
    ]
}
