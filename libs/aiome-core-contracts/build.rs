fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/a2a_internal.proto");

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
