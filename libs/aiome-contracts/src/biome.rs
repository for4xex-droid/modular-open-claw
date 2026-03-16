use serde::{Deserialize, Serialize};
use base64::Engine;

/// Biome プロトコルにおける基本メッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMessage {
    /// 送信元の公開鍵 (Base64)
    pub sender_pubkey: String,
    /// 宛先の公開鍵 (Base64)
    pub recipient_pubkey: String,
    /// 対話のトピックID (Dialogue ID)
    pub topic_id: String,
    /// メッセージ本文 (暗号化されている場合は暗号文)
    pub content: String,
    /// 送信者の Karma 状態を参照する CID (Merkle DAG)
    pub karma_root_cid: String,
    /// Ed25519 署名
    pub signature: String,
    /// Lamport Clock 等のロジカル時刻
    pub lamport_clock: u64,
    /// タイムスタンプ
    pub timestamp: String,
    /// 暗号化方式 ("none", "aes-256-gcm", etc.)
    /// Phase 20 MVP では "none" を使用。
    pub encryption: String,
}

/// 対話の要約・状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDialogue {
    pub topic_id: String,
    pub peer_pubkey: String,
    pub last_message_at: String,
    pub message_count: u32,
    pub summary: Option<String>,
    pub status: DialogueStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DialogueStatus {
    Active,
    Archived,
    Blocked,
}

/// 対話の蒸留 (要約と相互署名)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueDistillation {
    pub topic_id: String,
    /// LLM によって生成された対話の要約
    pub summary: String,
    /// 参加者の公開鍵リスト
    pub participants: Vec<String>,
    /// 参加者全員の署名 (Base64)
    pub signatures: Vec<String>,
    pub timestamp: String,
}

impl BiomeMessage {
    /// メッセージ本文を指定された共有鍵で暗号化する
    pub fn encrypt(
        &mut self,
        key: &[u8; 32],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use chacha20poly1305::{
            aead::{Aead, KeyInit},
            ChaCha20Poly1305, Key, Nonce,
        };
        use rand::{thread_rng, RngCore};

        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let mut nonce_bytes = [0u8; 12];
        thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, self.content.as_bytes())
            .map_err(|e| format!("Encryption failed: {:?}", e))?;

        // Prepend nonce to ciphertext
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);

        self.content = base64::engine::general_purpose::STANDARD.encode(combined);
        self.encryption = "chacha20-poly1305".to_string();
        Ok(())
    }

    /// メッセージ本文を指定された共有鍵で復号する
    pub fn decrypt(
        &mut self,
        key: &[u8; 32],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.encryption == "none" {
            return Ok(());
        }

        use chacha20poly1305::{
            aead::{Aead, KeyInit},
            ChaCha20Poly1305, Key, Nonce,
        };

        let combined = base64::engine::general_purpose::STANDARD.decode(&self.content)?;
        if combined.len() < 12 {
            return Err("Invalid ciphertext: too short for nonce".into());
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {:?}", e))?;

        self.content = String::from_utf8(plaintext)?;
        Ok(())
    }
}

/// [A-1] Docker Agent ↔ Karma Feedback Loop
/// Represents the result of an execution inside the Docker sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl DelegationResult {
    /// Determines if the execution was successful based on exit code.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// High-level classification of the failure type.
    pub fn failure_category(&self) -> FailureCategory {
        if self.is_success() {
            return FailureCategory::None;
        }

        // Common exit codes and stderr patterns
        if self.exit_code == 124 || self.stderr.contains("timeout") {
            FailureCategory::Timeout
        } else if self.exit_code == 137
            || self.stderr.contains("OOM")
            || self.stderr.contains("Out of memory")
        {
            FailureCategory::Oom
        } else if self.stderr.contains("Module not found") || self.stderr.contains("ImportError") {
            FailureCategory::DependencyMissing
        } else if self.stderr.contains("SyntaxError") {
            FailureCategory::SyntaxError
        } else {
            FailureCategory::UnknownRuntime
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureCategory {
    None,
    Timeout,
    Oom,
    DependencyMissing,
    SyntaxError,
    UnknownRuntime,
}
