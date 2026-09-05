// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;

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
            i18n_key: None,
        }
    }

    /// Attach an i18n key for runtime translation of the description.
    ///
    /// When set, protocol consumption points (MCP tool descriptions,
    /// CLI `--help`, OpenAPI specs, gRPC metadata) look up a translation
    /// via `sdforge::i18n::translate_or_fallback` using the active locale.
    /// Falls back to the English `description` when no translation is found.
    ///
    /// Builder-pattern method so existing `new()` call sites remain
    /// backward-compatible.
    pub fn with_i18n_key(mut self, key: Option<String>) -> Self {
        self.i18n_key = key;
        self
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

    /// Get the i18n key for runtime translation, if set.
    pub fn i18n_key(&self) -> Option<&str> {
        self.i18n_key.as_deref()
    }
}
