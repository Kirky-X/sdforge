use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use axiom::prelude::*;
use axiom_macros::service_api;

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    name: String,
    email: String,
}

type UserDatabase = Arc<Mutex<HashMap<u64, User>>>;

#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user"
)]
async fn get_user(id: u64, db: UserDatabase) -> Result<Option<User>, ApiError> {
    let guard = db.lock().unwrap();
    Ok(guard.get(&id).cloned())
}

#[service_api(
    name = "create_user",
    version = "v1",
    path = "/users",
    method = "POST"
)]
async fn create_user(user: User, db: UserDatabase) -> Result<u64, ApiError> {
    let id = user.id;
    let mut guard = db.lock().unwrap();
    guard.insert(id, user);
    Ok(id)
}

#[service_api(
    name = "list_users",
    version = "v1",
    path = "/users",
    method = "GET"
)]
async fn list_users(db: UserDatabase) -> Result<Vec<User>, ApiError> {
    let guard = db.lock().unwrap();
    Ok(guard.values().cloned().collect())
}

#[service_api(
    name = "search_users",
    version = "v1",
    tool_name = "search_users"
)]
async fn search_users(query: String, db: UserDatabase) -> Result<Vec<User>, ApiError> {
    let guard = db.lock().unwrap();
    let results: Vec<User> = guard.values()
        .filter(|u| u.name.contains(&query))
        .cloned()
        .collect();
    Ok(results)
}

#[tokio::main]
async fn main() {
    println!("Complex generics test compiled successfully!");
    
    // Create a test database
    let db: UserDatabase = Arc::new(Mutex::new(HashMap::new()));
    
    // Test the functions
    let user = User {
        id: 1,
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };
    
    // Insert user
    let result = create_user(user.clone(), db.clone()).await;
    println!("Created user: {:?}", result);
    
    // List users
    let result = list_users(db.clone()).await;
    println!("List users: {:?}", result);
    
    // Get user
    let result = get_user(1, db.clone()).await;
    println!("Get user: {:?}", result);
    
    // Search users
    let result = search_users("Test".to_string(), db.clone()).await;
    println!("Search users: {:?}", result);
}