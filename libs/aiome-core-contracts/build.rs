/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/a2a_internal.proto");

    // grpc feature が有効な場合のみ proto をコンパイル
    #[cfg(feature = "grpc")]
    {
        // protoc の探索とフォールバック
        let has_protoc = std::env::var("PROTOC").is_ok()
            || std::process::Command::new("protoc")
                .arg("--version")
                .output()
                .is_ok()
            || std::path::Path::new("/opt/homebrew/bin/protoc").exists()
            || std::path::Path::new("/usr/local/bin/protoc").exists();

        if !has_protoc {
            // システムに protoc が見つからない場合は protobuf-src (Pure Rust) をビルドして使用
            let protoc_path = protobuf_src::protoc();
            std::env::set_var("PROTOC", protoc_path);
        } else if std::env::var("PROTOC").is_err() {
            // PATH にはあるが PROTOC 環境変数がないか、既知のパスにある場合
            if std::path::Path::new("/opt/homebrew/bin/protoc").exists() {
                std::env::set_var("PROTOC", "/opt/homebrew/bin/protoc");
            } else if std::path::Path::new("/usr/local/bin/protoc").exists() {
                std::env::set_var("PROTOC", "/usr/local/bin/protoc");
            }
        }

        tonic_build::configure()
            .build_server(true)
            .build_client(true)
            .compile_protos(&["proto/a2a_internal.proto"], &["proto/"])?;
    }

    Ok(())
}
