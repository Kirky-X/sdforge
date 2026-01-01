fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/axiom.proto");

    // Always compile proto files when tonic-build is available
    // The generated code will be conditionally compiled via #[cfg(feature = "grpc")]
    println!("Compiling proto files...");
    tonic_build::configure().compile_protos(&["proto/axiom.proto"], &["proto/"])?;
    println!("Proto files compiled successfully");
    Ok(())
}
