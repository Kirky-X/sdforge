use axiom::prelude::*;

#[axiom::test_macro]
async fn test() -> Result<String, ApiError> {
    Ok("test".to_string())
}

fn main() {
    println!("Simple test");
}