use sdforge_macros::service_module;

#[service_module(prefix = "/api/v1")]
mod api_v1 {
    use sdforge_macros::service_api;

    #[service_api(name = "list_users", version = "v1")]
    async fn list_users() -> String {
        "users".to_string()
    }
}

fn main() {}
