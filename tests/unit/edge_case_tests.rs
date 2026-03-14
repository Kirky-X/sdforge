// Edge Case Tests
// Covers TC-EDGE-001, TC-EDGE-002, TC-EDGE-005, TC-EDGE-006, TC-EDGE-007, TC-EDGE-009

#[cfg(test)]
mod edge_case_tests {
    use sdforge::core::{ApiError, ServiceResponse};

    #[test]
    fn test_empty_response() {
        let response: ServiceResponse<String> = ServiceResponse::new(String::new());
        assert_eq!(response.data(), "");
        assert!(response.is_success());
    }

    #[test]
    fn test_large_response() {
        let large_data = "x".repeat(1_000_000);
        let response = ServiceResponse::new(large_data);
        assert_eq!(response.data().len(), 1_000_000);
    }

    #[test]
    fn test_unicode_content() {
        let unicode_data = "Hello 世界 🎉 Ñoño Москва";
        let response = ServiceResponse::new(unicode_data.to_string());
        assert_eq!(response.data(), unicode_data);
    }

    #[test]
    fn test_special_characters() {
        let special = "Tab:\tNewline:\nQuote:\"Backslash:\\";
        let response = ServiceResponse::new(special.to_string());
        assert_eq!(response.data(), special);
    }

    #[test]
    fn test_error_with_all_fields() {
        let error = ApiError::with_details(
            "CUSTOM_ERROR",
            "Custom error message",
            Some(serde_json::json!({"field": "value"})),
        );
        
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("CUSTOM_ERROR"));
    }

    #[test]
    fn test_error_serialization_roundtrip() {
        let original = ApiError::not_found("User", Some("123"));
        let json = serde_json::to_string(&original).unwrap();
        let restored: ApiError = serde_json::from_str(&json).unwrap();
        
        match restored {
            ApiError::NotFound { resource, .. } => {
                assert_eq!(resource, "User");
            }
            _ => panic!("Roundtrip failed"),
        }
    }
}

#[cfg(test)]
mod parameter_boundary_tests {
    use sdforge::core::validation::{validate_string, validate_email, validate_url};

    #[test]
    fn test_max_string_length() {
        let max_string = "a".repeat(10000);
        let result = validate_string(&max_string, 1, 10000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_exceed_max_string_length() {
        let too_long = "a".repeat(10001);
        let result = validate_string(&too_long, 1, 10000);
        assert!(result.is_err());
    }

    #[test]
    fn test_min_string_length() {
        let result = validate_string("a", 1, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_below_min_string_length() {
        let result = validate_string("", 1, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_various_email_formats() {
        assert!(validate_email("user@domain.com").is_ok());
        assert!(validate_email("user.name@domain.com").is_ok());
        assert!(validate_email("user+tag@domain.co.uk").is_ok());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@domain.com").is_err());
    }

    #[test]
    fn test_various_url_formats() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://localhost:8080").is_ok());
        assert!(validate_url("https://example.com/path?query=1").is_ok());
        assert!(validate_url("not-a-url").is_err());
    }
}

#[cfg(test)]
mod concurrency_tests {
    use sdforge::core::ApiMetadata;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn test_concurrent_metadata_access() {
        let metadata = Arc::new(Mutex::new(ApiMetadata::default()));
        let mut handles = vec![];

        for _ in 0..10 {
            let m = Arc::clone(&metadata);
            handles.push(thread::spawn(move || {
                let _ = m.lock().unwrap().name();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_error_creation() {
        use sdforge::core::ApiError;
        
        let handles: Vec<_> = (0..100)
            .map(|_| {
                thread::spawn(|| {
                    let _ = ApiError::not_found("Test", None);
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_response_cloning_thread_safety() {
        use sdforge::core::ServiceResponse;
        
        let response = Arc::new(ServiceResponse::new("test".to_string()));
        let mut handles = vec![];

        for _ in 0..10 {
            let r = Arc::clone(&response);
            handles.push(thread::spawn(move || {
                let _ = r.clone();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }
}
