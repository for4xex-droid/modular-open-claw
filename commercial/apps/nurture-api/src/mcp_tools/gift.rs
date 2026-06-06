/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

//! MCP Tool: Gifting.
//! Allows agents to send gifts (items) to other agents.
//!
//! CSAM パイプラインは `NurtureCommerceBridge::deliver_gift` 内部で強制適用されるため、
//! この層ではバリデーション済みの `commerce_engine` へ移譲するだけでよい。

use crate::state::SharedState;
use commerce_protocol::error::NurtureError;
use uuid::Uuid;

pub async fn handle_gift_delivery(
    state: SharedState,
    item_id: Uuid,
    sender_id: Uuid,
    receiver_id: Uuid,
) -> Result<(), NurtureError> {
    // 🚨 CSAM ガード: commerce_engine 内部 (deliver_gift) で再度スキャンされるが、
    // MCP Tool の層でもログ出力を強化するために呼び出してもよい。
    // 今回は NurtureCommerceBridge::deliver_gift が全ての責任を負うため移譲する。

    tracing::info!(
        "🎁 Initiating gift delivery: item={} from={} to={}",
        item_id,
        sender_id,
        receiver_id
    );

    state
        .commerce_engine
        .deliver_gift(item_id, sender_id, receiver_id)
        .await
        .map_err(|e| NurtureError::Infrastructure(format!("Gift delivery failed: {}", e)))?;

    Ok(())
}
