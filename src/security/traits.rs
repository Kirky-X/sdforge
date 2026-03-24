// Copyright (c) 2026 Kirky.X
//! Trait definitions for the security module
//!
//! This module contains the feature layer trait interfaces that define
//! the contracts for authentication, rate limiting, and audit logging.

use crate::security::types::AuditLog;

/// Feature layer trait for API key authentication.
///
/// Implement this trait to provide custom API key validation logic while
/// maintaining a consistent interface across the security module.
pub trait ApiKeyAuth: Send + Sync {
    /// Validate an API key and return permissions if valid.
    ///
    /// Returns `Some(Vec<String>)` with permissions if the key is valid,
    /// or `None` if the key is invalid or rate limited.
    ///
    /// # Arguments
    /// * `key` - The API key to validate
    /// * `client_ip` - The client IP address for rate limiting
    fn validate_key(&self, key: &str, client_ip: &str) -> Option<Vec<String>>;

    /// Add a valid API key with associated permissions.
    ///
    /// The key will be securely hashed before storage.
    ///
    /// # Arguments
    /// * `key` - The API key to add
    /// * `permissions` - List of permissions granted by this key
    fn add_key(&self, key: impl Into<String>, permissions: Vec<String>);
}

/// Feature layer trait for rate limiting.
///
/// Implement this trait to provide custom rate limiting logic while
/// maintaining a consistent interface across the security module.
pub trait RateLimiter: Send + Sync {
    /// Check if a request is allowed under the rate limit.
    ///
    /// Returns `true` if the request is allowed, `false` if rate limited.
    ///
    /// # Arguments
    /// * `key` - The rate limit key (typically client IP or user ID)
    fn allow(&self, key: &str) -> bool;

    /// Reset rate limit state for a key.
    ///
    /// Clears all rate limiting data for the given key, allowing
    /// new requests to be processed without restriction.
    ///
    /// # Arguments
    /// * `key` - The rate limit key to reset
    fn reset(&self, key: &str);
}

/// Feature layer trait for audit logging.
///
/// Implement this trait to provide custom audit logging logic while
/// maintaining a consistent interface across the security module.
pub trait AuditLogger: Send + Sync {
    /// Log an audit event.
    ///
    /// # Arguments
    /// * `log` - The audit log entry to record
    fn log(&self, log: AuditLog);
}
