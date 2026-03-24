use sdforge_macros::service_api;

#[service_api(name = "test@invalid", version = "v1")]
async fn test_invalid_name() -> String {
    "hello".to_string()
}

fn main() {}
