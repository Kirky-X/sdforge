fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto file should be compiled before source files
    // Use absolute path relative to manifest directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let proto_path = std::path::Path::new(&manifest_dir).join("proto/sdforge.v1.proto");
    println!("cargo:rerun-if-changed={}", proto_path.display());

    tonic_build::configure()
        .build_server(true)
        .compile_protos(&[proto_path], &[std::path::Path::new(&manifest_dir).join("proto")])?;

    Ok(())
}
