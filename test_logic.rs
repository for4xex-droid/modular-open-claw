/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
fn main() {
    let llm_configured = true;
    let db_exists = false;
    let admin_account_exists: Option<bool> = Some(false);

    let mode = if (!llm_configured && !db_exists) || !admin_account_exists.unwrap_or(true) {
        "Setup"
    } else {
        "Normal"
    };

    println!("Mode: {}", mode);
}
