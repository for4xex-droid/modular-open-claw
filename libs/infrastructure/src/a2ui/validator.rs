/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::a2ui::schema::{A2uiEnvelope, Component, Surface};
use aiome_core::error::AiomeError;

/// A2UI コンポーネントのホワイトリスト（許可されたタイプのみレンダリング可能）
const ALLOWED_COMPONENT_TYPES: &[&str] = &[
    "taskApproval",
    "taskResult",
    "text",
    "treasureItem",
    "list",
    "form",
    "button",
    "input",
    "progressBar",
    "dataTable",
    "chart",
    "alert",
    "cellStatus",
    "timeline",
    "codeBlock",
    "card",
    "voiceStore",
    "loraMarket",
    "walletWidget",
    "marketplaceItem",
];

/// ネストされた children の再帰深度上限（StackOverflow 防止）
const MAX_COMPONENT_DEPTH: u8 = 16;

/// 同一階層の children 要素数上限（広域 DoS 防止）
const MAX_CHILDREN_COUNT: usize = 64;

/// props JSON Value の再帰走査深度上限
const MAX_PROPS_DEPTH: u8 = 32;

/// Surface フィールドの最大長（XSS ペイロード過大防止）
const MAX_FIELD_LENGTH: usize = 256;

/// props 内の URL として危険なスキーム（SSRF 防止）
const BLOCKED_URL_SCHEMES: &[&str] = &[
    "file://",
    "ftp://",
    "gopher://",
    "javascript:",
    "data:text/html",
    "data:application",
];

pub struct A2uiValidator;

impl A2uiValidator {
    pub fn verify_a2ui_surface(envelope: &A2uiEnvelope) -> Result<(), AiomeError> {
        let components_to_check = match envelope {
            A2uiEnvelope::CreateSurface { surface } => {
                Self::verify_surface_fields(surface)?;
                &surface.components
            }
            A2uiEnvelope::UpdateComponents {
                surface_id,
                components,
            } => {
                Self::verify_field("surfaceId", surface_id)?;
                components
            }
            A2uiEnvelope::DeleteSurface { surface_id } => {
                Self::verify_field("surfaceId", surface_id)?;
                return Ok(());
            }
        };

        for component in components_to_check {
            Self::verify_component(component, 0)?;
        }

        Ok(())
    }

    /// Surface メタフィールドの長さとHTMLタグ混入を検証（XSS 防止）
    fn verify_surface_fields(surface: &Surface) -> Result<(), AiomeError> {
        for (field_name, value) in [
            ("id", &surface.id),
            ("version", &surface.version),
            ("source", &surface.source),
        ] {
            Self::verify_field(field_name, value)?;
        }
        Ok(())
    }

    /// 単一フィールドの長さ制限・HTMLタグ検証
    fn verify_field(field_name: &str, value: &str) -> Result<(), AiomeError> {
        if value.len() > MAX_FIELD_LENGTH {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "A2UI Surface.{} exceeds max length ({} > {})",
                    field_name,
                    value.len(),
                    MAX_FIELD_LENGTH
                ),
            });
        }
        if value.contains('<') || value.contains('>') {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "A2UI Surface.{} contains HTML-like characters: potential XSS",
                    field_name
                ),
            });
        }
        Ok(())
    }

    fn verify_component(component: &Component, depth: u8) -> Result<(), AiomeError> {
        if depth >= MAX_COMPONENT_DEPTH {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "A2UI component nesting too deep (max={}): possible DoS attack",
                    MAX_COMPONENT_DEPTH
                ),
            });
        }

        if component.children.len() > MAX_CHILDREN_COUNT {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "A2UI component children count exceeded (max={}): possible wide DoS attack",
                    MAX_CHILDREN_COUNT
                ),
            });
        }

        if !ALLOWED_COMPONENT_TYPES.contains(&component.r#type.as_str()) {
            return Err(AiomeError::SecurityViolation {
                reason: format!("Unauthorized A2UI component type: {}", component.r#type),
            });
        }

        // props 内の URL フィールドに対する SSRF 検証
        Self::verify_props_urls(&component.props, 0)?;

        for child in &component.children {
            Self::verify_component(child, depth + 1)?;
        }

        Ok(())
    }

    /// props 内の文字列値を再帰走査し、危険な URL スキームをブロック
    fn verify_props_urls(value: &serde_json::Value, depth: u8) -> Result<(), AiomeError> {
        if depth >= MAX_PROPS_DEPTH {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "A2UI props nesting too deep (max={}): possible DoS",
                    MAX_PROPS_DEPTH
                ),
            });
        }

        match value {
            serde_json::Value::String(s) => {
                // RED TEAM PATCH: Strip all whitespaces and control characters to prevent bypasses
                // like "  \t javascript:alert(1)" which browsers happily execute.
                let normalized = s
                    .replace(
                        |c: char| c.is_ascii_whitespace() || c.is_ascii_control(),
                        "",
                    )
                    .to_lowercase();

                // Add vbscript: and a broader block for data: to prevent bypasses like data:text/xml
                let blocked_schemes = [
                    "javascript:",
                    "vbscript:",
                    "file://",
                    "ftp://",
                    "gopher://",
                    "data:text/html",
                    "data:application",
                    "data:text/xml",
                    "data:image/svg+xml",
                ];

                for scheme in blocked_schemes {
                    if normalized.starts_with(scheme) {
                        return Err(AiomeError::SecurityViolation {
                            reason: format!("A2UI props contain blocked URL scheme: {}", scheme),
                        });
                    }
                }
                Ok(())
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    Self::verify_props_urls(item, depth + 1)?;
                }
                Ok(())
            }
            serde_json::Value::Object(map) => {
                for (_, v) in map {
                    Self::verify_props_urls(v, depth + 1)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a2ui::schema::Surface;

    #[test]
    fn test_validator_blocks_script_tags() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "test".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![Component {
                    r#type: "script".to_string(),
                    props: serde_json::Value::Null,
                    children: vec![],
                }],
            },
        };

        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(result.is_err());
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("Unauthorized"));
        } else {
            panic!("Expected SecurityViolation error");
        }
    }

    #[test]
    fn test_validator_allows_valid_nested_components() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "nested_test".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![Component {
                    r#type: "form".to_string(),
                    props: serde_json::Value::Null,
                    children: vec![
                        Component {
                            r#type: "input".to_string(),
                            props: serde_json::Value::Null,
                            children: vec![],
                        },
                        Component {
                            r#type: "button".to_string(),
                            props: serde_json::Value::Null,
                            children: vec![],
                        },
                    ],
                }],
            },
        };

        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(
            result.is_ok(),
            "Nested valid components should pass: {:?}",
            result
        );
    }

    #[test]
    fn test_validator_blocks_deeply_nested_dos_attack() {
        fn make_deep(depth: u8) -> Component {
            if depth == 0 {
                return Component {
                    r#type: "text".to_string(),
                    props: serde_json::Value::Null,
                    children: vec![],
                };
            }
            Component {
                r#type: "list".to_string(),
                props: serde_json::Value::Null,
                children: vec![make_deep(depth - 1)],
            }
        }

        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "dos_test".to_string(),
                version: "0.9".to_string(),
                source: "attacker".to_string(),
                components: vec![make_deep(MAX_COMPONENT_DEPTH + 1)],
            },
        };

        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(result.is_err(), "Deeply nested components must be rejected");
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("nesting too deep"));
        } else {
            panic!("Expected SecurityViolation for deep nesting");
        }
    }

    #[test]
    fn test_delete_surface_with_valid_id_passes() {
        let envelope = A2uiEnvelope::DeleteSurface {
            surface_id: "old_surface".to_string(),
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(result.is_ok(), "DeleteSurface with valid id should pass");
    }

    #[test]
    fn test_surface_id_xss_blocked() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "<script>alert(1)</script>".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![],
            },
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(result.is_err(), "Surface id with HTML tags must be blocked");
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("XSS"));
        }
    }

    #[test]
    fn test_surface_field_length_overflow_blocked() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "x".repeat(300),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![],
            },
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(result.is_err(), "Oversized Surface.id must be blocked");
    }

    #[test]
    fn test_props_ssrf_file_scheme_blocked() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "ssrf_test".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![Component {
                    r#type: "button".to_string(),
                    props: serde_json::json!({ "action_url": "file:///etc/passwd" }),
                    children: vec![],
                }],
            },
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(result.is_err(), "file:// scheme in props must be blocked");
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("blocked URL scheme"));
        }
    }

    #[test]
    fn test_props_ssrf_javascript_scheme_blocked() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "js_test".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![Component {
                    r#type: "button".to_string(),
                    props: serde_json::json!({ "href": "javascript:alert(1)" }),
                    children: vec![],
                }],
            },
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(
            result.is_err(),
            "javascript: scheme in props must be blocked"
        );
    }

    #[test]
    fn test_props_safe_https_url_allowed() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "safe_url_test".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![Component {
                    r#type: "button".to_string(),
                    props: serde_json::json!({ "action_url": "https://api.aiome.local/approve" }),
                    children: vec![],
                }],
            },
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(result.is_ok(), "HTTPS URLs in props should be allowed");
    }

    #[test]
    fn test_update_components_surface_id_xss_blocked() {
        let envelope = A2uiEnvelope::UpdateComponents {
            surface_id: "<img onerror=alert(1)>".to_string(),
            components: vec![Component {
                r#type: "text".to_string(),
                props: serde_json::Value::Null,
                children: vec![],
            }],
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(
            result.is_err(),
            "UpdateComponents with XSS surface_id must be blocked"
        );
    }

    #[test]
    fn test_delete_surface_id_xss_blocked() {
        let envelope = A2uiEnvelope::DeleteSurface {
            surface_id: "<script>steal()</script>".to_string(),
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(
            result.is_err(),
            "DeleteSurface with XSS surface_id must be blocked"
        );
    }

    #[test]
    fn test_validator_blocks_javascript_bypasses() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "test".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![Component {
                    r#type: "button".to_string(),
                    props: serde_json::json!({
                        "action": "  \t  \n javascript:alert(1)  "
                    }),
                    children: vec![],
                }],
            },
        };

        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(
            result.is_err(),
            "Red-Team bypass via whitespace/control-chars MUST be blocked"
        );
    }

    #[test]
    fn test_validator_blocks_wide_dos_attack() {
        let mut children = Vec::new();
        for _ in 0..65 {
            children.push(Component {
                r#type: "text".to_string(),
                props: serde_json::Value::Null,
                children: vec![],
            });
        }
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "wide-test".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![Component {
                    r#type: "list".to_string(),
                    props: serde_json::Value::Null,
                    children,
                }],
            },
        };

        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(
            result.is_err(),
            "Validator must block components with more than 64 children to prevent wide DoS"
        );
    }

    #[test]
    fn test_validator_allows_nurture_widget_types() {
        for component_type in [
            "voiceStore",
            "loraMarket",
            "walletWidget",
            "marketplaceItem",
        ] {
            let envelope = A2uiEnvelope::CreateSurface {
                surface: Surface {
                    id: format!("nurture-{}", component_type),
                    version: "0.9".to_string(),
                    source: "test".to_string(),
                    components: vec![Component {
                        r#type: component_type.to_string(),
                        props: serde_json::json!({"title": "Test", "price": 100}),
                        children: vec![],
                    }],
                },
            };
            let result = A2uiValidator::verify_a2ui_surface(&envelope);
            assert!(
                result.is_ok(),
                "Component type {} should be allowed: {:?}",
                component_type,
                result
            );
        }
    }

    #[test]
    fn test_validator_allows_card_with_nav_button_children() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "nav-card".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![Component {
                    r#type: "card".to_string(),
                    props: serde_json::json!({"title": "Go to logs", "content": "Open audit view"}),
                    children: vec![Component {
                        r#type: "button".to_string(),
                        props: serde_json::json!({"label": "View Logs", "action": "navigate:audit"}),
                        children: vec![],
                    }],
                }],
            },
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(
            result.is_ok(),
            "card with button children should pass: {:?}",
            result
        );
    }

    #[test]
    fn test_validator_blocks_unknown_nurture_widget_type() {
        let envelope = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "bad-widget".to_string(),
                version: "0.9".to_string(),
                source: "test".to_string(),
                components: vec![Component {
                    r#type: "paymentExecutor".to_string(),
                    props: serde_json::Value::Null,
                    children: vec![],
                }],
            },
        };
        let result = A2uiValidator::verify_a2ui_surface(&envelope);
        assert!(result.is_err(), "Unknown widget type must be rejected");
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("Unauthorized"));
        }
    }
}
