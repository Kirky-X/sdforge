// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! inklog 桥行为锚点（自研结构化日志栈删除后的结构化日志面）。
//!
//! 覆盖三条验收路径：
//! 1. 安装 inklog 后日志记录进入 inklog 管道（自定义捕获 sink 断言记录的
//!    结构化形态：级别/目标/消息/字段）。捕获走 inklog 原生 tracing 路径
//!    （`tracing::info!` 的结构化键值落入 `LogRecord.fields`）；`log`
//!    facade 路径按 inklog 设计喂 console/file 通道（`log` crate 无结构
//!    化字段），其接线由 `src/inklog.rs` 的 lib 用例覆盖。
//! 2. `LoggerManager::set_level` 调级后低于新级别的记录被过滤（观测：
//!    info 不再进 sink、warn 仍进）。
//! 3. `LoggerManager::from_file` 加载含轮转配置的 TOML 构造成功。
//!
//! 共享管理器构建在**泄漏的专用 runtime** 上：其 worker 常驻，若随用例
//! runtime 销毁，`Runtime::drop` 等待常驻 blocking 任务会挂死测试进程。
//! 用例本体只用当前 runtime sleep，不需要 flavor 切换。
//!
//! inklog 全局 logger/subscriber 为进程级单例，全部用例 `#[serial]`。

#![cfg(feature = "inklog")]

use sdforge::inklog::{InklogError, LogRecord, LogSink, LoggerManager, init_inklog_logger};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// 捕获 sink：把收到的 `LogRecord` 克隆进内存，供断言。
///
/// inklog `LogSink` 为 RPITIT trait，impl 走 `#[async_trait]`（与 inklog
/// 自身测试替身同款形态）。
struct CaptureSink {
    records: Mutex<Vec<LogRecord>>,
}

impl CaptureSink {
    fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    fn snapshot(&self) -> Vec<LogRecord> {
        self.records
            .lock()
            .expect("capture sink mutex not poisoned")
            .clone()
    }
}

#[async_trait::async_trait]
impl LogSink for CaptureSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        self.records
            .lock()
            .expect("capture sink mutex not poisoned")
            .push(record.clone());
        Ok(())
    }

    async fn flush(&self) -> Result<(), InklogError> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        Ok(())
    }
}

/// 进程级共享：首个用例构建带捕获 sink 的管理器（在常驻专用 runtime
/// 上），并安装全局 tracing/log 前端；后续用例复用同一实例。
struct SharedBridge {
    manager: LoggerManager,
    sink: Arc<CaptureSink>,
}

/// 常驻专用 runtime（静态存活 = 进程存活）：manager 的 worker 全部跑在
/// 它上面。若随用例 runtime 销毁，`Runtime::drop` 等待常驻 blocking 任务
/// 会挂死测试进程。
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

static SHARED: OnceLock<SharedBridge> = OnceLock::new();

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("shared logging runtime")
    })
}

async fn build_shared() -> SharedBridge {
    let sink = Arc::new(CaptureSink::new());
    let manager = LoggerManager::builder()
        .level("info")
        .console(true)
        .add_sink(sink.clone())
        .build()
        .await
        .expect("capture manager build must succeed");
    SharedBridge { manager, sink }
}

/// 同步用例入口。
fn shared() -> &'static SharedBridge {
    SHARED.get_or_init(|| runtime().block_on(build_shared()))
}

/// 异步用例入口：build future 派发到常驻 runtime 执行（worker 随之落在
/// 常驻 runtime 上），当前 runtime 只 await JoinHandle。
async fn shared_async() -> &'static SharedBridge {
    if let Some(bridge) = SHARED.get() {
        return bridge;
    }
    let handle = runtime().spawn(build_shared());
    let built = handle.await.expect("shared bridge build task");
    SHARED.get_or_init(|| built)
}

/// 结构化记录经 inklog 管道进入自定义 sink（级别/目标/消息/字段齐备）。
#[test]
#[serial_test::serial]
fn structured_record_reaches_capture_sink() {
    let bridge = shared();
    // 自备级别：不受其他用例 set_level 的全局状态影响（serial 但顺序不定）。
    bridge
        .manager
        .set_level(None, "info")
        .expect("set_level to info");

    tracing::info!(
        target: "sdforge_inklog_bridge",
        user_id = 42,
        request_id = "req-bridge-1",
        "structured record through the inklog pipeline"
    );

    let marker = "structured record through the inklog pipeline";
    let mut captured = Vec::new();
    for _ in 0..100 {
        captured = bridge
            .sink
            .snapshot()
            .into_iter()
            .filter(|r| r.message.contains(marker))
            .collect::<Vec<_>>();
        if !captured.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(
        captured.len(),
        1,
        "structured record must reach the custom inklog sink exactly once"
    );
    let record = &captured[0];
    assert_eq!(record.level, "INFO", "level carried through the pipeline");
    assert_eq!(
        record.target, "sdforge_inklog_bridge",
        "target carried through the pipeline"
    );
    assert_eq!(
        record.fields.get("user_id").and_then(|v| v.as_i64()),
        Some(42),
        "structured numeric field carried through the pipeline"
    );
    assert_eq!(
        record.fields.get("request_id").and_then(|v| v.as_str()),
        Some("req-bridge-1"),
        "structured string field carried through the pipeline"
    );
}

/// `init_inklog_logger` 幂等安装：无论本进程是否已安装过全局 logger 均 Ok，
/// 且安装后全局 `log` 级别可观测（facade 接线，`src/inklog.rs` lib 用例的
/// 集成面对应）。
///
/// 先 `shared()` 保证全局前端由共享管理器安装（其后再走
/// `init_inklog_logger` 的良性降级分支——这正是幂等语义本身）。
#[tokio::test]
#[serial_test::serial]
async fn init_inklog_logger_is_idempotent_ok() {
    shared_async().await;
    let manager = init_inklog_logger()
        .await
        .expect("init must be idempotent-ok");
    assert_ne!(
        log::max_level(),
        log::LevelFilter::Off,
        "facade backend installed: global log level observable"
    );
    drop(manager);
}

/// `set_level` 调级后低于新级别的记录被过滤：info 不再进 sink，warn 仍进。
#[test]
#[serial_test::serial]
fn set_level_filters_below_new_level() {
    let bridge = shared();
    bridge
        .manager
        .set_level(None, "warn")
        .expect("set_level to warn");

    tracing::info!(
        target: "sdforge_inklog_bridge",
        "info must be filtered after set_level(warn)"
    );
    // 给 worker 留出投递窗口：info 应被过滤、不出现。
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !bridge
            .sink
            .snapshot()
            .iter()
            .any(|r| r.message.contains("info must be filtered")),
        "info below the new level must be filtered out"
    );

    tracing::warn!(
        target: "sdforge_inklog_bridge",
        "warn still passes after set_level(warn)"
    );
    let mut warn_seen = false;
    for _ in 0..100 {
        if bridge
            .sink
            .snapshot()
            .iter()
            .any(|r| r.message.contains("warn still passes"))
        {
            warn_seen = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(warn_seen, "warn at the new level must still reach the sink");

    // 复位，避免影响其他用例的 info 级发射。
    bridge
        .manager
        .set_level(None, "info")
        .expect("set_level restore to info");
}

/// `from_file` 加载含轮转配置的 TOML 构造成功（真实轮换行为锚点）。
///
/// 先 `shared()`：全局前端由共享管理器安装后，本用例的 `from_file` 走良性
/// 降级分支（不影响自身构造成功性）。
#[tokio::test]
#[serial_test::serial]
async fn from_file_loads_rotation_config() {
    shared_async().await;
    let dir = std::env::temp_dir().join(format!("sdforge-inklog-bridge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let config_path = dir.join("inklog.toml");
    let log_path = dir.join("sdforge-bridge.log");
    std::fs::write(
        &config_path,
        format!(
            r#"[global]
level = "info"

[console_sink]
enabled = true

[file_sink]
enabled = true
path = "{}"
max_size = "10MB"
rotation_time = "daily"
keep_files = 5
"#,
            log_path.to_string_lossy()
        ),
    )
    .expect("write rotation config");

    let manager = LoggerManager::from_file(&config_path)
        .await
        .expect("from_file with rotation config must succeed");
    drop(manager);
}
