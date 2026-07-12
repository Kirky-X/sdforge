// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Bearer token authentication implementation
//!
//! This module provides JWT-based bearer token authentication with
//! HMAC-SHA256 signature verification and claim validation.

use crate::cache::SharedCache;

mod bearer_impl;
pub use bearer_impl::generate_secure_jwt_secret;

/// Bearer token authentication
///
/// Security features:
/// - HMAC-SHA256 signature verification
/// - Audience and issuer claim validation (prevents token substitution attacks)
/// - Expiration time checking
/// - Token blacklist for immediate invalidation
///
/// Storage: All internal state is stored via `Arc<dyn SyncCache>` trait.
#[derive(Clone)]
pub struct BearerAuth {
    /// JWT secret for HMAC-SHA256 signing
    secret: Vec<u8>,
    /// Valid tokens cache via SyncCache
    valid_tokens: SharedCache,
    /// Token blacklist (for logout) via SyncCache
    blacklisted_tokens: SharedCache,
    /// Expected audience claim (prevents token substitution)
    expected_audience: Option<String>,
    /// Expected issuer claim (validates token origin)
    expected_issuer: Option<String>,
}

/// Builder for BearerAuth configuration
///
/// This builder provides a fluent interface for configuring BearerAuth instances
/// with proper validation of the secret at build time.
///
/// # Security Requirements
///
/// The secret must meet the following requirements:
/// - At least 32 characters in length
/// - Contains at least one uppercase letter
/// - Contains at least one lowercase letter
/// - Contains at least one digit
/// - Contains at least one special character
///
/// # Examples
///
/// ```rust
/// use sdforge::security::BearerAuth;
///
/// // Basic usage with secret only
/// let auth = BearerAuth::builder()
///     .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
///     .build()
///     .expect("Failed to build BearerAuth");
///
/// // With audience and issuer validation
/// let auth = BearerAuth::builder()
///     .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
///     .audience("my-api")
///     .issuer("my-issuer")
///     .build()
///     .expect("Failed to build BearerAuth");
/// let _ = auth;
/// ```
#[derive(Debug, Clone, Default)]
pub struct BearerAuthBuilder {
    /// JWT signing secret
    secret: Option<String>,
    /// Expected audience claim for validation
    audience: Option<String>,
    /// Expected issuer claim for validation
    issuer: Option<String>,
}

#[cfg(test)]
mod tests;
