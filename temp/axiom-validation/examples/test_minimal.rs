use axiom::prelude::*;
use axiom::service_api;

#[service_api(
    name = "test",
    version = "v1",
    path = "/test",
    method = "GET"
)]
async fn test() -> Result<String, ApiError> {
    Ok("test".to_string())
}

fn main() {
    println!("Minimal test");
}