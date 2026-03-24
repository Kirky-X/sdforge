//! Macro compilation tests for sdforge-macros
//!
//! This file contains tests that verify the macro compiles correctly
//! For full trybuild integration, see the trybuild_tests.rs file

// Test 1: service_api macro with basic arguments
#[sdforge_macros::service_api(name = "test", version = "v1", description = "Test API")]
async fn test_basic_macro() -> String {
    "test".to_string()
}

// Test 2: service_module macro with prefix
#[sdforge_macros::service_module(prefix = "/api/v1")]
mod test_module {
    pub fn test_fn() -> String {
        "test".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_compilation() {
        // This test verifies that the macros compile correctly
        let _ = test_basic_macro;
        let _ = test_module::test_fn();
    }
}
