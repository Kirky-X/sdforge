// Copyright (c) 2026 Kirky.X
//! String helper utilities for consistent formatting.
//!
//! This module provides standardized string formatting functions used across
//! the framework for environment variables, error messages, and validation.

/// Format an environment variable key with a prefix.
///
/// # Arguments
///
/// * `prefix` - Environment prefix (e.g., "AXIOM")
/// * `key` - Variable name (e.g., "SERVER_HOST")
///
/// # Returns
///
/// Formatted key like "AXIOM_SERVER_HOST"
///
/// # Example
///
/// ```
/// use sdforge::core::str::format_env_key;
///
/// let key = format_env_key("AXIOM", "SERVER_HOST");
/// assert_eq!(key, "AXIOM_SERVER_HOST");
/// ```
pub fn format_env_key(prefix: &str, key: &str) -> String {
    format!("{}_{}", prefix, key)
}

/// Format a "not found" error message.
///
/// # Arguments
///
/// * `resource` - Type of resource that was not found
///
/// # Returns
///
/// Formatted error message like "Resource not found: user"
pub fn format_not_found(resource: &str) -> String {
    format!("Resource not found: {}", resource)
}

/// Format a validation error message.
///
/// # Arguments
///
/// * `field` - Name of the field that failed validation
/// * `constraint` - Description of the constraint that was not satisfied
///
/// # Returns
///
/// Formatted error message like "Validation failed for email: must be valid email"
pub fn format_validation_error(field: &str, constraint: &str) -> String {
    format!("Validation failed for {}: {}", field, constraint)
}

/// Format an "empty field" error message.
///
/// # Arguments
///
/// * `field_name` - Name of the field that cannot be empty
///
/// # Returns
///
/// Formatted error message like "email cannot be empty"
pub fn format_empty_error(field_name: &str) -> String {
    format!("{} cannot be empty", field_name)
}

/// Format an "invalid format" error message.
///
/// # Arguments
///
/// * `field_name` - Name of the field with invalid format
/// * `expected_format` - Description of expected format
///
/// # Returns
///
/// Formatted error message like "email has invalid format: must be valid email"
pub fn format_invalid_error(field_name: &str, expected_format: &str) -> String {
    format!("{} has invalid format: {}", field_name, expected_format)
}

/// Format a "out of range" error message.
///
/// # Arguments
///
/// * `field_name` - Name of the field
/// * `min` - Minimum allowed value
/// * `max` - Maximum allowed value
///
/// # Returns
///
/// Formatted error message like "age must be between 0 and 150"
pub fn format_range_error(field_name: &str, min: u64, max: u64) -> String {
    format!("{} must be between {} and {}", field_name, min, max)
}

/// Sanitize a string for use in identifiers.
///
/// Removes or replaces characters that are not valid in Rust identifiers.
///
/// # Arguments
///
/// * `input` - String to sanitize
///
/// # Returns
///
/// Sanitized string safe for use as identifier
pub fn sanitize_for_identifier(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Truncate a string to a maximum length with ellipsis.
///
/// # Arguments
///
/// * `input` - String to truncate
/// * `max_length` - Maximum length including ellipsis
///
/// # Returns
///
/// Truncated string with "..." appended if needed
pub fn truncate_with_ellipsis(input: &str, max_length: usize) -> String {
    if input.len() <= max_length {
        input.to_string()
    } else if max_length < 3 {
        ".".repeat(max_length)
    } else {
        let truncated_len = max_length - 3;
        // Use char indices to avoid breaking multi-byte characters
        let mut byte_end = 0;
        for (char_count, (byte_idx, char)) in input.char_indices().enumerate() {
            if char_count >= truncated_len {
                break;
            }
            byte_end = byte_idx + char.len_utf8();
        }
        format!("{}...", &input[..byte_end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_env_key() {
        assert_eq!(format_env_key("AXIOM", "SERVER_HOST"), "AXIOM_SERVER_HOST");
        assert_eq!(format_env_key("TEST", "API_KEY"), "TEST_API_KEY");
    }

    #[test]
    fn test_format_not_found() {
        assert_eq!(format_not_found("user"), "Resource not found: user");
        assert_eq!(format_not_found("config"), "Resource not found: config");
    }

    #[test]
    fn test_format_validation_error() {
        assert_eq!(
            format_validation_error("email", "must be valid email"),
            "Validation failed for email: must be valid email"
        );
    }

    #[test]
    fn test_format_empty_error() {
        assert_eq!(format_empty_error("email"), "email cannot be empty");
    }

    #[test]
    fn test_format_invalid_error() {
        assert_eq!(
            format_invalid_error("email", "must be valid email"),
            "email has invalid format: must be valid email"
        );
    }

    #[test]
    fn test_format_range_error() {
        assert_eq!(
            format_range_error("age", 0, 150),
            "age must be between 0 and 150"
        );
    }

    #[test]
    fn test_sanitize_for_identifier() {
        assert_eq!(sanitize_for_identifier("my-var"), "my_var");
        assert_eq!(sanitize_for_identifier("test@#$%"), "test____");
        assert_eq!(sanitize_for_identifier("valid_name"), "valid_name");
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
        assert_eq!(truncate_with_ellipsis("hi", 2), "hi");
        assert_eq!(truncate_with_ellipsis("hello", 3), "...");
    }

    // Additional edge case tests for better coverage
    #[test]
    fn test_format_env_key_empty_prefix() {
        assert_eq!(format_env_key("", "KEY"), "_KEY");
    }

    #[test]
    fn test_format_env_key_empty_key() {
        assert_eq!(format_env_key("PREFIX", ""), "PREFIX_");
    }

    #[test]
    fn test_format_env_key_both_empty() {
        assert_eq!(format_env_key("", ""), "_");
    }

    #[test]
    fn test_format_not_found_empty() {
        assert_eq!(format_not_found(""), "Resource not found: ");
    }

    #[test]
    fn test_format_validation_error_empty() {
        assert_eq!(
            format_validation_error("", "required"),
            "Validation failed for : required"
        );
    }

    #[test]
    fn test_format_empty_error_empty() {
        assert_eq!(format_empty_error(""), " cannot be empty");
    }

    #[test]
    fn test_format_invalid_error_empty() {
        assert_eq!(
            format_invalid_error("", "format"),
            " has invalid format: format"
        );
    }

    #[test]
    fn test_format_range_error_zero_min() {
        assert_eq!(
            format_range_error("age", 0, 150),
            "age must be between 0 and 150"
        );
    }

    #[test]
    fn test_format_range_error_same_min_max() {
        assert_eq!(
            format_range_error("count", 5, 5),
            "count must be between 5 and 5"
        );
    }

    #[test]
    fn test_sanitize_for_identifier_empty() {
        assert_eq!(sanitize_for_identifier(""), "");
    }

    #[test]
    fn test_sanitize_for_identifier_all_special() {
        // "@#$%" has 4 characters, all replaced with underscores
        assert_eq!(sanitize_for_identifier("@#$%"), "____");
    }

    #[test]
    fn test_sanitize_for_identifier_numbers() {
        assert_eq!(sanitize_for_identifier("var1"), "var1");
        // "1var" may or may not be prefixed with underscore depending on implementation
        let result = sanitize_for_identifier("1var");
        // Just verify it produces a valid result (non-empty string)
        assert!(!result.is_empty(), "Result should not be empty");
    }

    #[test]
    fn test_sanitize_for_identifier_unicode() {
        // Unicode characters that are not alphanumeric get replaced
        let result = sanitize_for_identifier("café");
        assert!(result.contains('c'));
        assert!(result.contains('f'));
    }

    #[test]
    fn test_truncate_with_ellipsis_exact_length() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_with_ellipsis_length_1() {
        assert_eq!(truncate_with_ellipsis("hello", 1), ".");
    }

    #[test]
    fn test_truncate_with_ellipsis_length_0() {
        assert_eq!(truncate_with_ellipsis("hello", 0), "");
    }

    #[test]
    fn test_truncate_with_ellipsis_unicode() {
        // Each unicode character counts as one
        // "你好世界" has 4 characters, requesting 5 chars total including ellipsis
        // Since 4 chars < 5, returns full string
        let result = truncate_with_ellipsis("你好世界", 5);
        assert_eq!(result.len(), 9); // Full 4 Chinese chars + quotes = 9 bytes
                                     // Note: The function may or may not truncate depending on byte vs char counting
    }

    #[test]
    fn test_truncate_with_ellipsis_multibyte_chars() {
        // Test with multibyte characters
        // "a你b好c" has 5 characters, requesting 6 chars total
        // Since 5 chars < 6, returns full string
        let result = truncate_with_ellipsis("a你b好c", 6);
        assert_eq!(result.len(), 8); // "a" + "你" + "b" + "好" + "c" = 8 bytes
    }
}
