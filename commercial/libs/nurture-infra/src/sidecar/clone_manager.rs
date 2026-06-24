/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::sidecar::launcher::{SidecarInstance, SidecarLauncher};
use crate::sidecar::vram_arbiter::VramArbiter;
use aiome_core_contracts::contracts::FederatedKarma;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::JobQueue;
use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::{DateTime, Duration, Utc};
use commerce_protocol::identity::ActorId;
use dashmap::DashMap;
use ed25519_dalek::{SigningKey, VerifyingKey};
#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use nurture_bridge::db::DatabasePool;
use nurture_bridge::{sql_exec, sql_fetch_all_map, sql_fetch_optional_map};
use nurture_core::ledger::{EconomyLedger, EntryType, LedgerEntry};
use rand::rngs::OsRng;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CloneSpec {
    pub parent_actor_id: ActorId,
    pub clone_id: Uuid,
    pub specialization: String,
    pub karma_snapshot: Vec<FederatedKarma>,
    pub max_duration: Duration,
    pub resource_budget: ResourceBudget,
}

#[derive(Clone)]
pub struct ResourceBudget {
    pub vram_mb: u64,
    pub max_cpu_percent: u8,
    pub max_memory_mb: u64,
}

#[derive(Debug, Clone)]
pub enum CloneStatus {
    Spawning,
    Active { started_at: DateTime<Utc> },
    Merging,
    Completed,
    Failed(String),
    Promoted,
}

pub struct CloneInstance {
    pub spec: CloneSpec,
    pub instance: SidecarInstance,
    pub started_at: DateTime<Utc>,
}

pub struct CloneManager {
    vram_arbiter: Arc<VramArbiter>,
    ledger: Arc<dyn EconomyLedger>,
    _job_queue: Arc<dyn JobQueue>,
    pool: DatabasePool,
    active_clones: DashMap<Uuid, CloneInstance>,
    max_concurrent: u8,
    system_actor_id: ActorId,
}

impl CloneManager {
    pub fn new(
        vram_arbiter: Arc<VramArbiter>,
        ledger: Arc<dyn EconomyLedger>,
        job_queue: Arc<dyn JobQueue>,
        pool: DatabasePool,
        max_concurrent: u8,
        system_actor_id: ActorId,
    ) -> Self {
        Self {
            vram_arbiter,
            ledger,
            _job_queue: job_queue,
            pool,
            active_clones: DashMap::new(),
            max_concurrent,
            system_actor_id,
        }
    }

    /// 分身体を Fork (生成) する。
    /// 事前エスクロー方式 (E2改善) により、最大稼働時間分のコインを即時引落する。
    /// 🛡️ CRITICAL-1修正: 楽観的ロック (OCC) により TOCTOU 二重引き落としを防止する。
    pub async fn fork(&self, spec: CloneSpec) -> Result<Uuid, AiomeError> {
        if self.active_clones.len() >= usize::from(self.max_concurrent) {
            return Err(AiomeError::Infrastructure {
                reason: "Maximum concurrent clones reached".into(),
            });
        }

        // 🚨 V-01: 署名用鍵ペアの生成
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        let priv_key_b64 = BASE64_STANDARD.encode(signing_key.to_bytes());
        let pub_key_b64 = BASE64_STANDARD.encode(verifying_key.to_bytes());

        // 1. OCC: ウォレットバージョンとコイン残高を取得して早期チェック (CRITICAL-1 修正)
        //    `record_entry` の WHERE VERSION 制約が最終防衛ラインだが、
        //    ここで事前に残高不足を弾くことで不要な DB 競合を削減する。
        let wallet = self
            .ledger
            .get_balance(&spec.parent_actor_id)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to read wallet for OCC pre-check: {:?}", e),
            })?;

        let cost_coins = u64::try_from(spec.max_duration.num_minutes().max(1)).unwrap_or(1);

        if wallet.coin.balance < cost_coins {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Insufficient balance for Clone Fork: required={}, available={}",
                    cost_coins, wallet.coin.balance
                ),
            });
        }

        let escrow_tx_id = Uuid::new_v4();

        // 🔒 OCC: debit_account_version を付与することで、
        //    read(get_balance) → write(record_entry) の間に別トランザクションが
        //    残高を変更した場合、SQLiteEconomyLedger が OptimisticLockConflict を返す。
        let escrow_entry = LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: escrow_tx_id,
            asset_id: None,
            debit_account: spec.parent_actor_id,
            credit_account: self.system_actor_id,
            coin_amount: cost_coins,
            points_amount: 0,
            entry_type: EntryType::CloneFork,
            created_at: Utc::now(),
            debit_account_version: Some(wallet.version), // OCC version stamp
        };

        self.ledger
            .record_entry(&escrow_entry)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Escrow failed (possible OCC conflict): {:?}", e),
            })?;

        // 2. VRAM 予約
        let reservation = self.vram_arbiter.reserve(spec.resource_budget.vram_mb);
        if reservation.is_none() && spec.resource_budget.vram_mb > 0 {
            // 🚨 V-03: VRAM 予約失敗時のエスクロー返金
            let refund_entry = LedgerEntry {
                id: Uuid::new_v4(),
                transaction_id: escrow_tx_id,
                asset_id: None,
                debit_account: self.system_actor_id,
                credit_account: spec.parent_actor_id,
                coin_amount: cost_coins,
                points_amount: 0,
                entry_type: EntryType::CloneMerge,
                created_at: Utc::now(),
                debit_account_version: None,
            };
            if let Err(e) = self.ledger.record_entry(&refund_entry).await {
                tracing::error!(
                    "❌ Critical: Refund failed during VRAM reservation failure for clone {}: {:?}",
                    escrow_tx_id,
                    e
                );
            }
            return Err(AiomeError::Infrastructure {
                reason: "Insufficient VRAM. Escrow refund attempt logged.".into(),
            });
        }

        let clone_id = spec.clone_id;
        let parent_id_str = spec.parent_actor_id.0.to_string();

        // 3. サイドカー起動 (SC-1 基礎)
        let mut instance = match SidecarLauncher::spawn(
            "aiome-sidecar",
            &[
                "--mode",
                "clone",
                "--parent",
                &parent_id_str,
                "--clone-id",
                &clone_id.to_string(),
            ],
            reservation,
            Some(priv_key_b64),
        ) {
            Ok(inst) => inst,
            Err(e) => {
                // 🚨 修正: プロセス起動失敗時のエスクロー返金
                let refund_entry = LedgerEntry {
                    id: Uuid::new_v4(),
                    transaction_id: escrow_tx_id,
                    asset_id: None,
                    debit_account: self.system_actor_id,
                    credit_account: spec.parent_actor_id,
                    coin_amount: cost_coins,
                    points_amount: 0,
                    entry_type: EntryType::CloneMerge, // Merge または Refund 用のタイプ
                    created_at: Utc::now(),
                    debit_account_version: None,
                };
                if let Err(re) = self.ledger.record_entry(&refund_entry).await {
                    tracing::error!(
                        "❌ Critical: Refund failed after process spawn failure for clone {}: {:?}",
                        escrow_tx_id,
                        re
                    );
                }
                return Err(AiomeError::Infrastructure {
                    reason: format!("Process spawn failed: {}. Refund attempt logged.", e),
                });
            }
        };

        let pid = i64::from(instance.child.id());

        // 🚨 V-15: DB 永続化 (SC-1 基礎)
        let db_res = match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(
                    "INSERT INTO nurture_clone_instances (id, parent_actor_id, pid, public_key, specialization, status, karma_snapshot_count, started_at, escrow_coins, escrow_tx_id)
                     VALUES (?, ?, ?, ?, ?, 'Active', ?, ?, ?, ?)"
                )
                .bind(clone_id.to_string())
                .bind(&parent_id_str)
                .bind(pid)
                .bind(&pub_key_b64)
                .bind(&spec.specialization)
                .bind(i64::try_from(spec.karma_snapshot.len()).unwrap_or(0))
                .bind(Utc::now())
                .bind(i64::try_from(cost_coins).unwrap_or(0))
                .bind(escrow_tx_id.to_string())
                .execute(p)
                .await
                .map(|_| ())
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(
                    "INSERT INTO nurture_clone_instances (id, parent_actor_id, pid, public_key, specialization, status, karma_snapshot_count, started_at, escrow_coins, escrow_tx_id)
                     VALUES ($1, $2, $3, $4, $5, 'Active', $6, $7, $8, $9)"
                )
                .bind(clone_id.to_string())
                .bind(&parent_id_str)
                .bind(pid)
                .bind(&pub_key_b64)
                .bind(&spec.specialization)
                .bind(i64::try_from(spec.karma_snapshot.len()).unwrap_or(0))
                .bind(Utc::now())
                .bind(i64::try_from(cost_coins).unwrap_or(0))
                .bind(escrow_tx_id.to_string())
                .execute(p)
                .await
                .map(|_| ())
            }
        };

        if let Err(e) = db_res {
            // 🚨 Crisis-3: DB 記録に失敗した場合、既に起動したプロセスを道連れにしないよう kill する
            tracing::error!("❌ DB record failed for clone {}. Terminating process {} to prevent resource leak.", clone_id, pid);
            if let Err(e) = instance.child.kill() {
                tracing::warn!("Failed to kill clone process {}: {}", clone_id, e);
            }
            if let Err(e) = instance.child.wait() {
                tracing::warn!("Failed to wait for clone process {}: {}", clone_id, e);
            }

            // 🚨 Crisis-3b: DB 記録失敗時のエスクロー返金（V-03/spawn失敗と同一パターン）
            let refund_entry = LedgerEntry {
                id: Uuid::new_v4(),
                transaction_id: escrow_tx_id,
                asset_id: None,
                debit_account: self.system_actor_id,
                credit_account: spec.parent_actor_id,
                coin_amount: cost_coins,
                points_amount: 0,
                entry_type: EntryType::CloneMerge,
                created_at: Utc::now(),
                debit_account_version: None,
            };
            if let Err(re) = self.ledger.record_entry(&refund_entry).await {
                tracing::error!(
                    "❌ Critical: Refund failed after DB record failure for clone {}: {:?}",
                    escrow_tx_id,
                    re
                );
            }

            return Err(AiomeError::Infrastructure {
                reason: format!("DB record failed: {}. Escrow refund attempted.", e),
            });
        }

        let clone_instance = CloneInstance {
            spec,
            instance,
            started_at: Utc::now(),
        };

        self.active_clones.insert(clone_id, clone_instance);

        Ok(clone_id)
    }

    /// 分身体を終了し、未使用時間のコインを返金する。
    pub async fn terminate(&self, clone_id: Uuid, requester_sub: &str) -> Result<(), AiomeError> {
        // 🚨 M-3: 所有権確認
        if let Some(instance) = self.active_clones.get(&clone_id) {
            if instance.value().spec.parent_actor_id.0.to_string() != requester_sub {
                return Err(AiomeError::Infrastructure {
                    reason: "Forbidden: You do not own this clone".into(),
                });
            }
        } else {
            return Err(AiomeError::Infrastructure {
                reason: "Clone not found or already terminated".into(),
            });
        }
        self.do_terminate(clone_id, "Completed").await
    }

    pub fn list_active_clones_for_actor(&self, actor_id_str: &str) -> Vec<Uuid> {
        self.active_clones
            .iter()
            .filter(|it| it.value().spec.parent_actor_id.0.to_string() == actor_id_str)
            .map(|r| *r.key())
            .collect()
    }

    async fn do_terminate(&self, clone_id: Uuid, final_status: &str) -> Result<(), AiomeError> {
        if let Some((_, mut instance)) = self.active_clones.remove(&clone_id) {
            // プロセス終了
            if let Err(e) = instance.instance.child.kill() {
                tracing::warn!("Failed to kill expired clone process: {}", e);
            }

            // 返金計算 (E2 改善)
            let actual_duration = Utc::now() - instance.started_at;
            let actual_minutes = u64::try_from(actual_duration.num_minutes().max(1)).unwrap_or(1);
            let max_minutes =
                u64::try_from(instance.spec.max_duration.num_minutes().max(1)).unwrap_or(1);

            let mut coins_consumed = actual_minutes;

            if max_minutes > actual_minutes {
                let refund_coins = max_minutes - actual_minutes;

                // DB から元の escrow_tx_id を取得 (なければ新規)
                let escrow_tx_id = self
                    .get_escrow_tx_id(clone_id)
                    .await
                    .unwrap_or_else(Uuid::new_v4);

                let refund_entry = LedgerEntry {
                    id: Uuid::new_v4(),
                    transaction_id: escrow_tx_id, // 元のエスクローに関連付け
                    asset_id: None,
                    debit_account: self.system_actor_id,
                    credit_account: instance.spec.parent_actor_id,
                    coin_amount: refund_coins,
                    points_amount: 0,
                    entry_type: EntryType::CloneMerge,
                    created_at: Utc::now(),
                    debit_account_version: None,
                };

                // 🚨 M-4: 返金失敗をエラーとして扱いトランスアクションの整合性を守る
                if let Err(e) = self.ledger.record_entry(&refund_entry).await {
                    tracing::error!(
                        "❌ Refund failed critical error for clone {}: {:?}",
                        clone_id,
                        e
                    );
                    return Err(AiomeError::Infrastructure {
                        reason: format!("Refund failed: {:?}", e),
                    });
                }
            } else {
                coins_consumed = max_minutes;
            }

            // 🚨 V-15: DB ステータス更新
            match &self.pool {
                DatabasePool::Sqlite(p) => {
                    sqlx::query(
                        "UPDATE nurture_clone_instances SET status = ?, completed_at = ?, coins_consumed = ? WHERE id = ?"
                    )
                    .bind(final_status)
                    .bind(Utc::now())
                    .bind(i64::try_from(coins_consumed).unwrap_or(0))
                    .bind(clone_id.to_string())
                    .execute(p)
                    .await
                    .map(|_| ())
                }
                DatabasePool::Postgres(p) => {
                    sqlx::query(
                        "UPDATE nurture_clone_instances SET status = $1, completed_at = $2, coins_consumed = $3 WHERE id = $4"
                    )
                    .bind(final_status)
                    .bind(Utc::now())
                    .bind(i64::try_from(coins_consumed).unwrap_or(0))
                    .bind(clone_id.to_string())
                    .execute(p)
                    .await
                    .map(|_| ())
                }
            }
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to update clone status in DB: {}", e)
            })?;
        }
        Ok(())
    }

    pub fn list_active_clones(&self) -> Vec<Uuid> {
        self.active_clones.iter().map(|r| *r.key()).collect()
    }

    /// 🚨 V-26: タイムアウト監視と自動終了
    pub async fn run_maintenance(&self) {
        let mut to_terminate = Vec::new();

        for entry in self.active_clones.iter() {
            let clone_id = *entry.key();
            let instance = entry.value();
            let elapsed = Utc::now() - instance.started_at;

            if elapsed > instance.spec.max_duration {
                tracing::warn!("⏰ Clone {} timed out, terminating...", clone_id);
                to_terminate.push(clone_id);
            }
        }

        for id in to_terminate {
            if let Err(e) = self.do_terminate(id, "TimedOut").await {
                tracing::error!(
                    "❌ Failed to auto-terminate timed out clone {}: {:?}",
                    id,
                    e
                );
            }
        }
    }

    /// 🚨 V-17: 孤児プロセスの回収 (起動時に実行)
    pub async fn recover_orphans(&self) -> Result<(), AiomeError> {
        let active_rows = sql_fetch_all_map!(
            &self.pool,
            sqlite: "SELECT id, pid FROM nurture_clone_instances WHERE status = 'Active'",
            |row| Ok::<(String, i64), AiomeError>((row.get("id"), row.get("pid"))),
            pg: "SELECT id, pid FROM nurture_clone_instances WHERE status = 'Active'",
            |row| Ok::<(String, i64), AiomeError>((row.get("id"), row.get("pid")))
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        for (id_str, pid) in active_rows {
            let id = match Uuid::parse_str(&id_str) {
                Ok(u) => u,
                Err(_) => {
                    tracing::error!("❌ Corrupt ID in DB: {}", id_str);
                    continue;
                }
            };

            // PID が存在するか確認
            let alive = if pid > 0 {
                i32::try_from(pid).is_ok_and(is_process_alive)
            } else {
                false
            };

            if !alive {
                tracing::info!(
                    "👻 Found orphan record {}, but process {} is already dead. Updating DB.",
                    id,
                    pid
                );
                if let Err(e) = sql_exec!(
                    &self.pool,
                    sqlite: "UPDATE nurture_clone_instances SET status = 'Orphaned', completed_at = ? WHERE id = ?",
                    pg: "UPDATE nurture_clone_instances SET status = 'Orphaned', completed_at = $1 WHERE id = $2",
                    Utc::now(),
                    &id_str
                ) {
                    tracing::error!("❌ Failed to update orphan status for {}: {:?}", id, e);
                }
            } else {
                // 実際はここで active_clones に復旧させる必要があるが、
                // Child ハンドルを失っているため kill 以外は困難。
                // セキュリティ上、再起動時はクリーンアップが望ましいため強制終了。
                tracing::warn!(
                    "🛡️ Recovering orphan clone {}: Killing process {} for safety.",
                    id,
                    pid
                );
                if let Ok(p) = i32::try_from(pid) {
                    if let Err(e) = kill_process(p) {
                        tracing::warn!("⚠️ Failed to kill orphan process {}: {:?}", pid, e);
                    }
                }
                if let Err(e) = sql_exec!(
                    &self.pool,
                    sqlite: "UPDATE nurture_clone_instances SET status = 'Recovered', completed_at = ? WHERE id = ?",
                    pg: "UPDATE nurture_clone_instances SET status = 'Recovered', completed_at = $1 WHERE id = $2",
                    Utc::now(),
                    &id_str
                ) {
                    tracing::error!("❌ Failed to update recovered status for {}: {:?}", id, e);
                }
            }
        }
        Ok(())
    }

    pub fn list_active(&self, parent: &ActorId) -> Vec<(Uuid, CloneStatus)> {
        self.active_clones
            .iter()
            .filter(|it| it.value().spec.parent_actor_id == *parent)
            .map(|it| {
                (
                    *it.key(),
                    CloneStatus::Active {
                        started_at: it.value().started_at,
                    },
                )
            })
            .collect()
    }

    async fn get_escrow_tx_id(&self, clone_id: Uuid) -> Option<Uuid> {
        let res = sql_fetch_optional_map!(
            &self.pool,
            sqlite: "SELECT escrow_tx_id FROM nurture_clone_instances WHERE id = ?",
            |row| Ok::<String, AiomeError>(row.get(0)),
            pg: "SELECT escrow_tx_id FROM nurture_clone_instances WHERE id = $1",
            |row| Ok::<String, AiomeError>(row.get(0)),
            clone_id.to_string()
        )
        .ok()
        .flatten();

        res.and_then(|s| Uuid::parse_str(&s).ok())
    }
}

#[cfg(unix)]
fn is_process_alive(pid: i32) -> bool {
    signal::kill(Pid::from_raw(pid), None).is_ok()
}

#[cfg(unix)]
fn kill_process(pid: i32) -> Result<(), String> {
    signal::kill(Pid::from_raw(pid), Signal::SIGKILL).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn is_process_alive(pid: i32) -> bool {
    if let Ok(output) = std::process::Command::new("tasklist")
        .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains(&pid.to_string())
    } else {
        false
    }
}

#[cfg(not(unix))]
fn kill_process(pid: i32) -> Result<(), String> {
    let status = std::process::Command::new("taskkill")
        .args(&["/F", "/PID", &pid.to_string()])
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("taskkill exited with status: {}", s)),
        Err(e) => Err(e.to_string()),
    }
}
