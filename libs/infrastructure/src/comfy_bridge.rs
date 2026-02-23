//! # ComfyBridge — ComfyUI API クライアント
//!
//! ComfyUI REST API と通信し、画像/動画生成ワークフローを実行する。
//! Bastion ShieldClient を使用して、SSRF や DNS Rebinding を防止する。

use async_trait::async_trait;
use bastion::net_guard::ShieldClient;
use factory_core::contracts::{VideoRequest, VideoResponse};
use factory_core::error::FactoryError;
use factory_core::traits::{AgentAct, VideoGenerator};
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::info;
use std::path::PathBuf;
use std::sync::Arc;
use std::process::Stdio;
use tokio::process::Command;

/// ComfyUI API クライアント
#[derive(Clone)]
pub struct ComfyBridgeClient {
    /// Bastion ネットワークシールド
    pub shield: Arc<ShieldClient>,
    /// ComfyUI の WebSocket/REST API エンドポイント
    pub api_url: String,
    /// ComfyUI のインストールベースディレクトリ (Zero-Copy I/O用)
    pub base_dir: PathBuf,
    /// タイムアウト（秒）
    pub timeout_secs: u64,
}

impl ComfyBridgeClient {
    pub fn new(shield: Arc<ShieldClient>, api_url: impl Into<String>, base_dir: impl Into<PathBuf>, timeout_secs: u64) -> Self {
        Self {
            shield,
            api_url: api_url.into(),
            base_dir: base_dir.into(),
            timeout_secs,
        }
    }

    /// Zero-Copy: 指定された入力素材を ComfyUI の `input/` フォルダに直接コピーし、一意なファイル名を返す
    pub async fn inject_input_file(&self, src_path: &std::path::Path, tracking_id: &str) -> Result<String, FactoryError> {
        let file_name = src_path.file_name()
            .ok_or_else(|| FactoryError::Infrastructure { reason: "Invalid source file path".into() })?
            .to_string_lossy();
        let unique_name = format!("{}_{}", tracking_id, file_name);
        
        let dest_path = self.base_dir.join("input").join(&unique_name);
        
        tokio::fs::copy(src_path, &dest_path).await.map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to zero-copy input to {:?}: {}", dest_path, e)
        })?;
        
        Ok(unique_name)
    }

    /// JSON: `_meta.title` を持つノードを検索し、そのノードID文字列を返す
    pub fn find_node_id_by_title(workflow: &serde_json::Value, title: &str) -> Option<String> {
        if let Some(nodes) = workflow.as_object() {
            for (id, node) in nodes {
                if let Some(meta) = node.get("_meta") {
                    if let Some(t) = meta.get("title") {
                        if t.as_str() == Some(title) {
                            return Some(id.clone());
                        }
                    }
                }
            }
        }
        None
    }

    /// JSON: 指定ノードの `inputs` 内のフィールドをセットする
    pub fn inject_node_value(workflow: &mut serde_json::Value, node_id: &str, field: &str, value: serde_json::Value) -> Result<(), FactoryError> {
        let node = workflow.get_mut(node_id)
            .ok_or_else(|| FactoryError::ComfyWorkflowFailed { reason: format!("Node {} not found", node_id) })?;
        
        let inputs = node.get_mut("inputs")
            .ok_or_else(|| FactoryError::ComfyWorkflowFailed { reason: format!("Node {} has no inputs", node_id) })?;
            
        if let Some(obj) = inputs.as_object_mut() {
            obj.insert(field.to_string(), value);
            Ok(())
        } else {
            Err(FactoryError::ComfyWorkflowFailed { reason: format!("Node {} inputs is not an object", node_id) })
        }
    }

    /// KSampler ノードの positive/negative 入力に繋がっている CLIPTextEncode ノードを特定し、
    /// Pony V6 XL 専用の品質タグ (score_9...) と 拒絶呪文 (uncanny, nsfw...) を強制挿入する。
    pub fn enforce_pony_quality_and_safety(workflow: &mut serde_json::Value) -> Result<(), FactoryError> {
        let neg_curse = ", score_6, score_5, score_4, score_3, score_2, score_1, \
            nsfw, explicit, deformed, ugly, bad anatomy, bad hands, bad fingers, extra digits, fewer digits, \
            text, watermark, signature, username, uncanny, creepy, fleshy, biological horror, gross, \
            worst quality, low quality, normal quality, blurry, out of focus, 3d, photo, realistic, \
            jpeg artifacts, mutation, extra limbs, simple background";
        
        let pos_blessing = "score_9, score_8_up, score_7_up, source_anime, masterpiece, best quality, rating_safe, ";
        
        let mut negative_node_ids = std::collections::HashSet::new();
        let mut positive_node_ids = std::collections::HashSet::new();
        
        if let Some(nodes) = workflow.as_object() {
            for (_, node) in nodes {
                if let Some(class_type) = node.get("class_type").and_then(|v| v.as_str()) {
                    if class_type == "KSampler" || class_type == "KSamplerAdvanced" {
                        if let Some(inputs) = node.get("inputs") {
                            // Negative
                            if let Some(negative) = inputs.get("negative").and_then(|v| v.as_array()) {
                                if let Some(neg_id) = negative.first().and_then(|v| v.as_str()) {
                                    negative_node_ids.insert(neg_id.to_string());
                                }
                            }
                            // Positive
                            if let Some(positive) = inputs.get("positive").and_then(|v| v.as_array()) {
                                if let Some(pos_id) = positive.first().and_then(|v| v.as_str()) {
                                    positive_node_ids.insert(pos_id.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Negative の呪い
        for neg_id in negative_node_ids {
            if let Some(node) = workflow.get_mut(&neg_id) {
                if let Some(class_type) = node.get("class_type").and_then(|v| v.as_str()) {
                    if class_type == "CLIPTextEncode" {
                        if let Some(inputs) = node.get_mut("inputs") {
                            if let Some(text) = inputs.get_mut("text") {
                                if let Some(t_str) = text.as_str() {
                                    if !t_str.contains("score_6") {
                                        let new_text = format!("{}{}", t_str, neg_curse);
                                        *text = serde_json::Value::String(new_text);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Positive の祝福 (Quality tags)
        for pos_id in positive_node_ids {
            if let Some(node) = workflow.get_mut(&pos_id) {
                if let Some(class_type) = node.get("class_type").and_then(|v| v.as_str()) {
                    if class_type == "CLIPTextEncode" {
                        if let Some(inputs) = node.get_mut("inputs") {
                            if let Some(text) = inputs.get_mut("text") {
                                if let Some(t_str) = text.as_str() {
                                    if !t_str.contains("score_9") {
                                        let new_text = format!("{}{}", pos_blessing, t_str);
                                        *text = serde_json::Value::String(new_text);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    pub async fn clear_comfy_queue(&self) -> Result<(), FactoryError> {
        let http_base = self.api_url.replace("ws://", "http://").replace("/ws", "");
        let url = format!("{}/queue", http_base);
        let payload = serde_json::json!({"clear": true});
        
        match self.shield.post(&url, &payload).await {
            Ok(res) if res.status().is_success() => Ok(()),
            Ok(res) => Err(FactoryError::ComfyConnection { url, source: anyhow::anyhow!("Failed to clear queue: HTTP {}", res.status()) }),
            Err(e) => Err(FactoryError::ComfyConnection { url, source: e.into() }),
        }
    }

    /// ComfyUI の output ディレクトリにある、指定した接頭辞 (job_id) を持つすべてのファイルを削除する
    pub fn delete_output_debris(&self, prefix: &str) {
        let output_dir = self.base_dir.join("output");
        if let Ok(entries) = std::fs::read_dir(output_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with(prefix) {
                        if let Err(e) = std::fs::remove_file(&path) {
                            tracing::warn!("Failed to delete output debris {:?}: {}", path, e);
                        } else {
                            tracing::info!("🧹 ComfyBridge: Erased output debris -> {:?}", path);
                        }
                    }
                }
            }
        }
    }
}

#[async_trait]
impl VideoGenerator for ComfyBridgeClient {
    async fn generate_video(
        &self,
        prompt: &str,
        workflow_id: &str,
        input_image: Option<&std::path::Path>,
    ) -> Result<VideoResponse, FactoryError> {
        // 1. The Zombie Queue 排除 (Pre-flight Queue Purge)
        self.clear_comfy_queue().await?;

        // 2. ワークフロー JSON のロード
        let workflow_path = std::env::current_dir()
            .map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?
            .join("resources").join("workflows").join(format!("{}.json", workflow_id));
            
        let mut workflow: serde_json::Value = {
            let json_str = tokio::fs::read_to_string(&workflow_path).await
                .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to read workflow JSON: {}", e) })?;
            serde_json::from_str(&json_str)
                .map_err(|e| FactoryError::ComfyWorkflowFailed { reason: format!("Invalid JSON: {}", e) })?
        };

        // 3. ランダムな追跡用ジョブIDとシードの発行
        let job_id = uuid::Uuid::new_v4().to_string();
        let seed: u64 = rand::random();

        // 4. The Trinity Injection (3点動的注入)
        let prompt_node = Self::find_node_id_by_title(&workflow, "[API_PROMPT]")
            .ok_or_else(|| FactoryError::ComfyWorkflowFailed { reason: "Missing [API_PROMPT] node".into() })?;
        Self::inject_node_value(&mut workflow, &prompt_node, "text", serde_json::Value::String(prompt.to_string()))?;

        if let Some(sampler_node) = Self::find_node_id_by_title(&workflow, "[API_SAMPLER]") {
            Self::inject_node_value(&mut workflow, &sampler_node, "seed", serde_json::Value::Number(seed.into()))?;
        }
        
        // （映像ワークフローの場合は API_SAVE_VIDEO という名前かもしれないが、基本は API_SAVE を使用）
        if let Some(save_node) = Self::find_node_id_by_title(&workflow, "[API_SAVE]") {
            Self::inject_node_value(&mut workflow, &save_node, "filename_prefix", serde_json::Value::String(job_id.clone()))?;
        }

        // 4.5 TOS Guillotine: 物理的な NSFW/Gore 遮断 & 品質タグ強制 (プロンプト注入後に適用)
        Self::enforce_pony_quality_and_safety(&mut workflow)?;

        // 5. Zero-Copy Input Injection (入力画像渡し)
        let mut injected_input_name = None;
        if let Some(img_path) = input_image {
            let unique_name = self.inject_input_file(img_path, &job_id).await?;
            injected_input_name = Some(unique_name.clone());
            if let Some(img_node) = Self::find_node_id_by_title(&workflow, "[API_IMAGE_INPUT]") {
                Self::inject_node_value(&mut workflow, &img_node, "image", serde_json::Value::String(unique_name))?;
            }
        }

        // 6. WebSocket 接続確立 (The Blind Submission 回避)
        let ws_url = format!("{}?clientId={}", self.api_url, job_id);
        let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
            .await.map_err(|e| FactoryError::ComfyConnection { url: ws_url.clone(), source: e.into() })?;

        // 7. プロンプト（実行指令）送信
        let http_base = self.api_url.replace("ws://", "http://").replace("/ws", "");
        let prompt_url = format!("{}/prompt", http_base);
        let payload = serde_json::json!({
            "prompt": workflow,
            "client_id": job_id
        });
        
        let post_res = self.shield.post(&prompt_url, &payload).await
            .map_err(|e| FactoryError::ComfyConnection { url: prompt_url.clone(), source: e.into() })?;
            
        if !post_res.status().is_success() {
            return Err(FactoryError::ComfyWorkflowFailed { reason: format!("POST /prompt failed: {}", post_res.status()) });
        }
        
        let post_body: serde_json::Value = post_res.json().await
            .map_err(|e| FactoryError::ComfyWorkflowFailed { reason: e.to_string() })?;
            
        let prompt_id = post_body.get("prompt_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FactoryError::ComfyWorkflowFailed { reason: "No prompt_id returned".into() })?
            .to_string();

        // 8. WebSocket Receiver Loop (タイムアウト付き沈黙クラッシュ回避)
        use futures_util::StreamExt;
        let timeout_duration = std::time::Duration::from_secs(self.timeout_secs);
        let mut final_filename = None;
        
        let ws_loop = async {
            while let Some(msg) = ws_stream.next().await {
                let msg = match msg {
                    Ok(m) => m,
                    Err(e) => return Err(FactoryError::ComfyWorkflowFailed { reason: format!("WS Error: {}", e) }),
                };
                
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(&text) {
                        let msg_type = event.get("type").and_then(|t| t.as_str());
                        let data = event.get("data");
                        
                        if msg_type == Some("execution_error") {
                            return Err(FactoryError::ComfyWorkflowFailed { reason: format!("ComfyUI reported execution_error: {:?}", data) });
                        }
                        
                        if msg_type == Some("executed") && data.and_then(|d| d.get("prompt_id")).and_then(|v| v.as_str()) == Some(&prompt_id) {
                            if let Some(d) = data {
                                // 9. The Output Divergence: 画像、GIF、動画の全フォールバック解析
                                if let Some(output) = d.get("output") {
                                    for key in ["images", "gifs", "videos"] {
                                        if let Some(arr) = output.get(key).and_then(|v| v.as_array()) {
                                            if let Some(first) = arr.first() {
                                                if let Some(fname) = first.get("filename").and_then(|v| v.as_str()) {
                                                    final_filename = Some(fname.to_string());
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            break; // 処理完了
                        }
                    }
                }
            }
            Ok(())
        };

        // タイムアウト監視を実行
        let res = tokio::time::timeout(timeout_duration, ws_loop).await
            .map_err(|_| FactoryError::ComfyWorkflowFailed { reason: "WebSocket Timeout while waiting for 'executed'".into() })?;
            
        // 10. The Input Debris (Input Garbage Collection)
        // タイムアウトや直前のエラー等に関わらず、Inputが作られていた場合は確実に清掃する
        if let Some(injected_name) = injected_input_name {
            let input_file_path = self.base_dir.join("input").join(&injected_name);
            if input_file_path.exists() {
                if let Err(e) = std::fs::remove_file(&input_file_path) {
                    tracing::warn!("Failed to GC input debris {:?}: {}", input_file_path, e);
                }
            }
        }

        res?; // ws_loop 内部のエラーをここで評価

        let name = final_filename.ok_or_else(|| FactoryError::ComfyWorkflowFailed { reason: "No filename collected from 'executed' event".into() })?;
        
        let out_path = self.base_dir.join("output").join(name);
        if !out_path.exists() {
            return Err(FactoryError::ComfyWorkflowFailed { reason: format!("Expected output file does not exist: {:?}", out_path) });
        }
        
        Ok(VideoResponse {
            output_path: out_path.to_string_lossy().to_string(),
            job_id,
        })
    }

    async fn health_check(&self) -> Result<bool, FactoryError> {
        // ws://127.0.0.1:8188/ws などの末尾の /ws を削って http に直すための簡易処理
        // ただし、今の `health_check` で `/system_stats` を叩くには REST HTTP が必要。
        // ここでは api_url が `ws://` から始まっている場合、 `http://` に書き換えてベースURLを作る
        let http_base = self.api_url.replace("ws://", "http://").replace("/ws", "");
        let url = format!("{}/system_stats", http_base);
        match self.shield.get(&url).await {
            Ok(res) => Ok(res.status().is_success()),
            Err(e) => Err(FactoryError::ComfyConnection {
                url: http_base,
                source: e.into(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ComfyArgs {
    /// 動画のプロンプト
    pub prompt: String,
    /// 使用するワークフローID
    pub workflow_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ComfyOutput {
    /// 生成されたファイルの保存パス
    pub output_path: String,
}

#[async_trait]
impl AgentAct for ComfyBridgeClient {
    type Input = VideoRequest;
    type Output = VideoResponse;

    async fn execute(
        &self,
        input: Self::Input,
        _jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, FactoryError> {
        let input_path = input.input_image.as_deref().map(std::path::Path::new);
        self.generate_video(&input.prompt, &input.workflow_id, input_path).await
    }
}

impl Tool for ComfyBridgeClient {
    const NAME: &'static str = "comfy_bridge";
    type Args = ComfyArgs;
    type Output = ComfyOutput;
    type Error = FactoryError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "ComfyUI を使用して、プロンプトに基づいた画像や動画を生成します。".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ComfyArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let res = self.generate_video(&args.prompt, &args.workflow_id, None).await?;
        Ok(ComfyOutput {
            output_path: res.output_path,
        })
    }
}

impl ComfyBridgeClient {
    /// 静止画に対して Ken Burns エフェクト (Pan & Zoom) を適用し、滑らかな動画クリップを生成する
    /// VE-01: 数学的なイージング関数による脱カクつき実装
    /// 静止画に対して Ken Burns エフェクト (Pan & Zoom) を適用し、滑らかな動画クリップを生成する
    /// VE-01: 数学的なイージング関数による脱カクつき実装
    pub async fn apply_ken_burns_effect(
        &self,
        image_path: &std::path::Path,
        duration_secs: f32,
        _jail: &bastion::fs_guard::Jail,
        style: &tuning::StyleProfile,
    ) -> Result<PathBuf, FactoryError> {
        let output_path = image_path.with_extension("mp4");
        info!("🎥 ComfyBridge: Applying Ken Burns effect (Style: {}) -> {}", style.name, output_path.display());

        // Polish: 30fps で 5秒間のズーム。
        // zoom='1 + zoom_speed * sin(...)': スタイルに応じた速度でサインカーブを描く
        // 30fps * duration_secs = total_frames
        let total_frames = (30.0 * duration_secs) as usize;
        let zoom_expr = format!("1+{}*sin(on/{}*3.14159/2)", style.zoom_speed * 100.0, total_frames); 
        
        // M4 Pro Optimization: Hardware acceleration + Proper Vertical Handling
        // First scale the image to a reasonable size (2K height) to allow zoom without extreme overhead.
        // 8K scale was causing massive slowdowns in the software zoompan filter.
        let filter = format!(
            "scale=-1:2160,zoompan=z='{}':d={}:s=1080x1920:fps=30,format=yuv420p",
            zoom_expr, total_frames
        );
        
        info!("MediaForge: Applying hardware-accelerated Ken Burns (M4 Pro)...");

        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-loop").arg("1")
            .arg("-i").arg(image_path)
            .arg("-vf").arg(filter)
            .arg("-c:v").arg("h264_videotoolbox") // M4 Pro Hardware Accel
            .arg("-b:v").arg("8000k")
            .arg("-t").arg(duration_secs.to_string())
            .arg("-pix_fmt").arg("yuv420p")
            .arg(&output_path)
            .stdin(Stdio::null()) // Avoid SIGTTIN on background execution
            .status()
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("FFmpeg execution failed: {}", e) })?;

        if !status.success() {
            return Err(FactoryError::Infrastructure { reason: "FFmpeg failed to apply Ken Burns effect".into() });
        }

        Ok(output_path)
    }
}


