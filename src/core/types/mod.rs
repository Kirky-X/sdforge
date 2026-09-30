// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Core type definitions for the Axiom framework
//!
//! This module provides fundamental types used across the framework.

/// Endpoint lifecycle declaration（`#[forge(deprecated, sunset, successor)]`）。
///
/// Protocol-agnostic: carried on [`ApiMetadata`] and consumed per protocol —
/// HTTP response headers (`Deprecation` / `Sunset` / `Link:
/// successor-version`), gRPC response metadata (same keys), MCP tool
/// description annotation, OpenAPI `deprecated` marker.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LifecycleMeta {
    /// Marks the endpoint as deprecated.
    pub deprecated: bool,
    /// Sunset date/value surfaced verbatim (e.g. `"2026-12-31"`).
    pub sunset: Option<String>,
    /// Successor endpoint hint (path or name) surfaced via the
    /// `successor-version` link.
    pub successor: Option<String>,
}

impl LifecycleMeta {
    /// Whether any lifecycle information is present (header injection is
    /// skipped entirely when `false`).
    pub fn is_present(&self) -> bool {
        self.deprecated || self.sunset.is_some() || self.successor.is_some()
    }

    /// Render the declared lifecycle parts as a semicolon-joined annotation,
    /// e.g. `deprecated; sunset: 2026-12-31; successor: /api/v2/thing` —
    /// the shared footnote format for protocol surfaces that carry text
    /// descriptions only (MCP tool description, OpenAPI operation
    /// description). Empty string when nothing is declared.
    pub fn annotation(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.deprecated {
            parts.push("deprecated".to_string());
        }
        if let Some(sunset) = &self.sunset {
            parts.push(format!("sunset: {sunset}"));
        }
        if let Some(successor) = &self.successor {
            parts.push(format!("successor: {successor}"));
        }
        parts.join("; ")
    }
}

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
    /// Optional endpoint lifecycle declaration (deprecated/sunset/successor).
    pub(crate) lifecycle: Option<LifecycleMeta>,
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

    /// `annotation()` 只渲染已声明部分、分号连接，与 MCP/OpenAPI 描述
    /// 尾注共用同一格式；全空时为空串（配合 `is_present` 不产出空括注）。
    #[test]
    fn test_lifecycle_annotation_renders_declared_parts_only() {
        let full = LifecycleMeta {
            deprecated: true,
            sunset: Some("2026-12-31".to_string()),
            successor: Some("/api/v2/thing".to_string()),
        };
        assert_eq!(
            full.annotation(),
            "deprecated; sunset: 2026-12-31; successor: /api/v2/thing"
        );

        let sunset_only = LifecycleMeta {
            deprecated: false,
            sunset: Some("2027-01-01".to_string()),
            successor: None,
        };
        assert_eq!(sunset_only.annotation(), "sunset: 2027-01-01");

        let empty = LifecycleMeta::default();
        assert_eq!(empty.annotation(), "");
    }
}
