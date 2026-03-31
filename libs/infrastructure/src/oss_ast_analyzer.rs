/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use std::path::Path;
use syn::{Item, Type, Visibility};

/// Rust ソースコードから抽出された API 要素
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiSurface {
    pub structs: Vec<StructInfo>,
    pub enums: Vec<EnumInfo>,
    pub traits: Vec<TraitInfo>,
    pub functions: Vec<FunctionInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub is_pub: bool,
    pub fields: Vec<FieldInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnumInfo {
    pub name: String,
    pub is_pub: bool,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraitInfo {
    pub name: String,
    pub is_pub: bool,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub is_pub: bool,
    pub args: Vec<String>,
    pub return_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub ty: String,
    pub is_pub: bool,
}

/// Rust AST を解析して API サーフェスを抽出する
pub struct OssAstAnalyzer;

impl Default for OssAstAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl OssAstAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// ソースコード文字列を解析して API サーフェスを抽出する
    pub fn analyze_source(&self, source: &str) -> Result<ApiSurface, AiomeError> {
        let file = syn::parse_file(source).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to parse Rust source: {}", e),
        })?;

        let mut surface = ApiSurface {
            structs: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            functions: Vec::new(),
        };

        for item in file.items {
            match item {
                Item::Struct(s) => {
                    let is_pub = matches!(s.vis, Visibility::Public(_));
                    let fields = s
                        .fields
                        .iter()
                        .map(|f| FieldInfo {
                            name: f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default(),
                            ty: self.type_to_string(&f.ty),
                            is_pub: matches!(f.vis, Visibility::Public(_)),
                        })
                        .collect();
                    surface.structs.push(StructInfo {
                        name: s.ident.to_string(),
                        is_pub,
                        fields,
                    });
                }
                Item::Enum(e) => {
                    let is_pub = matches!(e.vis, Visibility::Public(_));
                    let variants = e.variants.iter().map(|v| v.ident.to_string()).collect();
                    surface.enums.push(EnumInfo {
                        name: e.ident.to_string(),
                        is_pub,
                        variants,
                    });
                }
                Item::Trait(t) => {
                    let is_pub = matches!(t.vis, Visibility::Public(_));
                    let methods = t
                        .items
                        .iter()
                        .filter_map(|it| {
                            if let syn::TraitItem::Fn(m) = it {
                                Some(m.sig.ident.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    surface.traits.push(TraitInfo {
                        name: t.ident.to_string(),
                        is_pub,
                        methods,
                    });
                }
                Item::Fn(f) => {
                    let is_pub = matches!(f.vis, Visibility::Public(_));
                    let args = f
                        .sig
                        .inputs
                        .iter()
                        .map(|i| match i {
                            syn::FnArg::Receiver(_) => "self".to_string(),
                            syn::FnArg::Typed(pt) => self.type_to_string(&pt.ty),
                        })
                        .collect();
                    let return_type = match &f.sig.output {
                        syn::ReturnType::Default => "()".to_string(),
                        syn::ReturnType::Type(_, ty) => self.type_to_string(ty),
                    };
                    surface.functions.push(FunctionInfo {
                        name: f.sig.ident.to_string(),
                        is_pub,
                        args,
                        return_type,
                    });
                }
                _ => {}
            }
        }

        Ok(surface)
    }

    /// ディレクトリ内の全ての .rs ファイルを再帰的に解析する
    pub fn analyze_directory(&self, dir: &Path) -> Result<Vec<ApiSurface>, AiomeError> {
        let mut surfaces = Vec::new();
        if !dir.exists() {
            return Ok(surfaces);
        }

        let mut stack = vec![dir.to_path_buf()];
        while let Some(current_dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(current_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if name != "target" && name != ".git" && name != "node_modules" {
                            stack.push(path);
                        }
                    } else if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false)
                    {
                        let source = std::fs::read_to_string(&path).map_err(|e| {
                            AiomeError::Infrastructure {
                                reason: format!("Failed to read source file {:?}: {}", path, e),
                            }
                        })?;
                        if let Ok(surface) = self.analyze_source(&source) {
                            surfaces.push(surface);
                        }
                    }
                }
            }
        }

        Ok(surfaces)
    }

    fn type_to_string(&self, ty: &Type) -> String {
        quote::quote!(#ty).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_struct() {
        let source = "pub struct Test { pub field: String, private_field: i32 }";
        let analyzer = OssAstAnalyzer::new();
        let surface = analyzer.analyze_source(source).unwrap();

        assert_eq!(surface.structs.len(), 1);
        assert_eq!(surface.structs[0].name, "Test");
        assert!(surface.structs[0].is_pub);
        assert_eq!(surface.structs[0].fields.len(), 2);
        assert_eq!(surface.structs[0].fields[0].name, "field");
        assert!(surface.structs[0].fields[0].is_pub);
        assert!(!surface.structs[0].fields[1].is_pub);
    }

    #[test]
    fn test_analyze_fn() {
        let source = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
        let analyzer = OssAstAnalyzer::new();
        let surface = analyzer.analyze_source(source).unwrap();

        assert_eq!(surface.functions.len(), 1);
        assert_eq!(surface.functions[0].name, "add");
        assert_eq!(surface.functions[0].return_type, "i32");
    }
}
