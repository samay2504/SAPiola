fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["../proto/sapiola/v1/graph.proto", "../proto/sapiola/v1/common.proto"],
            &["../proto"],
        )?;
    Ok(())
}
