fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let proto_files = &[std::path::Path::new(&manifest_dir).join("proto/sdforge.v1.proto")];
    let proto_includes = &[std::path::Path::new(&manifest_dir).join("proto")];
    let out_dir = std::path::Path::new(&manifest_dir).join("src/grpc/pb");

    println!("cargo:rerun-if-changed={}", proto_files[0].display());

    std::fs::create_dir_all(&out_dir)?;

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile_protos(proto_files, proto_includes)?;

    println!("cargo:warning=Generated protobuf files to src/grpc/pb");

    Ok(())
}
