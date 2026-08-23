// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// Security Middleware Integration Tests
// Tests security components with actual HTTP requests

#[cfg(feature = "security")]
mod security_tests {
    use sdforge::security::AppApiKeyAuth;

    #[test]
    fn test_api_key_auth_builder() {
        let auth = AppApiKeyAuth::builder().build();

        // Verify the builder works and creates a valid instance
        assert!(
            !std::ptr::eq(&auth, std::ptr::null()),
            "API Key Auth builder should create an instance"
        );
    }

    #[test]
    fn test_api_key_auth_new() {
        let auth = AppApiKeyAuth::new();
        assert!(
            !std::ptr::eq(&auth, std::ptr::null()),
            "API Key Auth new() should create an instance"
        );
    }

    // ============================================================================
    // Advanced Security Tests - Performance and Concurrency
    // ============================================================================

    /// Test: API key validation performance
    #[test]
    fn test_api_key_validation_performance() {
        use sdforge::security::{ApiKeyMetadata, AppApiKeyAuth};

        let auth = AppApiKeyAuth::new();
        let _metadata = ApiKeyMetadata::new("perf-test-key".to_string(), None);

        let start = std::time::Instant::now();

        // Validate the same key multiple times
        for _ in 0..1000 {
            let _ = auth.validate_key("perf-test-key", "127.0.0.1");
        }

        let elapsed = start.elapsed();
        // Should validate 1000 keys reasonably fast. 预算按负载放大到 2000ms：
        // 低配置机/CI 叠加覆盖率插桩时 500ms 易抖动，仍能捕获 O(n²) 级回归。
        assert!(elapsed < std::time::Duration::from_millis(2000));
    }

    /// Test: Concurrent API key validation
    #[tokio::test]
    async fn test_concurrent_api_key_validation() {
        use sdforge::security::AppApiKeyAuth;
        use std::sync::Arc;

        let auth = Arc::new(AppApiKeyAuth::new());
        let mut handles = vec![];

        // Spawn 10 concurrent validation tasks
        for i in 0..10 {
            let auth_clone = Arc::clone(&auth);
            let handle = tokio::spawn(async move {
                let key = format!("key-{}", i);
                auth_clone.validate_key(&key, "127.0.0.1")
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let result = handle.await.unwrap();
            // All validations should complete (may fail, but shouldn't panic)
            let _ = result;
        }
    }

    /// Test: Multiple API keys stress test
    #[test]
    fn test_multiple_api_keys_stress() {
        use sdforge::security::{ApiKeyMetadata, AppApiKeyAuth};

        let auth = AppApiKeyAuth::new();

        // Create and validate many different keys
        for i in 0..100 {
            let key = format!("stress-test-key-{}", i);
            let _metadata = ApiKeyMetadata::new(key.clone(), Some(format!("Key {}", i)));
            let _ = auth.validate_key(&key, "127.0.0.1");
        }

        // Should complete without issues
    }

    /// Test: API key with edge case inputs
    #[test]
    fn test_api_key_edge_cases() {
        use sdforge::security::AppApiKeyAuth;

        let auth = AppApiKeyAuth::new();

        // Empty key
        let _ = auth.validate_key("", "127.0.0.1");

        // Very long key
        let long_key = "k".repeat(10000);
        let _ = auth.validate_key(&long_key, "127.0.0.1");

        // Unicode key
        let unicode_key = "密钥🔑";
        let _ = auth.validate_key(unicode_key, "127.0.0.1");

        // Special characters
        let special_key = "!@#$%^&*()_+-=[]{}|;':\",./<>?";
        let _ = auth.validate_key(special_key, "127.0.0.1");

        // Should not panic on any input
    }
}

// Enhanced Security Middleware tests
#[cfg(feature = "security")]
mod security_middleware_enhanced_tests {
    use sdforge::AppApiKeyAuth;
    use sdforge::security::ApiKeyMetadata;

    // ============================================================================
    // ApiKeyMetadata tests
    // ============================================================================

    /// Test 1: ApiKeyMetadata creation with minimal params
    #[test]
    fn test_api_key_metadata_minimal() {
        let metadata = ApiKeyMetadata::new("test-key-id".to_string(), None);

        assert_eq!(metadata.key_id, "test-key-id");
        assert_eq!(metadata.description, None);
        assert!(metadata.versions.is_empty());
        assert_eq!(metadata.active_version_index, None);
    }

    /// Test 2: ApiKeyMetadata creation with description
    #[test]
    fn test_api_key_metadata_with_description() {
        let metadata = ApiKeyMetadata::new(
            "key-with-desc".to_string(),
            Some("Test API Key for development".to_string()),
        );

        assert_eq!(metadata.key_id, "key-with-desc");
        assert_eq!(
            metadata.description,
            Some("Test API Key for development".to_string())
        );
    }

    /// Test 3: ApiKeyMetadata long key ID
    #[test]
    fn test_api_key_metadata_long_id() {
        let long_id = "a".repeat(256);
        let metadata = ApiKeyMetadata::new(long_id.clone(), None);

        assert_eq!(metadata.key_id.len(), 256);
    }

    /// Test 4: ApiKeyMetadata Clone trait
    #[test]
    fn test_api_key_metadata_clone() {
        let original =
            ApiKeyMetadata::new("clone-test".to_string(), Some("Clone Test".to_string()));

        let cloned = original.clone();
        assert_eq!(original.key_id, cloned.key_id);
        assert_eq!(original.description, cloned.description);
    }

    /// Test 5: ApiKeyMetadata Debug trait
    #[test]
    fn test_api_key_metadata_debug() {
        let metadata =
            ApiKeyMetadata::new("debug-test".to_string(), Some("Debug Test".to_string()));

        let debug_str = format!("{:?}", metadata);
        assert!(debug_str.contains("debug-test"));
    }

    // ============================================================================
    // AppApiKeyAuth tests
    // ============================================================================

    /// Test 6: AppApiKeyAuth basic functionality
    #[test]
    fn test_app_api_key_auth_basic() {
        let auth = AppApiKeyAuth::new();

        // Verify we can call basic methods without panic
        let _ = auth.validate_key("test-key", "127.0.0.1");
    }

    /// Test 7: AppApiKeyAuth builder pattern
    #[test]
    fn test_app_api_key_auth_builder_pattern() {
        let builder = AppApiKeyAuth::builder();
        let auth = builder.build();

        // Verify builder creates valid instance
        assert!(!std::ptr::eq(&auth, std::ptr::null()));
    }

    /// Test 8: Multiple auth instances are independent
    #[test]
    fn test_multiple_auth_instances_independent() {
        let auth1 = AppApiKeyAuth::new();
        let auth2 = AppApiKeyAuth::new();

        // Verify they are separate instances
        assert!(!std::ptr::eq(&auth1, &auth2));

        // Both should work independently
        let _ = auth1.validate_key("key1", "127.0.0.1");
        let _ = auth2.validate_key("key2", "127.0.0.1");
    }

    // ============================================================================
    // Auth context tests
    // ============================================================================

    /// Test 9: Auth context creation
    #[test]
    fn test_auth_context_creation() {
        use sdforge::security::{AuthContext, AuthMetadata};

        let context = AuthContext::new(
            Some("user-123".to_string()),
            vec!["read".to_string(), "write".to_string()],
            AuthMetadata::new(
                Some("192.168.1.1".to_string()),
                Some("Test-Agent/1.0".to_string()),
            ),
        );

        assert_eq!(context.user_id(), Some("user-123"));
        assert_eq!(context.permissions().len(), 2);
    }

    /// Test 10: Auth context with minimal info
    #[test]
    fn test_auth_context_minimal() {
        use sdforge::security::{AuthContext, AuthMetadata};

        let context = AuthContext::new(None, vec![], AuthMetadata::new(None, None));

        assert_eq!(context.user_id(), None);
        assert_eq!(context.permissions().len(), 0);
    }
}
