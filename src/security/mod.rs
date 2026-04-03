// Copyright (c) 2026 Kirky.X
//! Security module providing authentication and audit logging
//!
//! This module provides utilities for securing API endpoints.
//! Requires the `http` feature.

// Re-export all public APIs from submodules
pub use traits::*;
pub use types::*;

pub use api_key::{AppApiKeyAuth, AppApiKeyAuthBuilder};
pub use audit::{AppAuditLogger, AppAuditLoggerBuilder};
pub use bearer::{generate_secure_jwt_secret, BearerAuth, BearerAuthBuilder};
pub use middleware::auth_middleware;

// Submodules
mod api_key;
mod api_key_manager;
mod audit;
mod bearer;
mod middleware;
mod traits;
mod types;

// Re-export key management types
pub use api_key_manager::{
    ApiKeyMetadata, ApiKeyVersion, LruCacheManager, LruConfig, LruStats, RotationConfig,
};

// Implement traits for concrete types
impl traits::ApiKeyAuth for AppApiKeyAuth {
    fn validate_key(&self, key: &str, client_ip: &str) -> Option<Vec<String>> {
        AppApiKeyAuth::validate_key(self, key, client_ip)
    }

    fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        AppApiKeyAuth::add_key(self, key, permissions);
    }
}

// Note: AuditLogger trait implementation is already in audit.rs
