// Copyright (c) 2026 Kirky.X
//! Security module providing authentication, rate limiting, and audit logging
//!
//! This module provides utilities for securing API endpoints.
//! Requires the `http` feature.

// Re-export all public APIs from submodules
pub use types::*;
pub use traits::*;

pub use api_key::{AppApiKeyAuth, AppApiKeyAuthBuilder};
pub use bearer::{BearerAuth, BearerAuthBuilder};
pub use rate_limiter::{AppRateLimiter, AppRateLimiterBuilder};
pub use audit::{AppAuditLogger, AppAuditLoggerBuilder};
pub use middleware::{auth_middleware, rate_limit_middleware};

// Submodules
mod types;
mod traits;
mod api_key;
mod bearer;
mod rate_limiter;
mod audit;
mod middleware;

// Implement traits for concrete types
impl traits::ApiKeyAuth for AppApiKeyAuth {
    fn validate_key(&self, key: &str, client_ip: &str) -> Option<Vec<String>> {
        AppApiKeyAuth::validate_key(self, key, client_ip)
    }

    fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        AppApiKeyAuth::add_key(self, key, permissions);
    }
}

impl traits::RateLimiter for AppRateLimiter {
    fn allow(&self, key: &str) -> bool {
        AppRateLimiter::allow(self, key)
    }

    fn reset(&self, key: &str) {
        AppRateLimiter::reset(self, key)
    }
}

// Note: AuditLogger trait implementation is already in audit.rs
