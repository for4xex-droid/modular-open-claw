/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use clap::{Parser, Subcommand};
use infrastructure::db::DatabasePool;
use infrastructure::security::sqlite_vault_backend::UniversalVaultBackend;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "abyss-vault")]
#[command(about = "CLI tool to manage AbyssVault secrets and macOS Keychain integration", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Bootstraps critical secrets (VAULT_MASTER_PASSWORD, VAULT_SECRET, GEMINI_API_KEY) into macOS Keychain
    Bootstrap {
        #[arg(long)]
        master_password: Option<String>,
        #[arg(long)]
        vault_secret: Option<String>,
        #[arg(long)]
        gemini_api_key: Option<String>,
    },
    /// Imports secrets from a .env.secret file into the AbyssVault database
    Import {
        #[arg(short, long, default_value = ".env.secret")]
        file: PathBuf,
    },
    /// Sets a specific secret key/value in the AbyssVault database
    Set { key: String, value: Option<String> },
    /// Displays settings status compared against the whitelisted secrets
    Status,
    /// Lists all whitelisted key names currently stored in the AbyssVault
    List,
    /// Decrypts and shows the value of a specific secret key
    Get { key: String },
    /// Deletes a specific secret key from the AbyssVault database
    Delete {
        key: String,
        /// Bypass interactive confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// Iteratively prompts to set up all missing whitelisted secrets
    Setup,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load existing environment variables just in case
    dotenvy::dotenv().ok();
    dotenvy::from_path(".env.secret").ok();

    match cli.command {
        Commands::Bootstrap {
            master_password,
            vault_secret,
            gemini_api_key,
        } => {
            #[cfg(target_os = "macos")]
            {
                if let Some(pw) = master_password {
                    shared::security::set_keychain_secret("com.aiome.vault-master-password", &pw)?;
                    println!("✅ Stored com.aiome.vault-master-password in Keychain");
                }
                if let Some(sec) = vault_secret {
                    shared::security::set_keychain_secret("com.aiome.vault-secret", &sec)?;
                    println!("✅ Stored com.aiome.vault-secret in Keychain");
                }
                if let Some(gemini) = gemini_api_key {
                    shared::security::set_keychain_secret("com.aiome.gemini-api-key", &gemini)?;
                    println!("✅ Stored com.aiome.gemini-api-key in Keychain");
                }
                println!("🎉 Bootstrap process completed successfully!");
            }
            #[cfg(not(target_os = "macos"))]
            {
                println!("⚠️ Keychain bootstrap is only supported on macOS. On other platforms, configure via environment variables.");
            }
        }
        Commands::Import { file } => {
            let resolver =
                shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;
            let vault_backend = init_vault_backend(&resolver).await?;

            if !file.exists() {
                anyhow::bail!("Secrets file not found: {}", file.display());
            }

            println!("Reading secrets from {}...", file.display());
            let mut count = 0;
            for entry in dotenvy::from_path_iter(&file)? {
                let (key, value) = entry?;
                // Skip bootstrap keys if they are going to Keychain
                if key == "VAULT_MASTER_PASSWORD"
                    || key == "VAULT_SECRET"
                    || key == "GEMINI_API_KEY"
                {
                    println!("ℹ️ Skipping bootstrap key: {} (Store in Keychain using 'bootstrap' command)", key);
                    continue;
                }

                vault_backend.store_secret(&key, &value).await?;
                println!("🔒 Securely stored: {}", key);
                count += 1;
            }
            println!(
                "🎉 Successfully imported {} secrets into AbyssVault DB",
                count
            );
        }
        Commands::Set { key, value } => {
            if !shared::security::ALLOWED_VAULT_SECRETS.contains(&key.as_str()) {
                anyhow::bail!("Key '{}' is not in the allowed secrets whitelist.", key);
            }

            let value = match value {
                Some(v) => v,
                None => {
                    print!("Enter value for {}: ", key);
                    io::stdout().flush()?;
                    rpassword::read_password()?
                }
            };

            let resolver =
                shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;
            let vault_backend = init_vault_backend(&resolver).await?;

            vault_backend.store_secret(&key, &value).await?;
            println!("🔒 Securely stored secret: {}", key);
        }
        Commands::Status => {
            let resolver =
                shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;
            let vault_backend = init_vault_backend(&resolver).await?;
            let existing_keys = vault_backend.list_secret_keys().await?;

            println!("--- Abyss Vault Status ---");
            let mut configured = 0;
            for &key in shared::security::ALLOWED_VAULT_SECRETS {
                let is_set = existing_keys.contains(&key.to_string());
                if is_set {
                    configured += 1;
                }
                println!("[{}] {}", if is_set { "✅" } else { "❌" }, key);
            }
            println!("--------------------------");
            println!(
                "Summary: {} / {} keys configured.",
                configured,
                shared::security::ALLOWED_VAULT_SECRETS.len()
            );
        }
        Commands::List => {
            let resolver =
                shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;
            let vault_backend = init_vault_backend(&resolver).await?;
            let existing_keys = vault_backend.list_secret_keys().await?;

            for key in existing_keys {
                println!("{}", key);
            }
        }
        Commands::Get { key } => {
            let resolver =
                shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;
            let vault_backend = init_vault_backend(&resolver).await?;

            match vault_backend.get_secret(&key).await {
                Ok(val) => {
                    // Expose the zeroizing string
                    println!("{}", *val);
                }
                Err(e) => {
                    anyhow::bail!("Failed to get secret for key '{}': {}", key, e);
                }
            }
        }
        Commands::Delete { key, yes } => {
            let resolver =
                shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;
            let vault_backend = init_vault_backend(&resolver).await?;

            if !yes {
                print!("Are you sure you want to delete '{}'? [y/N]: ", key);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim().to_lowercase();
                if input != "y" && input != "yes" {
                    println!("Abort deletion.");
                    return Ok(());
                }
            }

            let deleted = vault_backend.delete_secret(&key).await?;
            if deleted {
                println!("🗑️ Deleted secret: {}", key);
            } else {
                println!("ℹ️ Key '{}' was not found in vault.", key);
            }
        }
        Commands::Setup => {
            let resolver =
                shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;
            let vault_backend = init_vault_backend(&resolver).await?;
            let existing_keys = vault_backend.list_secret_keys().await?;

            let mut count = 0;
            for &key in shared::security::ALLOWED_VAULT_SECRETS {
                if !existing_keys.contains(&key.to_string()) {
                    print!("Setup value for {} (press Enter to skip): ", key);
                    io::stdout().flush()?;
                    let val = rpassword::read_password()?;
                    if !val.is_empty() {
                        vault_backend.store_secret(key, &val).await?;
                        println!("🔒 Securely stored: {}", key);
                        count += 1;
                    } else {
                        println!("Skipped: {}", key);
                    }
                }
            }
            println!(
                "🎉 Interactive setup finished. Registered {} new secrets.",
                count
            );
        }
    }

    Ok(())
}

async fn init_vault_backend(
    resolver: &shared::app_data::AppDataResolver,
) -> anyhow::Result<UniversalVaultBackend> {
    // Ensure database exists
    let vault_db_path = env::var("ABYSS_VAULT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| resolver.root().join("abyss_vault.db"));

    if let Some(parent) = vault_db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let vault_db_url = format!("sqlite:{}?mode=rwc", vault_db_path.to_string_lossy());
    let vault_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&vault_db_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to AbyssVault DB: {}", e))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS vault_secrets (key TEXT PRIMARY KEY, encrypted_value BLOB NOT NULL)"
    )
    .execute(&vault_pool)
    .await?;

    let db_pool = DatabasePool::Sqlite(vault_pool);

    // Attempt to retrieve VAULT_MASTER_PASSWORD
    let _pw = shared::security::get_keychain_secret("com.aiome.vault-master-password")
        .or_else(|| env::var("VAULT_MASTER_PASSWORD").ok())
        .ok_or_else(|| anyhow::anyhow!("VAULT_MASTER_PASSWORD must be configured in Keychain or environment to access AbyssVault"))?;

    // UniversalVaultBackend will fetch the master password from get_global_master_key()
    // inside the infrastructure crate, which utilizes get_master_key() which we modified.
    Ok(UniversalVaultBackend::new(db_pool))
}
