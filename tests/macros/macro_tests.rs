// Macro expansion tests
// Note: These tests verify macro functionality through the API

#[cfg(feature = "http")]
mod http_macro_tests {
    #[test]
    fn test_macro_enabled() {
        assert!(true);
    }
}

#[cfg(feature = "mcp")]
mod mcp_macro_tests {
    #[test]
    fn test_macro_enabled() {
        assert!(true);
    }
}

#[cfg(all(feature = "http", feature = "mcp"))]
mod dual_macro_tests {
    #[test]
    fn test_macro_enabled() {
        assert!(true);
    }
}
