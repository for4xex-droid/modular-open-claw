/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info};

/// 安全に検出されたスキルの定義
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedSkill {
    /// 領域 (例: coding, data, web, devops)
    pub domain: String,
    /// スキル名 (例: rust, typescript, database, container)
    pub skill: String,
    /// 検出レベル (1: 基本, 2: 応用, 3: 専門)
    pub level: u8,
}

/// ワークスペースの環境要素からスキルプロファイルを安全に自律生成するエンジン
pub struct AutoProfileEngine;

impl AutoProfileEngine {
    /// 指定ディレクトリをスキャンし、許可リストベースでスキルリストを返す。
    /// 正規表現による内容の抽出等は行わず、特定のファイル名および既知の文字列（依存関係）の有無のみを検証する。
    pub fn scan_workspace(root: &Path) -> Vec<DetectedSkill> {
        let mut skills = Vec::new();
        debug!("Scanning workspace for skills at: {:?}", root);

        // Rust プロジェクトの検出
        if root.join("Cargo.toml").exists() {
            skills.push(DetectedSkill {
                domain: "coding".to_string(),
                skill: "rust".to_string(),
                level: 2,
            });

            // 依存クレートからの追加スキル（許可リスト）
            if let Ok(content) = std::fs::read_to_string(root.join("Cargo.toml")) {
                if content.contains("tokio") {
                    skills.push(DetectedSkill {
                        domain: "coding".to_string(),
                        skill: "async_rust".to_string(),
                        level: 2,
                    });
                }
                if content.contains("axum") || content.contains("actix-web") {
                    skills.push(DetectedSkill {
                        domain: "web".to_string(),
                        skill: "web_api".to_string(),
                        level: 2,
                    });
                }
                if content.contains("sqlx") || content.contains("diesel") {
                    skills.push(DetectedSkill {
                        domain: "data".to_string(),
                        skill: "database".to_string(),
                        level: 2,
                    });
                }
                if content.contains("candle-core") || content.contains("tch") {
                    skills.push(DetectedSkill {
                        domain: "ai".to_string(),
                        skill: "machine_learning".to_string(),
                        level: 3,
                    });
                }
            }
        }

        // JavaScript / TypeScript プロジェクトの検出
        if root.join("package.json").exists() {
            skills.push(DetectedSkill {
                domain: "coding".to_string(),
                skill: "javascript".to_string(),
                level: 2,
            });

            if let Ok(content) = std::fs::read_to_string(root.join("package.json")) {
                if content.contains("typescript") {
                    skills.push(DetectedSkill {
                        domain: "coding".to_string(),
                        skill: "typescript".to_string(),
                        level: 2,
                    });
                }
                if content.contains("react") || content.contains("next") || content.contains("vue")
                {
                    skills.push(DetectedSkill {
                        domain: "web".to_string(),
                        skill: "frontend_framework".to_string(),
                        level: 2,
                    });
                }
            }
        }

        // Python プロジェクトの検出
        if root.join("requirements.txt").exists() || root.join("pyproject.toml").exists() {
            skills.push(DetectedSkill {
                domain: "coding".to_string(),
                skill: "python".to_string(),
                level: 2,
            });

            // 簡単な検証（もし requirements.txt が読めれば）
            if let Ok(content) = std::fs::read_to_string(root.join("requirements.txt")) {
                if content.contains("torch") || content.contains("tensorflow") {
                    skills.push(DetectedSkill {
                        domain: "ai".to_string(),
                        skill: "deep_learning".to_string(),
                        level: 3,
                    });
                }
                if content.contains("flask")
                    || content.contains("fastapi")
                    || content.contains("django")
                {
                    skills.push(DetectedSkill {
                        domain: "web".to_string(),
                        skill: "python_web".to_string(),
                        level: 2,
                    });
                }
            }
        }

        // DevOps 関連
        if root.join("Dockerfile").exists() {
            skills.push(DetectedSkill {
                domain: "devops".to_string(),
                skill: "docker".to_string(),
                level: 1,
            });
        }
        if root.join("docker-compose.yml").exists() || root.join("docker-compose.yaml").exists() {
            skills.push(DetectedSkill {
                domain: "devops".to_string(),
                skill: "container_orchestration".to_string(),
                level: 2,
            });
        }

        // TLA+ 形式検証
        if root.join("tla").is_dir() || has_extension(root, "tla") {
            skills.push(DetectedSkill {
                domain: "verification".to_string(),
                skill: "tla_plus".to_string(),
                level: 3,
            });
        }

        info!(
            "🛡️ [AutoProfile] Safely detected {} skills from workspace",
            skills.len()
        );
        skills
    }
}

/// 指定した拡張子を持つファイルがディレクトリ直下に存在するか確認する補助関数
fn has_extension(dir: &Path, ext: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(path_ext) = entry.path().extension() {
                if path_ext == ext {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_scan_workspace_rust() {
        let dir = tempdir().unwrap(); // allow-anti-pattern
        let cargo_toml_path = dir.path().join("Cargo.toml");
        let mut file = File::create(cargo_toml_path).unwrap(); // allow-anti-pattern
        writeln!(file, "[dependencies]\ntokio = \"1.0\"\naxum = \"0.7\"").unwrap(); // allow-anti-pattern

        let skills = AutoProfileEngine::scan_workspace(dir.path());

        assert!(skills.iter().any(|s| s.skill == "rust"));
        assert!(skills.iter().any(|s| s.skill == "async_rust"));
        assert!(skills.iter().any(|s| s.skill == "web_api"));
    }

    #[test]
    fn test_scan_workspace_python_ml() {
        let dir = tempdir().unwrap(); // allow-anti-pattern
        let req_path = dir.path().join("requirements.txt");
        let mut file = File::create(req_path).unwrap(); // allow-anti-pattern
        writeln!(file, "torch==2.0.0\nfastapi==0.95.0").unwrap(); // allow-anti-pattern

        let docker_path = dir.path().join("Dockerfile");
        File::create(docker_path).unwrap(); // allow-anti-pattern

        let skills = AutoProfileEngine::scan_workspace(dir.path());

        assert!(skills.iter().any(|s| s.skill == "python"));
        assert!(skills.iter().any(|s| s.skill == "deep_learning"));
        assert!(skills.iter().any(|s| s.skill == "python_web"));
        assert!(skills.iter().any(|s| s.skill == "docker"));
    }
}
