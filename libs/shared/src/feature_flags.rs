/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/// フェデレーション v1.5 の機能を有効化するフラグ名
pub const FEDERATION_V1_5_FLAG: &str = "federation_v1_5";

/// A2UI の生成 UI 機能を有効化するフラグ名
pub const A2UI_GENERATIVE_UI_FLAG: &str = "a2ui_generative_ui";

/// ヘッドレスブラウザ (obscura) によるJSフォールバックを有効化するフラグ名
pub const JS_FALLBACK_FLAG: &str = "js_fallback";

/// SEO コンテンツの自動投稿（WordPress 等への外部送信）を許可するフラグ名。
/// 未設定は無効（fail-closed）。Settings UI の「SEO Publishing」トグルが書き込む。
pub const SEO_PUBLISH_FLAG: &str = "seo_publish";
