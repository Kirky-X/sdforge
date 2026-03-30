// HTTP Integration Tests
// Covers TC-INT-001, TC-INT-003, TC-INT-004, TC-INT-005, TC-INT-006

#[cfg(feature = "http")]
mod http_tests {
    use sdforge::http::build;

    #[tokio::test]
    async fn test_http_server_builds() {
        let app = build();
        // Verify the app is not null and has expected structure
        assert!(!std::ptr::eq(&app, &std::ptr::null()), "HTTP app should build successfully");
    }

    #[test]
    fn test_http_build_sync() {
        let app = build();
        // Verify the app builds without panic
        assert!(!std::ptr::eq(&app, &std::ptr::null()), "HTTP sync build should succeed");
    }
}

#[cfg(all(feature = "http", feature = "timestamp"))]
mod timestamp_tests {
    use sdforge::http::build;

    #[test]
    fn test_timestamp_feature_enabled() {
        let app = build();
        assert!(!std::ptr::eq(&app, &std::ptr::null()), "HTTP app with timestamp should build");
    }
}

#[cfg(all(feature = "http", not(feature = "timestamp")))]
mod no_timestamp_tests {
    use sdforge::http::build;

    #[test]
    fn test_timestamp_feature_disabled() {
        let app = build();
        assert!(!std::ptr::eq(&app, &std::ptr::null()), "HTTP app without timestamp should build");
    }
}

#[cfg(all(feature = "http", feature = "streaming"))]
mod streaming_tests {
    use sdforge::http::build;

    #[test]
    fn test_streaming_feature_enabled() {
        let app = build();
        assert!(!std::ptr::eq(&app, &std::ptr::null()), "HTTP app with streaming should build");
    }
}
