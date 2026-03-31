use crate::security::BastionGuard;
use aiome_core::security::PermissionManifest;
use extism::{Manifest, Plugin, Wasm};
use serde_json::Value;
use tracing::warn;

use crate::constraint_checker::ActionHarness;

/// LLM が生成したコードを WASM 隔離空間で実行するハーネス
pub struct WasmHarness {
    domain: String,
    description: String,
    wasm_data: Vec<u8>,
    severity: u8,
}

impl WasmHarness {
    /// 新しい WasmHarness を作成する
    pub fn new(
        domain: impl Into<String>,
        description: impl Into<String>,
        wasm_data: Vec<u8>,
        severity: u8,
    ) -> Self {
        Self {
            domain: domain.into(),
            description: description.into(),
            wasm_data,
            severity,
        }
    }
}

impl ActionHarness for WasmHarness {
    fn is_legal_action(&self, action: &str, input: &Value, output: &Value) -> bool {
        let manifest = Manifest::new([Wasm::data(self.wasm_data.clone())])
            .with_timeout(std::time::Duration::from_secs(5));

        let mut plugin = match Plugin::new(&manifest, [], true) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "WasmHarness ({}): Failed to initialize plugin - {}",
                    self.domain, e
                );
                return false;
            }
        };

        let payload = serde_json::json!({
            "action": action,
            "input": input,
            "output": output,
        });

        match serde_json::to_vec(&payload) {
            Ok(input_bytes) => {
                let permissions = PermissionManifest::default();
                let _guard = BastionGuard::new(permissions);

                match plugin.call::<&[u8], String>("is_legal_action", &input_bytes) {
                    Ok(res_str) => {
                        let trimmed = res_str.trim();
                        trimmed == "true" || trimmed == "1"
                    }
                    Err(e) => {
                        warn!("WasmHarness ({}): Execution failed - {}", self.domain, e);
                        false
                    }
                }
            }
            Err(_) => false,
        }
    }

    fn describe_constraint(&self) -> String {
        self.description.clone()
    }

    fn domain(&self) -> &str {
        &self.domain
    }

    fn severity(&self) -> u8 {
        self.severity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_wasm_harness_invalid_wasm() {
        let harness = WasmHarness::new("TestDomain", "Test Desc", vec![0, 1, 2, 3], 90); // Invalid WASM bytes

        let is_legal = harness.is_legal_action("speak", &json!({}), &json!({}));
        assert!(!is_legal); // Fails safe (closed) on invalid WASM
    }
}
