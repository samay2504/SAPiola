fn main() {
    tonic_build::configure()
        .compile_protos(
            &["../proto/sapiola/v1/cdc.proto", "../proto/sapiola/v1/common.proto", "../proto/sapiola/v1/storage.proto"],
            &["../proto"],
        )
        .unwrap();
}
