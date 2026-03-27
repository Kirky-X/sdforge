// Copyright (c) 2026 Kirky.X
//! Centralized default values for configuration
//!
//! This module provides a single source of truth for all default configuration values.
//! All default values are defined here to avoid scattering across the codebase.

/// Server configuration defaults
pub mod server {
    /// Default server host
    pub const DEFAULT_HOST: &str = "0.0.0.0";
    /// Default server port
    pub const DEFAULT_PORT: u16 = 8080;
    /// Default request timeout in seconds
    pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;
}

/// Timeout configuration defaults
pub mod timeout {
    /// Default request timeout in seconds
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
    /// Route-specific timeout for file uploads (5 minutes)
    pub const UPLOAD_TIMEOUT_SECS: u64 = 300;
    /// Route-specific timeout for exports (2 minutes)
    pub const EXPORT_TIMEOUT_SECS: u64 = 120;
}

/// API key authentication defaults
pub mod api_key {
    /// Default maximum failed attempts before lockout
    pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;
    /// Default lockout duration in seconds (15 minutes)
    pub const DEFAULT_LOCKOUT_DURATION_SECS: u64 = 900;
    /// Minimum API key prefix length
    pub const MIN_PREFIX_LENGTH: usize = 1;
}

/// Security headers defaults
pub mod security_headers {
    /// Default Content-Type-Options header value
    pub const CONTENT_TYPE_OPTIONS: &str = "nosniff";
    /// Default X-Frame-Options header value
    pub const FRAME_OPTIONS: &str = "DENY";
    /// Default X-XSS-Protection header value
    pub const XSS_PROTECTION: &str = "1; mode=block";
    /// Default Cache-Control header value
    pub const CACHE_CONTROL: &str = "no-store, no-cache, must-revalidate";
    /// Default Content-Security-Policy header value
    pub const CONTENT_SECURITY_POLICY: &str =
        "default-src 'self'; script-src 'self'; style-src 'self'";
    /// Default Strict-Transport-Security header value
    pub const STRICT_TRANSPORT_SECURITY: &str = "max-age=31536000; includeSubDomains; preload";
    /// Default Referrer-Policy header value
    pub const REFERRER_POLICY: &str = "strict-origin-when-cross-origin";
    /// Default Permissions-Policy header value
    pub const PERMISSIONS_POLICY: &str = "geolocation=(), microphone=(), camera=()";
}

/// JWT authentication defaults
pub mod jwt {
    /// Minimum JWT secret length for security
    pub const MIN_SECRET_LENGTH: usize = 32;
    /// Default token expiration in seconds (1 hour)
    pub const DEFAULT_EXPIRATION_SECS: u64 = 3600;
}

/// API prefix defaults
pub mod api {
    /// Default API prefix
    pub const DEFAULT_PREFIX: &str = "/api";
    /// Default API version
    pub const DEFAULT_VERSION: &str = "v1";
}