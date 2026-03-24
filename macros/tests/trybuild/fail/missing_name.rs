use sdforge_macros::service_api;

#[service_api(version = "v1")]
async fn test_no_name() -> String {
    "hello".to_string()
}

fn main() {}
