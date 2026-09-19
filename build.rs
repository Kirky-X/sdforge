// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");

    // proto 编译仅在 grpc feature 下执行（CARGO_FEATURE_GRPC 由 cargo 对
    // 非 build-dependency 的 feature 自动注入）：default = [] 的纯核心
    // 用户无需安装 protoc。grpc 生成代码由 src/grpc 在 cfg(feature = "grpc")
    // 下 include OUT_DIR 产物，两侧门控必须保持一致。
    if std::env::var_os("CARGO_FEATURE_GRPC").is_some() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let proto_files = &[std::path::Path::new(&manifest_dir).join("proto/sdforge.v1.proto")];
        let proto_includes = &[std::path::Path::new(&manifest_dir).join("proto")];
        // 生成物写入标准 OUT_DIR（此前写入源树 src/grpc/pb 并检入，
        // 破坏 cargo package 且易与再生成内容漂移）
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

        println!("cargo:rerun-if-changed={}", proto_files[0].display());

        std::fs::create_dir_all(&out_dir)?;

        tonic_prost_build::configure()
            .build_server(true)
            .build_client(true)
            .out_dir(&out_dir)
            .compile_protos(proto_files, proto_includes)?;
    }

    Ok(())
}
