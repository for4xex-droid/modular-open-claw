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
