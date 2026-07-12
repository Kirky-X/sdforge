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
