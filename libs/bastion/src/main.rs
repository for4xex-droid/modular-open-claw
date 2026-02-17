use anyhow::Result;
use clap::{Parser, Subcommand};

/// Bastion - 🏰 産業グレード セキュリティツールキット
#[derive(Parser)]
#[command(name = "bastion")]
#[command(version, about = "🏰 Bastion Security Toolkit - スキャン・ガードレール・テンプレート生成", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// プロジェクトの脆弱性スキャン・シークレット検出を実行する
    Scan,

    /// セキュリティテンプレートをプロジェクトに展開する
    Init {
        /// 対象言語 (rust / python / auto)
        #[arg(default_value = "auto")]
        language: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // サブコマンドが指定されない場合はデフォルトでスキャン実行
        None | Some(Commands::Scan) => {
            bastion::scanner::run_scan()?;
        }
        Some(Commands::Init { language }) => {
            bastion::init::run_init(&language)?;
        }
    }

    Ok(())
}
