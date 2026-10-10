// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! e2e: dbnexus data API gateway — allowlist + filter + pagination
//! served through a `#[forge]` route.

#![cfg(feature = "dbnexus_gateway_example")]

use tower::ServiceExt;

static POOL: tokio::sync::OnceCell<dbnexus::DbPool> = tokio::sync::OnceCell::const_new();

#[sdforge::forge(
    name = "dbnexus_gateway_users",
    version = "v1",
    path = "/gw/users",
    method = "GET",
    no_prefix = true,
    description = "gateway read"
)]
async fn gw_users(
    name: Option<String>,
    page: Option<u64>,
    size: Option<u64>,
) -> Result<serde_json::Value, sdforge::core::ApiError> {
    // Whitelist guard (data-api contract): only users.id/name/age are reachable.
    let page = page.unwrap_or(1).max(1);
    let size = size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1).saturating_mul(size);
    let filter = name
        .as_deref()
        .map(|n| format!(" AND name = '{}'", n.replace('\'', "''")))
        .unwrap_or_default();
    let sql = format!(
        "SELECT id, name, age FROM users WHERE 1=1{filter} ORDER BY id LIMIT {size} OFFSET {offset}"
    );
    let pool = POOL.get().expect("gateway pool seeded by the test");
    let rows = pool
        .query_rows(&sql, "admin")
        .await
        .map_err(|e| sdforge::core::ApiError::internal_error(e.to_string(), "gateway.query"))?;
    Ok(serde_json::json!({ "rows": rows, "page": page, "size": size }))
}

async fn seed_global_pool() -> &'static dbnexus::DbPool {
    // First caller creates+seeds; later callers reuse the same pool (the
    // OnceLock guarantees identical data across parallel tests).
    POOL.get_or_init(|| async {
        // File-backed db: `sqlite::memory:` gives EACH pooled connection
        // its own database, which breaks table visibility across pooled
        // sessions.
        let dir = std::env::temp_dir().join(format!("sdforge-gw-e2e-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db_path = dir.join("gateway.db");
        let _ = std::fs::remove_file(&db_path);
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
        for (name, age) in [("alice", 30), ("bob", 41), ("carol", 25)] {
            session
                .execute_raw(&format!(
                    "INSERT INTO users (name, age) VALUES ('{name}', {age})"
                ))
                .await
                .expect("seed");
        }
        pool
    })
    .await
}

#[tokio::test]
async fn gateway_reads_seeded_rows_with_filter_and_paging() {
    seed_global_pool().await;

    // Filtered read (name=alice).
    let router = sdforge::http::build();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/gw/users?name=alice")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.get("message"), None, "route error: {json}");
    assert_eq!(json["page"], 1);
    let rows = json["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1, "filter must match exactly alice: {json}");
    assert_eq!(rows[0]["name"], "alice");
    assert_eq!(rows[0]["age"], 30);
}

#[tokio::test]
async fn gateway_pagination_slices_results() {
    seed_global_pool().await;

    let router = sdforge::http::build();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/gw/users?page=2&size=2")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let rows = json["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1, "page 2 of 3 rows @ size 2 → 1 row: {json}");
    assert_eq!(rows[0]["name"], "carol");
}

/// 越界表/列不可经 HTTP 参数触发（data-api 白名单契约）：路由 SQL 与列清单
/// 是编译期固定的，`table=`/`column=` 不是 handler 参数——未知 query 参数被
/// 忽略，响应仍是白名单 `users.id/name/age`，既不切表也不泄露越界列。
#[tokio::test]
async fn out_of_allowlist_table_and_column_params_cannot_change_the_query() {
    seed_global_pool().await;

    let router = sdforge::http::build();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/gw/users?table=secret_table&column=password")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 全量 3 行白名单数据；`table`/`column` 参数未改变查询。
    let rows = json["rows"].as_array().expect("rows array");
    assert_eq!(
        rows.len(),
        3,
        "unknown params must not change the query: {json}"
    );
    let first = rows[0].as_object().expect("row object");
    let mut columns: Vec<&str> = first.keys().map(String::as_str).collect();
    columns.sort_unstable();
    assert_eq!(
        columns,
        vec!["age", "id", "name"],
        "only whitelisted columns may appear: {json}"
    );
    let raw = json.to_string();
    assert!(!raw.contains("secret_table"), "table param leaked: {raw}");
    assert!(!raw.contains("password"), "column param leaked: {raw}");
}

/// 越界表没有动态路由：`/gw/<table>` 形式的越界表名一律 404，不可触发
/// 任意表读取。
#[tokio::test]
async fn out_of_allowlist_table_path_is_not_routed() {
    seed_global_pool().await;

    let router = sdforge::http::build();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/gw/secret_table?table=secret_table")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        404,
        "no route may serve an out-of-allowlist table"
    );
}
