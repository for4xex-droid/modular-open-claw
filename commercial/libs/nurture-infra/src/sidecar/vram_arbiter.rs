/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

//! VRAM 使用量の調停器 (Concrete Implementation)。
//! 複数エージェントが同一GPU上で推論する際の競合を、論理的なクォータ管理によって制御する。

use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// VRAM 予約情報のハンドル。
/// DROP 時に自動的にメモリ枠を解放する (RAII)。
pub struct VramReservation {
    amount_mb: u64,
    state: Arc<Mutex<VramState>>,
}

impl Drop for VramReservation {
    fn drop(&mut self) {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("❌ VramArbiter state poisoned in Drop: {}", e);
                e.into_inner()
            }
        };
        state.allocated = state.allocated.saturating_sub(self.amount_mb);
        debug!(
            "📦 VRAM Released: {} MB (Current: {}/{} MB)",
            self.amount_mb, state.allocated, state.total_mb
        );
    }
}

struct VramState {
    total_mb: u64,
    allocated: u64,
}

/// VRAM 調停器。
/// システム全体で単一の状態を共有し、リソースの競合を防ぐ。
pub struct VramArbiter {
    state: Arc<Mutex<VramState>>,
}

impl VramArbiter {
    /// 指定された容量 (MB) で調停器を初期化する。
    pub fn new(total_mb: u64) -> Self {
        info!("🚀 Initializing VramArbiter with {} MB capacity", total_mb);
        Self {
            state: Arc::new(Mutex::new(VramState {
                total_mb,
                allocated: 0,
            })),
        }
    }

    /// VRAM を予約する。容量が不足している場合は None を返す。
    pub fn reserve(&self, amount_mb: u64) -> Option<VramReservation> {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("❌ VramArbiter state poisoned in reserve: {}", e);
                e.into_inner()
            }
        };

        let remaining = state.total_mb.saturating_sub(state.allocated);
        if remaining >= amount_mb {
            state.allocated += amount_mb;
            debug!(
                "✅ VRAM Reserved: {} MB (Current: {}/{} MB)",
                amount_mb, state.allocated, state.total_mb
            );
            Some(VramReservation {
                amount_mb,
                state: Arc::clone(&self.state),
            })
        } else {
            debug!(
                "❌ VRAM Shortage: Requested {} MB, but only {} MB available",
                amount_mb, remaining
            );
            None
        }
    }

    /// 現在の割当状況を取得する (デバッグ用)。
    pub fn status(&self) -> (u64, u64) {
        let state = match self.state.lock() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("❌ VramArbiter state poisoned in status: {}", e);
                e.into_inner()
            }
        };
        (state.allocated, state.total_mb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vram_arbitration_lifecycle() {
        let arbiter = VramArbiter::new(1024); // 1GB

        // 成功する予約
        let res1 = arbiter.reserve(400);
        assert!(res1.is_some());
        assert_eq!(arbiter.status().0, 400);

        // 重なる予約
        let res2 = arbiter.reserve(400);
        assert!(res2.is_some());
        assert_eq!(arbiter.status().0, 800);

        // Cap 超過による失敗
        let res3 = arbiter.reserve(300);
        assert!(res3.is_none());
        assert_eq!(arbiter.status().0, 800);

        // 解放 (RAII)
        drop(res1);
        assert_eq!(arbiter.status().0, 400);

        // 解放後に再試行して成功
        let res4 = arbiter.reserve(300);
        assert!(res4.is_some());
        assert_eq!(arbiter.status().0, 700);
    }
}
