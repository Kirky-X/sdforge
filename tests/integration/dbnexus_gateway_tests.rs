// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! dbnexus 数据 API 网关端到端测试（`db-integration` feature）。
//!
//! sqlite 文件库（`sqlite::memory:` 每连接独立库会破坏跨池会话的表可见
//! 性，故用临时目录文件库）+ `#[forge]` 路由经 oneshot 全链路，覆盖：
//! 白名单放行/拒绝、等值过滤、分页夹紧、过滤值注入面中性化。
//!
//! Feature requirement: `cargo test --features "db-integration,http" --test dbnexus_gateway_tests`.

#![cfg(all(feature = "db-integration", feature = "http"))]

use sdforge::forge;
use sdforge::integrations::{DbGateway, GatewayQuery};
use std::sync::OnceLock;
use tokio::sync::OnceCell;
use tower::ServiceExt;

static GATEWAY: OnceCell<DbGateway> = OnceCell::const_new();
static POOL: OnceCell<dbnexus::DbPool> = OnceCell::const_new();
static CLEANUP_DIR: OnceLock<String> = OnceLock::new();

/// 共享的 sqlite 文件库（池化连接各自拿到独立内存库会使表不可见——示例
/// e2e 同源教训，故用临时目录文件库 + 测试结束清理）。
async fn pool() -> &'static dbnexus::DbPool {
    POOL.get_or_init(|| async {
        let dir = std::env::temp_dir().join(format!("sdforge-gw-lib-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db_path = dir.join("gateway.db");
        let _ = std::fs::remove_file(&db_path);
        let _ = CLEANUP_DIR.set(dir.to_string_lossy().to_string());

        let pool = dbnexus::DbPoolBuilder::new()
            .url(format!("sqlite://{}?mode=rwc", db_path.display()).as_str())
            .build()
            .await
            .expect("sqlite pool");
        let session = pool.get_session("admin").await.expect("admin session");
        session
            .execute_raw_ddl(
                "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)",
            )
            .await
            .expect("create table");
        session
            .execute_raw_ddl("CREATE TABLE IF NOT EXISTS secrets (payload TEXT)")
            .await
            .expect("create secrets table");
        session
            .execute_raw("INSERT INTO secrets (payload) VALUES ('TOPSECRET')")
            .await
            .expect("seed secret");
        for (name, age) in [("alice", 30), ("bob", 41), ("carol", 25)] {
            session
                .execute_raw(&format!(
                    "INSERT INTO users (name, age) VALUES ('{name}', {age})"
                ))
                .await
                .expect("seed row");
        }
        pool
    })
    .await
}

async fn gateway() -> &'static DbGateway {
    GATEWAY
        .get_or_init(|| async {
            DbGateway::new(pool().await.clone())
                .with_session_role("admin")
                .allow_table("users", &["id", "name", "age"])
        })
        .await
}

/// 夹具端点：白名单网关经 `#[forge]` 暴露（生产形态）。
#[forge(
    name = "db_gateway_users",
    version = "v1",
    path = "/db/users",
    method = "GET",
    no_prefix = true,
    description = "Whitelisted paginated read over the users table"
)]
async fn db_gateway_users(
    name: Option<String>,
    page: Option<u64>,
    size: Option<u64>,
) -> Result<serde_json::Value, sdforge::core::ApiError> {
    let mut query = GatewayQuery::all("users").paging(page.unwrap_or(1), size.unwrap_or(20));
    if let Some(name) = name {
        query = query.filter("name", name);
    }
    gateway().await.query(query).await
}

async fn get_json(uri: &str) -> serde_json::Value {
    let router = sdforge::http::build();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), 200, "route must succeed for {uri}");
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).expect("json body")
}

#[tokio::test]
async fn forge_route_serves_whitelisted_reads() {
    gateway().await;
    let json = get_json("/db/users?name=alice").await;
    assert!(json.get("message").is_none(), "route error: {json}");
    let rows = json["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1, "filter must match exactly alice: {json}");
    assert_eq!(rows[0]["name"], "alice");
    assert_eq!(rows[0]["age"], 30);
}

#[tokio::test]
async fn forge_route_pagination_clamps() {
    gateway().await;
    // page=0 视为 1；size=500 夹紧到 100——3 行全回。
    let json = get_json("/db/users?page=0&size=500").await;
    let rows = json["rows"].as_array().expect("rows array");
    assert_eq!(
        rows.len(),
        3,
        "page=0/size=500 clamps to full table: {json}"
    );
    assert_eq!(json["page"], 1, "page 0 must normalize to 1: {json}");
}

#[tokio::test]
async fn non_whitelisted_table_is_unreachable() {
    let gw = gateway().await;
    // secrets 表在库里但不在白名单——网关必须拒绝。
    let err = gw.query(GatewayQuery::all("secrets")).await.unwrap_err();
    assert!(
        matches!(err, sdforge::core::ApiError::NotFound { .. }),
        "non-whitelisted table → not found: {err}"
    );
}

#[tokio::test]
async fn non_whitelisted_column_is_rejected() {
    let gw = gateway().await;
    let err = gw
        .query(GatewayQuery::all("users").filter("payload", "x"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, sdforge::core::ApiError::InvalidInput { .. }),
        "non-whitelisted column → invalid input: {err}"
    );
}

/// 强制归属谓词：服务端配置的列条件与调用方过滤 AND 连接，调用方无法
/// 放宽（白名单只约束表/列，不提供行级隔离）。
#[tokio::test]
async fn required_filter_is_enforced_and_not_overridable() {
    let gw = DbGateway::new(pool().await.clone())
        .with_session_role("admin")
        .with_required_filter("name", "alice")
        .allow_table("users", &["id", "name", "age"]);

    // 无调用方过滤：谓词把结果收敛到归属行。
    let json = gw.query(GatewayQuery::all("users")).await.expect("query");
    let rows = json["rows"].as_array().expect("rows array");
    assert_eq!(
        rows.len(),
        1,
        "the required filter must narrow the result: {json}"
    );
    assert_eq!(rows[0]["name"], "alice");

    // 调用方过滤与谓词 AND：越权取值拿不到行。
    let json = gw
        .query(GatewayQuery::all("users").filter("name", "bob"))
        .await
        .expect("query");
    let rows = json["rows"].as_array().expect("rows array");
    assert!(
        rows.is_empty(),
        "the caller filter must not widen the required filter: {json}"
    );
}

/// 强制谓词列不在白名单 → 查询显式失败（fail-closed），不静默放行。
#[tokio::test]
async fn required_filter_column_must_be_whitelisted() {
    let gw = DbGateway::new(pool().await.clone())
        .with_session_role("admin")
        .with_required_filter("age", "30")
        .allow_table("users", &["id", "name"]);

    let err = gw
        .query(GatewayQuery::all("users"))
        .await
        .expect_err("a required filter outside the allowlist must fail");
    assert!(
        matches!(err, sdforge::core::ApiError::InvalidInput { .. }),
        "required filter column outside the allowlist → invalid input: {err}"
    );
}

/// 会话角色的强制效果取决于 dbnexus 的 `permission` 特性是否编译进来。
///
/// 未开启（仅 `db-integration`）时 dbnexus 的逐表权限检查整块被编译掉，
/// 非 admin 会话角色不被强制执行——网关的隔离面只有白名单与强制谓词。
/// 该事实必须被测试钉住：否则"角色已设置"会被误读成"角色已生效"。
#[cfg(not(feature = "db-permission"))]
#[tokio::test]
async fn session_role_is_not_enforced_without_dbnexus_permission() {
    let gw = DbGateway::new(pool().await.clone())
        .with_session_role("sdforge_gateway")
        .allow_table("users", &["id", "name", "age"]);

    let json = gw
        .query(GatewayQuery::all("users"))
        .await
        .expect("without dbnexus permission the role is not enforced");
    let rows = json["rows"].as_array().expect("rows array");
    assert_eq!(
        rows.len(),
        3,
        "the session role does not restrict anything without dbnexus permission: {json}"
    );
}

/// 开启 `db-permission` 后，非 admin 会话角色的查询必须经 dbnexus 逐表
/// 权限检查，且未配置权限时 fail-closed（拒绝而非放行）。
#[cfg(feature = "db-permission")]
#[tokio::test]
async fn session_role_is_enforced_with_dbnexus_permission() {
    let gw = DbGateway::new(pool().await.clone())
        .with_session_role("sdforge_gateway")
        .allow_table("users", &["id", "name", "age"]);

    let err = gw
        .query(GatewayQuery::all("users"))
        .await
        .expect_err("a non-admin role without permissions must be denied");
    assert!(
        matches!(err, sdforge::core::ApiError::Internal { .. }),
        "the dbnexus permission denial must surface as an error: {err}"
    );
}

/// 注入面：过滤值经转义成为字面量。非 DDL 载荷按字面量执行（查不到行、/// 表完好）；含 DDL 关键字的载荷被 dbnexus 权限层直接拒绝（关键词即使在
/// 字面量内也触发守卫）——两条路径都不可达注入。
#[tokio::test]
async fn filter_injection_payload_is_neutralized() {
    let gw = gateway().await;

    // 非 DDL 注入载荷：转义后恒为字面量，匹配 0 行。
    let json = gw
        .query(GatewayQuery::all("users").filter("name", "x' OR '1'='1"))
        .await
        .expect("escaped literal query must execute");
    let rows = json["rows"].as_array().expect("rows array");
    assert!(
        rows.is_empty(),
        "injection payload must match no rows: {json}"
    );

    // DDL 载荷：dbnexus 权限层拒执行（错误显性暴露）。
    let outcome = gw
        .query(GatewayQuery::all("users").filter("name", "x'; DROP TABLE users;--"))
        .await;
    assert!(outcome.is_err(), "DDL-bearing payload must not execute");

    // 两种载荷后 users 表均完好。
    let after = gw.query(GatewayQuery::all("users")).await.unwrap();
    assert_eq!(
        after["rows"].as_array().expect("rows").len(),
        3,
        "users table must survive the injection attempts"
    );
}
