/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::invariant_dag::InvariantDag;

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_chain_append_and_verify() {
        let mut dag = InvariantDag::new();

        let n1 = dag.append(1, "job-1", "ls", vec!["path_in_sandbox".to_string()]);
        assert_eq!(n1.parent_hash, "0");

        let n2 = dag.append(
            2,
            "job-1",
            "cat",
            vec!["path_in_sandbox".to_string(), "no_vault_access".to_string()],
        );
        assert_eq!(n2.parent_hash, n1.hash);

        assert!(dag.verify_chain().is_ok());
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_chain_tamper_detect() {
        let mut dag = InvariantDag::new();
        dag.append(1, "job-1", "ls", vec!["p1".into()]);
        dag.append(2, "job-1", "cat", vec!["p2".into()]);

        // 正常時はパス
        assert!(dag.verify_chain().is_ok());

        // JSON 経由でデータを改竄
        let json = dag.to_json();
        let mut mal_dag = InvariantDag::from_json(&json).unwrap();

        // データの書き換え
        if let Some(node) = mal_dag.nodes_mut().get_mut(0) {
            node.action = "rm -rf /".to_string(); // 内容を改竄
        }

        // 検証失敗するはず
        assert!(
            mal_dag.verify_chain().is_err(),
            "Chain should be invalid after data tampering"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_rollback() {
        let mut dag = InvariantDag::new();
        let n1 = dag.append(1, "job-1", "ls", vec![]);
        let n2 = dag.append(2, "job-1", "grep", vec![]);
        let n3 = dag.append(3, "job-1", "cat", vec![]);

        let removed = dag.rollback_to(&n2.hash);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].hash, n3.hash);
        assert!(dag.verify_chain().is_ok());
    }
}
