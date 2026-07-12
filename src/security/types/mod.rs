// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Shared types for the security module
//!
//! This module contains all common types used across the security module.

use serde::{Deserialize, Serialize};

mod types_impl;
pub use types_impl::{
    deserialize_audit_logs, deserialize_auth_context, deserialize_permissions, parse_audit_log,
    serialize_audit_logs, serialize_auth_context, serialize_permissions,
};

// =============================================================================
// Cache Key Namespaces
// =============================================================================

/// Cache key namespaces for type-safe key generation
///
/// Centralizes all cache key prefixes to avoid string duplication
/// and enable easy refactoring of cache key formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheNamespace {
    /// API Key storage: `sdforge:apikey:{key_hash}`
    ApiKey,
    /// Bearer token blacklist: `sdforge:bearer:blacklist:{token}`
    BearerBlacklist,
    /// Bearer token valid cache: `sdforge:bearer:valid:{token}`
    BearerValid,
    /// Idempotency key cache: `sdforge:idempotency:{key}`
    Idempotency,
}

// =============================================================================
// Authentication Types
// =============================================================================

/// Authentication errors
#[derive(Debug, thiserror::Error, Clone)]
pub enum AuthError {
    /// Missing or invalid authorization header
    #[error("Missing or invalid authorization header")]
    MissingAuth,

    /// Invalid or expired token
    #[error("Invalid or expired token")]
    InvalidToken,

    /// Insufficient permissions for the requested operation
    #[error("Insufficient permissions: {required}")]
    InsufficientPermissions {
        /// Required permission
        required: String,
        /// User's permissions
        user_permissions: Vec<String>,
    },
}

/// Authentication context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// User ID
    pub(crate) user_id: Option<String>,
    /// User permissions
    pub(crate) permissions: Vec<String>,
    /// Request metadata
    pub(crate) metadata: AuthMetadata,
}

/// Authentication metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthMetadata {
    /// Client IP address
    pub(crate) client_ip: Option<String>,
    /// User agent
    pub(crate) user_agent: Option<String>,
    /// Request ID
    pub(crate) request_id: String,
    /// Timestamp
    pub(crate) timestamp: i64,
}

/// Authentication result
pub type AuthResult<T = AuthContext> = Result<T, AuthError>;

/// Authentication extractor
#[derive(Debug)]
pub struct AuthExtractor(pub AuthContext);

// =============================================================================
// JWT & Bearer Token Types
// =============================================================================

/// JWT verification errors
#[derive(Debug, Clone)]
pub enum JwtError {
    /// Invalid JWT format
    InvalidFormat,
    /// Failed to decode base64
    Base64DecodeError,
    /// Invalid JWT signature
    InvalidSignature,
    /// JWT token expired
    Expired,
    /// JWT token not yet valid
    NotYetValid,
    /// Invalid JWT payload
    InvalidPayload,
    /// Clock skew detected
    ClockSkew,
}

/// Errors that can occur during authentication configuration
#[derive(Debug, thiserror::Error)]
pub enum AuthConfigError {
    /// Secret validation failed
    #[error("Invalid secret: {0}")]
    InvalidSecret(String),

    /// Secret too short
    #[error("Secret too short: {length} chars. Minimum 32 characters required for security.")]
    SecretTooShort {
        /// The length of the provided secret
        length: usize,
    },

    /// Missing required character class
    #[error("Secret must contain at least one {required_type}")]
    MissingCharacterClass {
        /// The type of character that is missing (e.g., "uppercase letter")
        required_type: &'static str,
    },

    /// IO error during configuration
    #[error("Configuration I/O error: {source}")]
    IoError {
        /// The underlying IO error
        #[from]
        source: std::io::Error,
    },

    /// TOML parse error
    #[error("Configuration parse error: {source}")]
    ParseError {
        /// The underlying TOML parse error
        #[from]
        source: toml::de::Error,
    },
}

// =============================================================================
// Audit Types
// =============================================================================

/// Audit log entry with cryptographic signature for integrity verification
#[derive(Debug, Clone)]
pub struct AuditLog {
    /// Log ID
    pub(crate) id: String,
    /// Timestamp
    pub(crate) timestamp: i64,
    /// User ID
    pub(crate) user_id: Option<String>,
    /// Action
    pub(crate) action: String,
    /// Resource
    pub(crate) resource: String,
    /// Result
    pub(crate) result: AuditResult,
    /// Request metadata
    pub(crate) metadata: AuthMetadata,
    /// Cryptographic signature (HMAC-SHA256) for tamper detection
    /// Base64-encoded signature of the log entry's canonical form
    pub(crate) signature: Option<String>,
}

/// Audit result
#[derive(Debug, Clone)]
pub enum AuditResult {
    /// Success
    Success,
    /// Failure
    Failure {
        /// Error message
        message: String,
    },
}

#[cfg(test)]
mod tests;
