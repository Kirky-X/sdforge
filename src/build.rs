fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc")]
    {
        tonic_build::configure()
            .build_server(true)
            .compile_protos(&["../proto/sdforge.v1.proto"], &["../proto"])?;
    }
    Ok(())
}
