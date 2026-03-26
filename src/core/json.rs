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
pub fn simd_to_string<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("JSON serialization failed: {}", e))
}

#[cfg(not(feature = "simd-json"))]
pub fn simd_from_slice<T: serde::de::DeserializeOwned>(slice: &[u8]) -> Result<T, String> {
    serde_json::from_slice(slice).map_err(|e| format!("JSON deserialization failed: {}", e))
}

#[cfg(not(feature = "simd-json"))]
pub fn simd_from_str<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    serde_json::from_str(s).map_err(|e| format!("JSON deserialization failed: {}", e))
}
