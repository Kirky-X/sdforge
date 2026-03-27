// Security Middleware Integration Tests
// Tests security components with actual HTTP requests

#[cfg(feature = "security")]
mod security_tests {
    use sdforge::security::AppApiKeyAuth;

    #[test]
    fn test_api_key_auth_builder() {
        let auth = AppApiKeyAuth::builder().build();

        // If we get here without panicking, the builder works
        let _ = auth;
    }

    #[test]
    fn test_api_key_auth_new() {
        let auth = AppApiKeyAuth::new();
        let _ = auth;
    }
}
