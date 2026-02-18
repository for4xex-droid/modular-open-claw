use std::path::PathBuf;
use factory_core::contracts::ConceptResponse;
use factory_core::error::FactoryError;
use tuning::StyleProfile;

/// 中間素材と最終成果物の管理、および永続化 (Remix Mode の基盤)
pub struct AssetManager {
    base_dir: PathBuf,
}

impl AssetManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// プロジェクトディレクトリを初期化
    pub fn init_project(&self, project_id: &str) -> Result<PathBuf, FactoryError> {
        let path = self.base_dir.join(project_id);
        std::fs::create_dir_all(&path).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to create project dir: {}", e),
        })?;
        
        // サブディレクトリ作成
        std::fs::create_dir_all(path.join("visuals")).ok();
        std::fs::create_dir_all(path.join("audio")).ok();
        
        Ok(path)
    }

    /// コンセプトを保存
    pub fn save_concept(&self, project_id: &str, concept: &ConceptResponse) -> Result<(), FactoryError> {
        let path = self.base_dir.join(project_id).join("concept.json");
        let json = serde_json::to_string_pretty(concept).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to serialize concept: {}", e),
        })?;
        std::fs::write(path, json).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to write concept.json: {}", e),
        })
    }

    /// コンセプトを読み込み
    pub fn load_concept(&self, project_id: &str) -> Result<ConceptResponse, FactoryError> {
        let path = self.base_dir.join(project_id).join("concept.json");
        let content = std::fs::read_to_string(path).map_err(|e| FactoryError::MediaNotFound {
            path: format!("concept.json for {}: {}", project_id, e),
        })?;
        serde_json::from_str(&content).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to parse concept.json: {}", e),
        })
    }

    /// 素材（動画・音声）の存在チェック
    #[allow(dead_code)]
    pub fn check_assets(&self, project_id: &str, scene_count: usize) -> bool {
        let root = self.base_dir.join(project_id);
        
        // 音声チェック
        for i in 0..scene_count {
            if !root.join(format!("audio/scene_{}.wav", i)).exists() {
                return false;
            }
        }
        
        // 動画チェック
        for i in 0..scene_count {
            if !root.join(format!("visuals/scene_{}.mp4", i)).exists() {
                return false;
            }
        }
        
        true
    }

    /// 最終的な実行パラメータをスナップショットとして保存
    pub fn save_metadata(&self, project_id: &str, style: &StyleProfile) -> Result<(), FactoryError> {
        let path = self.base_dir.join(project_id).join("metadata.json");
        let metadata = serde_json::json!({
            "project_id": project_id,
            "style_used": style,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        let json = serde_json::to_string_pretty(&metadata).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to serialize metadata: {}", e),
        })?;
        
        std::fs::write(path, json).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to write metadata.json: {}", e),
        })
    }
}
