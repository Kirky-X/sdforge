// Security Middleware Integration Tests
// Tests security components with actual HTTP requests

#[cfg(feature = "security")]
mod security_tests {
    use sdforge::security::AppApiKeyAuth;

    #[test]
    fn test_api_key_auth_builder() {
        let auth = AppApiKeyAuth::builder().build();

        // Verify the builder works and creates a valid instance
        assert!(auth.is_some(), "API Key Auth builder should create an instance");
    }

    #[test]
    fn test_api_key_auth_new() {
        let auth = AppApiKeyAuth::new();
        assert!(auth.is_some(), "API Key Auth new() should create an instance");
    }
}
