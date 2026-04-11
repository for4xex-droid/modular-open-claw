/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/a2a_internal.proto");

    // macOS Homebrew environment fallback for protoc
    if std::env::var("PROTOC").is_err() {
        if std::path::Path::new("/opt/homebrew/bin/protoc").exists() {
            std::env::set_var("PROTOC", "/opt/homebrew/bin/protoc");
        } else if std::path::Path::new("/usr/local/bin/protoc").exists() {
            std::env::set_var("PROTOC", "/usr/local/bin/protoc");
        }
    }

    // grpc feature が有効な場合のみ proto をコンパイル
    #[cfg(feature = "grpc")]
    {
        tonic_build::configure()
            .build_server(true)
            .build_client(true)
            .compile_protos(&["proto/a2a_internal.proto"], &["proto/"])?;
    }

    Ok(())
}
