// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Security module providing authentication and audit logging
//!
//! This module provides utilities for securing API endpoints.
//! Requires the `http` feature.
//!
//! The `ratelimit` submodule (backed by limiteron 0.2.1) is gated by
//! `feature = "ratelimit"` and is reachable even when the full `security`
//! feature is off — see `src/lib.rs` for the parent-module gate
//! (`any(feature = "security", feature = "ratelimit")`).

// Re-export all public APIs from submodules (full security feature only)
#[cfg(feature = "security")]
pub use traits::*;
#[cfg(feature = "security")]
pub use types::*;

#[cfg(feature = "security")]
pub use api_key::{AppApiKeyAuth, AppApiKeyAuthBuilder};
#[cfg(feature = "security")]
pub use audit::{AppAuditLogger, AppAuditLoggerBuilder};
#[cfg(feature = "security")]
pub use bearer::{BearerAuth, BearerAuthBuilder, generate_secure_jwt_secret};
#[cfg(feature = "security")]
pub use middleware::auth_middleware;

// Submodules (full security feature only — these pull in uuid/hmac/sha2/etc.)
#[cfg(feature = "security")]
mod api_key;
#[cfg(feature = "security")]
mod api_key_manager;
#[cfg(feature = "security")]
mod audit;
#[cfg(feature = "security")]
mod bearer;
#[cfg(feature = "security")]
mod middleware;
#[cfg(feature = "security")]
mod traits;
#[cfg(feature = "security")]
mod types;

// Shared IP-extraction utilities — available to both `security` and
// `ratelimit` features so the rate-limit adapter reuses the same
// spoofing-defense logic as the auth middleware (single source of truth).
#[cfg(any(feature = "security", feature = "ratelimit"))]
mod ip_util;

// Rate limiting module — backed by limiteron 0.2.1.
// Available when the `ratelimit` feature is enabled (inherited by `security`).
#[cfg(feature = "ratelimit")]
pub mod ratelimit;

// Re-export key management types (full security feature only)
#[cfg(feature = "security")]
pub use api_key_manager::{
    ApiKeyMetadata, ApiKeyVersion, LruCacheManager, LruConfig, LruStats, RotationConfig,
};

// Implement traits for concrete types (full security feature only)
#[cfg(feature = "security")]
impl traits::ApiKeyAuth for AppApiKeyAuth {
    fn validate_key(&self, key: &str, client_ip: &str) -> Option<Vec<String>> {
        AppApiKeyAuth::validate_key(self, key, client_ip)
    }

    fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        AppApiKeyAuth::add_key(self, key, permissions);
    }
}

// Note: AuditLogger trait implementation is already in audit.rs

#[cfg(all(test, feature = "security"))]
mod tests {
    use super::*;
    use crate::security::traits::ApiKeyAuth;

    /// Trait dispatch path for `validate_key` returns permissions for a valid key.
    /// Uses fully-qualified syntax to force the trait impl in this module.
    #[test]
    fn trait_validate_key_returns_permissions_for_valid_key() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("test-key", vec!["read".to_string()]);
        let result = <AppApiKeyAuth as ApiKeyAuth>::validate_key(&auth, "test-key", "127.0.0.1");
        assert_eq!(result, Some(vec!["read".to_string()]));
    }

    /// Trait dispatch path returns `None` for an unknown key.
    #[test]
    fn trait_validate_key_returns_none_for_unknown_key() {
        let auth = AppApiKeyAuth::new();
        let result = <AppApiKeyAuth as ApiKeyAuth>::validate_key(&auth, "unknown-key", "127.0.0.1");
        assert_eq!(result, None);
    }

    /// Trait dispatch path returns `None` for an empty key.
    #[test]
    fn trait_validate_key_returns_none_for_empty_key() {
        let auth = AppApiKeyAuth::new();
        let result = <AppApiKeyAuth as ApiKeyAuth>::validate_key(&auth, "", "127.0.0.1");
        assert_eq!(result, None);
    }

    /// Trait dispatch path for `add_key` delegates to the inherent method,
    /// making the key immediately validatable via the trait path.
    #[test]
    fn trait_add_key_delegates_to_inherent_method() {
        let auth = AppApiKeyAuth::new();
        <AppApiKeyAuth as ApiKeyAuth>::add_key(&auth, "trait-key", vec!["admin".to_string()]);
        let result = <AppApiKeyAuth as ApiKeyAuth>::validate_key(&auth, "trait-key", "127.0.0.1");
        assert_eq!(result, Some(vec!["admin".to_string()]));
    }

    /// Trait `add_key` accepts `impl Into<String>`, so a `&str` literal works
    /// through the trait dispatch path.
    #[test]
    fn trait_add_key_with_string_literal_works() {
        let auth = AppApiKeyAuth::new();
        <AppApiKeyAuth as ApiKeyAuth>::add_key(&auth, "literal-key", vec!["read".to_string()]);
        let result = <AppApiKeyAuth as ApiKeyAuth>::validate_key(&auth, "literal-key", "127.0.0.1");
        assert_eq!(result, Some(vec!["read".to_string()]));
    }

    /// Trait dispatch and inherent dispatch produce identical results,
    /// confirming the trait impl is a pure delegation wrapper.
    #[test]
    fn trait_dispatch_matches_inherent_dispatch() {
        let auth1 = AppApiKeyAuth::new();
        let auth2 = AppApiKeyAuth::new();
        let perms = vec!["read".to_string(), "write".to_string()];
        auth1.add_key("shared-key", perms.clone());
        <AppApiKeyAuth as ApiKeyAuth>::add_key(&auth2, "shared-key", perms);

        let inherent_result = auth1.validate_key("shared-key", "127.0.0.1");
        let trait_result =
            <AppApiKeyAuth as ApiKeyAuth>::validate_key(&auth2, "shared-key", "127.0.0.1");

        assert_eq!(inherent_result, trait_result);
    }
}
