/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::workflow::validator::WorkflowValidator;
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::{Job, JobQueue, JobStatus};
use serde_json::{json, Value};
use std::time::Duration;

pub const SKIP_MARKER: &str = "__skipped__";
const PARENT_POLL_MS: u64 = 500;
const HTTP_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct KarmaDirectives {
    pub parent_job_id: Option<String>,
    pub parent_job_ids: Vec<String>,
    pub wait_mode: Option<String>,
    pub wait_mode_n: Option<usize>,
    pub branch: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParentOutcome {
    pub skipped: bool,
    pub input: String,
}

pub fn parse_karma_directives(job: &Job) -> KarmaDirectives {
    let mut out = KarmaDirectives {
        parent_job_id: None,
        parent_job_ids: Vec::new(),
        wait_mode: None,
        wait_mode_n: None,
        branch: None,
    };
    let Some(raw) = job.karma_directives.as_deref() else {
        return out;
    };
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return out;
    };
    out.parent_job_id = v
        .get("parent_job_id")
        .and_then(|x| x.as_str())
        .map(str::to_string);
    if let Some(arr) = v.get("parent_job_ids").and_then(|x| x.as_array()) {
        out.parent_job_ids = arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
    }
    out.wait_mode = v
        .get("wait_mode")
        .and_then(|x| x.as_str())
        .map(str::to_string);
    out.wait_mode_n = v
        .get("wait_mode_n")
        .and_then(|x| x.as_u64())
        .map(|n| n as usize);
    out.branch = v.get("branch").and_then(|x| x.as_str()).map(str::to_string);
    out
}

fn dep_timeout_secs() -> u64 {
    std::env::var("AIOME_WF_DEP_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600)
}

async fn wait_for_job_output(
    job_queue: &dyn JobQueue,
    job_id: &str,
    deadline: tokio::time::Instant,
) -> Result<String, AiomeError> {
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(AiomeError::RemoteServiceTimeout {
                timeout_secs: dep_timeout_secs(),
            });
        }
        match job_queue.fetch_job(job_id).await? {
            Some(j) => match j.status {
                JobStatus::Completed => {
                    return Ok(j.output_artifacts.unwrap_or_default());
                }
                JobStatus::Failed | JobStatus::Cancelled => {
                    return Err(AiomeError::Infrastructure {
                        reason: format!("親ジョブ {} が {:?} のため中断", job_id, j.status),
                    });
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(PARENT_POLL_MS)).await;
                }
            },
            None => {
                tokio::time::sleep(Duration::from_millis(PARENT_POLL_MS)).await;
            }
        }
    }
}

async fn wait_for_parallel_parents(
    job_queue: &dyn JobQueue,
    parent_ids: &[String],
    wait_mode: &str,
    wait_mode_n: Option<usize>,
    deadline: tokio::time::Instant,
) -> Result<Vec<String>, AiomeError> {
    if parent_ids.is_empty() {
        return Ok(vec![]);
    }

    if wait_mode == "Any" {
        for pid in parent_ids {
            if let Ok(out) = wait_for_job_output(job_queue, pid, deadline).await {
                if out != SKIP_MARKER {
                    return Ok(vec![out]);
                }
            }
        }
        return Err(AiomeError::Infrastructure {
            reason: "Parallel Any: 完了した親ジョブがありません".to_string(),
        });
    }

    let mut outputs = Vec::new();
    for pid in parent_ids {
        let out = wait_for_job_output(job_queue, pid, deadline).await?;
        if out != SKIP_MARKER {
            outputs.push(out);
        }
    }

    let required = match wait_mode {
        "N" => wait_mode_n.unwrap_or(1),
        _ => parent_ids.len(),
    };

    if outputs.len() < required {
        return Err(AiomeError::Infrastructure {
            reason: format!(
                "Parallel 合流未達: {}/{} 親ジョブ完了 (mode={})",
                outputs.len(),
                required,
                wait_mode
            ),
        });
    }

    Ok(outputs)
}

/// 親ジョブ完了待ち + Condition 偽枝 skip 判定
pub async fn await_parents(
    job: &Job,
    job_queue: &dyn JobQueue,
) -> Result<ParentOutcome, AiomeError> {
    let karma = parse_karma_directives(job);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(dep_timeout_secs());

    if !karma.parent_job_ids.is_empty() {
        let wait_mode = karma.wait_mode.as_deref().unwrap_or("All");
        let outputs = wait_for_parallel_parents(
            job_queue,
            &karma.parent_job_ids,
            wait_mode,
            karma.wait_mode_n,
            deadline,
        )
        .await?;
        let merged = if outputs.is_empty() {
            String::new()
        } else {
            serde_json::to_string(&outputs).unwrap_or_default()
        };
        return Ok(ParentOutcome {
            skipped: false,
            input: merged,
        });
    }

    let Some(parent_id) = karma.parent_job_id else {
        return Ok(ParentOutcome {
            skipped: false,
            input: String::new(),
        });
    };

    let parent_out = wait_for_job_output(job_queue, &parent_id, deadline).await?;
    if parent_out == SKIP_MARKER {
        return Ok(ParentOutcome {
            skipped: true,
            input: String::new(),
        });
    }

    if let Some(branch) = &karma.branch {
        if (branch == "true" || branch == "false") && parent_out != *branch {
            return Ok(ParentOutcome {
                skipped: true,
                input: parent_out,
            });
        }
    }

    Ok(ParentOutcome {
        skipped: false,
        input: parent_out,
    })
}

/// minijinja で `{{ input }}` 等を展開
pub fn render_workflow_template(template: &str, input: &str) -> Result<String, AiomeError> {
    if !template.contains("{{") {
        return Ok(template.to_string());
    }
    let mut env = minijinja::Environment::new();
    env.add_template("wf", template)
        .map_err(|e| AiomeError::Validation {
            reason: format!("テンプレート構文エラー: {}", e),
        })?;
    let ctx = json!({ "input": input });
    env.get_template("wf")
        .map_err(|e| AiomeError::Validation {
            reason: format!("テンプレート取得エラー: {}", e),
        })?
        .render(ctx)
        .map_err(|e| AiomeError::Validation {
            reason: format!("テンプレート展開エラー: {}", e),
        })
}

pub fn evaluate_transform(expression: &str, input: &str) -> Result<String, AiomeError> {
    let expr = expression.trim();
    if expr.is_empty() {
        return Ok(input.to_string());
    }
    if expr.starts_with("$.") {
        let key = expr.trim_start_matches("$.").trim();
        let val: Value = serde_json::from_str(input).unwrap_or(json!(input));
        return Ok(extract_json_path(&val, key).to_string());
    }
    render_workflow_template(expr, input)
}

fn extract_json_path(val: &Value, path: &str) -> Value {
    let mut current = val;
    for part in path.split('.') {
        if part.is_empty() {
            continue;
        }
        current = match current {
            Value::Object(map) => map.get(part).unwrap_or(&Value::Null),
            _ => return Value::Null,
        };
    }
    current.clone()
}

pub fn evaluate_condition_expression(expression: &str, input: &str) -> bool {
    let expr = expression.trim();
    if expr.is_empty() {
        return false;
    }
    let input_val: Value = serde_json::from_str(input).unwrap_or(json!(input));

    if let Some((left, right)) = expr.split_once("==") {
        let left_val = resolve_condition_operand(&input_val, left.trim());
        let right_trimmed = right.trim().trim_matches('"').trim_matches('\'');
        return left_val.as_str() == Some(right_trimmed);
    }

    if let Some(rest) = expr.strip_prefix("contains:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2 {
            let field_val = resolve_condition_operand(&input_val, parts[0].trim());
            if let Some(s) = field_val.as_str() {
                return s.contains(parts[1]);
            }
        }
        return false;
    }

    let key = expr.trim_start_matches("$.").trim();
    if let Some(obj) = input_val.as_object() {
        if let Some(v) = obj.get(key) {
            return v
                .as_bool()
                .unwrap_or(!v.is_null() && v != &json!("") && v != &json!(0));
        }
    }
    false
}

fn resolve_condition_operand(input_val: &Value, operand: &str) -> Value {
    if operand.starts_with("$.") {
        extract_json_path(input_val, operand.trim_start_matches("$."))
    } else if operand.starts_with('"') || operand.starts_with('\'') {
        json!(operand.trim_matches('"').trim_matches('\''))
    } else {
        extract_json_path(input_val, operand)
    }
}

pub async fn assert_runtime_url_safe(url: &str) -> Result<(), AiomeError> {
    WorkflowValidator::assert_resolved_url_safe(url)
        .await
        .map_err(|e| AiomeError::SecurityViolation {
            reason: format!("{:?}", e),
        })
}

pub async fn execute_http_request(
    client: &reqwest::Client,
    method: &str,
    url: &str,
    input: &str,
) -> Result<String, AiomeError> {
    let rendered_url = render_workflow_template(url, input)?;
    assert_runtime_url_safe(&rendered_url).await?;

    let m = method.to_uppercase();
    let req = match m.as_str() {
        "GET" => client.get(&rendered_url),
        "POST" => client.post(&rendered_url),
        "PUT" => client.put(&rendered_url),
        "PATCH" => client.patch(&rendered_url),
        "DELETE" => client.delete(&rendered_url),
        other => {
            return Err(AiomeError::Validation {
                reason: format!("未対応 HTTP メソッド: {}", other),
            });
        }
    };

    let resp = req
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AiomeError::RemoteServiceExecutionFailed {
            reason: format!("HTTP リクエスト失敗: {}", e),
        })?;

    let status = resp.status();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AiomeError::RemoteServiceExecutionFailed {
            reason: format!("HTTP レスポンス読取失敗: {}", e),
        })?;

    if bytes.len() > HTTP_MAX_BODY_BYTES {
        return Err(AiomeError::Validation {
            reason: format!(
                "HTTP レスポンスが上限 {} bytes を超過 ({} bytes)",
                HTTP_MAX_BODY_BYTES,
                bytes.len()
            ),
        });
    }

    let body = String::from_utf8_lossy(&bytes).into_owned();
    Ok(json!({ "status": status.as_u16(), "body": body }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mock_jq::MockJQ;
    use aiome_core_contracts::traits::JobStatus;
    use std::sync::Arc;

    fn completed_job(id: &str, output: &str) -> Job {
        Job {
            id: id.to_string(),
            category: "wf_test".to_string(),
            status: JobStatus::Completed,
            output_artifacts: Some(output.to_string()),
            ..Default::default()
        }
    }

    fn child_job(parent_id: &str, branch: Option<&str>) -> Job {
        let karma = if let Some(b) = branch {
            json!({ "parent_job_id": parent_id, "branch": b })
        } else {
            json!({ "parent_job_id": parent_id })
        };
        Job {
            id: "child-1".to_string(),
            category: "wf_llm".to_string(),
            status: JobStatus::Pending,
            karma_directives: Some(karma.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_render_workflow_template_input() {
        let out = render_workflow_template("Hello {{ input }}!", "World").unwrap();
        assert_eq!(out, "Hello World!");
    }

    #[test]
    fn test_evaluate_condition_expression_eq() {
        assert!(evaluate_condition_expression(
            "$.status == ok",
            r#"{"status":"ok"}"#
        ));
        assert!(!evaluate_condition_expression(
            "$.status == fail",
            r#"{"status":"ok"}"#
        ));
    }

    #[tokio::test]
    async fn test_await_parents_skip_marker() {
        let jq = Arc::new(MockJQ::default());
        jq.stored_jobs.lock().unwrap().insert(
            "parent-1".to_string(),
            completed_job("parent-1", SKIP_MARKER),
        );

        let job = child_job("parent-1", None);
        let outcome = await_parents(&job, jq.as_ref()).await.unwrap();
        assert!(outcome.skipped);
        assert!(outcome.input.is_empty());
    }

    #[tokio::test]
    async fn test_await_parents_branch_mismatch_skips() {
        let jq = Arc::new(MockJQ::default());
        jq.stored_jobs
            .lock()
            .unwrap()
            .insert("cond-1".to_string(), completed_job("cond-1", "false"));

        let job = child_job("cond-1", Some("true"));
        let outcome = await_parents(&job, jq.as_ref()).await.unwrap();
        assert!(outcome.skipped);
    }

    #[tokio::test]
    async fn test_await_parents_branch_match_passes() {
        let jq = Arc::new(MockJQ::default());
        jq.stored_jobs
            .lock()
            .unwrap()
            .insert("cond-1".to_string(), completed_job("cond-1", "true"));

        let job = child_job("cond-1", Some("true"));
        let outcome = await_parents(&job, jq.as_ref()).await.unwrap();
        assert!(!outcome.skipped);
        assert_eq!(outcome.input, "true");
    }

    #[tokio::test]
    async fn test_assert_runtime_url_safe_rejects_localhost() {
        let err = assert_runtime_url_safe("http://127.0.0.1:8080/api")
            .await
            .unwrap_err();
        assert!(matches!(err, AiomeError::SecurityViolation { .. }));
    }
}
