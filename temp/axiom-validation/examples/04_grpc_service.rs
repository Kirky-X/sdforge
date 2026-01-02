//! 04_grpc_service - gRPC 服务示例
//!
//! 这个示例演示如何使用 Axiom 框架创建 gRPC 服务。
//!
//! 运行方式:
//! ```bash
//! cargo run --bin 04_grpc_service
//! ```
//!
//! 测试方式:
//! 使用 grpcurl 或其他 gRPC 客户端工具

use axiom::grpc::{AxiomGrpcService, GrpcServerConfig};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    println!("========================================");
    println!("Axiom gRPC 服务示例");
    println!("========================================");
    println!();

    let service = AxiomGrpcService::default();

    println!("✅ gRPC 服务已创建");
    println!();
    println!("📡 gRPC 服务地址: 0.0.0.0:50051");
    println!();
    println!("📝 可用的 gRPC 方法:");
    println!("  Call      - 调用 API 方法");
    println!("  GetInfo   - 获取服务信息");
    println!();
    println!("按 Ctrl+C 停止服务");
    println!("========================================");
    println!();

    let addr = "0.0.0.0:50051".parse()?;

    Server::builder()
        .add_service(axiom::grpc::axiom_v1::axiom_service_server::AxiomServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}