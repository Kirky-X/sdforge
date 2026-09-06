// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
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
    /// Optional i18n key for runtime translation of the description.
    ///
    /// When set, protocol consumption points (MCP `build_tool_model`,
    /// CLI `build_subcommand`; OpenAPI `build` and gRPC info are planned)
    /// look up a translation via `sdforge::i18n::translate_or_fallback`
    /// using the active locale. When no translation is found, the English
    /// `description` is used as fallback.
    pub(crate) i18n_key: Option<String>,
}

mod types_impl;

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
