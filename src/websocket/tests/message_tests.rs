// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use crate::websocket::message::*;

/// Test WebSocketMessage serialization and deserialization
#[test]
fn test_websocket_message_request() {
    let msg = WebSocketMessage::Request {
        id: "test-123".to_string(),
        method: "get_data".to_string(),
        params: serde_json::json!({"key": "value"}),
    };

    // Test serialization
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"request\""));
    assert!(json.contains("\"id\":\"test-123\""));
    assert!(json.contains("\"method\":\"get_data\""));

    // Test deserialization
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();

    assert!(
        matches!(
            decoded,
            WebSocketMessage::Request {
                ref id,
                ref method,
                ref params,
            } if id == "test-123" && method == "get_data" && params["key"] == "value"
        ),
        "Expected Request variant with correct values"
    );
}

/// Test WebSocketMessage Response variant
#[test]
fn test_websocket_message_response() {
    let msg = WebSocketMessage::Response {
        id: "resp-456".to_string(),
        result: serde_json::json!({"status": "ok"}),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"response\""));

    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();

    assert!(
        matches!(
            decoded,
            WebSocketMessage::Response { ref id, ref result }
                if id == "resp-456" && result["status"] == "ok"
        ),
        "Expected Response variant with correct values"
    );
}

/// Test WebSocketMessage Error variant
#[test]
fn test_websocket_message_error() {
    let msg = WebSocketMessage::Error {
        id: "err-789".to_string(),
        error: "Something went wrong".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"error\""));

    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();

    assert!(
        matches!(
            decoded,
            WebSocketMessage::Error { ref id, ref error }
                if id == "err-789" && error == "Something went wrong"
        ),
        "Expected Error variant with correct values"
    );
}

/// Test WebSocketMessage Notification variant
#[test]
fn test_websocket_message_notification() {
    let msg = WebSocketMessage::Notification {
        event: "user_joined".to_string(),
        data: serde_json::json!({"user": "alice"}),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"notification\""));

    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();

    assert!(
        matches!(
            decoded,
            WebSocketMessage::Notification { ref event, ref data }
                if event == "user_joined" && data["user"] == "alice"
        ),
        "Expected Notification variant with correct values"
    );
}

/// Test calculate_json_depth function
#[test]
fn test_calculate_json_depth_empty() {
    assert_eq!(calculate_json_depth(""), 0);
}

#[test]
fn test_calculate_json_depth_simple() {
    assert_eq!(calculate_json_depth("{}"), 1);
    assert_eq!(calculate_json_depth("[]"), 1);
}

#[test]
fn test_calculate_json_depth_nested() {
    // The function counts maximum nesting depth of braces/brackets
    // {"a":{"b":{"c":1}}} returns 3 as the max depth
    assert_eq!(calculate_json_depth(r#"{"a":{"b":{"c":1}}}"#), 3);
    // [{"a":[{"b":1}]}] starts with [ so returns 4
    assert_eq!(calculate_json_depth(r#"[{"a":[{"b":1}]}]"#), 4);
}

#[test]
fn test_calculate_json_depth_with_strings() {
    // Strings should not count toward depth
    assert_eq!(calculate_json_depth(r#"{"a":"{"}"}"#), 1);
}

#[test]
fn test_calculate_json_depth_array_nesting() {
    assert_eq!(calculate_json_depth("[[[[1]]]]"), 4);
}

/// Test parse_websocket_message with valid JSON
#[test]
fn test_parse_websocket_message_valid() {
    let valid_json = r#"{"type":"request","id":"123","method":"test","params":{}}"#;
    let result = parse_websocket_message(valid_json);
    assert!(result.is_ok());
    match result.unwrap() {
        WebSocketMessage::Request { id, method, .. } => {
            assert_eq!(id, "123");
            assert_eq!(method, "test");
        }
        _ => panic!("Expected Request"),
    }
}

/// Test parse_websocket_message with invalid JSON
#[test]
fn test_parse_websocket_message_invalid() {
    let invalid_json = "not valid json";
    let result = parse_websocket_message(invalid_json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid JSON"));
}

/// Test parse_websocket_message with too large message
#[test]
fn test_parse_websocket_message_too_large() {
    let large_json = "x".repeat(MAX_MESSAGE_SIZE + 1);
    let result = parse_websocket_message(&large_json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Message too large"));
}

/// Test parse_websocket_message with deeply nested JSON
#[test]
fn test_parse_websocket_message_too_deep() {
    // Create a valid deeply nested JSON structure
    let mut deep_json = String::from("0");
    for _ in 0..=MAX_JSON_DEPTH {
        deep_json = format!(r#"{{"a":{}}}"#, deep_json);
    }

    let result = parse_websocket_message(&deep_json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("nesting too deep"));
}

#[test]
fn test_websocket_message_request_empty_fields() {
    let msg = WebSocketMessage::Request {
        id: String::new(),
        method: String::new(),
        params: serde_json::json!(null),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        decoded,
        WebSocketMessage::Request { ref id, ref method, ref params }
            if id.is_empty() && method.is_empty() && params.is_null()
    ));
}

#[test]
fn test_websocket_message_request_unicode() {
    let msg = WebSocketMessage::Request {
        id: "日本語".to_string(),
        method: "方法".to_string(),
        params: serde_json::json!({"键": "值"}),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        decoded,
        WebSocketMessage::Request { ref id, ref method, .. }
            if id == "日本語" && method == "方法"
    ));
}

#[test]
fn test_websocket_message_request_large_params() {
    let large_array: Vec<i32> = (0..10000).collect();
    let msg = WebSocketMessage::Request {
        id: "large".to_string(),
        method: "test".to_string(),
        params: serde_json::json!({ "data": large_array }),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.len() > 40000);
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, WebSocketMessage::Request { .. }));
}

#[test]
fn test_websocket_message_response_empty_result() {
    let msg = WebSocketMessage::Response {
        id: "test".to_string(),
        result: serde_json::json!(null),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, WebSocketMessage::Response { .. }));
}

#[test]
fn test_websocket_message_response_nested_result() {
    let nested = serde_json::json!({
        "level1": {
            "level2": {
                "level3": "deep"
            }
        }
    });
    let msg = WebSocketMessage::Response {
        id: "nested".to_string(),
        result: nested.clone(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
    if let WebSocketMessage::Response { result, .. } = decoded {
        assert_eq!(result["level1"]["level2"]["level3"], "deep");
    } else {
        panic!("Expected Response");
    }
}

#[test]
fn test_websocket_message_error_empty_error() {
    let msg = WebSocketMessage::Error {
        id: String::new(),
        error: String::new(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, WebSocketMessage::Error { .. }));
}

#[test]
fn test_websocket_message_error_long_message() {
    let long_error = "x".repeat(10000);
    let msg = WebSocketMessage::Error {
        id: "err".to_string(),
        error: long_error.clone(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
    if let WebSocketMessage::Error { error, .. } = decoded {
        assert_eq!(error.len(), 10000);
    } else {
        panic!("Expected Error");
    }
}

#[test]
fn test_websocket_message_notification_empty_event() {
    let msg = WebSocketMessage::Notification {
        event: String::new(),
        data: serde_json::json!({}),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, WebSocketMessage::Notification { .. }));
}

#[test]
fn test_websocket_message_notification_array_data() {
    let msg = WebSocketMessage::Notification {
        event: "list_update".to_string(),
        data: serde_json::json!([1, 2, 3, 4, 5]),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
    if let WebSocketMessage::Notification { data, .. } = decoded {
        assert_eq!(data.as_array().unwrap().len(), 5);
    } else {
        panic!("Expected Notification");
    }
}

#[test]
fn test_websocket_message_deserialize_missing_type() {
    let json = r#"{"id":"123","method":"test","params":{}}"#;
    let result: Result<WebSocketMessage, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_websocket_message_deserialize_invalid_type() {
    let json = r#"{"type":"invalid","id":"123"}"#;
    let result: Result<WebSocketMessage, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_websocket_message_deserialize_request_missing_field() {
    let json = r#"{"type":"request","id":"123"}"#;
    let result: Result<WebSocketMessage, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn parse_websocket_message_empty_string() {
    let result = parse_websocket_message("");
    assert!(result.is_err());
}

#[test]
fn parse_websocket_message_whitespace() {
    let result = parse_websocket_message("   ");
    assert!(result.is_err());
}

#[test]
fn parse_websocket_message_response_variant() {
    let json = r#"{"type":"response","id":"resp-1","result":{"status":"success"}}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_ok());
    if let WebSocketMessage::Response { id, result } = result.unwrap() {
        assert_eq!(id, "resp-1");
        assert_eq!(result["status"], "success");
    } else {
        panic!("Expected Response");
    }
}

#[test]
fn parse_websocket_message_error_variant() {
    let json = r#"{"type":"error","id":"err-1","error":"Something failed"}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_ok());
    if let WebSocketMessage::Error { id, error } = result.unwrap() {
        assert_eq!(id, "err-1");
        assert_eq!(error, "Something failed");
    } else {
        panic!("Expected Error");
    }
}

#[test]
fn parse_websocket_message_notification_variant() {
    let json = r#"{"type":"notification","event":"update","data":{"value":42}}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_ok());
    if let WebSocketMessage::Notification { event, data } = result.unwrap() {
        assert_eq!(event, "update");
        assert_eq!(data["value"], 42);
    } else {
        panic!("Expected Notification");
    }
}

#[test]
fn calculate_value_depth_primitive() {
    let value = serde_json::json!(42);
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 0);
}

#[test]
fn calculate_value_depth_string() {
    let value = serde_json::json!("hello");
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 0);
}

#[test]
fn calculate_value_depth_simple_object() {
    let value = serde_json::json!({"a": 1});
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 1);
}

#[test]
fn calculate_value_depth_nested_object() {
    let value = serde_json::json!({"a": {"b": {"c": 1}}});
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 3);
}

#[test]
fn calculate_value_depth_simple_array() {
    let value = serde_json::json!([1, 2, 3]);
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 1);
}

#[test]
fn calculate_value_depth_nested_array() {
    let value = serde_json::json!([[[1, 2], [3, 4]], [[5, 6]]]);
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 3);
}

#[test]
fn calculate_value_depth_mixed() {
    let value = serde_json::json!({
        "users": [
            {"name": "Alice", "tags": ["a", "b"]},
            {"name": "Bob", "tags": ["c"]}
        ]
    });
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 4);
}

/// Test calculate_value_depth with empty object
#[test]
fn calculate_value_depth_empty_object() {
    let value = serde_json::json!({});
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 0);
}

/// Test calculate_value_depth with empty array
#[test]
fn calculate_value_depth_empty_array() {
    let value = serde_json::json!([]);
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 0);
}

/// Test calculate_value_depth with boolean
#[test]
fn calculate_value_depth_boolean() {
    let value_true = serde_json::json!(true);
    let value_false = serde_json::json!(false);
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value_true, &mut depth), 0);
    depth = 0;
    assert_eq!(calculate_value_depth(&value_false, &mut depth), 0);
}

/// Test calculate_value_depth with null
#[test]
fn calculate_value_depth_null() {
    let value = serde_json::json!(null);
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 0);
}

/// Test calculate_value_depth with float
#[test]
fn calculate_value_depth_float() {
    let value = serde_json::json!(std::f64::consts::PI);
    let mut depth = 0;
    assert_eq!(calculate_value_depth(&value, &mut depth), 0);
}

/// Test calculate_value_depth with deeply nested mixed structure
#[test]
fn calculate_value_depth_deep_mixed() {
    let value = serde_json::json!({
        "level1": [
            {"level2a": {"level3": "value"}},
            [{"level2b": {"level3": [1, 2, {"level4": "deep"}]}}]
        ]
    });
    let mut depth = 0;
    let result = calculate_value_depth(&value, &mut depth);
    assert!(result >= 4, "Expected depth >= 4, got {}", result);
}

/// Test parse_websocket_message with unknown type
#[test]
fn parse_websocket_message_unknown_type() {
    let json = r#"{"type":"unknown_type","data":{}}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_err());
}

/// Test parse_websocket_message with malformed JSON (trailing comma)
#[test]
fn parse_websocket_message_malformed_trailing_comma() {
    let json = r#"{"type":"request","id":"123",}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_err());
}

/// Test parse_websocket_message with incomplete JSON
#[test]
fn parse_websocket_message_incomplete_json() {
    let json = r#"{"type":"request","id":"123""#;
    let result = parse_websocket_message(json);
    assert!(result.is_err());
}

/// Test parse_websocket_message with array at top level
#[test]
fn parse_websocket_message_array_top_level() {
    let json = r#"[1, 2, 3]"#;
    let result = parse_websocket_message(json);
    assert!(result.is_err());
}

/// Test parse_websocket_message with string at top level
#[test]
fn parse_websocket_message_string_top_level() {
    let json = r#""just a string""#;
    let result = parse_websocket_message(json);
    assert!(result.is_err());
}

/// Test parse_websocket_message with number at top level
#[test]
fn parse_websocket_message_number_top_level() {
    let json = r#"42"#;
    let result = parse_websocket_message(json);
    assert!(result.is_err());
}

/// Test parse_websocket_message with boolean at top level
#[test]
fn parse_websocket_message_bool_top_level() {
    let json = r#"true"#;
    let result = parse_websocket_message(json);
    assert!(result.is_err());
}

/// Test parse_websocket_message with null at top level
#[test]
fn parse_websocket_message_null_top_level() {
    let json = r#"null"#;
    let result = parse_websocket_message(json);
    assert!(result.is_err());
}

/// Test parse_websocket_message with minimal valid request
#[test]
fn parse_websocket_message_minimal_request() {
    let json = r#"{"type":"request","id":"","method":"","params":{}}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_ok());
    match result.unwrap() {
        WebSocketMessage::Request { id, method, params } => {
            assert!(id.is_empty());
            assert!(method.is_empty());
            assert!(params.is_object());
        }
        _ => panic!("Expected Request"),
    }
}

/// Test parse_websocket_message with minimal valid notification
#[test]
fn parse_websocket_message_minimal_notification() {
    let json = r#"{"type":"notification","event":"","data":null}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_ok());
    match result.unwrap() {
        WebSocketMessage::Notification { event, data } => {
            assert!(event.is_empty());
            assert!(data.is_null());
        }
        _ => panic!("Expected Notification"),
    }
}

/// Test parse_websocket_message with nested arrays in params
#[test]
fn parse_websocket_message_nested_arrays() {
    let json = r#"{"type":"request","id":"arr","method":"test","params":{"matrix":[[1,2],[3,4]]}}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_ok());
    if let WebSocketMessage::Request { params, .. } = result.unwrap() {
        assert!(params["matrix"].is_array());
        assert_eq!(params["matrix"][0][0], 1);
        assert_eq!(params["matrix"][1][1], 4);
    }
}

/// Test parse_websocket_message with empty object params
#[test]
fn parse_websocket_message_empty_object_params() {
    let json = r#"{"type":"request","id":"1","method":"test","params":{}}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_ok());
    match result.unwrap() {
        WebSocketMessage::Request { params, .. } => {
            assert!(params.is_object());
            assert_eq!(params.as_object().unwrap().len(), 0);
        }
        _ => panic!("Expected Request"),
    }
}

/// Test parse_websocket_message with empty array in result
#[test]
fn parse_websocket_message_empty_array_result() {
    let json = r#"{"type":"response","id":"1","result":[]}"#;
    let result = parse_websocket_message(json);
    assert!(result.is_ok());
    match result.unwrap() {
        WebSocketMessage::Response { result, .. } => {
            assert!(result.is_array());
            assert_eq!(result.as_array().unwrap().len(), 0);
        }
        _ => panic!("Expected Response"),
    }
}

/// Test parse_websocket_message roundtrip for all variants
#[test]
fn parse_websocket_message_roundtrip_request() {
    let msg = WebSocketMessage::Request {
        id: "roundtrip".to_string(),
        method: "test_method".to_string(),
        params: serde_json::json!({"key": "value"}),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed = parse_websocket_message(&json).unwrap();
    match parsed {
        WebSocketMessage::Request { id, method, params } => {
            assert_eq!(id, "roundtrip");
            assert_eq!(method, "test_method");
            assert_eq!(params["key"], "value");
        }
        _ => panic!("Expected Request"),
    }
}

#[test]
fn parse_websocket_message_roundtrip_response() {
    let msg = WebSocketMessage::Response {
        id: "roundtrip".to_string(),
        result: serde_json::json!({"status": "success"}),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed = parse_websocket_message(&json).unwrap();
    match parsed {
        WebSocketMessage::Response { id, result } => {
            assert_eq!(id, "roundtrip");
            assert_eq!(result["status"], "success");
        }
        _ => panic!("Expected Response"),
    }
}

#[test]
fn parse_websocket_message_roundtrip_error() {
    let msg = WebSocketMessage::Error {
        id: "roundtrip".to_string(),
        error: "test error".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed = parse_websocket_message(&json).unwrap();
    match parsed {
        WebSocketMessage::Error { id, error } => {
            assert_eq!(id, "roundtrip");
            assert_eq!(error, "test error");
        }
        _ => panic!("Expected Error"),
    }
}

#[test]
fn parse_websocket_message_roundtrip_notification() {
    let msg = WebSocketMessage::Notification {
        event: "roundtrip".to_string(),
        data: serde_json::json!({"event_data": "test"}),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed = parse_websocket_message(&json).unwrap();
    match parsed {
        WebSocketMessage::Notification { event, data } => {
            assert_eq!(event, "roundtrip");
            assert_eq!(data["event_data"], "test");
        }
        _ => panic!("Expected Notification"),
    }
}

/// Test calculate_json_depth with escaped quotes
#[test]
fn calculate_json_depth_escaped_quotes() {
    assert_eq!(calculate_json_depth(r#"{"a":""nested""}"#), 1);
}

/// Test calculate_json_depth with braces in strings
#[test]
fn calculate_json_depth_braces_in_strings() {
    assert_eq!(calculate_json_depth(r#"{"a":"{not depth}"}"#), 1);
}

/// Test calculate_json_depth with mixed nesting
#[test]
fn calculate_json_depth_mixed_nesting() {
    assert_eq!(calculate_json_depth(r#"{"a":[1,{"b":2}]}"#), 3);
}

/// Test calculate_json_depth with whitespace
#[test]
fn calculate_json_depth_with_whitespace() {
    let json = r#"{ "a": { "b": 1 } }"#;
    assert_eq!(calculate_json_depth(json), 2);
}

/// Test WebSocketMessage Clone derive
#[test]
fn websocket_message_clone() {
    let msg = WebSocketMessage::Request {
        id: "clone".to_string(),
        method: "test".to_string(),
        params: serde_json::json!({"key": "value"}),
    };
    let cloned = msg.clone();
    let json1 = serde_json::to_string(&msg).unwrap();
    let json2 = serde_json::to_string(&cloned).unwrap();
    assert_eq!(json1, json2);
}

/// Test WebSocketMessage Debug derive
#[test]
fn websocket_message_debug() {
    let msg = WebSocketMessage::Request {
        id: "debug".to_string(),
        method: "test".to_string(),
        params: serde_json::json!({}),
    };
    let debug_str = format!("{:?}", msg);
    assert!(debug_str.contains("Request"));
    assert!(debug_str.contains("debug"));
}

/// Test parse_websocket_message with large valid params
#[test]
fn parse_websocket_message_large_valid_params() {
    let large_params = serde_json::json!({
        "array": (0..1000).collect::<Vec<_>>(),
        "nested": {"deep": {"value": "test"}}
    });
    let msg = WebSocketMessage::Request {
        id: "large-params".to_string(),
        method: "test".to_string(),
        params: large_params,
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.len() < MAX_MESSAGE_SIZE);
    let result = parse_websocket_message(&json);
    assert!(result.is_ok());
}

/// Test Constants
#[test]
fn max_message_size_constant() {
    assert_eq!(MAX_MESSAGE_SIZE, 1_048_576);
}

#[test]
fn max_json_depth_constant() {
    assert_eq!(MAX_JSON_DEPTH, 16);
}

#[test]
fn max_string_length_constant() {
    assert_eq!(MAX_STRING_LENGTH, 64 * 1024);
}
