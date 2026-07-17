// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Lightweight JSON Schema validation for MCP tool arguments (vuln-0002).
//!
//! This module provides a minimal, dependency-free JSON Schema validator that
//! covers the security-critical keywords most relevant to MCP tool argument
//! validation:
//!
//! - `type`: top-level type check (object/string/number/integer/boolean/array/null)
//! - `required`: required property presence check
//! - `properties` + `additionalProperties: false`: unknown field rejection
//!
//! # Why not use the `jsonschema` crate?
//!
//! The `jsonschema` crate is a full-featured validator but pulls in heavy
//! dependencies (`fraction`, `num-bigint`, `fancy-regex`, etc.) that are
//! overkill for MCP tool argument validation. Most MCP tools use simple
//! schemas (`type: object` + `properties` + `required`), so a lightweight
//! validator covering these keywords provides 90%+ of the security benefit
//! at zero dependency cost.
//!
//! Tools that need full JSON Schema validation (e.g. complex `format`,
//! `pattern`, `minimum`/`maximum` constraints) should validate in their
//! `call()` implementation via `serde` deserialization (which macros-generated
//! tools already do via `#[serde(deny_unknown_fields)]`).
//!
//! # vuln-0002 context
//!
//! Before this validator, `call_tool_internal` only checked payload size and
//! tool existence. The `input_schema` was used solely for documentation
//! (building the `Tool` model for `tools/list`). This meant hand-written
//! `SdForgeTool` implementations that forgot to validate arguments in their
//! `call()` method would accept arbitrary input. This validator closes that
//! gap by checking the schema at the entry point, before the tool is invoked.

use rmcp::model::ErrorData;
use serde_json::Value;

/// Validate a JSON instance against a lightweight JSON Schema.
///
/// Returns `Ok(())` if the instance is valid, or `Err(ErrorData)` with
/// `invalid_params` if validation fails.
///
/// # Supported keywords
///
/// - `type`: single type or array of types
/// - `required`: list of required property names (only checked when instance is an object)
/// - `properties` + `additionalProperties: false`: reject unknown properties
///
/// # Unsupported keywords (intentionally)
///
/// - `format`, `pattern`: too complex for a lightweight validator
/// - `minimum`, `maximum`, `minLength`, `maxLength`: handled by serde deserialization
/// - `$ref`, `$defs`, `allOf`, `oneOf`, `anyOf`: complex composition
///
/// These are deferred to the tool's `call()` implementation (serde does this
/// automatically for macros-generated tools).
pub(crate) fn validate_arguments(schema: &Value, arguments: &Value) -> Result<(), ErrorData> {
    // Schema must be an object; if not, skip validation (fail-open for
    // malformed schemas — the tool's call() is the final authority).
    let schema_obj = match schema.as_object() {
        Some(obj) => obj,
        None => return Ok(()),
    };

    // 1. Type check
    if let Some(type_value) = schema_obj.get("type")
        && !check_type(type_value, arguments)
    {
        return Err(ErrorData::invalid_params(
            format!(
                "arguments type mismatch: schema requires type {}, got {}",
                type_value,
                json_type_name(arguments)
            ),
            None,
        ));
    }

    // 2. Required fields check (only when arguments is an object)
    if let Some(required) = schema_obj.get("required").and_then(|v| v.as_array())
        && let Some(args_obj) = arguments.as_object()
    {
        for req in required {
            if let Some(field) = req.as_str()
                && !args_obj.contains_key(field)
            {
                return Err(ErrorData::invalid_params(
                    format!(
                        "missing required field: '{}' (required by tool input_schema)",
                        field
                    ),
                    None,
                ));
            }
        }
    }

    // 3. additionalProperties: false → reject unknown fields
    //
    // Per JSON Schema spec, `additionalProperties: false` means only properties
    // listed in `properties` are allowed. If `properties` is absent, NO properties
    // are allowed (only an empty object `{}` is valid).
    if let Some(additional) = schema_obj.get("additionalProperties") {
        // Only check when additionalProperties is explicitly false
        if additional.as_bool() == Some(false) {
            let properties = schema_obj.get("properties").and_then(|v| v.as_object());
            if let Some(args_obj) = arguments.as_object() {
                for key in args_obj.keys() {
                    // If properties is absent, no key is allowed.
                    let allowed = properties.map(|p| p.contains_key(key)).unwrap_or(false);
                    if !allowed {
                        return Err(ErrorData::invalid_params(
                            format!(
                                "unknown field: '{}' (tool input_schema has additionalProperties: false)",
                                key
                            ),
                            None,
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check whether a JSON value matches a JSON Schema `type` specifier.
///
/// `type_value` can be a single type string (`"object"`) or an array of
/// type strings (`["string", "null"]`).
fn check_type(type_value: &Value, instance: &Value) -> bool {
    match type_value {
        Value::String(t) => matches_json_type(t, instance),
        Value::Array(types) => types.iter().any(|t| {
            t.as_str()
                .map(|s| matches_json_type(s, instance))
                .unwrap_or(false)
        }),
        // Malformed type — fail-open (let the tool's call() handle it)
        _ => true,
    }
}

/// Check whether a JSON value matches a single JSON Schema type name.
fn matches_json_type(type_name: &str, instance: &Value) -> bool {
    match type_name {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "integer" => instance.is_i64() || instance.is_u64(),
        "number" => instance.is_number(),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        // Unknown type name — fail-open
        _ => true,
    }
}

/// Human-readable JSON type name for error messages.
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => {
            if value.is_i64() || value.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ============================================================================
// Unit tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- Type validation tests ---

    #[test]
    fn test_type_object_matches_object() {
        let schema = json!({"type": "object"});
        let args = json!({"key": "value"});
        assert!(validate_arguments(&schema, &args).is_ok());
    }

    #[test]
    fn test_type_object_rejects_string() {
        let schema = json!({"type": "object"});
        let args = json!("not an object");
        let err = validate_arguments(&schema, &args).unwrap_err();
        assert!(err.message.contains("type mismatch"));
    }

    #[test]
    fn test_type_string_matches_string() {
        let schema = json!({"type": "string"});
        assert!(validate_arguments(&schema, &json!("hello")).is_ok());
    }

    #[test]
    fn test_type_integer_matches_integer() {
        let schema = json!({"type": "integer"});
        assert!(validate_arguments(&schema, &json!(42)).is_ok());
    }

    #[test]
    fn test_type_integer_rejects_float() {
        let schema = json!({"type": "integer"});
        assert!(validate_arguments(&schema, &json!(42.5)).is_err());
    }

    #[test]
    fn test_type_number_matches_float() {
        let schema = json!({"type": "number"});
        assert!(validate_arguments(&schema, &json!(42.5)).is_ok());
    }

    #[test]
    fn test_type_array_matches_array() {
        let schema = json!({"type": "array"});
        assert!(validate_arguments(&schema, &json!([1, 2, 3])).is_ok());
    }

    #[test]
    fn test_type_null_matches_null() {
        let schema = json!({"type": "null"});
        assert!(validate_arguments(&schema, &json!(null)).is_ok());
    }

    #[test]
    fn test_type_boolean_matches_boolean() {
        let schema = json!({"type": "boolean"});
        assert!(validate_arguments(&schema, &json!(true)).is_ok());
    }

    #[test]
    fn test_type_array_union() {
        // type: ["string", "null"] — accepts either
        let schema = json!({"type": ["string", "null"]});
        assert!(validate_arguments(&schema, &json!("hello")).is_ok());
        assert!(validate_arguments(&schema, &json!(null)).is_ok());
        assert!(validate_arguments(&schema, &json!(42)).is_err());
    }

    // --- Required fields tests ---

    #[test]
    fn test_required_all_present() {
        let schema = json!({
            "type": "object",
            "required": ["name", "age"]
        });
        let args = json!({"name": "Alice", "age": 30});
        assert!(validate_arguments(&schema, &args).is_ok());
    }

    #[test]
    fn test_required_missing_field() {
        let schema = json!({
            "type": "object",
            "required": ["name", "age"]
        });
        let args = json!({"name": "Alice"});
        let err = validate_arguments(&schema, &args).unwrap_err();
        assert!(err.message.contains("missing required field: 'age'"));
    }

    #[test]
    fn test_required_not_checked_for_non_object() {
        // required only applies to objects; if args is not an object, skip
        let schema = json!({
            "type": "string",
            "required": ["name"]
        });
        let args = json!("hello");
        assert!(validate_arguments(&schema, &args).is_ok());
    }

    // --- additionalProperties tests ---

    #[test]
    fn test_additional_properties_false_rejects_unknown() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "additionalProperties": false
        });
        let args = json!({"name": "Alice", "age": 30, "extra": "bad"});
        let err = validate_arguments(&schema, &args).unwrap_err();
        assert!(err.message.contains("unknown field: 'extra'"));
    }

    #[test]
    fn test_additional_properties_false_allows_known() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "additionalProperties": false
        });
        let args = json!({"name": "Alice"});
        assert!(validate_arguments(&schema, &args).is_ok());
    }

    #[test]
    fn test_additional_properties_true_allows_unknown() {
        // additionalProperties: true (or absent) → allow unknown fields
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "additionalProperties": true
        });
        let args = json!({"name": "Alice", "extra": "ok"});
        assert!(validate_arguments(&schema, &args).is_ok());
    }

    #[test]
    fn test_additional_properties_absent_allows_unknown() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let args = json!({"name": "Alice", "extra": "ok"});
        assert!(validate_arguments(&schema, &args).is_ok());
    }

    // --- Edge cases ---

    #[test]
    fn test_empty_schema_allows_anything() {
        let schema = json!({});
        assert!(validate_arguments(&schema, &json!("string")).is_ok());
        assert!(validate_arguments(&schema, &json!(42)).is_ok());
        assert!(validate_arguments(&schema, &json!({"key": "value"})).is_ok());
    }

    #[test]
    fn test_non_object_schema_skips_validation() {
        // Malformed schema (not an object) → fail-open
        let schema = json!("not a schema");
        assert!(validate_arguments(&schema, &json!("anything")).is_ok());
    }

    #[test]
    fn test_combined_type_required_additional() {
        // All three checks together
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "additionalProperties": false
        });
        // Valid
        assert!(validate_arguments(&schema, &json!({"name": "Alice", "age": 30})).is_ok());
        // Missing required
        assert!(validate_arguments(&schema, &json!({"age": 30})).is_err());
        // Unknown field
        assert!(validate_arguments(&schema, &json!({"name": "Alice", "extra": "bad"})).is_err());
        // Wrong type
        assert!(validate_arguments(&schema, &json!("not an object")).is_err());
    }

    #[test]
    fn test_null_arguments_with_object_schema() {
        // null does not match type: object
        let schema = json!({"type": "object"});
        assert!(validate_arguments(&schema, &json!(null)).is_err());
    }

    #[test]
    fn test_null_arguments_with_null_schema() {
        let schema = json!({"type": "null"});
        assert!(validate_arguments(&schema, &json!(null)).is_ok());
    }

    // --- additionalProperties: false without properties (security fix) ---

    #[test]
    fn test_additional_properties_false_without_properties_rejects_non_empty() {
        // Per JSON Schema spec: additionalProperties: false + no properties
        // means NO properties are allowed (only empty object is valid).
        let schema = json!({
            "type": "object",
            "additionalProperties": false
        });
        // Non-empty object should be rejected
        let err = validate_arguments(&schema, &json!({"any_field": "value"})).unwrap_err();
        assert!(err.message.contains("unknown field: 'any_field'"));
    }

    #[test]
    fn test_additional_properties_false_without_properties_allows_empty() {
        // Empty object should be allowed (no fields to reject)
        let schema = json!({
            "type": "object",
            "additionalProperties": false
        });
        assert!(validate_arguments(&schema, &json!({})).is_ok());
    }
}
