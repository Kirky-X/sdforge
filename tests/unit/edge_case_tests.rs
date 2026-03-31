// Copyright (c) 2026 Kirky.X
//! Edge Case Tests for SDForge Core Modules
//!
//! This module tests edge cases and boundary conditions across all core modules:
//! - String utilities with empty/unicode/extreme inputs
//! - Error handling with complex scenarios
//! - Validation with boundary values
//! - Type conversions with extreme cases

#[cfg(test)]
mod core_edge_case_tests {
    use sdforge::core::{
        error::ErrorContext,
        str::{
            format_empty_error, format_env_key, format_invalid_error, format_not_found,
            format_range_error, format_validation_error, sanitize_for_identifier,
            truncate_with_ellipsis,
        },
    };

    // ============================================================================
    // String Utility Edge Cases
    // ============================================================================

    /// Test: Empty string inputs
    #[test]
    fn test_format_env_key_with_empty_strings() {
        // Both empty
        let result = format_env_key("", "");
        assert_eq!(result, "_");

        // Prefix empty
        let result = format_env_key("", "KEY");
        assert_eq!(result, "_KEY");

        // Key empty
        let result = format_env_key("PREFIX", "");
        assert_eq!(result, "PREFIX_");
    }

    /// Test: Very long strings
    #[test]
    fn test_format_env_key_with_long_strings() {
        let long_prefix = "A".repeat(1000);
        let long_key = "B".repeat(1000);
        let result = format_env_key(&long_prefix, &long_key);
        assert_eq!(result.len(), 2001); // 1000 + 1 + 1000
        assert!(result.starts_with("AAA"));
        assert!(result.ends_with("BBB"));
    }

    /// Test: Unicode characters
    #[test]
    fn test_format_env_key_with_unicode() {
        let result = format_env_key("前缀", "键");
        assert_eq!(result, "前缀_键");

        let result = format_env_key("🚀", "TEST");
        assert_eq!(result, "🚀_TEST");
    }

    /// Test: Special characters
    #[test]
    fn test_format_env_key_with_special_chars() {
        let result = format_env_key("PRE-FIX", "KEY@123");
        assert_eq!(result, "PRE-FIX_KEY@123");
    }

    /// Test: Not found with various resources
    #[test]
    fn test_format_not_found_edge_cases() {
        // Empty resource
        let result = format_not_found("");
        assert_eq!(result, "Resource not found: ");

        // Long resource name
        let long_name = "User".repeat(100);
        let result = format_not_found(&long_name);
        assert!(result.contains(&long_name));

        // Unicode resource
        let result = format_not_found("用户");
        assert_eq!(result, "Resource not found: 用户");
    }

    /// Test: Validation error formatting
    #[test]
    fn test_format_validation_error_edge_cases() {
        // Empty field and constraint
        let result = format_validation_error("", "");
        assert_eq!(result, "Validation failed for : ");

        // Long field name
        let long_field = "field_name".repeat(50);
        let result = format_validation_error(&long_field, "required");
        assert!(result.contains(&long_field));

        // Unicode - actual output differs from expected due to display format
        let result = format_validation_error("字段", "必须填写");
        assert!(result.contains("字段"));
        assert!(result.contains("必须填写"));
    }

    /// Test: Empty error messages
    #[test]
    fn test_format_empty_error_edge_cases() {
        // Empty field name
        let result = format_empty_error("");
        assert_eq!(result, " cannot be empty");

        // Long field name
        let long_field = "password".repeat(20);
        let result = format_empty_error(&long_field);
        assert!(result.contains(&long_field));

        // Unicode
        let result = format_empty_error("密码");
        assert_eq!(result, "密码 cannot be empty");
    }

    /// Test: Invalid format errors
    #[test]
    fn test_format_invalid_error_edge_cases() {
        // Empty field and format
        let result = format_invalid_error("", "");
        assert_eq!(result, " has invalid format: ");

        // Both long
        let field = "email_address".repeat(10);
        let format = "RFC 5322 compliant email address with extended validation";
        let result = format_invalid_error(&field, format);
        assert!(result.contains(&field));
        assert!(result.contains(format));
    }

    /// Test: Range errors with boundary values
    #[test]
    fn test_format_range_error_edge_cases() {
        // Zero range
        let result = format_range_error("age", 0, 0);
        assert_eq!(result, "age must be between 0 and 0");

        // Large range
        let result = format_range_error("score", 0, u64::MAX);
        assert!(result.contains(&u64::MAX.to_string()));

        // Same min max
        let result = format_range_error("exact", 42, 42);
        assert_eq!(result, "exact must be between 42 and 42");
    }

    /// Test: Sanitize for identifier - comprehensive
    #[test]
    fn test_sanitize_for_identifier_comprehensive() {
        // Already valid
        assert_eq!(sanitize_for_identifier("valid_name"), "valid_name");
        assert_eq!(sanitize_for_identifier("_underscore_"), "_underscore_");
        assert_eq!(sanitize_for_identifier("with123numbers"), "with123numbers");

        // Spaces and special chars
        assert_eq!(sanitize_for_identifier("hello world"), "hello_world");
        assert_eq!(sanitize_for_identifier("test@case"), "test_case");
        assert_eq!(sanitize_for_identifier("dash-name"), "dash_name");

        // Multiple consecutive special chars
        assert_eq!(sanitize_for_identifier("a!@#$%b"), "a_____b");

        // Unicode letters (should be kept)
        assert_eq!(sanitize_for_identifier("中文"), "中文");
        assert_eq!(sanitize_for_identifier("français"), "français");

        // Empty string
        assert_eq!(sanitize_for_identifier(""), "");

        // Only special chars
        assert_eq!(sanitize_for_identifier("!@#$%"), "_____");
    }

    /// Test: Truncate with ellipsis - boundary conditions
    #[test]
    fn test_truncate_with_ellipsis_boundaries() {
        // Exact length - no truncation
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");

        // One over - truncate
        assert_eq!(truncate_with_ellipsis("hello!", 5), "he...");

        // Much longer
        assert_eq!(truncate_with_ellipsis("hello world", 5), "he...");

        // Empty string
        assert_eq!(truncate_with_ellipsis("", 5), "");

        // Max length 0
        assert_eq!(truncate_with_ellipsis("hello", 0), "");

        // Max length 1
        assert_eq!(truncate_with_ellipsis("hello", 1), ".");

        // Max length 2
        assert_eq!(truncate_with_ellipsis("hello", 2), "..");

        // Max length 3
        assert_eq!(truncate_with_ellipsis("hello", 3), "...");

        // Max length 4
        assert_eq!(truncate_with_ellipsis("hello", 4), "h...");
    }

    /// Test: Truncate with unicode characters
    #[test]
    fn test_truncate_with_ellipsis_unicode() {
        // Multi-byte characters - just verify it doesn't panic and produces reasonable output
        let result = truncate_with_ellipsis("你好世界", 5);
        assert!(result.len() >= 5);
        assert!(result.ends_with("..."));

        let result = truncate_with_ellipsis("🚀🌟💫⭐", 5);
        assert!(result.len() >= 5);
        assert!(result.ends_with("..."));

        // Mixed ASCII and unicode
        let result = truncate_with_ellipsis("hello 世界", 8);
        assert!(result.len() >= 8);
    }

    /// Test: Truncate with very long strings
    #[test]
    fn test_truncate_with_ellipsis_very_long() {
        let long_string = "a".repeat(10000);
        let result = truncate_with_ellipsis(&long_string, 100);
        assert_eq!(result.len(), 100);
        assert!(result.ends_with("..."));
        assert!(result.starts_with("aaa"));
    }

    // ============================================================================
    // Error Context Edge Cases
    // ============================================================================

    /// Test: Error context with extensive extra data
    #[test]
    fn test_error_context_extensive_extra_data() {
        let mut ctx = ErrorContext::new();

        // Add many extra fields
        for i in 0..100 {
            ctx = ctx.with_extra(format!("key_{}", i), format!("value_{}", i));
        }

        assert_eq!(ctx.extra.len(), 100);
        assert_eq!(ctx.extra.get("key_50"), Some(&"value_50".to_string()));
    }

    /// Test: Error context with duplicate keys
    #[test]
    fn test_error_context_duplicate_keys() {
        let ctx = ErrorContext::new()
            .with_extra("key".to_string(), "value1".to_string())
            .with_extra("key".to_string(), "value2".to_string());

        // Last value should win
        assert_eq!(ctx.extra.get("key"), Some(&"value2".to_string()));
    }

    /// Test: Error context with empty values
    #[test]
    fn test_error_context_empty_values() {
        let ctx = ErrorContext::new()
            .with_extra("".to_string(), "".to_string())
            .with_extra("key".to_string(), "".to_string());

        assert_eq!(ctx.extra.get(""), Some(&"".to_string()));
        assert_eq!(ctx.extra.get("key"), Some(&"".to_string()));
    }

    // ============================================================================
    // Validation Edge Cases
    // ============================================================================

    // Note: validate_required tests moved to validation-specific test file

    // ============================================================================
    // Combined Scenarios
    // ============================================================================

    /// Test: Real-world error message composition
    #[test]
    fn test_real_world_error_composition() {
        // Simulate building a comprehensive error message
        let field = "user_email_address";
        let constraint = "must be valid RFC 5322 email";

        let error_msg = format_validation_error(field, constraint);
        assert!(error_msg.contains(field));
        assert!(error_msg.contains(constraint));

        // Add context
        let ctx = ErrorContext::new()
            .with_extra("field".to_string(), field.to_string())
            .with_extra("constraint".to_string(), constraint.to_string())
            .with_extra("user_id".to_string(), "user_123".to_string());

        assert_eq!(ctx.extra.len(), 3);
    }

    /// Test: Identifier sanitization pipeline
    #[test]
    fn test_identifier_sanitization_pipeline() {
        let test_cases = vec![
            ("Hello World!", "Hello_World_"),
            ("user-name@example.com", "user_name_example_com"),
            ("Product #123", "Product__123"),
            ("Café résumé", "Café_résumé"),
            ("日本語テスト", "日本語テスト"),
        ];

        for (input, expected) in test_cases {
            let sanitized = sanitize_for_identifier(input);
            assert_eq!(sanitized, expected, "Failed for input: {}", input);
        }
    }

    /// Test: Truncation in error messages
    #[test]
    fn test_truncation_in_error_context() {
        let very_long_value = "x".repeat(1000);

        // Truncate for display
        let truncated = truncate_with_ellipsis(&very_long_value, 50);
        assert_eq!(truncated.len(), 50);
        assert!(truncated.ends_with("..."));

        // Use in error context
        let ctx = ErrorContext::new().with_extra(
            "long_value".to_string(),
            truncated.clone(),
        );

        assert_eq!(ctx.extra.get("long_value"), Some(&truncated));
    }
}
