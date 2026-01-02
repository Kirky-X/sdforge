//! 测试类型别名

use axiom::prelude::*;
use axiom::service_api;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

// 类型别名
type UserDatabase = Arc<Mutex<HashMap<u64, User>>>;

#[service_api(
    name = "list_users",
    version = "v1",
    path = "/users",
    method = "GET"
)]
async fn list_users(db: UserDatabase) -> Result<Vec<User>, ApiError> {
    let users = db.lock().unwrap();
    Ok(users.values().cloned().collect())
}

#[tokio::main]
async fn main() {
    println!("Type alias test");
}
