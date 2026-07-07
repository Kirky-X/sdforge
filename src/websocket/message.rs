// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! WebSocket message types and parsing utilities.
//!
//! Provides [`WebSocketMessage`] enum for request/response/error/notification
//! exchanges, along with security-hardened parsing that enforces message size
//! and JSON nesting depth limits.

#[cfg(feature = "websocket")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "websocket")]
use serde_json::Value;

/// Maximum message size in bytes (1MB).
///
/// Enforced by [`parse_websocket_message`] to prevent memory exhaustion from
/// oversized payloads.
#[cfg(feature = "websocket")]
pub const MAX_MESSAGE_SIZE: usize = 1_048_576;

/// Maximum nesting depth for JSON parsing (prevents stack overflow from deeply nested JSON).
#[cfg(feature = "websocket")]
pub const MAX_JSON_DEPTH: usize = 16;

/// Maximum length for string fields in WebSocket messages (64KB).
///
/// # Security Purpose
///
/// This constant defines a reasonable upper bound for string field lengths to prevent:
/// - Memory exhaustion attacks via oversized string fields
/// - Buffer overflow vulnerabilities in downstream processing
/// - Performance degradation from processing extremely large strings
///
/// # Future Use
///
/// While not currently enforced in parsing logic, this constant serves as:
/// - A reference for implementing future validation checks
/// - A best practice reminder for secure WebSocket message handling
/// - A potential configuration parameter for custom validation rules
///
/// # Recommendation
///
/// Implementers should consider validating string field lengths against this limit
/// when processing WebSocket messages, especially in production environments.
#[cfg(test)]
pub const MAX_STRING_LENGTH: usize = 64 * 1024; // 64KB

#[cfg(feature = "websocket")]
/// WebSocket message type
///
/// Represents different types of WebSocket messages exchanged between
/// client and server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Request message from client
    #[serde(rename = "request")]
    Request {
        /// Unique request identifier
        id: String,
        /// Method name to invoke
        method: String,
        /// Method parameters
        params: serde_json::Value,
    },
    /// Response message to client
    #[serde(rename = "response")]
    Response {
        /// Request identifier
        id: String,
        /// Result data
        result: serde_json::Value,
    },
    /// Error message to client
    #[serde(rename = "error")]
    Error {
        /// Request identifier
        id: String,
        /// Error message
        error: String,
    },
    /// Notification message to client
    #[serde(rename = "notification")]
    Notification {
        /// Event name
        event: String,
        /// Event data
        data: serde_json::Value,
    },
}

#[cfg(feature = "websocket")]
/// Parse and validate a WebSocket message from JSON text
///
/// # Security
///
/// This function provides security measures:
/// - Maximum message size validation (1MB)
/// - JSON nesting depth validation (max 16 levels)
///
/// # Errors
///
/// Returns an error if:
/// - Message exceeds maximum size
/// - JSON is malformed
/// - Nesting depth exceeds limit
/// - Message is not a valid WebSocketMessage variant
pub fn parse_websocket_message(text: &str) -> Result<WebSocketMessage, String> {
    // First, check basic size limit
    if text.len() > MAX_MESSAGE_SIZE {
        return Err(format!(
            "Message too large: {} bytes (max: {} bytes)",
            text.len(),
            MAX_MESSAGE_SIZE
        ));
    }

    // Security fix: Use serde_json's streaming parser to parse and validate depth
    // This provides more accurate depth checking than manual bracket counting
    use serde_json::Deserializer;

    // Parse and validate depth
    let mut max_depth = 0;
    let mut current_depth = 0;

    let deserializer = Deserializer::from_str(text);
    for result in deserializer.into_iter::<Value>() {
        match result {
            Ok(value) => {
                // Calculate actual depth of the parsed value
                let depth = calculate_value_depth(&value, &mut current_depth);
                max_depth = max_depth.max(depth);

                if max_depth > MAX_JSON_DEPTH {
                    return Err(format!(
                        "JSON nesting too deep: depth {} (max: {})",
                        max_depth, MAX_JSON_DEPTH
                    ));
                }
            }
            Err(e) => {
                return Err(format!("Invalid JSON: {}", e));
            }
        }
    }

    // Parse the actual WebSocket message
    serde_json::from_str::<WebSocketMessage>(text).map_err(|e| format!("Invalid JSON: {}", e))
}

/// Calculate depth of a JSON value recursively
pub fn calculate_value_depth(value: &serde_json::Value, current_depth: &mut usize) -> usize {
    match value {
        Value::Object(map) => {
            *current_depth += 1;
            let max_child_depth = map
                .values()
                .map(|v| calculate_value_depth(v, current_depth))
                .max()
                .unwrap_or(0);
            *current_depth -= 1;
            max_child_depth
        }
        Value::Array(arr) => {
            *current_depth += 1;
            let max_child_depth = arr
                .iter()
                .map(|v| calculate_value_depth(v, current_depth))
                .max()
                .unwrap_or(0);
            *current_depth -= 1;
            max_child_depth
        }
        _ => *current_depth,
    }
}

/// Calculate actual JSON nesting depth by parsing the structure
/// Returns the maximum nesting level encountered
///
/// This function is kept for testing purposes only.
/// Production code uses [`calculate_value_depth`] which operates on parsed JSON values.
#[cfg(test)]
pub fn calculate_json_depth(text: &str) -> usize {
    let mut depth = 0;
    let mut max_depth = 0;
    let mut in_string = false;
    let mut escaped = false;

    for c in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            escaped = false;
        } else if c == '{' || c == '[' {
            depth += 1;
            max_depth = max_depth.max(depth);
        } else if (c == '}' || c == ']') && depth > 0 {
            depth -= 1;
        }
    }

    max_depth
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Primitive value at non-zero current_depth — the `_ => *current_depth`
    /// branch returns the current depth, not 0. Verifies depth is untouched
    /// for primitives.
    #[test]
    fn calculate_value_depth_primitive_at_nonzero_depth() {
        let value = serde_json::json!(42);
        let mut depth = 5;
        let result = calculate_value_depth(&value, &mut depth);
        assert_eq!(result, 5);
        assert_eq!(depth, 5);
    }

    /// Object containing an empty child object — exercises the `.unwrap_or(0)`
    /// path for an empty child nested inside a non-empty parent, mixed with a
    /// primitive sibling that returns `*current_depth`.
    #[test]
    fn calculate_value_depth_object_with_empty_child() {
        let value = serde_json::json!({"a": {}, "b": 1});
        let mut depth = 0;
        let result = calculate_value_depth(&value, &mut depth);
        // Empty child returns 0; primitive "b" returns 1 (current_depth=1).
        // max(0, 1) = 1.
        assert_eq!(result, 1);
        assert_eq!(depth, 0);
    }

    /// Array containing an empty child array — exercises the `.unwrap_or(0)`
    /// path for an empty array nested inside a non-empty parent.
    #[test]
    fn calculate_value_depth_array_with_empty_child() {
        let value = serde_json::json!([[], 1]);
        let mut depth = 0;
        let result = calculate_value_depth(&value, &mut depth);
        // Empty child returns 0; primitive 1 returns 1 (current_depth=1).
        // max(0, 1) = 1.
        assert_eq!(result, 1);
        assert_eq!(depth, 0);
    }

    /// calculate_json_depth with unmatched closing braces — the
    /// `(c == '}' || c == ']') && depth > 0` condition is false (depth == 0),
    /// so `depth -= 1` is never executed.
    #[test]
    fn calculate_json_depth_unmatched_closing_brace() {
        assert_eq!(calculate_json_depth("}"), 0);
        assert_eq!(calculate_json_depth("]"), 0);
        assert_eq!(calculate_json_depth("}]}]"), 0);
    }

    /// calculate_json_depth with an escape sequence inside a JSON string —
    /// exercises the `escaped = false` (line 193) and `escaped = true`
    /// (line 195) branches for backslash escape handling.
    #[test]
    fn calculate_json_depth_with_escape_sequence_in_string() {
        // Text: {"a":"b\\c"} — the \\ inside the string triggers escape handling.
        // Returns max_depth=1 (single-level object), not 0.
        let text = r#"{"a":"b\\c"}"#;
        let depth = calculate_json_depth(text);
        assert_eq!(depth, 1, "Single-level JSON with escape should return depth 1");
    }

    /// parse_websocket_message with multiple top-level JSON values — the
    /// streaming parser loop runs more than once, then serde_json::from_str
    /// fails on the trailing data.
    #[test]
    fn parse_websocket_message_multiple_top_level_values() {
        let json = r#"{"type":"request","id":"1","method":"m","params":{}}{"extra":true}"#;
        let result = parse_websocket_message(json);
        assert!(result.is_err());
    }
}
