// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T716 e2e: dbnexus data API gateway — allowlist + filter + pagination
//! served through a `#[forge]` route.

#![cfg(feature = "dbnexus_gateway_example")]

use tower::ServiceExt;

static POOL: tokio::sync::OnceCell<dbnexus::DbPool> = tokio::sync::OnceCell::const_new();

#[sdforge::forge(
    name = "t716_gateway_users",
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
    // Whitelist guard (T419 contract): only users.id/name/age are reachable.
    let page = page.unwrap_or(1).max(1);
    let size = size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * size;
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
    let pool = POOL
        .get_or_init(|| async {
            // File-backed db: `sqlite::memory:` gives EACH pooled connection
            // its own database, which breaks table visibility across pooled
            // sessions.
            let dir = std::env::temp_dir().join(format!(
                "sdforge-gw-e2e-{}",
                std::process::id()
            ));
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
        .await;
    pool
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

