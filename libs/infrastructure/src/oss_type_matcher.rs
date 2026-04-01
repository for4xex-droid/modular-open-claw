/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::llm::utils::extract_code_block;
use crate::oss_ast_analyzer::{ApiSurface, EnumInfo, FunctionInfo, StructInfo};
use aiome_core::error::AiomeError;
use aiome_core_contracts::llm::LlmProvider;
use std::sync::Arc;

/// 型やシグネチャの不一致情報
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeMismatch {
    pub category: MismatchCategory,
    pub name: String,
    pub detail: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MismatchCategory {
    MissingItem,
    TypeMismatch,
    VisibilityMismatch,
    ArgumentMismatch,
}

/// 2つの API サーフェスを比較して不一致を検出する
pub struct OssTypeMatcher;

impl Default for OssTypeMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl OssTypeMatcher {
    pub fn new() -> Self {
        Self
    }

    /// source (自プロジェクト) と target (OSS) を比較する
    pub fn compare(&self, source: &ApiSurface, target: &ApiSurface) -> Vec<TypeMismatch> {
        let mut mismatches = Vec::new();

        // 1. Structs
        for s_struct in &source.structs {
            if let Some(t_struct) = target.structs.iter().find(|s| s.name == s_struct.name) {
                // フィールドの比較
                for s_field in &s_struct.fields {
                    if let Some(t_field) = t_struct.fields.iter().find(|f| f.name == s_field.name) {
                        if s_field.ty != t_field.ty {
                            mismatches.push(TypeMismatch {
                                category: MismatchCategory::TypeMismatch,
                                name: format!("{}.{}", s_struct.name, s_field.name),
                                detail: format!(
                                    "Source type: {}, Target type: {}",
                                    s_field.ty, t_field.ty
                                ),
                                suggestion: format!(
                                    "Create a wrapper to convert {} to {}",
                                    t_field.ty, s_field.ty
                                ),
                            });
                        }
                    } else {
                        mismatches.push(TypeMismatch {
                            category: MismatchCategory::MissingItem,
                            name: format!("{}.{}", s_struct.name, s_field.name),
                            detail: "Field missing in target struct".to_string(),
                            suggestion: "Implement a default value or manual mapping".to_string(),
                        });
                    }
                }
            } else {
                mismatches.push(TypeMismatch {
                    category: MismatchCategory::MissingItem,
                    name: s_struct.name.clone(),
                    detail: "Struct missing in target surface".to_string(),
                    suggestion: "Implement the struct in the adapter".to_string(),
                });
            }
        }

        // 2. Functions
        for s_fn in &source.functions {
            if let Some(t_fn) = target.functions.iter().find(|f| f.name == s_fn.name) {
                if s_fn.args.len() != t_fn.args.len() {
                    mismatches.push(TypeMismatch {
                        category: MismatchCategory::ArgumentMismatch,
                        name: s_fn.name.clone(),
                        detail: format!(
                            "Argument count mismatch: source={}, target={}",
                            s_fn.args.len(),
                            t_fn.args.len()
                        ),
                        suggestion: "Adjust argument mapping".to_string(),
                    });
                }
                if s_fn.return_type != t_fn.return_type {
                    mismatches.push(TypeMismatch {
                        category: MismatchCategory::TypeMismatch,
                        name: s_fn.name.clone(),
                        detail: format!(
                            "Return type mismatch: source={}, target={}",
                            s_fn.return_type, t_fn.return_type
                        ),
                        suggestion: "Implement return value conversion".to_string(),
                    });
                }
            }
        }

        // 3. Enums
        for s_enum in &source.enums {
            if let Some(t_enum) = target.enums.iter().find(|e| e.name == s_enum.name) {
                for s_variant in &s_enum.variants {
                    if !t_enum.variants.contains(s_variant) {
                        mismatches.push(TypeMismatch {
                            category: MismatchCategory::MissingItem,
                            name: format!("{}:{}", s_enum.name, s_variant),
                            detail: "Enum variant missing in target enum".to_string(),
                            suggestion: "Add the missing variant or implement a mapping"
                                .to_string(),
                        });
                    }
                }
            } else {
                mismatches.push(TypeMismatch {
                    category: MismatchCategory::MissingItem,
                    name: s_enum.name.clone(),
                    detail: "Enum missing in target surface".to_string(),
                    suggestion: "Implement the enum in the adapter".to_string(),
                });
            }
        }

        // 4. Traits
        for s_trait in &source.traits {
            if let Some(t_trait) = target.traits.iter().find(|t| t.name == s_trait.name) {
                for s_method in &s_trait.methods {
                    if !t_trait.methods.contains(s_method) {
                        mismatches.push(TypeMismatch {
                            category: MismatchCategory::MissingItem,
                            name: format!("{}::{}", s_trait.name, s_method),
                            detail: "Trait method missing in target trait".to_string(),
                            suggestion: "Implement the missing method".to_string(),
                        });
                    }
                }
            } else {
                mismatches.push(TypeMismatch {
                    category: MismatchCategory::MissingItem,
                    name: s_trait.name.clone(),
                    detail: "Trait missing in target surface".to_string(),
                    suggestion: "Implement the trait or a wrapper in the adapter".to_string(),
                });
            }
        }

        mismatches
    }
}

/// 検出された不一致を解消するためのアダプタコードを生成する
pub struct OssAdapterCodeGen {
    llm: Arc<dyn LlmProvider>,
}

impl OssAdapterCodeGen {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    pub async fn generate_adapter(
        &self,
        mismatches: &[TypeMismatch],
        source_context: &str,
        target_context: &str,
    ) -> Result<String, AiomeError> {
        let prompt = format!(
            "You are an expert Rust developer. Generate an adapter module for Aiome OSS Integration.\n\
            Source (Our Project) Context:\n{}\n\n\
            Target (OSS Library) Context:\n{}\n\n\
            Mismatches Found:\n{:?}\n\n\
            Requirements:\n\
            1. Use 'pub' visibility for exported items.\n\
            2. Implement conversion traits (From/Into) where applicable.\n\
            3. Ensure the code is self-contained and compilable.\n\
            4. Wrap target calls in safe adapters.\n\
            Output ONLY the Rust code block.",
            source_context, target_context, mismatches
        );

        let response = self.llm.complete(&prompt, None).await?;

        // 共通ユーティリティを使用してコードブロックを抽出
        let code = extract_code_block(&response.content);

        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oss_ast_analyzer::{ApiSurface, EnumInfo, FieldInfo, StructInfo, TraitInfo};

    #[test]
    fn test_type_matcher_mismatch() {
        let source = ApiSurface {
            structs: vec![StructInfo {
                name: "User".to_string(),
                is_pub: true,
                fields: vec![FieldInfo {
                    name: "id".to_string(),
                    ty: "u64".to_string(),
                    is_pub: true,
                }],
            }],
            enums: vec![],
            traits: vec![],
            functions: vec![],
        };
        let target = ApiSurface {
            structs: vec![StructInfo {
                name: "User".to_string(),
                is_pub: true,
                fields: vec![FieldInfo {
                    name: "id".to_string(),
                    ty: "String".to_string(),
                    is_pub: true,
                }],
            }],
            enums: vec![],
            traits: vec![],
            functions: vec![],
        };

        let matcher = OssTypeMatcher::new();
        let mismatches = matcher.compare(&source, &target);

        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].category, MismatchCategory::TypeMismatch);
        assert!(mismatches[0].detail.contains("u64"));
        assert!(mismatches[0].detail.contains("String"));
    }

    #[test]
    fn test_compare_enums_and_traits() {
        let matcher = OssTypeMatcher::new();
        let source = ApiSurface {
            structs: vec![],
            functions: vec![],
            enums: vec![EnumInfo {
                name: "Status".into(),
                is_pub: true,
                variants: vec!["Active".into(), "Inactive".into()],
            }],
            traits: vec![TraitInfo {
                name: "Plugin".into(),
                is_pub: true,
                methods: vec!["init".into(), "shutdown".into()],
            }],
        };

        let target = ApiSurface {
            structs: vec![],
            functions: vec![],
            enums: vec![EnumInfo {
                name: "Status".into(),
                is_pub: true,
                variants: vec!["Active".into()], // Missing Inactive
            }],
            traits: vec![TraitInfo {
                name: "Plugin".into(),
                is_pub: true,
                methods: vec!["init".into()], // Missing shutdown
            }],
        };

        let mismatches = matcher.compare(&source, &target);
        assert_eq!(mismatches.len(), 2);

        let names: Vec<String> = mismatches.iter().map(|m| m.name.clone()).collect();
        assert!(names.contains(&"Status:Inactive".to_string()));
        assert!(names.contains(&"Plugin::shutdown".to_string()));
    }
}
