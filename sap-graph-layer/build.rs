fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["../proto/sapiola/v1/storage.proto", "../proto/sapiola/v1/graph.proto"],
            &["../proto"],
        )?;
    Ok(())
}
