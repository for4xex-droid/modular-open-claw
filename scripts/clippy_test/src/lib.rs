/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::disallowed_methods)]
pub async fn test() {
    // 実際には sqlx::query!() はコンパイル時にマクロ展開される
    let _ = sqlx::query::<sqlx::Sqlite>("SELECT 1"); // 関数としての呼び出し
}
