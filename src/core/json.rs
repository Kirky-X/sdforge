// Copyright (c) 2026 Kirky.X
//! JSON helper utilities for consistent response formatting.
//!
//! This module provides standardized JSON response helpers used across
//! the framework for error responses, success responses, and API responses.
//!
//! Performance: Uses simd-json for 2-10x faster JSON operations when available.

use serde::Serialize;

/// Create a standardized error response JSON string.
///
/// # Arguments
///
/// * `code` - Error code identifier (e.g., "NOT_FOUND", "VALIDATION_ERROR")
/// * `message` - Human-readable error message
///
/// # Returns
///
/// JSON string with standardized error format
pub fn error_response(code: &str, message: &str) -> String {
    serde_json::to_string(&serde_json::json!({
        "success": false,
        "error": {
            "code": code,
            "message": message
        }
    }))
    .unwrap_or_else(|e| {
        // Fallback for serialization failures - escapes special characters safely
        let escaped_code = code.replace('"', "\\\"");
        let escaped_message = e.to_string().replace('"', "\\\"");
        format!(
            r#"{{"success":false,"error":{{"code":"{}","message":"{}"}}}}"#,
            escaped_code, escaped_message
        )
    })
}

/// Create a standardized success response JSON string.
///
/// # Arguments
///
/// * `data` - Serializable data to include in response
///
/// # Returns
///
/// JSON string with standardized success format
pub fn success_response<T: Serialize>(data: &T) -> String {
    serde_json::to_string(&serde_json::json!({
        "success": true,
        "data": data
    }))
    .unwrap_or_else(|e| error_response("SERIALIZATION_ERROR", &e.to_string()))
}

/// Create a paginated response wrapper.
///
/// # Arguments
///
/// * `items` - Serializable collection of items
/// * `page` - Current page number (1-indexed)
/// * `page_size` - Number of items per page
/// * `total_items` - Total number of items across all pages
///
/// # Returns
///
/// JSON string with standardized pagination format
pub fn paginated_response<T: Serialize>(
    items: &[T],
    page: u32,
    page_size: u32,
    total_items: u32,
) -> String {
    let total_pages = if page_size > 0 {
        total_items.div_ceil(page_size)
    } else {
        0
    };

    serde_json::to_string(&serde_json::json!({
        "success": true,
        "data": {
            "items": items,
            "pagination": {
                "page": page,
                "page_size": page_size,
                "total_items": total_items,
                "total_pages": total_pages,
                "has_next": page < total_pages,
                "has_previous": page > 1
            }
        }
    }))
    .unwrap_or_else(|e| error_response("SERIALIZATION_ERROR", &e.to_string()))
}

/// Create an API metadata response.
///
/// # Arguments
///
/// * `name` - API endpoint name
/// * `version` - API version string
/// * `description` - API description
///
/// # Returns
///
/// JSON string with standardized API metadata format
pub fn api_metadata_response(name: &str, version: &str, description: &str) -> String {
    serde_json::to_string(&serde_json::json!({
        "success": true,
        "api": {
            "name": name,
            "version": version,
            "description": description
        }
    }))
    .unwrap_or_else(|e| error_response("SERIALIZATION_ERROR", &e.to_string()))
}

// =============================================================================
// SIMD-Accelerated JSON Operations (PERF-002)
// =============================================================================

/// Serialize using simd-json when available (performance optimization)
///
/// This provides 2-10x faster JSON serialization for most workloads.
/// Automatically falls back to serde_json if simd-json is not available.
#[cfg(feature = "simd-json")]
pub fn simd_to_string<T: Serialize>(value: &T) -> Result<String, String> {
    use simd_json::serde::to_string;

    to_string(value).map_err(|e| format!("JSON serialization failed: {}", e))
}

/// Deserialize using simd-json when available (performance optimization)
///
/// This provides 2-5x faster JSON deserialization for most workloads.
/// Automatically falls back to serde_json if simd-json is not available.
#[cfg(feature = "simd-json")]
pub fn simd_from_slice<T: serde::de::DeserializeOwned>(slice: &[u8]) -> Result<T, String> {
    use simd_json::serde::from_slice;

    // simd-json requires mutable reference
    let mut owned = slice.to_vec();
    from_slice(&mut owned).map_err(|e| format!("JSON deserialization failed: {}", e))
}

/// Deserialize from string using simd-json when available
#[cfg(feature = "simd-json")]
pub fn simd_from_str<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    use simd_json::serde::from_slice;

    let mut owned = s.as_bytes().to_vec();
    from_slice(&mut owned).map_err(|e| format!("JSON deserialization failed: {}", e))
}

// Stub implementations when simd-json feature is not enabled
#[cfg(not(feature = "simd-json"))]
/// Serialize using serde_json (fallback when simd-json is not available)
///
/// This provides standard JSON serialization using serde_json.
/// Enable the simd-json feature for 2-10x faster serialization.
pub fn simd_to_string<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("JSON serialization failed: {}", e))
}

#[cfg(not(feature = "simd-json"))]
/// Deserialize using serde_json (fallback when simd-json is not available)
///
/// This provides standard JSON deserialization using serde_json.
/// Enable the simd-json feature for 2-5x faster deserialization.
pub fn simd_from_slice<T: serde::de::DeserializeOwned>(slice: &[u8]) -> Result<T, String> {
    serde_json::from_slice(slice).map_err(|e| format!("JSON deserialization failed: {}", e))
}

#[cfg(not(feature = "simd-json"))]
/// Deserialize from string using serde_json (fallback when simd-json is not available)
///
/// This provides standard JSON deserialization from string using serde_json.
/// Enable the simd-json feature for improved performance.
pub fn simd_from_str<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    serde_json::from_str(s).map_err(|e| format!("JSON deserialization failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_error_response_structure() {
        let json = error_response("NOT_FOUND", "Resource not found");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "NOT_FOUND");
        assert_eq!(parsed["error"]["message"], "Resource not found");
    }

    #[test]
    fn test_success_response_with_data() {
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };
        let json = success_response(&data);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"]["name"], "test");
        assert_eq!(parsed["data"]["value"], 42);
    }

    #[test]
    fn test_paginated_response_basic() {
        let items = vec!["item1", "item2", "item3"];
        let json = paginated_response(&items, 1, 2, 5);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"]["pagination"]["page"], 1);
        assert_eq!(parsed["data"]["pagination"]["page_size"], 2);
        assert_eq!(parsed["data"]["pagination"]["total_items"], 5);
        assert_eq!(parsed["data"]["pagination"]["total_pages"], 3);
        assert_eq!(parsed["data"]["pagination"]["has_next"], true);
        assert_eq!(parsed["data"]["pagination"]["has_previous"], false);
    }

    #[test]
    fn test_paginated_response_last_page() {
        let items = vec!["item5"];
        let json = paginated_response(&items, 3, 2, 5);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["data"]["pagination"]["has_next"], false);
        assert_eq!(parsed["data"]["pagination"]["has_previous"], true);
    }

    #[test]
    fn test_paginated_response_zero_page_size() {
        let items: Vec<&str> = vec![];
        let json = paginated_response(&items, 1, 0, 0);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["data"]["pagination"]["total_pages"], 0);
    }

    #[test]
    fn test_api_metadata_response_structure() {
        let json = api_metadata_response("users", "v1", "User management API");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["api"]["name"], "users");
        assert_eq!(parsed["api"]["version"], "v1");
        assert_eq!(parsed["api"]["description"], "User management API");
    }

    #[test]
    fn test_simd_to_string_basic() {
        let data = TestData {
            name: "simd".to_string(),
            value: 100,
        };
        let result = simd_to_string(&data);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("simd"));
    }

    #[test]
    fn test_simd_from_slice_basic() {
        let json = br#"{"name":"test","value":42}"#;
        let result: Result<TestData, _> = simd_from_slice(json);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.name, "test");
        assert_eq!(data.value, 42);
    }

    #[test]
    fn test_simd_from_slice_invalid() {
        let result: Result<serde_json::Value, _> = simd_from_slice(b"not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_simd_from_str_basic() {
        let json = r#"{"name":"str-test","value":99}"#;
        let result: Result<TestData, _> = simd_from_str(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "str-test");
    }

    #[test]
    fn test_simd_from_str_invalid() {
        let result: Result<serde_json::Value, _> = simd_from_str("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_simd_roundtrip() {
        let original = TestData {
            name: "roundtrip".to_string(),
            value: 777,
        };
        let json = simd_to_string(&original).unwrap();
        let restored: TestData = simd_from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_simd_roundtrip_via_slice() {
        let original = vec![1, 2, 3];
        let json = simd_to_string(&original).unwrap();
        let restored: Vec<i32> = simd_from_slice(json.as_bytes()).unwrap();
        assert_eq!(original, restored);
    }

    // ============================================================================
    // Serialization error path tests
    //
    // simd_to_string uses serde_json::to_string internally. When the value's
    // Serialize impl fails, the map_err branch should produce a descriptive
    // error message. These tests cover that error path.
    // ============================================================================

    /// A type whose Serialize impl always fails, to exercise the error
    /// branch in simd_to_string.
    struct FailingSerialize;

    impl serde::Serialize for FailingSerialize {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom(
                "intentional serialization failure",
            ))
        }
    }

    #[test]
    fn test_simd_to_string_serialization_error() {
        let result = simd_to_string(&FailingSerialize);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("JSON serialization failed"),
            "Error message should describe the failure, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_simd_from_slice_deserialization_error_type_mismatch() {
        // Provide valid JSON that doesn't match the expected type to cover
        // the map_err branch in simd_from_slice.
        let result: Result<TestData, _> = simd_from_slice(br#"{"name":123}"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON deserialization failed"));
    }

    #[test]
    fn test_simd_from_str_deserialization_error_type_mismatch() {
        // Provide valid JSON that doesn't match the expected type to cover
        // the map_err branch in simd_from_str.
        let result: Result<TestData, _> = simd_from_str(r#"{"name":true}"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON deserialization failed"));
    }

    // ============================================================================
    // Additional edge case tests
    // ============================================================================

    #[test]
    fn test_paginated_response_single_page() {
        // page=1, total_pages=1: has_next=false, has_previous=false
        let items = vec!["only"];
        let json = paginated_response(&items, 1, 10, 1);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["data"]["pagination"]["total_pages"], 1);
        assert_eq!(parsed["data"]["pagination"]["has_next"], false);
        assert_eq!(parsed["data"]["pagination"]["has_previous"], false);
    }

    #[test]
    fn test_paginated_response_exact_multiple() {
        // total_items exactly divisible by page_size (no partial last page)
        let items = vec!["a", "b"];
        let json = paginated_response(&items, 1, 2, 4);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["data"]["pagination"]["total_pages"], 2);
        assert_eq!(parsed["data"]["pagination"]["has_next"], true);
    }

    #[test]
    fn test_error_response_empty_strings() {
        let json = error_response("", "");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "");
        assert_eq!(parsed["error"]["message"], "");
    }

    #[test]
    fn test_error_response_special_characters() {
        // Special characters that would need escaping in JSON
        let json = error_response("ERR_\"QUOTE\"", "Message with \"quotes\" and \\backslash");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["error"]["code"], "ERR_\"QUOTE\"");
        assert_eq!(
            parsed["error"]["message"],
            "Message with \"quotes\" and \\backslash"
        );
    }

    #[test]
    fn test_success_response_with_nested_data() {
        #[derive(serde::Serialize)]
        struct Nested {
            inner: InnerData,
        }
        #[derive(serde::Serialize)]
        struct InnerData {
            value: i32,
        }

        let data = Nested {
            inner: InnerData { value: 99 },
        };
        let json = success_response(&data);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"]["inner"]["value"], 99);
    }

    #[test]
    fn test_success_response_with_empty_vec() {
        let empty: Vec<i32> = vec![];
        let json = success_response(&empty);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"], serde_json::json!([]));
    }

    #[test]
    fn test_api_metadata_response_empty_description() {
        let json = api_metadata_response("api", "v2", "");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["api"]["description"], "");
    }

    #[test]
    fn test_paginated_response_empty_items() {
        let items: Vec<&str> = vec![];
        let json = paginated_response(&items, 1, 10, 100);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["data"]["items"], serde_json::json!([]));
        assert_eq!(parsed["data"]["pagination"]["total_pages"], 10);
        assert_eq!(parsed["data"]["pagination"]["has_next"], true);
    }

    #[test]
    fn test_simd_to_string_simple_value() {
        // Cover simd_to_string with a primitive value
        let result = simd_to_string(&42i32);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn test_simd_to_string_string_value() {
        let result = simd_to_string(&"hello world".to_string());
        assert!(result.is_ok());
        assert!(result.unwrap().contains("hello world"));
    }

    #[test]
    fn test_simd_from_slice_empty_json() {
        // Empty object should deserialize successfully
        let result: Result<serde_json::Value, _> = simd_from_slice(b"{}");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!({}));
    }

    #[test]
    fn test_simd_from_str_empty_object() {
        let result: Result<serde_json::Value, _> = simd_from_str("{}");
        assert!(result.is_ok());
    }

    // ============================================================================
    // Defensive fallback path documentation
    //
    // The `unwrap_or_else` fallback closures in `error_response`,
    // `success_response`, `paginated_response`, and `api_metadata_response`
    // are defensive code paths that are effectively unreachable in practice:
    //
    // - `error_response` serializes a `serde_json::Value` built from `&str`
    //   slices via `serde_json::json!()`. Converting `&str` to `Value` never
    //   fails, and `serde_json::to_string` on a `Value` always succeeds.
    // - `success_response`/`paginated_response`/`api_metadata_response` embed
    //   the caller's `T: Serialize` value inside `serde_json::json!()`. The
    //   `json!` macro calls `serde_json::to_value(..).expect(..)`, which
    //   *panics* if `T`'s `Serialize` impl fails — so execution never reaches
    //   the `to_string` call, let alone its `unwrap_or_else` fallback.
    //
    // The following `#[should_panic]` test documents this actual behavior:
    // passing a non-serializable type panics rather than triggering the
    // fallback. This is the real runtime behavior and must not be silently
    // changed without updating the fallback design.
    // ============================================================================

    /// Document that `success_response` panics (rather than falling back) when
    /// given a value whose `Serialize` implementation fails, because the
    /// `serde_json::json!` macro calls `to_value(..).expect(..)` internally.
    #[test]
    #[should_panic]
    fn test_success_response_panics_on_unserializable_data() {
        let _ = success_response(&FailingSerialize);
    }

    /// Document that `paginated_response` panics (rather than falling back)
    /// when given items whose `Serialize` implementation fails.
    #[test]
    #[should_panic]
    fn test_paginated_response_panics_on_unserializable_data() {
        let items = [FailingSerialize];
        let _ = paginated_response(&items, 1, 1, 1);
    }

    // ============================================================================
    // Edge case tests for the success paths
    // ============================================================================

    /// Test error_response with unicode characters in both code and message.
    #[test]
    fn test_error_response_unicode_characters() {
        let json = error_response("错误", "资源未找到：用户");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "错误");
        assert_eq!(parsed["error"]["message"], "资源未找到：用户");
    }

    /// Test success_response with a boolean value.
    #[test]
    fn test_success_response_with_boolean() {
        let json = success_response(&true);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"], true);
    }

    /// Test success_response with None to verify Option serialization.
    #[test]
    fn test_success_response_with_none() {
        let json = success_response(&None::<i32>);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"], serde_json::Value::Null);
    }

    /// Test paginated_response with a large total_items to verify div_ceil
    /// calculates total_pages correctly for non-exact multiples.
    #[test]
    fn test_paginated_response_non_exact_multiple() {
        let items = vec!["a"];
        // 7 items, page_size 3 -> total_pages = ceil(7/3) = 3
        let json = paginated_response(&items, 1, 3, 7);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["data"]["pagination"]["total_pages"], 3);
        assert_eq!(parsed["data"]["pagination"]["has_next"], true);
    }

    /// Test paginated_response on the last page of a non-exact multiple.
    #[test]
    fn test_paginated_response_last_page_non_exact() {
        let items = vec!["c"];
        // page=3, total_pages=3 -> has_next=false, has_previous=true
        let json = paginated_response(&items, 3, 3, 7);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["data"]["pagination"]["has_next"], false);
        assert_eq!(parsed["data"]["pagination"]["has_previous"], true);
    }

    /// Test api_metadata_response with special characters in description.
    #[test]
    fn test_api_metadata_response_special_characters() {
        let json = api_metadata_response(
            "api/v1",
            "2.0.0-beta",
            "Handles \"quotes\", backslashes \\, and unicode 世界",
        );
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["api"]["name"], "api/v1");
        assert_eq!(parsed["api"]["version"], "2.0.0-beta");
        assert!(parsed["api"]["description"]
            .as_str()
            .unwrap()
            .contains("世界"));
    }

    /// Test paginated_response with page_size=1 and many items.
    #[test]
    fn test_paginated_response_page_size_one() {
        let items = vec!["only"];
        let json = paginated_response(&items, 1, 1, 5);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["data"]["pagination"]["total_pages"], 5);
        assert_eq!(parsed["data"]["pagination"]["has_next"], true);
        assert_eq!(parsed["data"]["pagination"]["has_previous"], false);
    }

    /// Test simd_to_string with a Vec of mixed values.
    #[test]
    fn test_simd_to_string_with_vec() {
        let data = vec![1, 2, 3, 4, 5];
        let result = simd_to_string(&data);
        assert!(result.is_ok());
        let json = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, serde_json::json!([1, 2, 3, 4, 5]));
    }

    /// Test simd_from_slice with a valid JSON array.
    #[test]
    fn test_simd_from_slice_array() {
        let json = br#"[1, 2, 3]"#;
        let result: Result<Vec<i32>, _> = simd_from_slice(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    /// Test simd_from_str with a JSON boolean.
    #[test]
    fn test_simd_from_str_boolean() {
        let result: Result<bool, _> = simd_from_str("true");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
