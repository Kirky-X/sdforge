// Copyright (c) 2026 Kirky.X
//! Core type definitions for the Axiom framework
//!
//! This module provides fundamental types used across the framework.

/// API metadata (protocol-agnostic)
///
/// Contains metadata about an API endpoint that is used across
/// HTTP, MCP, WebSocket, and gRPC protocols.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApiMetadata {
    /// API name
    pub(crate) name: String,
    /// API version
    pub(crate) version: String,
    /// API description
    pub(crate) description: String,
    /// Cache TTL in seconds (None means no caching)
    pub(crate) cache_ttl: Option<u64>,
    /// Whether this is a streaming endpoint
    pub(crate) is_streaming: bool,
}

impl ApiMetadata {
    /// Create new API metadata
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the API endpoint
    /// * `version` - The version string (e.g., "v1")
    /// * `description` - Human-readable description of the API
    /// * `cache_ttl` - Optional cache TTL in seconds (None means no caching)
    /// * `is_streaming` - Whether this is a streaming endpoint (SSE, WebSocket, etc.)
    pub fn new(
        name: String,
        version: String,
        description: String,
        cache_ttl: Option<u64>,
        is_streaming: bool,
    ) -> Self {
        Self {
            name,
            version,
            description,
            cache_ttl,
            is_streaming,
        }
    }

    /// Get API name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get API version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get API description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get cache TTL
    ///
    /// Returns the cache TTL in seconds, or None if caching is disabled.
    pub fn cache_ttl(&self) -> Option<u64> {
        self.cache_ttl
    }

    /// Check if this is a streaming endpoint
    pub fn is_streaming(&self) -> bool {
        self.is_streaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_metadata_new_and_accessors() {
        let metadata = ApiMetadata::new(
            "name".to_string(),
            "v1".to_string(),
            "desc".to_string(),
            Some(300),
            true,
        );

        assert_eq!(metadata.name(), "name");
        assert_eq!(metadata.version(), "v1");
        assert_eq!(metadata.description(), "desc");
        assert_eq!(metadata.cache_ttl(), Some(300));
        assert!(metadata.is_streaming());
    }

    #[test]
    fn test_api_metadata_default_values() {
        let metadata = ApiMetadata::default();
        assert_eq!(metadata.name(), "");
        assert_eq!(metadata.version(), "");
        assert_eq!(metadata.description(), "");
        assert_eq!(metadata.cache_ttl(), None);
        assert!(!metadata.is_streaming());
    }

    #[test]
    fn test_api_metadata_clone_and_eq() {
        let metadata = ApiMetadata::new(
            "clone".to_string(),
            "v2".to_string(),
            "desc".to_string(),
            None,
            false,
        );

        let cloned = metadata.clone();
        assert_eq!(metadata, cloned);
    }
}
