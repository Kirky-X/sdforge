//! `McpToolInstance` and helpers — runtime representation of a registered tool.
//!
//! This module defines the runtime tool instance type and the conversion
//! helper used to bridge `serde_json::Value` schemas to rmcp's `Arc<JsonObject>`.

use rmcp::model::JsonObject;
use serde_json::Value;
use std::sync::Arc;

use crate::core::ApiMetadata;
use crate::mcp::SdForgeTool;

/// MCP tool instance with runtime-allocated Arc.
///
/// This is the runtime representation created from `McpToolRegistration`.
/// It holds the actual tool implementation (as `Arc<dyn SdForgeTool>`) and
/// the associated `ApiMetadata`.
#[derive(Clone)]
pub struct McpToolInstance {
    /// The actual tool implementation (Arc for shared ownership)
    pub(crate) tool: Arc<dyn SdForgeTool>,
    /// API metadata
    pub(crate) metadata: ApiMetadata,
}

impl McpToolInstance {
    /// Create a new tool instance from a tool and metadata.
    pub fn new(tool: Arc<dyn SdForgeTool>, metadata: ApiMetadata) -> Self {
        Self { tool, metadata }
    }

    /// Get a reference to the tool implementation.
    pub fn tool(&self) -> &Arc<dyn SdForgeTool> {
        &self.tool
    }

    /// Get a reference to the metadata.
    pub fn metadata(&self) -> &ApiMetadata {
        &self.metadata
    }
}

/// Convert a `serde_json::Value` to `Arc<JsonObject>` for rmcp's `Tool::input_schema`.
///
/// If the value is a JSON object, its map is wrapped in `Arc`. Otherwise, an
/// empty object schema is returned (the MCP spec requires `input_schema` to be
/// a JSON Schema object).
pub(crate) fn value_to_json_object_arc(value: Value) -> Arc<JsonObject> {
    match value {
        Value::Object(map) => Arc::new(map),
        // Non-object schemas are invalid per MCP spec; use empty object as fallback.
        _ => Arc::new(serde_json::Map::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that a JSON object value is wrapped directly into the Arc.
    /// The returned map should contain the same key/value pairs as the input.
    #[test]
    fn test_value_to_json_object_arc_with_object() {
        let map = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let arc = value_to_json_object_arc(map);
        assert_eq!(arc.len(), 2);
        assert!(arc.contains_key("type"));
        assert!(arc.contains_key("properties"));
    }

    /// Verify that an empty JSON object produces an empty (but valid) JsonObject.
    #[test]
    fn test_value_to_json_object_arc_with_empty_object() {
        let arc = value_to_json_object_arc(serde_json::json!({}));
        assert!(arc.is_empty());
        assert_eq!(arc.len(), 0);
    }

    /// Verify that a JSON string value falls back to an empty JsonObject
    /// (non-object branch of the match).
    #[test]
    fn test_value_to_json_object_arc_with_string_falls_back_to_empty() {
        let arc = value_to_json_object_arc(serde_json::json!("not an object"));
        assert!(arc.is_empty());
    }

    /// Verify that a JSON number value falls back to an empty JsonObject.
    #[test]
    fn test_value_to_json_object_arc_with_number_falls_back_to_empty() {
        let arc = value_to_json_object_arc(serde_json::json!(42));
        assert!(arc.is_empty());
    }

    /// Verify that a JSON array value falls back to an empty JsonObject.
    #[test]
    fn test_value_to_json_object_arc_with_array_falls_back_to_empty() {
        let arc = value_to_json_object_arc(serde_json::json!([1, 2, 3]));
        assert!(arc.is_empty());
    }

    /// Verify that a JSON null value falls back to an empty JsonObject.
    #[test]
    fn test_value_to_json_object_arc_with_null_falls_back_to_empty() {
        let arc = value_to_json_object_arc(serde_json::Value::Null);
        assert!(arc.is_empty());
    }

    /// Verify that a JSON boolean value falls back to an empty JsonObject.
    #[test]
    fn test_value_to_json_object_arc_with_bool_falls_back_to_empty() {
        let arc = value_to_json_object_arc(serde_json::json!(true));
        assert!(arc.is_empty());
    }

    /// Verify that a nested JSON object is preserved correctly inside the Arc.
    #[test]
    fn test_value_to_json_object_arc_nested_object() {
        let nested = serde_json::json!({
            "level1": {"level2": "value"}
        });
        let arc = value_to_json_object_arc(nested);
        assert_eq!(arc.len(), 1);
        let level1 = arc.get("level1").expect("level1 key should exist");
        assert!(level1.is_object());
        assert_eq!(level1["level2"], "value");
    }
}
