use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Deserialize)]
struct InputParams {
    name: String,
    factor: i32,
}

#[derive(Serialize)]
struct OutputResult {
    message: String,
    value: i32,
}

/// エージェントが実行環境からこの関数を呼び出します。
/// 引数は常に JSON 文字列として渡され、戻り値も JSON 文字列で返却されます。
#[wasm_bindgen]
pub fn execute(json_input: &str) -> String {
    // デシリアライズに失敗した場合はエラーJSONを返却
    let params: InputParams = match serde_json::from_str(json_input) {
        Ok(p) => p,
        Err(e) => {
            return serde_json::json!({
                "status": "error",
                "message": format!("Invalid JSON input: {}", e)
            }).to_string();
        }
    };

    // ドメインロジックの実行
    let computed_value = params.factor * 42;
    let result = OutputResult {
        message: format!("Hello, {}! Computation successful.", params.name),
        value: computed_value,
    };

    serde_json::to_string(&result).unwrap_or_else(|_| {
        r#"{"status":"error","message":"Failed to serialize result"}"#.to_string()
    })
}
