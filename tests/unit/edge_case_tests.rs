// Edge Case Tests
// Covers TC-EDGE-001, TC-EDGE-002, TC-EDGE-005, TC-EDGE-006, TC-EDGE-007, TC-EDGE-009

#[cfg(test)]
mod edge_case_tests {
    use sdforge::core::{ApiError, ServiceResponse};

    #[test]
    fn test_empty_response() {
        let response: ServiceResponse<String> = ServiceResponse::success(String::new());
        assert_eq!(response.data(), Some(&String::new()));
        assert!(response.is_success());
    }

    #[test]
    fn test_large_response() {
        let large_data = "x".repeat(1_000_000);
        let response = ServiceResponse::success(large_data);
        assert_eq!(response.data().map(|s| s.len()), Some(1_000_000));
    }

    #[test]
    fn test_unicode_content() {
        let unicode_data = "Hello 世界 🎉 Ñoño Москва";
        let response = ServiceResponse::success(unicode_data.to_string());
        assert_eq!(response.data(), Some(&unicode_data.to_string()));
    }

    #[test]
    fn test_special_characters() {
        let special = "Tab:\tNewline:\nQuote:\"Backslash:\\";
        let response = ServiceResponse::success(special.to_string());
        assert_eq!(response.data(), Some(&special.to_string()));
    }

    #[test]
    fn test_error_serialization_roundtrip() {
        let original = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".to_string()),
        };
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
                    let _ = ApiError::NotFound {
                        resource: "Test".to_string(),
                        resource_id: None,
                    };
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

        let response: ServiceResponse<String> = ServiceResponse::success("test".to_string());
        let arc_response: Arc<ServiceResponse<String>> = Arc::new(response);
        let mut handles = vec![];

        for _ in 0..10 {
            let r = Arc::clone(&arc_response);
            handles.push(thread::spawn(move || {
                let _ = r.clone();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }
}
