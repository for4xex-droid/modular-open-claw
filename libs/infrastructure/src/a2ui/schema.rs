/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum A2uiEnvelope {
    #[serde(rename = "createSurface")]
    CreateSurface { surface: Surface },
    #[serde(rename = "updateComponents")]
    UpdateComponents {
        #[serde(rename = "surfaceId")]
        surface_id: String,
        components: Vec<Component>,
    },
    #[serde(rename = "deleteSurface")]
    DeleteSurface {
        #[serde(rename = "surfaceId")]
        surface_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Surface {
    pub id: String,
    pub version: String,
    pub source: String,
    #[serde(default)]
    pub components: Vec<Component>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Component {
    pub r#type: String,
    #[serde(default = "default_props")]
    pub props: serde_json::Value,
    #[serde(default)]
    pub children: Vec<Component>,
}

/// serde `#[serde(default = "...")]` 用ヘルパー。
/// `props` フィールドが JSON に含まれない場合、`null` ではなく空オブジェクト `{}` を返す。
fn default_props() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_a2ui_create_envelope() {
        let json_str = r#"{
            "type": "createSurface",
            "surface": {
                "id": "task_approval_1",
                "version": "0.9",
                "source": "aiome_backend",
                "components": [
                    {
                        "type": "taskApproval",
                        "props": { "description": "Delete files?" }
                    }
                ]
            }
        }"#;

        let envelope: A2uiEnvelope = serde_json::from_str(json_str)
            .expect("A2UI test: valid JSON must deserialize without error"); // allow-anti-pattern

        match envelope {
            A2uiEnvelope::CreateSurface { surface } => {
                assert_eq!(surface.id, "task_approval_1");
                assert_eq!(surface.components.len(), 1);
                assert_eq!(surface.components[0].r#type, "taskApproval");
                assert_eq!(surface.components[0].props["description"], "Delete files?");
            }
            _ => panic!("Wrong envelope type"),
        }
    }

    #[test]
    fn test_deserialize_a2ui_update_components() {
        let json_str = r#"{
            "type": "updateComponents",
            "surfaceId": "surface_42",
            "components": [
                { "type": "text", "props": { "content": "Updated!" } }
            ]
        }"#;

        let envelope: A2uiEnvelope =
            serde_json::from_str(json_str).expect("A2UI test: updateComponents must deserialize"); // allow-anti-pattern

        match envelope {
            A2uiEnvelope::UpdateComponents {
                surface_id,
                components,
            } => {
                assert_eq!(surface_id, "surface_42");
                assert_eq!(components.len(), 1);
                assert_eq!(components[0].r#type, "text");
            }
            _ => panic!("Expected UpdateComponents variant"),
        }
    }

    #[test]
    fn test_deserialize_a2ui_delete_surface() {
        let json_str = r#"{
            "type": "deleteSurface",
            "surfaceId": "old_surface"
        }"#;

        let envelope: A2uiEnvelope =
            serde_json::from_str(json_str).expect("A2UI test: deleteSurface must deserialize"); // allow-anti-pattern

        match envelope {
            A2uiEnvelope::DeleteSurface { surface_id } => {
                assert_eq!(surface_id, "old_surface");
            }
            _ => panic!("Expected DeleteSurface variant"),
        }
    }

    #[test]
    fn test_default_props_is_empty_object() {
        let json_str = r#"{"type": "text"}"#;
        let component: Component = serde_json::from_str(json_str)
            .expect("Component without props/children must deserialize"); // allow-anti-pattern

        assert!(
            component.props.is_object(),
            "Default props should be an empty object, not null"
        );
        assert_eq!(component.children.len(), 0);
    }

    #[test]
    fn test_serialize_round_trip_preserves_structure() {
        let original = A2uiEnvelope::CreateSurface {
            surface: Surface {
                id: "rt_test".to_string(),
                version: "0.9".to_string(),
                source: "backend".to_string(),
                components: vec![Component {
                    r#type: "taskApproval".to_string(),
                    props: serde_json::json!({"title": "Confirm?"}),
                    children: vec![],
                }],
            },
        };

        let json_str = serde_json::to_string(&original).expect("A2UI envelope must serialize"); // allow-anti-pattern
        let deserialized: A2uiEnvelope =
            serde_json::from_str(&json_str).expect("Serialized A2UI must round-trip back"); // allow-anti-pattern

        assert_eq!(original, deserialized, "Round-trip must preserve structure");
    }
}
