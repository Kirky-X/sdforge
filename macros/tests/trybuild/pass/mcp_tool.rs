use sdforge_macros::service_api;

#[service_api(name = "test_tool", version = "v1", tool_name = "my_tool")]
async fn my_tool() -> String {
    "tool result".to_string()
}

fn main() {}
