//! JSON helper utilities for consistent response formatting.
//!
//! This module provides standardized JSON response helpers used across
//! the framework for error responses, success responses, and API responses.

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
        (total_items + page_size - 1) / page_size
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_format() {
        let response = error_response("NOT_FOUND", "Resource not found");
        assert!(response.contains("\"success\":false"));
        assert!(response.contains("\"code\":\"NOT_FOUND\""));
        assert!(response.contains("\"message\":\"Resource not found\""));
    }

    #[test]
    fn test_success_response_format() {
        let data = serde_json::json!({"key": "value"});
        let response = success_response(&data);
        assert!(response.contains("\"success\":true"));
        assert!(response.contains("\"key\":\"value\""));
    }

    #[test]
    fn test_paginated_response_format() {
        let items = vec!["item1", "item2"];
        let response = paginated_response(&items, 1, 10, 25);
        assert!(response.contains("\"success\":true"));
        assert!(response.contains("\"page\":1"));
        assert!(response.contains("\"page_size\":10"));
        assert!(response.contains("\"total_items\":25"));
        assert!(response.contains("\"total_pages\":3"));
    }

    #[test]
    fn test_api_metadata_response_format() {
        let response = api_metadata_response("test-api", "v1", "Test API description");
        assert!(response.contains("\"success\":true"));
        assert!(response.contains("\"name\":\"test-api\""));
        assert!(response.contains("\"version\":\"v1\""));
        assert!(response.contains("\"description\":\"Test API description\""));
    }
}
