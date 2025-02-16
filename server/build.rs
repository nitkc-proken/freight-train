fn main() -> Result<(), Box<dyn std::error::Error>> {
    /* tonic_build::compile_protos("proto/gateway/Gateway.proto")?; */
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&["proto/gateway/Gateway.proto"], &["proto/gateway"])?;
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["proto/backend/Backend.proto"], &["proto/backend"])?;
    Ok(())
}
