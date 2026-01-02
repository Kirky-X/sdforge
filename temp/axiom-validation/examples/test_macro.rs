use axiom::prelude::*;
use axiom::test_macro;

#[test_macro]
async fn test() -> Result<String, ApiError> {
    Ok("test".to_string())
}

fn main() {
    println!("Macro test");
}