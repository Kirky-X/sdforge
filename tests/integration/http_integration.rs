// HTTP Integration Tests
// Covers TC-INT-001, TC-INT-003, TC-INT-004, TC-INT-005, TC-INT-006

#[cfg(feature = "http")]
mod http_tests {
    use sdforge::http::build;
    use sdforge::core::{ApiError, ServiceResponse};

    #[tokio::test]
    async fn test_http_server_builds() {
        let app = build();
        assert!(app.is_ok(), "HTTP service should build successfully");
    }

    #[test]
    fn test_http_build_sync() {
        let app = build();
        assert!(app.is_ok());
    }
}

#[cfg(all(feature = "http", feature = "timestamp"))]
mod timestamp_tests {
    use sdforge::http::build;

    #[test]
    fn test_timestamp_feature_enabled() {
        let app = build();
        assert!(app.is_ok());
    }
}

#[cfg(all(feature = "http", not(feature = "timestamp")))]
mod no_timestamp_tests {
    use sdforge::http::build;

    #[test]
    fn test_timestamp_feature_disabled() {
        let app = build();
        assert!(app.is_ok());
    }
}

#[cfg(all(feature = "http", feature = "streaming"))]
mod streaming_tests {
    use sdforge::http::build;

    #[test]
    fn test_streaming_feature_enabled() {
        let app = build();
        assert!(app.is_ok());
    }
}
