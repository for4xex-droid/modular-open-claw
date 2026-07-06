/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::state::SharedState;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension};
use chrono::Utc;
use nurture_bridge::{sql_tx_exec, sql_tx_fetch_all};
use tracing::{error, info};
use uuid::Uuid;

pub async fn forget_actor(
    Path(actor_id): Path<Uuid>,
    Extension(state): axum::Extension<SharedState>,
) -> impl IntoResponse {
    info!("🗑️ [Internal/Forget] Request to forget actor: {}", actor_id);

    let actor_id_str = actor_id.to_string();

    // GDPR Right to be forgotten (Article 17)
    // 全 PII 関連テーブルをアトミックにパージする。
    // 物理アセット削除は DB commit 後に実行（外部 I/O は rollback 不可能なため）。
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("❌ [Internal/Forget] Failed to begin transaction: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to begin PII scrub transaction",
            )
                .into_response();
        }
    };

    // 1. KYC ステータスのパージ
    if let Err(e) = sql_tx_exec!(
        &mut tx,
        sqlite: "DELETE FROM nurture_kyc_status WHERE actor_id = ?",
        pg: "DELETE FROM nurture_kyc_status WHERE actor_id = $1",
        &actor_id_str
    ) {
        error!(
            "❌ [Internal/Forget] Failed to scrub KYC for actor {}: {}",
            actor_id, e
        );
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to scrub KYC").into_response();
    }

    // 2. DRM ライセンスのパージ（暗号化された復号キーを含む PII）
    if let Err(e) = sql_tx_exec!(
        &mut tx,
        sqlite: "DELETE FROM nurture_licenses WHERE owner_id = ?",
        pg: "DELETE FROM nurture_licenses WHERE owner_id = $1",
        &actor_id_str
    ) {
        error!(
            "❌ [Internal/Forget] Failed to scrub licenses for actor {}: {}",
            actor_id, e
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to scrub licenses",
        )
            .into_response();
    }

    // 3. エスクロー処理:
    //    - pending 状態のエスクローは wallet に返金してから削除
    //    - released/refunded 状態のエスクローはそのまま削除
    let pending_escrows: Vec<(String, i64)> = match sql_tx_fetch_all!(
        &mut tx,
        (String, i64),
        sqlite: "SELECT escrow_id, amount FROM nurture_escrows WHERE agent_id = ? AND status = 'pending'",
        pg: "SELECT escrow_id, amount FROM nurture_escrows WHERE agent_id = $1 AND status = 'pending'",
        &actor_id_str
    ) {
        Ok(rows) => rows,
        Err(e) => {
            error!(
                "❌ [Internal/Forget] Failed to query pending escrows for actor {}: {}",
                actor_id, e
            );
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query escrows").into_response();
        }
    };

    // pending エスクローの返金（wallet が存在しない場合は無視 — どうせ wallet も削除される）
    for (escrow_id, amount) in &pending_escrows {
        if let Err(e) = sql_tx_exec!(
            &mut tx,
            sqlite: "UPDATE nurture_wallets SET balance = balance + ? WHERE actor_id = ?",
            pg: "UPDATE nurture_wallets SET balance = balance + $1 WHERE actor_id = $2",
            amount,
            &actor_id_str
        ) {
            error!(
                "⚠️ [Internal/Forget] Failed to refund escrow {} (non-fatal, wallet will be deleted): {}",
                escrow_id, e
            );
            // 続行: wallet 自体も削除されるため、返金失敗は致命的ではない
        }

        // 🚨 WARNING-3 修正: Merkle 監査ハッシュチェーンの連続性を保持するため、
        //   Ledger に返金エントリを挿入する。
        //   これにより GDPR 忘却処理でも nurture_ledger の金融記録が完全になる。
        //   amount が 0 の場合は insert_ledger_refund_entry が Err を返すのでスキップする。
        if *amount > 0 {
            use nurture_infra::economy::bridge::NurtureCommerceBridge;
            if let Err(e) = NurtureCommerceBridge::insert_ledger_refund_entry_pub(
                &mut tx,
                &actor_id_str,
                *amount,
                Utc::now(),
            )
            .await
            {
                error!(
                    "⚠️ [Internal/Forget] Ledger refund entry failed for escrow {} (non-fatal, wallet will be deleted): {}",
                    escrow_id, e
                );
                // 返金の Ledger 記録失敗は非致命的: wallet はどうせ削除されるため導き続く
            }
        }
    }

    // 全エスクロー削除（pending/released/refunded すべて）
    if let Err(e) = sql_tx_exec!(
        &mut tx,
        sqlite: "DELETE FROM nurture_escrows WHERE agent_id = ?",
        pg: "DELETE FROM nurture_escrows WHERE agent_id = $1",
        &actor_id_str
    ) {
        error!(
            "❌ [Internal/Forget] Failed to scrub escrows for actor {}: {}",
            actor_id, e
        );
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to scrub escrows").into_response();
    }

    // 4. ウォレットのパージ（残高・支出履歴を含む経済 PII）
    if let Err(e) = sql_tx_exec!(
        &mut tx,
        sqlite: "DELETE FROM nurture_wallets WHERE actor_id = ?",
        pg: "DELETE FROM nurture_wallets WHERE actor_id = $1",
        &actor_id_str
    ) {
        error!(
            "❌ [Internal/Forget] Failed to scrub wallet for actor {}: {}",
            actor_id, e
        );
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to scrub wallet").into_response();
    }

    // 5. サブスクリプションのパージ
    if let Err(e) = sql_tx_exec!(
        &mut tx,
        sqlite: "DELETE FROM nurture_subscriptions WHERE actor_id = ?",
        pg: "DELETE FROM nurture_subscriptions WHERE actor_id = $1",
        &actor_id_str
    ) {
        error!(
            "❌ [Internal/Forget] Failed to scrub subscriptions for actor {}: {}",
            actor_id, e
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to scrub subscriptions",
        )
            .into_response();
    }

    // 5.5. ウィッシュリストのパージ（欲求の行動記録 = PII 相当。D-5 で追加）
    if let Err(e) = sql_tx_exec!(
        &mut tx,
        sqlite: "DELETE FROM nurture_wishlist WHERE agent_id = ?",
        pg: "DELETE FROM nurture_wishlist WHERE agent_id = $1",
        &actor_id_str
    ) {
        error!(
            "❌ [Internal/Forget] Failed to scrub wishlist for actor {}: {}",
            actor_id, e
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to scrub wishlist",
        )
            .into_response();
    }

    // 6. Customer PII の難読化 (emailはNULL化、stripe_customer_idは難読化)
    // レコード自体は購入履歴等の参照完全性のために残すが、PIIは消す
    // stripe_customer_id は NOT NULL 制約があるため、ダミー値（'purged_' + actor_id等）に置き換える
    if let Err(e) = sql_tx_exec!(
        &mut tx,
        sqlite: "UPDATE nurture_customers SET email = NULL, stripe_customer_id = 'purged_' || actor_id WHERE actor_id = ?",
        pg: "UPDATE nurture_customers SET email = NULL, stripe_customer_id = 'purged_' || actor_id WHERE actor_id = $1",
        &actor_id_str
    ) {
        error!(
            "❌ [Internal/Forget] Failed to scrub customer PII for actor {}: {}",
            actor_id, e
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to scrub customer PII",
        )
            .into_response();
    }

    // 5. DB トランザクションのコミット
    if let Err(e) = tx.commit().await {
        error!(
            "❌ [Internal/Forget] Failed to commit PII scrub for actor {}: {}",
            actor_id, e
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to commit PII scrub",
        )
            .into_response();
    }

    info!(
        "✅ [Internal/Forget] DB PII scrub committed for actor {}. Starting physical asset purge...",
        actor_id
    );

    // 6. 物理アセットの非同期パージ（DB commit 後 — ロールバック不可能な外部 I/O）
    //    失敗しても DB 側は既にクリーン。孤立アセットは後日の GC バッチで回収可能。
    if let Err(e) = state
        .asset_storage
        .delete_assets_for_actor(&commerce_protocol::identity::ActorId(actor_id))
        .await
    {
        error!(
            "⚠️ [Internal/Forget] Physical asset purge failed for actor {} (DB already clean): {}",
            actor_id, e
        );
        // HTTP 200 を返す: DB はクリーン済み。物理アセットはバックグラウンド GC で回収。
        return (
            StatusCode::OK,
            "PII scrubbed from DB. Physical asset purge deferred due to storage error.",
        )
            .into_response();
    }

    info!(
        "✅ [Internal/Forget] Complete GDPR purge finished for actor {}",
        actor_id
    );
    (StatusCode::OK, "PII and assets successfully purged").into_response()
}
