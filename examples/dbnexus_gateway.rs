// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Example: dbnexus data API gateway exposed via `#[forge]`.
//!
//! A read-only data API over a whitelisted table (`users`, columns
//! `id/name/age`): equality filter on the whitelisted column plus
//! LIMIT/OFFSET pagination, executed through dbnexus's `DbPool` (sqlite
//! in-memory). The gateway layer validates the allowlist before SQL is built,
//! so only whitelisted tables/columns can ever reach the database.

use std::sync::OnceLock;

use sdforge::forge;

/// Gateway allowlist: table → columns (the data-api contract surface).
const ALLOWED_TABLES: &[(&str, &[&str])] = &[("users", &["id", "name", "age"])];

static POOL: OnceLock<dbnexus::DbPool> = OnceLock::new();

/// Create the in-memory pool and seed the whitelisted table.
async fn init_pool() -> dbnexus::DbPool {
    let pool = dbnexus::DbPoolBuilder::new()
        .url("sqlite::memory:")
        .build()
        .await
        .expect("sqlite memory pool");
    let session = pool.get_session("admin").await.expect("admin session");
    session
        .execute_raw_ddl("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
        .await
        .expect("create table");
    for (name, age) in [("alice", 30), ("bob", 41), ("carol", 25)] {
        session
            .execute_raw(&format!(
                "INSERT INTO users (name, age) VALUES ('{name}', {age})"
            ))
            .await
            .expect("seed row");
    }
    pool
}

/// Validate the table/column against the allowlist (gateway guard).
fn assert_allowed(table: &str, column: &str) -> Result<(), sdforge::core::ApiError> {
    let entry = ALLOWED_TABLES
        .iter()
        .find(|(t, _)| *t == table)
        .ok_or_else(|| sdforge::core::ApiError::not_found("table", Some(table.to_string())))?;
    if !entry.1.contains(&column) {
        return Err(sdforge::core::ApiError::InvalidInput {
            message: format!("column '{column}' is not whitelisted"),
            field: Some("column".to_string()),
            value: None,
        });
    }
    Ok(())
}

/// Whitelisted, filtered, paginated read over the `users` table.
#[forge(
    name = "gateway_users",
    version = "v1",
    path = "/db/gateway/users",
    method = "GET",
    no_prefix = true,
    description = "Whitelisted paginated read over the users table"
)]
async fn gateway_users(
    name: Option<String>,
    page: Option<u64>,
    size: Option<u64>,
) -> Result<serde_json::Value, sdforge::core::ApiError> {
    assert_allowed("users", "id")?;
    let page = page.unwrap_or(1).max(1);
    let size = size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1).saturating_mul(size);

    // Filter value is bound through escaping of the simple demo text; the
    // whitelist guard above covers identifier injection.
    let filter = name
        .as_deref()
        .map(|n| format!(" AND name = '{}'", n.replace('\'', "''")))
        .unwrap_or_default();
    let sql = format!(
        "SELECT id, name, age FROM users WHERE 1=1{filter} ORDER BY id LIMIT {size} OFFSET {offset}"
    );

    let pool = POOL.get().expect("gateway pool initialised in main/tests");
    let rows = pool
        .query_rows(&sql, "admin")
        .await
        .map_err(|e| sdforge::core::ApiError::internal_error(e.to_string(), "gateway.query"))?;
    Ok(serde_json::json!({ "table": "users", "page": page, "size": size, "rows": rows }))
}

#[tokio::main]
async fn main() {
    let pool = init_pool().await;
    let _ = POOL.set(pool);
    let router = sdforge::http::build();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8093").await.unwrap();
    eprintln!("dbnexus gateway example on http://0.0.0.0:8093");
    sdforge::axum::serve(listener, router).await.unwrap();
}
