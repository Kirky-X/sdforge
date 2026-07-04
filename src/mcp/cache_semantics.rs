// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! MCP 2026-07-28 cache semantics.
//!
//! The 2026-07-28 MCP protocol introduces cache control fields for tool
//! results:
//!
//! - `ttlMs`: Time-to-live in milliseconds for the cached result
//! - `cacheScope`: Whether to cache globally or per-request
//!
//! This module provides utilities to apply these cache semantics to
//! `CallToolResult` values.

use rmcp::model::CallToolResult;
use serde_json::Value;
use std::time::Duration;

/// Cache scope for tool results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheScope {
    /// Cache globally across all requests
    Global,
    /// Cache only for the current request
    #[default]
    Request,
    /// Do not cache
    None,
}

impl CacheScope {
    /// Parse a cache scope from a string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "global" => Self::Global,
            "request" => Self::Request,
            _ => Self::None,
        }
    }

    /// Returns true if this scope should be cached.
    pub fn should_cache(&self) -> bool {
        matches!(self, Self::Global | Self::Request)
    }
}

impl std::fmt::Display for CacheScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Request => write!(f, "request"),
            Self::None => write!(f, "none"),
        }
    }
}

/// Cache metadata extracted from a tool result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheMetadata {
    /// Time-to-live in milliseconds
    pub ttl_ms: u64,
    /// Cache scope
    pub scope: CacheScope,
}

impl CacheMetadata {
    /// Create new cache metadata.
    pub fn new(ttl_ms: u64, scope: CacheScope) -> Self {
        Self { ttl_ms, scope }
    }

    /// Get the TTL as a Duration.
    pub fn ttl_duration(&self) -> Duration {
        Duration::from_millis(self.ttl_ms)
    }

    /// Returns true if this result should be cached.
    pub fn should_cache(&self) -> bool {
        self.scope.should_cache() && self.ttl_ms > 0
    }

    /// Parse cache metadata from a JSON value (the structured_content field).
    ///
    /// Expected format:
    /// ```json
    /// {
    ///   "ttlMs": 300000,
    ///   "cacheScope": "global"
    /// }
    /// ```
    pub fn from_json(json: &Value) -> Option<Self> {
        let ttl_ms = json.get("ttlMs")?.as_u64()?;
        let scope = json
            .get("cacheScope")
            .and_then(|v| v.as_str())
            .map(CacheScope::parse)
            .unwrap_or_default();
        Some(Self { ttl_ms, scope })
    }

    /// Convert to JSON for storage in structured_content.
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "ttlMs": self.ttl_ms,
            "cacheScope": self.scope.to_string()
        })
    }
}

impl Default for CacheMetadata {
    fn default() -> Self {
        Self {
            ttl_ms: 0,
            scope: CacheScope::Request,
        }
    }
}

/// Extract cache metadata from a `CallToolResult`.
///
/// This checks the `structured_content` field for `ttlMs` and `cacheScope`.
/// Returns `None` if no cache metadata is present.
pub fn extract_cache_metadata(result: &CallToolResult) -> Option<CacheMetadata> {
    result
        .structured_content
        .as_ref()
        .and_then(CacheMetadata::from_json)
}

/// Check if a cached result has expired.
///
/// Returns `true` if the result should be evicted based on the elapsed time
/// since it was cached.
pub fn is_expired(metadata: &CacheMetadata, elapsed: Duration) -> bool {
    elapsed >= metadata.ttl_duration()
}

/// Apply cache semantics to a tool result.
///
/// This function:
/// 1. Extracts cache metadata from the result's `structured_content`
/// 2. Returns the metadata if the result should be cached
///
/// Returns `None` if the result should not be cached.
pub fn apply_cache_semantics(result: &CallToolResult) -> Option<CacheMetadata> {
    let metadata = extract_cache_metadata(result)?;
    if metadata.should_cache() {
        Some(metadata)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ContentBlock;

    fn make_result(structured_content: Option<Value>) -> CallToolResult {
        let mut result = CallToolResult::success(vec![ContentBlock::text("test".to_string())]);
        result.is_error = None;
        result.structured_content = structured_content;
        result
    }

    #[test]
    fn test_cache_scope_from_str() {
        assert_eq!(CacheScope::parse("global"), CacheScope::Global);
        assert_eq!(CacheScope::parse("request"), CacheScope::Request);
        assert_eq!(CacheScope::parse("none"), CacheScope::None);
        assert_eq!(CacheScope::parse("invalid"), CacheScope::None);
    }

    #[test]
    fn test_cache_scope_should_cache() {
        assert!(CacheScope::Global.should_cache());
        assert!(CacheScope::Request.should_cache());
        assert!(!CacheScope::None.should_cache());
    }

    #[test]
    fn test_cache_scope_display() {
        assert_eq!(format!("{}", CacheScope::Global), "global");
        assert_eq!(format!("{}", CacheScope::Request), "request");
        assert_eq!(format!("{}", CacheScope::None), "none");
    }

    #[test]
    fn test_cache_metadata_new() {
        let metadata = CacheMetadata::new(300000, CacheScope::Global);
        assert_eq!(metadata.ttl_ms, 300000);
        assert_eq!(metadata.scope, CacheScope::Global);
    }

    #[test]
    fn test_cache_metadata_ttl_duration() {
        let metadata = CacheMetadata::new(300000, CacheScope::Global);
        assert_eq!(metadata.ttl_duration(), Duration::from_millis(300000));
    }

    #[test]
    fn test_cache_metadata_should_cache_global() {
        let metadata = CacheMetadata::new(300000, CacheScope::Global);
        assert!(metadata.should_cache());
    }

    #[test]
    fn test_cache_metadata_should_cache_request() {
        let metadata = CacheMetadata::new(300000, CacheScope::Request);
        assert!(metadata.should_cache());
    }

    #[test]
    fn test_cache_metadata_should_not_cache_none_scope() {
        let metadata = CacheMetadata::new(300000, CacheScope::None);
        assert!(!metadata.should_cache());
    }

    #[test]
    fn test_cache_metadata_should_not_cache_zero_ttl() {
        let metadata = CacheMetadata::new(0, CacheScope::Global);
        assert!(!metadata.should_cache());
    }

    #[test]
    fn test_cache_metadata_from_json() {
        let json = serde_json::json!({
            "ttlMs": 300000,
            "cacheScope": "global"
        });
        let metadata = CacheMetadata::from_json(&json).unwrap();
        assert_eq!(metadata.ttl_ms, 300000);
        assert_eq!(metadata.scope, CacheScope::Global);
    }

    #[test]
    fn test_cache_metadata_from_json_missing_ttl() {
        let json = serde_json::json!({"cacheScope": "global"});
        assert!(CacheMetadata::from_json(&json).is_none());
    }

    #[test]
    fn test_cache_metadata_from_json_default_scope() {
        let json = serde_json::json!({"ttlMs": 60000});
        let metadata = CacheMetadata::from_json(&json).unwrap();
        assert_eq!(metadata.ttl_ms, 60000);
        assert_eq!(metadata.scope, CacheScope::Request); // default
    }

    #[test]
    fn test_cache_metadata_to_json() {
        let metadata = CacheMetadata::new(300000, CacheScope::Global);
        let json = metadata.to_json();
        assert_eq!(json["ttlMs"], 300000);
        assert_eq!(json["cacheScope"], "global");
    }

    #[test]
    fn test_extract_cache_metadata_present() {
        let structured = serde_json::json!({"ttlMs": 300000, "cacheScope": "global"});
        let result = make_result(Some(structured));
        let metadata = extract_cache_metadata(&result).unwrap();
        assert_eq!(metadata.ttl_ms, 300000);
        assert_eq!(metadata.scope, CacheScope::Global);
    }

    #[test]
    fn test_extract_cache_metadata_absent() {
        let result = make_result(None);
        assert!(extract_cache_metadata(&result).is_none());
    }

    #[test]
    fn test_is_expired_not_expired() {
        let metadata = CacheMetadata::new(300000, CacheScope::Global);
        assert!(!is_expired(&metadata, Duration::from_millis(100000)));
    }

    #[test]
    fn test_is_expired_expired() {
        let metadata = CacheMetadata::new(300000, CacheScope::Global);
        assert!(is_expired(&metadata, Duration::from_millis(300000)));
    }

    #[test]
    fn test_is_expired_exceeded() {
        let metadata = CacheMetadata::new(300000, CacheScope::Global);
        assert!(is_expired(&metadata, Duration::from_millis(400000)));
    }

    #[test]
    fn test_apply_cache_semantics_global() {
        let structured = serde_json::json!({"ttlMs": 300000, "cacheScope": "global"});
        let result = make_result(Some(structured));
        let metadata = apply_cache_semantics(&result).unwrap();
        assert_eq!(metadata.scope, CacheScope::Global);
        assert!(metadata.should_cache());
    }

    #[test]
    fn test_apply_cache_semantics_request() {
        let structured = serde_json::json!({"ttlMs": 300000, "cacheScope": "request"});
        let result = make_result(Some(structured));
        let metadata = apply_cache_semantics(&result).unwrap();
        assert_eq!(metadata.scope, CacheScope::Request);
    }

    #[test]
    fn test_apply_cache_semantics_none_scope() {
        let structured = serde_json::json!({"ttlMs": 300000, "cacheScope": "none"});
        let result = make_result(Some(structured));
        assert!(apply_cache_semantics(&result).is_none());
    }

    #[test]
    fn test_apply_cache_semantics_zero_ttl() {
        let structured = serde_json::json!({"ttlMs": 0, "cacheScope": "global"});
        let result = make_result(Some(structured));
        assert!(apply_cache_semantics(&result).is_none());
    }

    #[test]
    fn test_apply_cache_semantics_no_metadata() {
        let result = make_result(None);
        assert!(apply_cache_semantics(&result).is_none());
    }

    #[test]
    fn test_cache_metadata_default() {
        let metadata = CacheMetadata::default();
        assert_eq!(metadata.ttl_ms, 0);
        assert_eq!(metadata.scope, CacheScope::Request);
    }

    #[test]
    fn test_cache_metadata_equality() {
        let a = CacheMetadata::new(300000, CacheScope::Global);
        let b = CacheMetadata::new(300000, CacheScope::Global);
        assert_eq!(a, b);
    }

    #[test]
    fn test_cache_metadata_clone() {
        let metadata = CacheMetadata::new(300000, CacheScope::Global);
        let cloned = metadata.clone();
        assert_eq!(metadata, cloned);
    }

    #[test]
    fn test_cache_metadata_debug() {
        let metadata = CacheMetadata::new(300000, CacheScope::Global);
        let debug_str = format!("{:?}", metadata);
        assert!(debug_str.contains("300000"));
        assert!(debug_str.contains("Global"));
    }
}
