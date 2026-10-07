// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! SDK 产物快照测试（`sdk` feature）。
//!
//! 固定路由集 → 产物文本与入库快照逐字节比对（渲染确定性锁定）；Rust 产
//! 物另以 `rustc --crate-type lib --emit=metadata` 做真实编译冒烟（零外部
//! 依赖产物必须独立通过 rustc）。
//!
//! 快照再生成：临时删除 `.snap` 后运行 `cargo test --features sdk --test
//! sdk_snapshots -- --nocapture`，按失败输出中的提示回填。
//!
//! Feature requirement: `cargo test --features sdk --test sdk_snapshots`.

#![cfg(feature = "sdk")]

use sdforge::sdk::{ClientRoute, GrpcMethodInfo, generate_rust_client, generate_typescript_client};

fn fixture_routes() -> Vec<ClientRoute> {
    vec![
        ClientRoute {
            method: "GET".to_string(),
            path: "/api/v1/users/{id}".to_string(),
            path_params: vec!["id".to_string()],
            has_body: false,
            summary: "Fetch a user".to_string(),
        },
        ClientRoute {
            method: "GET".to_string(),
            path: "/api/v1/users".to_string(),
            path_params: vec![],
            has_body: false,
            summary: "List users".to_string(),
        },
        ClientRoute {
            method: "POST".to_string(),
            path: "/api/v1/users".to_string(),
            path_params: vec![],
            has_body: true,
            summary: "Create a user".to_string(),
        },
    ]
}

fn fixture_grpc() -> Vec<GrpcMethodInfo> {
    vec![GrpcMethodInfo {
        method: "examples.users.get".to_string(),
        body_param: Some("payload".to_string()),
    }]
}

/// 快照比对（文件缺失时写出并提示回填入库）。
fn assert_snapshot(name: &str, actual: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/sdk/snapshots")
        .join(format!("{name}.snap"));
    match std::fs::read_to_string(&path) {
        Ok(expected) => assert_eq!(
            actual,
            expected,
            "snapshot {name} drifted; regenerate by writing the new output to {}",
            path.display()
        ),
        Err(_) => {
            std::fs::write(&path, actual).expect("write snapshot");
            panic!(
                "snapshot {name} created at {}; review and keep it in VCS",
                path.display()
            );
        }
    }
}

#[test]
fn rust_client_matches_snapshot() {
    let out = generate_rust_client(&fixture_routes(), &fixture_grpc(), false);
    assert_snapshot("rust_client", &out);
}

#[test]
fn rust_client_reqwest_variant_matches_snapshot() {
    let out = generate_rust_client(&fixture_routes(), &fixture_grpc(), true);
    assert_snapshot("rust_client_reqwest", &out);
}

#[test]
fn typescript_client_matches_snapshot() {
    let out = generate_typescript_client(&fixture_routes(), &fixture_grpc());
    assert_snapshot("typescript_client", &out);
}

/// Rust 产物真实编译冒烟：零外部依赖产物必须独立通过 `rustc
/// --crate-type lib --emit=metadata`。
#[test]
fn generated_rust_client_compiles() {
    let out = generate_rust_client(&fixture_routes(), &fixture_grpc(), false);
    let dir = std::env::temp_dir().join(format!("sdforge-sdk-compile-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let source = dir.join("sdforge_client.rs");
    let metadata = dir.join("libclient.rmeta");
    std::fs::write(&source, &out).expect("write generated source");

    let status = std::process::Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--crate-name",
            "sdforge_client",
            "--emit",
            "metadata",
            "-o",
        ])
        .arg(&metadata)
        .arg(&source)
        .status()
        .expect("rustc must be available (same toolchain as the test runner)");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        status.success(),
        "generated rust client must compile with rustc"
    );
}

/// 执行面测试：产物在真实执行下 `base_url` 必须生效——客户端 `call` 把
/// `base_url` 拼进传给 Transport 的完整 URL（含尾斜杠剥离），体按值移交。
/// rustc 编译「产物 + 录制传输 harness」为可执行文件并运行，断言录制到的
/// URL/方法/体。
#[test]
fn generated_client_prepends_base_url_in_execution() {
    let generated = generate_rust_client(&fixture_routes(), &[], false);
    let harness = r#"
struct Recording;

static SEEN: std::sync::Mutex<Vec<(String, String, Option<String>)>> =
    std::sync::Mutex::new(Vec::new());

impl Transport for Recording {
    fn execute<'a>(
        &'a self,
        method: &'a str,
        path: &'a str,
        body: Option<String>,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send + 'a {
        SEEN
            .lock()
            .unwrap()
            .push((method.to_string(), path.to_string(), body));
        std::future::ready(Ok("{}".to_string()))
    }
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    let mut fut = std::pin::pin!(fut);
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);
    loop {
        if let std::task::Poll::Ready(value) = fut.as_mut().poll(&mut cx) {
            return value;
        }
        std::thread::yield_now();
    }
}

fn main() {
    let client = SdforgeClient::new("http://127.0.0.1:8080/", Recording);
    let ok = block_on(client.get_api_v1_users()).expect("call ok");
    assert_eq!(ok, "{}");
    let _ = block_on(client.get_api_v1_users_by_id(7));
    let _ = block_on(client.post_api_v1_users("{\"k\":1}".to_string()));

    let seen = SEEN.lock().unwrap();
    assert_eq!(seen.len(), 3, "three calls recorded: {seen:?}");
    assert_eq!(seen[0].1, "http://127.0.0.1:8080/api/v1/users", "base_url prepended: {seen:?}");
    assert_eq!(seen[1].1, "http://127.0.0.1:8080/api/v1/users/7", "path param bound onto full URL");
    assert_eq!(seen[1].2, None, "GET carries no body");
    assert_eq!(seen[2].0, "POST");
    assert_eq!(seen[2].1, "http://127.0.0.1:8080/api/v1/users");
    assert_eq!(seen[2].2.as_deref(), Some("{\"k\":1}"), "body handed over owned");
    println!("execution-surface ok");
}
"#;
    let dir = std::env::temp_dir().join(format!("sdforge-sdk-exec-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let source = dir.join("client_exec.rs");
    let binary = dir.join("client_exec");
    std::fs::write(&source, format!("{generated}{harness}")).expect("write exec source");

    let compiled = std::process::Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&binary)
        .arg(&source)
        .status()
        .expect("rustc must be available");
    let result = if compiled.success() {
        std::process::Command::new(&binary)
            .output()
            .expect("run generated client binary")
    } else {
        let _ = std::fs::remove_dir_all(&dir);
        panic!("generated client + harness must compile");
    };
    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let success = result.status.success();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        success,
        "execution harness failed\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("execution-surface ok"),
        "harness assertions must pass: {stdout}{stderr}"
    );
}
