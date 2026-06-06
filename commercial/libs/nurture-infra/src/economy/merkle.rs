/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use base64::{engine::general_purpose, Engine as _};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// 台帳の改竄防止のためのハッシュ連鎖 (Audit Trail)
pub struct MerkleAudit;

impl MerkleAudit {
    /// 直前のハッシュと現在のエントリデータから、現在のエントリ用の監査ハッシュを生成する
    pub fn calculate(
        prev_hash: &str,
        entry_id: Uuid,
        entry_type: &str,
        debit: &str,
        credit: &str,
        amount_coins: u64,
        amount_points: u64,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(entry_id.as_bytes());
        hasher.update(entry_type.as_bytes());
        hasher.update(debit.as_bytes());
        hasher.update(credit.as_bytes());
        hasher.update(amount_coins.to_be_bytes());
        hasher.update(amount_points.to_be_bytes());

        let result = hasher.finalize();
        general_purpose::STANDARD.encode(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurture_core::ledger::EntryType;

    #[test]
    fn test_calculate_includes_entry_type() {
        let entry_id = Uuid::new_v4();
        let prev_hash = "GENESIS";
        let debit = "ACCOUNT_A";
        let credit = "ACCOUNT_B";

        // 本番コード (ledger.rs) と同一の serde_json シリアライズを使用
        let purchase_str = serde_json::to_string(&EntryType::Purchase).expect("serialize");
        let transfer_str = serde_json::to_string(&EntryType::Transfer).expect("serialize");

        let hash1 =
            MerkleAudit::calculate(prev_hash, entry_id, &purchase_str, debit, credit, 100, 0);

        let hash2 =
            MerkleAudit::calculate(prev_hash, entry_id, &transfer_str, debit, credit, 100, 0);

        assert_ne!(hash1, hash2, "Hashes should differ when entry_type differs");
    }

    #[test]
    fn test_hash_chain_continuity() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let entry_type = serde_json::to_string(&EntryType::Purchase).expect("serialize");

        let hash1 = MerkleAudit::calculate("sha256:initial", id1, &entry_type, "A", "B", 100, 10);

        let hash2 = MerkleAudit::calculate(&hash1, id2, &entry_type, "B", "C", 200, 20);

        // チェーンが連結されていることを確認
        assert_ne!(hash1, hash2, "Sequential hashes should differ");

        // 同じ入力なら同じ出力（決定論性）
        let hash2_again = MerkleAudit::calculate(&hash1, id2, &entry_type, "B", "C", 200, 20);
        assert_eq!(hash2, hash2_again, "Same inputs should produce same hash");
    }
}
