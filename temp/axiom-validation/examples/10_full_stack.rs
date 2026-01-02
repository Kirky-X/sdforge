//! 10_full_stack - 完整功能示例
//!
//! 这个示例演示 Axiom 框架的所有功能集成。

use axiom::cache::CacheConfig;
use axiom::config::{ConfigLoader, init_logging};
use axiom::http::build_with_config;
use axiom::prelude::*;
use axiom::service_api;
use axiom::security::{ApiKeyAuth, RateLimiter};
use axiom::streaming::{create_stream_channel, StreamEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: u64,
    title: String,
    description: String,
    completed: bool,
}

#[derive(Debug, Deserialize)]
struct CreateTaskRequest {
    title: String,
    description: String,
}

type TaskDatabase = Arc<Mutex<HashMap<u64, Task>>>;

#[service_api(
    name = "list_tasks",
    version = "v1",
    description = "List all tasks",
    path = "/tasks",
    method = "GET",
    cache_ttl = 30
)]
async fn list_tasks(db: TaskDatabase) -> Result<Vec<Task>, ApiError> {
    let tasks = db.lock().unwrap();
    Ok(tasks.values().cloned().collect())
}

#[service_api(
    name = "create_task",
    version = "v1",
    description = "Create a new task",
    path = "/tasks",
    method = "POST"
)]
async fn create_task(req: CreateTaskRequest, db: TaskDatabase) -> Result<Task, ApiError> {
    let mut tasks = db.lock().unwrap();
    let new_id = tasks.len() as u64 + 1;

    let task = Task {
        id: new_id,
        title: req.title,
        description: req.description,
        completed: false,
    };

    tasks.insert(new_id, task.clone());
    Ok(task)
}

#[service_api(
    name = "stream_tasks",
    version = "v1",
    description = "Stream task updates",
    path = "/tasks/stream",
    method = "GET",
    stream = true
)]
async fn stream_tasks(db: TaskDatabase) -> Result<axiom::streaming::StreamResponse<Task>, ApiError> {
    let (tx, rx) = create_stream_channel(32);
    let tasks = db.lock().unwrap().clone();

    tokio::spawn(async move {
        for task in tasks.values() {
            let _ = tx.send(Ok(task.clone())).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });

    Ok(axiom::streaming::StreamResponse::new(rx))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("Axiom 完整功能示例 - 任务管理 API");
    println!("========================================");
    println!();

    // 加载配置
    let config_path = "configs/full.toml";
    let loader = ConfigLoader::new(config_path, "AXIOM");
    let config = loader.load()?;

    // 初始化日志
    if let Some(logging_config) = &config.logging {
        init_logging(logging_config);
    }

    println!("✅ 配置加载完成");
    println!();

    // 创建数据库
    let db: TaskDatabase = Arc::new(Mutex::new(HashMap::new()));

    // 初始化示例数据
    {
        let mut tasks = db.lock().unwrap();
        tasks.insert(1, Task {
            id: 1,
            title: "Learn Axiom".to_string(),
            description: "Study the Axiom framework".to_string(),
            completed: false,
        });
        tasks.insert(2, Task {
            id: 2,
            title: "Build API".to_string(),
            description: "Create REST API with Axiom".to_string(),
            completed: false,
        });
    }

    println!("✅ 数据库初始化完成");
    println!();

    // 配置缓存
    let cache_config = CacheConfig::default();
    println!("✅ 缓存配置: TTL={}秒", cache_config.ttl_seconds);
    println!();

    // 配置认证和速率限制
    let auth = Arc::new(ApiKeyAuth::new());
    auth.add_key("demo-api-key", vec!["read".to_string(), "write".to_string()]);

    let limiter = Arc::new(RateLimiter::new(None));

    println!("✅ 安全配置完成");
    println!("  API Key: demo-api-key");
    println!("  速率限制: 100 请求 / 60 秒");
    println!();

    // 构建路由器
    let router = axiom::http::build();

    println!("✅ HTTP 路由器构建完成");
    println!();
    println!("========================================");
    println!("📡 服务地址: http://0.0.0.0:8080");
    println!();
    println!("📚 可用的 API 端点:");
    println!("  GET    /api/v1/tasks         - 获取任务列表（缓存）");
    println!("  POST   /api/v1/tasks         - 创建新任务");
    println!("  GET    /api/v1/tasks/stream  - 流式获取任务");
    println!();
    println!("💡 测试命令:");
    println!("  # 获取任务列表");
    println!("  curl http://localhost:8080/api/v1/tasks");
    println!();
    println!("  # 创建任务");
    println!("  curl -X POST http://localhost:8080/api/v1/tasks \\");
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -H \"X-API-Key: demo-api-key\" \\");
    println!("    -d '{\"title\":\"New Task\",\"description\":\"Test task\"}'");
    println!();
    println!("  # 流式获取任务");
    println!("  curl -N http://localhost:8080/api/v1/tasks/stream");
    println!();
    println!("按 Ctrl+C 停止服务");
    println!("========================================");
    println!();

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, router).await?;

    Ok(())
}