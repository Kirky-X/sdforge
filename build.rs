// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let proto_files = &[std::path::Path::new(&manifest_dir).join("proto/sdforge.v1.proto")];
    let proto_includes = &[std::path::Path::new(&manifest_dir).join("proto")];
    // diting LOW-001 修复：生成物写入标准 OUT_DIR（此前写入源树 src/grpc/pb 并检入，
    // 破坏 cargo package 且易与再生成内容漂移）
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    println!("cargo:rerun-if-changed={}", proto_files[0].display());

    std::fs::create_dir_all(&out_dir)?;

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile_protos(proto_files, proto_includes)?;

    println!("cargo:warning=Generated protobuf files to OUT_DIR");

    Ok(())
}
