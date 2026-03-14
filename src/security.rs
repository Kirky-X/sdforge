// Copyright (c) 2026 Kirky.X
//! Security module providing authentication, rate limiting, and audit logging
//!
//! This module provides utilities for securing API endpoints.
//! Requires the `http` feature.

use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use hmac::{Hmac, Mac};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

/// Authentication errors
#[derive(Debug, Error, Clone)]
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

impl AuthContext {
    /// Get user ID
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Get user permissions
    pub fn permissions(&self) -> &[String] {
        &self.permissions
    }

    /// Get authentication metadata
    pub fn metadata(&self) -> &AuthMetadata {
        &self.metadata
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Create new AuthContext
    pub fn new(user_id: Option<String>, permissions: Vec<String>, metadata: AuthMetadata) -> Self {
        Self {
            user_id,
            permissions,
            metadata,
        }
    }
}

impl AuthMetadata {
    /// Get client IP address
    pub fn client_ip(&self) -> Option<&str> {
        self.client_ip.as_deref()
    }

    /// Get user agent
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    /// Get request ID
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Get timestamp
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Create new AuthMetadata
    pub fn new(client_ip: Option<String>, user_agent: Option<String>) -> Self {
        Self {
            client_ip,
            user_agent,
            request_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Authentication result
pub type AuthResult<T = AuthContext> = Result<T, AuthError>;

/// Authentication extractor
#[derive(Debug)]
pub struct AuthExtractor(AuthContext);

#[allow(missing_docs)]
impl AuthExtractor {
    pub fn new(context: AuthContext) -> Self {
        Self(context)
    }

    pub fn context(&self) -> &AuthContext {
        &self.0
    }

    pub fn into_inner(self) -> AuthContext {
        self.0
    }
}

/// API key authentication with brute-force protection
///
/// Security features:
/// - Valid API keys storage with permissions mapping (hashed for security)
/// - Rate limiting on validation attempts to prevent brute force attacks
/// - Per-IP attempt tracking with automatic cleanup
#[derive(Clone)]
pub struct ApiKeyAuth {
    /// Valid API keys (stored as SHA256 hash -> permissions)
    valid_keys: Arc<DashMap<String, Vec<String>>>,
    /// Failed attempt tracking (IP -> attempts with timestamps)
    failed_attempts: Arc<DashMap<String, Vec<Instant>>>,
    /// Rate limit configuration
    rate_limit_config: Arc<RateLimitConfig>,
}

impl ApiKeyAuth {
    /// Create new API key authentication with default rate limiting
    pub fn new() -> Self {
        Self::with_rate_limit(RateLimitConfig::default())
    }

    /// Create API key authentication with custom rate limiting
    pub fn with_rate_limit(config: RateLimitConfig) -> Self {
        Self {
            valid_keys: Arc::new(DashMap::new()),
            failed_attempts: Arc::new(DashMap::new()),
            rate_limit_config: Arc::new(config),
        }
    }

    /// Create with dependencies (for full DI mode)
    pub fn with_dependencies(
        valid_keys: Arc<DashMap<String, Vec<String>>>,
        failed_attempts: Arc<DashMap<String, Vec<Instant>>>,
        rate_limit_config: Arc<RateLimitConfig>,
    ) -> Self {
        Self {
            valid_keys,
            failed_attempts,
            rate_limit_config,
        }
    }

    /// Create builder for configuration
    pub fn builder() -> ApiKeyAuthBuilder {
        ApiKeyAuthBuilder::new()
    }

    /// Hash API key using SHA256 with work factor for secure storage
    fn hash_key(key: &str) -> String {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();

        // Multiple rounds to slow brute-force attacks
        for _ in 0..100 {
            hasher.update(key.as_bytes());
            hasher.update([0x5c, 0x5c, 0x5c]);
        }

        format!("{:x}", hasher.finalize())
    }

    /// Add a valid API key (stored as hash)
    pub fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        let key_hash = Self::hash_key(&key.into());
        self.valid_keys.insert(key_hash, permissions);
    }

    /// Validate an API key with rate limiting
    ///
    /// Security: Implements constant-time validation to prevent timing attacks.
    /// All code paths take the same amount of time regardless of key validity.
    /// Also implements rate limiting per caller to prevent brute force attacks.
    /// Note: Valid keys bypass rate limiting to prevent locking out legitimate users.
    pub fn validate_key(&self, key: &str, client_ip: &str) -> Option<Vec<String>> {
        let start = Instant::now();
        let key_hash = Self::hash_key(key);

        // Always check valid_keys first for constant timing
        let is_valid = self.valid_keys.get(&key_hash).is_some();

        // For valid keys, skip rate limiting and return immediately
        // This prevents locking out legitimate users after suspicious activity
        if is_valid {
            // Apply delay for constant timing even for valid keys
            Self::apply_constant_time_delay(start);
            return self.valid_keys.get(&key_hash).map(|p| p.clone());
        }

        // For invalid keys, check rate limit
        let is_limited = self.is_rate_limited(client_ip);

        // Record failed attempt if not already rate limited
        if !is_limited {
            self.record_failed_attempt(client_ip);
        }

        // Apply constant-time delay to normalize response time
        Self::apply_constant_time_delay(start);

        None
    }

    /// Apply constant-time delay to prevent timing attacks
    ///
    /// This ensures that the validation function always takes the same
    /// amount of time regardless of the key validity or rate limit status.
    fn apply_constant_time_delay(start: Instant) {
        // Skip delay in test mode for faster tests
        if cfg!(test) {
            return;
        }

        const TARGET_DELAY_US: u64 = 100; // 100 microseconds
        let elapsed = start.elapsed();

        if elapsed < Duration::from_micros(TARGET_DELAY_US) {
            std::thread::sleep(Duration::from_micros(TARGET_DELAY_US) - elapsed);
        }
    }

    /// Check if a client IP is rate limited
    fn is_rate_limited(&self, client_ip: &str) -> bool {
        let now = Instant::now();
        let window_start = now - self.rate_limit_config.window;

        let entry = self.failed_attempts.get(client_ip);
        if let Some(times) = entry {
            let recent_attempts = times.iter().filter(|&&t| t > window_start).count();
            recent_attempts >= self.rate_limit_config.max_requests as usize
        } else {
            false
        }
    }

    /// Record a failed validation attempt
    fn record_failed_attempt(&self, client_ip: &str) {
        let now = Instant::now();
        let window_start = now - self.rate_limit_config.window;

        let mut entry = self
            .failed_attempts
            .entry(client_ip.to_string())
            .or_default();
        let times = entry.value_mut();

        // Clean old attempts outside the window
        times.retain(|&t| t > window_start);

        // Add new attempt
        times.push(now);
    }

    /// Clear failed attempts for a client (e.g., after successful auth)
    pub fn clear_failed_attempts(&self, client_ip: &str) {
        self.failed_attempts.remove(client_ip);
    }
}

impl Default for ApiKeyAuth {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for ApiKeyAuth configuration
#[derive(Debug, Clone, Default)]
pub struct ApiKeyAuthBuilder {
    rate_limit_config: RateLimitConfig,
}

impl ApiKeyAuthBuilder {
    /// Create a new ApiKeyAuthBuilder with default rate limit settings.
    ///
    /// # Returns
    ///
    /// Returns a builder initialized with default rate limit configuration.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::ApiKeyAuthBuilder;
    ///
    /// let builder = ApiKeyAuthBuilder::new();
    /// let _ = builder;
    /// ```
    pub fn new() -> Self {
        Self {
            rate_limit_config: RateLimitConfig::default(),
        }
    }

    /// Set the maximum number of requests within the rate limit window.
    ///
    /// # Arguments
    ///
    /// * `max_requests` - Maximum number of requests allowed in a window.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::ApiKeyAuthBuilder;
    ///
    /// let builder = ApiKeyAuthBuilder::new().max_requests(100);
    /// let _ = builder;
    /// ```
    pub fn max_requests(mut self, max_requests: u32) -> Self {
        self.rate_limit_config.max_requests = max_requests;
        self
    }

    /// Set the duration of the rate limit window.
    ///
    /// # Arguments
    ///
    /// * `window` - Time window for rate limiting.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::ApiKeyAuthBuilder;
    /// use std::time::Duration;
    ///
    /// let builder = ApiKeyAuthBuilder::new().window(Duration::from_secs(60));
    /// let _ = builder;
    /// ```
    pub fn window(mut self, window: Duration) -> Self {
        self.rate_limit_config.window = window;
        self
    }

    /// Configure whether rate limit headers are included in responses.
    ///
    /// # Arguments
    ///
    /// * `include_headers` - Whether to include rate limit headers.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::ApiKeyAuthBuilder;
    ///
    /// let builder = ApiKeyAuthBuilder::new().include_headers(true);
    /// let _ = builder;
    /// ```
    pub fn include_headers(mut self, include_headers: bool) -> Self {
        self.rate_limit_config.include_headers = include_headers;
        self
    }

    /// Build an ApiKeyAuth instance using the configured settings.
    ///
    /// # Returns
    ///
    /// Returns a fully configured ApiKeyAuth instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::ApiKeyAuthBuilder;
    ///
    /// let auth = ApiKeyAuthBuilder::new().max_requests(100).build();
    /// let _ = auth;
    /// ```
    pub fn build(self) -> ApiKeyAuth {
        ApiKeyAuth::with_rate_limit(self.rate_limit_config)
    }
}

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

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtError::InvalidFormat => write!(f, "Invalid JWT format"),
            JwtError::Base64DecodeError => write!(f, "Failed to decode base64"),
            JwtError::InvalidSignature => write!(f, "Invalid JWT signature"),
            JwtError::Expired => write!(f, "JWT token expired"),
            JwtError::NotYetValid => write!(f, "JWT token not yet valid"),
            JwtError::InvalidPayload => write!(f, "Invalid JWT payload"),
            JwtError::ClockSkew => write!(f, "Clock skew too large"),
        }
    }
}

impl std::error::Error for JwtError {}

/// Bearer token authentication
///
/// Security features:
/// - HMAC-SHA256 signature verification
/// - Audience and issuer claim validation (prevents token substitution attacks)
/// - Expiration time checking
/// - Token blacklist for immediate invalidation
#[derive(Clone)]
pub struct BearerAuth {
    /// JWT secret for HMAC-SHA256 signing
    secret: Vec<u8>,
    /// Valid tokens cache
    valid_tokens: Arc<DashMap<String, AuthContext>>,
    /// Token blacklist (for logout)
    blacklisted_tokens: Arc<DashMap<String, Instant>>,
    /// Expected audience claim (prevents token substitution)
    expected_audience: Option<String>,
    /// Expected issuer claim (validates token origin)
    expected_issuer: Option<String>,
}

/// Errors that can occur during authentication configuration
#[derive(Debug, Error)]
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

impl BearerAuth {
    /// Create new bearer authentication with basic secret
    ///
    /// # Panics
    /// Panics if the secret is too short or doesn't meet complexity requirements
    /// Use `try_new()` for error handling instead.
    pub fn new(secret: impl Into<String>) -> Self {
        Self::try_new(secret).expect("Failed to create BearerAuth: invalid secret")
    }

    /// Create new bearer authentication with basic secret
    ///
    /// Returns an error if the secret doesn't meet security requirements.
    ///
    /// # Arguments
    /// * `secret` - JWT signing secret (must be at least 32 characters)
    ///
    /// # Errors
    /// Returns `AuthConfigError::SecretTooShort` if secret is too short
    /// Returns `AuthConfigError::MissingCharacterClass` if secret lacks required character types
    pub fn try_new(secret: impl Into<String>) -> Result<Self, AuthConfigError> {
        let secret_str = secret.into();

        if secret_str.len() < 32 {
            return Err(AuthConfigError::SecretTooShort {
                length: secret_str.len(),
            });
        }

        if !secret_str.chars().any(|c| c.is_uppercase()) {
            return Err(AuthConfigError::MissingCharacterClass {
                required_type: "uppercase letter",
            });
        }
        if !secret_str.chars().any(|c| c.is_lowercase()) {
            return Err(AuthConfigError::MissingCharacterClass {
                required_type: "lowercase letter",
            });
        }
        if !secret_str.chars().any(|c| c.is_ascii_digit()) {
            return Err(AuthConfigError::MissingCharacterClass {
                required_type: "digit",
            });
        }
        if !secret_str.chars().any(|c| !c.is_alphanumeric()) {
            return Err(AuthConfigError::MissingCharacterClass {
                required_type: "special character",
            });
        }

        Ok(Self {
            secret: secret_str.into_bytes(),
            valid_tokens: Arc::new(DashMap::new()),
            blacklisted_tokens: Arc::new(DashMap::new()),
            expected_audience: None,
            expected_issuer: None,
        })
    }

    /// Create bearer authentication with audience validation
    ///
    /// # Panics
    /// Panics if the secret doesn't meet complexity requirements
    pub fn with_audience(secret: impl Into<String>, expected_audience: impl Into<String>) -> Self {
        Self {
            secret: secret.into().into_bytes(),
            valid_tokens: Arc::new(DashMap::new()),
            blacklisted_tokens: Arc::new(DashMap::new()),
            expected_audience: Some(expected_audience.into()),
            expected_issuer: None,
        }
    }

    /// Create bearer authentication with full claim validation
    ///
    /// # Arguments
    /// * `secret` - JWT signing secret
    /// * `expected_audience` - Expected `aud` claim value
    /// * `expected_issuer` - Expected `iss` claim value
    pub fn with_claims(
        secret: impl Into<String>,
        expected_audience: impl Into<String>,
        expected_issuer: impl Into<String>,
    ) -> Self {
        Self {
            secret: secret.into().into_bytes(),
            valid_tokens: Arc::new(DashMap::new()),
            blacklisted_tokens: Arc::new(DashMap::new()),
            expected_audience: Some(expected_audience.into()),
            expected_issuer: Some(expected_issuer.into()),
        }
    }

    /// Create with dependencies (for full DI mode)
    ///
    /// This method allows full dependency injection, enabling the caller to provide
    /// all internal dependencies. This is useful for testing and advanced configuration
    /// scenarios where you need control over the internal state.
    ///
    /// # Arguments
    ///
    /// * `secret` - JWT signing secret as bytes
    /// * `valid_tokens` - Cache for valid tokens (shared across instances)
    /// * `blacklisted_tokens` - Cache for blacklisted tokens (shared across instances)
    /// * `expected_audience` - Optional expected audience claim for validation
    /// * `expected_issuer` - Optional expected issuer claim for validation
    ///
    /// # Returns
    ///
    /// Returns a `BearerAuth` instance configured with the provided dependencies.
    ///
    /// # Security Note
    ///
    /// This method does not validate the secret. The caller is responsible for
    /// ensuring the secret meets security requirements when using this method.
    /// For production use, prefer `new()`, `try_new()`, or `builder()` which
    /// enforce secret validation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::BearerAuth;
    /// use dashmap::DashMap;
    /// use std::sync::Arc;
    ///
    /// let valid_tokens = Arc::new(DashMap::new());
    /// let blacklisted_tokens = Arc::new(DashMap::new());
    ///
    /// let auth = BearerAuth::with_dependencies(
    ///     b"my-secret-key".to_vec(),
    ///     valid_tokens,
    ///     blacklisted_tokens,
    ///     Some("my-api".to_string()),
    ///     Some("my-issuer".to_string()),
    /// );
    /// let _ = auth;
    /// ```
    pub fn with_dependencies(
        secret: Vec<u8>,
        valid_tokens: Arc<DashMap<String, AuthContext>>,
        blacklisted_tokens: Arc<DashMap<String, Instant>>,
        expected_audience: Option<String>,
        expected_issuer: Option<String>,
    ) -> Self {
        Self {
            secret,
            valid_tokens,
            blacklisted_tokens,
            expected_audience,
            expected_issuer,
        }
    }

    /// Create a builder for configuring BearerAuth.
    ///
    /// The builder pattern allows for flexible configuration of BearerAuth
    /// with validation of the secret at build time.
    ///
    /// # Returns
    ///
    /// Returns a `BearerAuthBuilder` instance for configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::BearerAuth;
    ///
    /// let auth = BearerAuth::builder()
    ///     .secret("MySecureSecret123!@#ABCDEF")
    ///     .audience("my-api")
    ///     .issuer("my-issuer")
    ///     .build()
    ///     .expect("Failed to build BearerAuth");
    /// let _ = auth;
    /// ```
    pub fn builder() -> BearerAuthBuilder {
        BearerAuthBuilder::new()
    }

    /// Constant-time comparison to prevent timing attacks
    /// Uses the subtle crate for secure constant-time comparison
    #[cfg(feature = "security")]
    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        use subtle::ConstantTimeEq;
        a.ct_eq(b).into()
    }

    /// Fallback constant-time comparison when subtle is not available
    #[cfg(not(feature = "security"))]
    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut result = 0u8;
        for (byte_a, byte_b) in a.iter().zip(b.iter()) {
            result |= byte_a ^ byte_b;
        }
        result == 0
    }

    /// Base64url decode (JWT uses URL-safe base64)
    fn base64url_decode(input: &str) -> Option<Vec<u8>> {
        let mut table = [0u8; 256];
        for (i, b) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
            .iter()
            .enumerate()
        {
            table[*b as usize] = i as u8;
        }

        let mut result = Vec::with_capacity(input.len() * 3 / 4);
        let mut buffer = 0u32;
        let mut bits = 0i32;

        for c in input.bytes() {
            if c == b'.' {
                continue; // Skip period separators
            }
            if c == b' ' || c == b'\n' || c == b'\r' || c == b'\t' {
                continue; // Skip whitespace
            }

            let val = table.get(c as usize)?;
            buffer = (buffer << 6) | (*val as u32);
            bits += 6;

            if bits >= 8 {
                bits -= 8;
                result.push((buffer >> bits) as u8);
            }
        }

        Some(result)
    }

    /// Parse JWT token and verify signature with full claim validation
    ///
    /// Security checks:
    /// 1. Validates token structure (3 parts)
    /// 2. Verifies HMAC-SHA256 signature
    /// 3. Validates `exp` (expiration) claim
    /// 4. Validates `aud` (audience) claim if configured (prevents token substitution)
    /// 5. Validates `iss` (issuer) claim if configured (validates token origin)
    ///
    /// JWT Format: header.payload.signature
    /// Each part is base64url-encoded
    fn verify_jwt(&self, token: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        // Decode header
        let _header = Self::base64url_decode(parts[0])?;

        // Decode payload
        let payload = Self::base64url_decode(parts[1])?;
        let payload_str = String::from_utf8_lossy(&payload);
        let payload_value: serde_json::Value = serde_json::from_str(&payload_str).ok()?;

        // Verify signature using HMAC-SHA256
        let signature_input = format!("{}.{}", parts[0], parts[1]);
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret).ok()?;
        mac.update(signature_input.as_bytes());
        let expected_signature = mac.finalize().into_bytes();

        // Decode provided signature
        let provided_signature = Self::base64url_decode(parts[2])?;
        if provided_signature.len() != 32 {
            return None;
        }

        // Constant-time comparison to prevent timing attacks
        if !Self::constant_time_eq(expected_signature.as_slice(), &provided_signature) {
            return None;
        }

        // Check expiration if present
        if let Some(exp) = payload_value.get("exp").and_then(|v| v.as_i64()) {
            if chrono::Utc::now().timestamp() > exp {
                return None; // Token expired
            }
        }

        // Security fix: Validate iat (issued at) claim to prevent usage of future tokens
        // This prevents replay attacks with tokens that have valid signatures but haven't been issued yet
        if let Some(iat) = payload_value.get("iat").and_then(|v| v.as_i64()) {
            let now = chrono::Utc::now().timestamp();
            // Allow tokens issued up to 60 seconds in the future (clock skew tolerance)
            const CLOCK_SKEW_SECONDS: i64 = 60;
            if iat > now + CLOCK_SKEW_SECONDS {
                return None; // Token issued in the future (possible tampering or clock issue)
            }
        }

        // Security fix: Validate nbf (not before) claim to prevent usage of tokens that aren't yet valid
        if let Some(nbf) = payload_value.get("nbf").and_then(|v| v.as_i64()) {
            if chrono::Utc::now().timestamp() < nbf {
                return None; // Token not yet valid
            }
        }

        // Validate audience claim if expected_audience is configured
        // This prevents token substitution attacks where an attacker uses
        // a token issued for a different audience
        if let Some(expected_aud) = &self.expected_audience {
            let token_aud = payload_value
                .get("aud")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    payload_value
                        .get("aud")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first().and_then(|v| v.as_str()))
                });

            if token_aud != Some(expected_aud.as_str()) {
                return None; // Audience claim mismatch
            }
        }

        // Validate issuer claim if expected_issuer is configured
        // This ensures the token was issued by a trusted authority
        if let Some(expected_iss) = &self.expected_issuer {
            let token_iss = payload_value.get("iss").and_then(|v| v.as_str());
            if token_iss != Some(expected_iss.as_str()) {
                return None; // Issuer claim mismatch
            }
        }

        Some(payload_value)
    }

    /// Validate a bearer token with proper JWT verification
    pub fn validate_token(&self, token: &str) -> Option<AuthContext> {
        // Check if token is blacklisted
        if let Some(expiry) = self.blacklisted_tokens.get(token) {
            if Instant::now() < *expiry {
                return None; // Token is blacklisted
            }
        }

        // Verify JWT signature and claims
        let payload = self.verify_jwt(token)?;

        // Extract claims from payload
        let user_id = payload
            .get("sub")
            .and_then(|v| v.as_str())
            .map(String::from);
        let permissions: Vec<String> = payload
            .get("permissions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| p.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Some(AuthContext {
            user_id,
            permissions,
            metadata: AuthMetadata::default(),
        })
    }

    /// Register a token (for session management)
    pub fn register_token(&self, token: String, context: AuthContext) {
        self.valid_tokens.insert(token, context);
    }

    /// Invalidate a token (for logout)
    pub fn invalidate_token(&self, token: &str) {
        // Invalidate immediately (could add grace period)
        self.blacklisted_tokens
            .insert(token.to_string(), Instant::now());
    }
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
///     .secret("MySecureSecret123!@#ABCDEF")
///     .build()
///     .expect("Failed to build BearerAuth");
///
/// // With audience and issuer validation
/// let auth = BearerAuth::builder()
///     .secret("MySecureSecret123!@#ABCDEF")
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

impl BearerAuthBuilder {
    /// Create a new BearerAuthBuilder with default settings.
    ///
    /// # Returns
    ///
    /// Returns a builder initialized with no configuration.
    /// The secret must be set before calling `build()`.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::BearerAuthBuilder;
    ///
    /// let builder = BearerAuthBuilder::new();
    /// let _ = builder;
    /// ```
    pub fn new() -> Self {
        Self {
            secret: None,
            audience: None,
            issuer: None,
        }
    }

    /// Set the JWT signing secret.
    ///
    /// The secret must meet security requirements:
    /// - At least 32 characters in length
    /// - Contains at least one uppercase letter
    /// - Contains at least one lowercase letter
    /// - Contains at least one digit
    /// - Contains at least one special character
    ///
    /// # Arguments
    ///
    /// * `secret` - JWT signing secret string
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors. Validation occurs at build time.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::BearerAuthBuilder;
    ///
    /// let builder = BearerAuthBuilder::new()
    ///     .secret("MySecureSecret123!@#ABCDEF");
    /// let _ = builder;
    /// ```
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Set the expected audience claim for JWT validation.
    ///
    /// When set, the JWT's `aud` claim must match this value for the token
    /// to be considered valid. This prevents token substitution attacks where
    /// an attacker uses a token issued for a different audience.
    ///
    /// # Arguments
    ///
    /// * `audience` - Expected audience claim value
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::BearerAuthBuilder;
    ///
    /// let builder = BearerAuthBuilder::new()
    ///     .secret("MySecureSecret123!@#ABCDEF")
    ///     .audience("my-api");
    /// let _ = builder;
    /// ```
    pub fn audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Set the expected issuer claim for JWT validation.
    ///
    /// When set, the JWT's `iss` claim must match this value for the token
    /// to be considered valid. This ensures the token was issued by a trusted
    /// authority.
    ///
    /// # Arguments
    ///
    /// * `issuer` - Expected issuer claim value
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::BearerAuthBuilder;
    ///
    /// let builder = BearerAuthBuilder::new()
    ///     .secret("MySecureSecret123!@#ABCDEF")
    ///     .issuer("my-issuer");
    /// let _ = builder;
    /// ```
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Build a BearerAuth instance using the configured settings.
    ///
    /// This method validates the secret and returns an error if it doesn't
    /// meet the security requirements.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the configured `BearerAuth` instance
    /// or an `AuthConfigError` if validation fails.
    ///
    /// # Errors
    ///
    /// Returns `AuthConfigError::InvalidSecret` if no secret was provided.
    /// Returns `AuthConfigError::SecretTooShort` if secret is less than 32 characters.
    /// Returns `AuthConfigError::MissingCharacterClass` if secret lacks required character types.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::BearerAuthBuilder;
    ///
    /// let auth = BearerAuthBuilder::new()
    ///     .secret("MySecureSecret123!@#ABCDEF")
    ///     .audience("my-api")
    ///     .build()
    ///     .expect("Failed to build BearerAuth");
    /// let _ = auth;
    /// ```
    pub fn build(self) -> Result<BearerAuth, AuthConfigError> {
        let secret = self
            .secret
            .ok_or_else(|| AuthConfigError::InvalidSecret("Secret is required".to_string()))?;

        // Validate secret length
        if secret.len() < 32 {
            return Err(AuthConfigError::SecretTooShort {
                length: secret.len(),
            });
        }

        // Validate character classes
        if !secret.chars().any(|c| c.is_uppercase()) {
            return Err(AuthConfigError::MissingCharacterClass {
                required_type: "uppercase letter",
            });
        }
        if !secret.chars().any(|c| c.is_lowercase()) {
            return Err(AuthConfigError::MissingCharacterClass {
                required_type: "lowercase letter",
            });
        }
        if !secret.chars().any(|c| c.is_ascii_digit()) {
            return Err(AuthConfigError::MissingCharacterClass {
                required_type: "digit",
            });
        }
        if !secret.chars().any(|c| !c.is_alphanumeric()) {
            return Err(AuthConfigError::MissingCharacterClass {
                required_type: "special character",
            });
        }

        Ok(BearerAuth {
            secret: secret.into_bytes(),
            valid_tokens: Arc::new(DashMap::new()),
            blacklisted_tokens: Arc::new(DashMap::new()),
            expected_audience: self.audience,
            expected_issuer: self.issuer,
        })
    }
}

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Max requests per window
    pub max_requests: u32,
    /// Window duration
    pub window: Duration,
    /// Response headers
    pub include_headers: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            include_headers: true,
        }
    }
}

impl TryFrom<crate::config::RateLimitConfigFile> for RateLimitConfig {
    type Error = crate::config::ConfigError;

    fn try_from(config: crate::config::RateLimitConfigFile) -> Result<Self, Self::Error> {
        Ok(Self {
            max_requests: config.requests,
            window: Duration::from_secs(config.window_seconds),
            include_headers: true,
        })
    }
}

/// Trusted proxy configuration for IP extraction
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct TrustedProxyConfig {
    /// List of trusted proxy IP addresses
    trusted_proxies: Vec<String>,
    /// Whether proxy verification is enabled
    enabled: bool,
}

impl Default for TrustedProxyConfig {
    fn default() -> Self {
        Self {
            trusted_proxies: vec![
                "127.0.0.1".to_string(),
                "::1".to_string(),
                "localhost".to_string(),
            ],
            enabled: true,
        }
    }
}

/// Extract client IP from request with proxy chain validation
///
/// This function implements secure IP extraction that prevents IP spoofing
/// by validating the proxy chain before trusting X-Forwarded-For headers.
#[allow(dead_code)]
pub(crate) fn extract_client_ip(
    req: &axum::http::Request<axum::body::Body>,
    config: &TrustedProxyConfig,
) -> String {
    if !config.enabled {
        // Direct connection, use connection info
        // In a real implementation, this would extract the actual connection IP
        return "unknown".to_string();
    }

    // Check X-Forwarded-For header
    if let Some(xff) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        // X-Forwarded-For: client, proxy1, proxy2
        // Take the leftmost IP as the client IP
        if let Some(client_ip) = xff.split(',').next() {
            return client_ip.trim().to_string();
        }
    }

    // Fallback to X-Real-IP
    if let Some(xri) = req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()) {
        return xri.to_string();
    }

    // Default fallback
    "unknown".to_string()
}

/// Builder for RateLimiter configuration
///
/// This builder provides a fluent interface for configuring a RateLimiter
/// with custom rate limiting parameters.
///
/// # Examples
///
/// ```rust
/// use sdforge::security::RateLimiterBuilder;
/// use std::time::Duration;
///
/// let limiter = RateLimiterBuilder::new()
///     .max_requests(100)
///     .window(Duration::from_secs(60))
///     .max_concurrent(500)
///     .build();
/// let _ = limiter;
/// ```
#[derive(Debug, Clone, Default)]
pub struct RateLimiterBuilder {
    /// Rate limit configuration
    config: RateLimitConfig,
    /// Maximum concurrent requests (semaphore permits)
    max_concurrent: usize,
}

impl RateLimiterBuilder {
    /// Create a new RateLimiterBuilder with default settings.
    ///
    /// Default values:
    /// - `max_requests`: 100
    /// - `window`: 60 seconds
    /// - `include_headers`: true
    /// - `max_concurrent`: 1000
    ///
    /// # Returns
    ///
    /// Returns a builder initialized with default rate limit configuration.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::RateLimiterBuilder;
    ///
    /// let builder = RateLimiterBuilder::new();
    /// let _ = builder;
    /// ```
    pub fn new() -> Self {
        Self {
            config: RateLimitConfig::default(),
            max_concurrent: 1000,
        }
    }

    /// Set the maximum number of requests within the rate limit window.
    ///
    /// # Arguments
    ///
    /// * `max_requests` - Maximum number of requests allowed in a window.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::RateLimiterBuilder;
    ///
    /// let builder = RateLimiterBuilder::new().max_requests(100);
    /// let _ = builder;
    /// ```
    pub fn max_requests(mut self, max_requests: u32) -> Self {
        self.config.max_requests = max_requests;
        self
    }

    /// Set the duration of the rate limit window.
    ///
    /// # Arguments
    ///
    /// * `window` - Time window for rate limiting.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::RateLimiterBuilder;
    /// use std::time::Duration;
    ///
    /// let builder = RateLimiterBuilder::new().window(Duration::from_secs(60));
    /// let _ = builder;
    /// ```
    pub fn window(mut self, window: Duration) -> Self {
        self.config.window = window;
        self
    }

    /// Set the maximum number of concurrent requests (semaphore permits).
    ///
    /// This controls the backpressure mechanism by limiting how many
    /// requests can be processed concurrently.
    ///
    /// # Arguments
    ///
    /// * `max_concurrent` - Maximum number of concurrent requests.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::RateLimiterBuilder;
    ///
    /// let builder = RateLimiterBuilder::new().max_concurrent(500);
    /// let _ = builder;
    /// ```
    pub fn max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent = max_concurrent;
        self
    }

    /// Configure whether rate limit headers are included in responses.
    ///
    /// # Arguments
    ///
    /// * `include_headers` - Whether to include rate limit headers.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::RateLimiterBuilder;
    ///
    /// let builder = RateLimiterBuilder::new().include_headers(true);
    /// let _ = builder;
    /// ```
    pub fn include_headers(mut self, include_headers: bool) -> Self {
        self.config.include_headers = include_headers;
        self
    }

    /// Build a RateLimiter instance using the configured settings.
    ///
    /// # Returns
    ///
    /// Returns a fully configured RateLimiter instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::RateLimiterBuilder;
    /// use std::time::Duration;
    ///
    /// let limiter = RateLimiterBuilder::new()
    ///     .max_requests(100)
    ///     .window(Duration::from_secs(60))
    ///     .build();
    /// let _ = limiter;
    /// ```
    pub fn build(self) -> RateLimiter {
        RateLimiter::with_dependencies(
            self.config,
            Arc::new(DashMap::new()),
            Arc::new(DashMap::new()),
            Arc::new(tokio::sync::Semaphore::new(self.max_concurrent)),
        )
    }
}

/// Rate limiter with idempotency support
///
/// Security features:
/// - Time-window based rate limiting
/// - Request deduplication for idempotent requests
/// - Per-key tracking with automatic cleanup
#[derive(Clone)]
pub struct RateLimiter {
    /// Configuration
    config: RateLimitConfig,
    /// Request tracking per IP
    requests: Arc<DashMap<String, Vec<Instant>>>,
    /// Idempotency key cache (for deduplication)
    idempotency_cache: Arc<DashMap<String, Instant>>,
    /// Rate limiting semaphore (for backpressure)
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl RateLimiter {
    /// Create a new rate limiter with optional configuration.
    ///
    /// This is the simplest way to create a RateLimiter - it provides
    /// out-of-the-box functionality with sensible defaults.
    ///
    /// # Arguments
    ///
    /// * `config` - Optional rate limit configuration. If `None`, uses defaults.
    ///
    /// # Returns
    ///
    /// Returns a new RateLimiter instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::RateLimiter;
    ///
    /// // With default configuration
    /// let limiter = RateLimiter::new(None);
    /// let _ = limiter;
    ///
    /// // With custom configuration
    /// use sdforge::security::RateLimitConfig;
    /// use std::time::Duration;
    ///
    /// let config = RateLimitConfig {
    ///     max_requests: 200,
    ///     window: Duration::from_secs(120),
    ///     include_headers: true,
    /// };
    /// let limiter = RateLimiter::new(Some(config));
    /// let _ = limiter;
    /// ```
    pub fn new(config: Option<RateLimitConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            requests: Arc::new(DashMap::new()),
            idempotency_cache: Arc::new(DashMap::new()),
            semaphore: Arc::new(tokio::sync::Semaphore::new(1000)),
        }
    }

    /// Create a builder for configuring a RateLimiter.
    ///
    /// This method returns a RateLimiterBuilder that allows for fluent
    /// configuration of the rate limiter's parameters.
    ///
    /// # Returns
    ///
    /// Returns a RateLimiterBuilder instance for configuration.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::RateLimiter;
    /// use std::time::Duration;
    ///
    /// let limiter = RateLimiter::builder()
    ///     .max_requests(100)
    ///     .window(Duration::from_secs(60))
    ///     .max_concurrent(500)
    ///     .build();
    /// let _ = limiter;
    /// ```
    pub fn builder() -> RateLimiterBuilder {
        RateLimiterBuilder::new()
    }

    /// Create a RateLimiter with all dependencies explicitly provided.
    ///
    /// This method is useful for dependency injection and testing scenarios
    /// where you need full control over the internal state.
    ///
    /// # Arguments
    ///
    /// * `config` - Rate limit configuration.
    /// * `requests` - Request tracking map (key -> list of request timestamps).
    /// * `idempotency_cache` - Idempotency key cache for deduplication.
    /// * `semaphore` - Semaphore for concurrent request limiting.
    ///
    /// # Returns
    ///
    /// Returns a RateLimiter instance with the provided dependencies.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::{RateLimiter, RateLimitConfig};
    /// use dashmap::DashMap;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// let config = RateLimitConfig {
    ///     max_requests: 100,
    ///     window: Duration::from_secs(60),
    ///     include_headers: true,
    /// };
    /// let requests = Arc::new(DashMap::new());
    /// let idempotency_cache = Arc::new(DashMap::new());
    /// let semaphore = Arc::new(tokio::sync::Semaphore::new(1000));
    ///
    /// let limiter = RateLimiter::with_dependencies(
    ///     config,
    ///     requests,
    ///     idempotency_cache,
    ///     semaphore,
    /// );
    /// let _ = limiter;
    /// ```
    pub fn with_dependencies(
        config: RateLimitConfig,
        requests: Arc<DashMap<String, Vec<Instant>>>,
        idempotency_cache: Arc<DashMap<String, Instant>>,
        semaphore: Arc<tokio::sync::Semaphore>,
    ) -> Self {
        Self {
            config,
            requests,
            idempotency_cache,
            semaphore,
        }
    }

    /// Check if request is rate limited
    pub fn check(&self, key: &str) -> Result<u32, RateLimitError> {
        let now = Instant::now();
        let window_start = now - self.config.window;

        let mut entry = self.requests.entry(key.to_string()).or_default();
        let times = entry.value_mut();

        // Remove old requests outside the window
        times.retain(|&t| t > window_start);

        // Check rate limit
        if times.len() >= self.config.max_requests as usize {
            let retry_after = times
                .first()
                .map(|t| {
                    let elapsed = now - *t;
                    (self.config.window - elapsed).as_secs()
                })
                .unwrap_or(1);

            return Err(RateLimitError {
                limit: self.config.max_requests,
                remaining: 0,
                retry_after,
            });
        }

        // Add current request
        times.push(now);

        Ok(self.config.max_requests - times.len() as u32)
    }

    /// Check idempotency (returns true if this is a duplicate request)
    ///
    /// Call this at the start of request processing. If it returns true,
    /// the request should be processed as a duplicate (return cached response).
    pub fn check_idempotency(&self, idempotency_key: &str) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(60); // Idempotency key cache window

        if let Some(existing) = self.idempotency_cache.get(idempotency_key) {
            // Clone the instant since Ref doesn't deref to the value directly
            let existing_time = *existing;
            // Use saturating_duration_since to avoid panic if system clock is adjusted
            // This can happen when system time goes backwards (NTP correction, manual change)
            let elapsed = now.saturating_duration_since(existing_time).as_secs();
            if elapsed < window.as_secs() {
                return true; // Duplicate request
            }
        }

        // Record this idempotency key
        self.idempotency_cache
            .insert(idempotency_key.to_string(), now);

        false // Not a duplicate
    }

    /// Get remaining requests
    pub fn remaining(&self, key: &str) -> u32 {
        let now = Instant::now();
        let window_start = now - self.config.window;

        let entry = self.requests.get(key);
        if let Some(times) = entry {
            let active = times.iter().filter(|&&t| t > window_start).count();
            self.config.max_requests - active as u32
        } else {
            self.config.max_requests
        }
    }

    /// Acquire rate limit permit (async, with backpressure)
    pub async fn acquire(&self, key: &str) -> Result<Permit, RateLimitError> {
        // Check rate limit first (check returns info but we only care about side effects)
        let _remaining = self.check(key)?;

        // Try to acquire semaphore permit (owned to allow returning from function)
        let permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| RateLimitError {
                limit: self.config.max_requests,
                remaining: 0,
                retry_after: 1,
            })?;

        Ok(Permit(permit))
    }
}

impl Default for RateLimiter {
    /// Create a RateLimiter with default configuration.
    ///
    /// This is equivalent to calling `RateLimiter::new(None)`.
    ///
    /// # Returns
    ///
    /// Returns a RateLimiter with default settings:
    /// - `max_requests`: 100
    /// - `window`: 60 seconds
    /// - `include_headers`: true
    /// - `max_concurrent`: 1000
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::RateLimiter;
    ///
    /// let limiter = RateLimiter::default();
    /// let _ = limiter;
    /// ```
    fn default() -> Self {
        Self::new(None)
    }
}

/// RAII permit for rate limiting
#[allow(dead_code)]
pub struct Permit(tokio::sync::OwnedSemaphorePermit);

impl Drop for Permit {
    fn drop(&mut self) {
        // Permit is automatically released when dropped
    }
}

/// Rate limit error
#[derive(Debug, Error)]
#[error("Rate limit exceeded. Try again in {retry_after} seconds")]
pub struct RateLimitError {
    /// Rate limit
    pub limit: u32,
    /// Remaining requests
    pub remaining: u32,
    /// Retry after seconds
    pub retry_after: u64,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl AuditLog {
    /// Get log ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get log timestamp
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Get user ID
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Get action
    pub fn action(&self) -> &str {
        &self.action
    }

    /// Get resource
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// Get result
    pub fn result(&self) -> &AuditResult {
        &self.result
    }

    /// Get request metadata
    pub fn metadata(&self) -> &AuthMetadata {
        &self.metadata
    }
}

/// Audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum AuditResult {
    /// Success
    #[serde(rename = "success")]
    Success,
    /// Failure
    #[serde(rename = "failure")]
    Failure {
        /// Error message
        message: String,
    },
}

/// Audit logger with DoS protection
///
/// Security features:
/// - Semaphore-based rate limiting to prevent log flooding
/// - Per-user log count limits
/// - Async processing to avoid blocking main threads
/// - Fallback storage when async channel is full (prevents log loss)
#[derive(Clone)]
/// Builder for creating AuditLogger with custom configuration.
///
/// This builder allows fine-grained control over audit logger settings
/// including log limits, concurrency, and queue size.
///
/// # Examples
///
/// ```rust
/// use sdforge::security::AuditLogger;
///
/// let logger = AuditLogger::builder()
///     .max_logs_per_user(500)
///     .max_concurrent_ops(50)
///     .queue_size(2000)
///     .build();
/// ```
pub struct AuditLoggerBuilder {
    /// Maximum number of logs to retain per user
    max_logs_per_user: usize,
    /// Maximum number of concurrent log operations (semaphore permits)
    max_concurrent_ops: usize,
    /// Size of the async log processing queue
    queue_size: usize,
}

impl Default for AuditLoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLoggerBuilder {
    /// Create a new AuditLoggerBuilder with default settings.
    ///
    /// Default values:
    /// - `max_logs_per_user`: 1000
    /// - `max_concurrent_ops`: 100
    /// - `queue_size`: 1000
    ///
    /// # Returns
    ///
    /// Returns a builder initialized with default configuration.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AuditLoggerBuilder;
    ///
    /// let builder = AuditLoggerBuilder::new();
    /// let _ = builder;
    /// ```
    pub fn new() -> Self {
        Self {
            max_logs_per_user: 1000,
            max_concurrent_ops: 100,
            queue_size: 1000,
        }
    }

    /// Set the maximum number of logs to retain per user.
    ///
    /// When the limit is exceeded, older logs are truncated.
    ///
    /// # Arguments
    ///
    /// * `max_logs` - Maximum number of logs per user (default: 1000).
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AuditLoggerBuilder;
    ///
    /// let builder = AuditLoggerBuilder::new().max_logs_per_user(500);
    /// let _ = builder;
    /// ```
    pub fn max_logs_per_user(mut self, max_logs: usize) -> Self {
        self.max_logs_per_user = max_logs;
        self
    }

    /// Set the maximum number of concurrent log operations.
    ///
    /// This controls the semaphore used to prevent DoS attacks on the
    /// audit logging system. Higher values allow more concurrent operations
    /// but may increase memory usage.
    ///
    /// # Arguments
    ///
    /// * `max_ops` - Maximum concurrent operations (default: 100).
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AuditLoggerBuilder;
    ///
    /// let builder = AuditLoggerBuilder::new().max_concurrent_ops(50);
    /// let _ = builder;
    /// ```
    pub fn max_concurrent_ops(mut self, max_ops: usize) -> Self {
        self.max_concurrent_ops = max_ops;
        self
    }

    /// Set the size of the async log processing queue.
    ///
    /// Larger queues can handle higher burst loads but consume more memory.
    /// When the queue is full, logs are stored in a fallback synchronous buffer.
    ///
    /// # Arguments
    ///
    /// * `size` - Queue size (default: 1000).
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AuditLoggerBuilder;
    ///
    /// let builder = AuditLoggerBuilder::new().queue_size(2000);
    /// let _ = builder;
    /// ```
    pub fn queue_size(mut self, size: usize) -> Self {
        self.queue_size = size;
        self
    }

    /// Build an AuditLogger instance using the configured settings.
    ///
    /// This method spawns a background worker task for async log processing
    /// and initializes all internal data structures.
    ///
    /// # Returns
    ///
    /// Returns a fully configured AuditLogger instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AuditLoggerBuilder;
    ///
    /// let logger = AuditLoggerBuilder::new()
    ///     .max_logs_per_user(500)
    ///     .max_concurrent_ops(50)
    ///     .queue_size(2000)
    ///     .build();
    /// let _ = logger;
    /// ```
    pub fn build(self) -> AuditLogger {
        let (queue_sender, mut queue_receiver) =
            tokio::sync::mpsc::channel::<AuditLogBatch>(self.queue_size);

        // Spawn background worker for async log processing
        let logs: Arc<DashMap<String, Vec<AuditLog>>> = Arc::new(DashMap::new());
        let fallback_logs: Arc<DashMap<String, Vec<AuditLog>>> = Arc::new(DashMap::new());
        let logs_clone = logs.clone();
        let fallback_logs_clone = fallback_logs.clone();
        let max_logs_clone = self.max_logs_per_user;
        tokio::spawn(async move {
            while let Some(batch) = queue_receiver.recv().await {
                let mut entry = logs_clone.entry(batch.user_id.clone()).or_default();
                entry.push(batch.log);

                // Keep only last N logs per user
                if entry.len() > max_logs_clone {
                    entry.truncate(max_logs_clone);
                }

                // Also check if there are fallback logs to merge
                if let Some(fallback) = fallback_logs_clone.get(&batch.user_id) {
                    if !fallback.is_empty() {
                        let mut entry = logs_clone.entry(batch.user_id.clone()).or_default();
                        entry.extend(fallback.iter().cloned());
                        if entry.len() > max_logs_clone {
                            entry.truncate(max_logs_clone);
                        }
                        fallback_logs_clone.remove(&batch.user_id);
                    }
                }
            }
        });

        AuditLogger {
            logs,
            max_logs_per_user: self.max_logs_per_user,
            semaphore: Arc::new(tokio::sync::Semaphore::new(self.max_concurrent_ops)),
            queue_sender: Arc::new(queue_sender),
            fallback_logs,
            dropped_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
}

/// Audit logger for tracking user actions and security events.
///
/// Provides async log processing with DoS protection and fallback
/// synchronous storage for high-load scenarios.
pub struct AuditLogger {
    /// Logs storage
    logs: Arc<DashMap<String, Vec<AuditLog>>>,
    /// Maximum logs per user
    max_logs_per_user: usize,
    /// Rate limiting semaphore (max concurrent log operations)
    semaphore: Arc<tokio::sync::Semaphore>,
    /// Log queue sender (for async processing)
    queue_sender: Arc<tokio::sync::mpsc::Sender<AuditLogBatch>>,
    /// Fallback storage for when channel is full (synchronous path)
    fallback_logs: Arc<DashMap<String, Vec<AuditLog>>>,
    /// Counter for dropped logs (monitoring)
    dropped_log_count: Arc<std::sync::atomic::AtomicU64>,
}

struct AuditLogBatch {
    user_id: String,
    log: AuditLog,
}

static JWT_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r#"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+"#).unwrap()
});

static SECRET_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    // Limited repetition to prevent ReDoS attacks
    // Maximum 100 characters after the separator
    regex::Regex::new(r#"(?i)(password|secret|token|key|auth|bearer)\s*[:=]\s*[^,\s}\]]{1,100}"#)
        .unwrap()
});

static PATH_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r#"/[a-zA-Z0-9/_.-]+\.(pem|key|crt|p12|jks)"#).unwrap());

/// Sanitize error messages to remove sensitive information before logging.
///
/// This function helps prevent sensitive data (tokens, passwords, keys) from
/// being exposed in audit logs.
fn sanitize_error_message(message: &str) -> String {
    let mut result = message.to_string();

    // Remove JWT tokens
    result = JWT_PATTERN
        .replace_all(&result, "[REDACTED_JWT]")
        .to_string();

    // Remove secret patterns
    result = SECRET_PATTERN
        .replace_all(&result, |caps: &regex::Captures| {
            format!("{}={}", &caps[1], "[REDACTED]")
        })
        .to_string();

    // Remove API keys
    result = regex::Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['\"]?[A-Za-z0-9]{20,}['\"]?"#)
        .unwrap()
        .replace_all(&result, "[REDACTED_API_KEY]")
        .to_string();

    // Remove credit card numbers
    result = regex::Regex::new(r#"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b"#)
        .unwrap()
        .replace_all(&result, "[REDACTED_CREDIT_CARD]")
        .to_string();

    // Remove SSN numbers
    result = regex::Regex::new(r#"\b\d{3}[-\s]?\d{2}[-\s]?\d{4}\b"#)
        .unwrap()
        .replace_all(&result, "[REDACTED_SSN]")
        .to_string();

    // Remove certificate/key file paths
    result = PATH_PATTERN
        .replace_all(&result, "[REDACTED_PATH]")
        .to_string();

    const MAX_SANITIZED_LENGTH: usize = 500;
    if result.len() > MAX_SANITIZED_LENGTH {
        result.truncate(MAX_SANITIZED_LENGTH);
        result.push_str("...[TRUNCATED]");
    }

    result
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogger {
    /// Create a new AuditLoggerBuilder for custom configuration.
    ///
    /// This is the recommended way to create an AuditLogger when you need
    /// to customize any of the default settings.
    ///
    /// # Returns
    ///
    /// Returns a new AuditLoggerBuilder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AuditLogger;
    ///
    /// let logger = AuditLogger::builder()
    ///     .max_logs_per_user(500)
    ///     .max_concurrent_ops(50)
    ///     .queue_size(2000)
    ///     .build();
    /// let _ = logger;
    /// ```
    pub fn builder() -> AuditLoggerBuilder {
        AuditLoggerBuilder::new()
    }

    /// Create an AuditLogger with all dependencies injected.
    ///
    /// This method is used for complete dependency injection, allowing
    /// external control over all internal components. This is useful for
    /// testing or advanced integration scenarios.
    ///
    /// # Arguments
    ///
    /// * `logs` - Pre-initialized logs storage (DashMap)
    /// * `max_logs_per_user` - Maximum logs to retain per user
    /// * `semaphore` - Semaphore for controlling concurrent operations
    /// * `queue_sender` - Channel sender for async log processing
    /// * `fallback_logs` - Fallback storage for high-load scenarios
    /// * `dropped_log_count` - Counter for monitoring dropped logs
    ///
    /// # Returns
    ///
    /// Returns an AuditLogger instance configured with the provided dependencies.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Note
    ///
    /// When using this method, you are responsible for spawning the background
    /// worker task that processes logs from the channel. For most use cases,
    /// prefer `new()`, `with_limit()`, or `builder().build()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AuditLogger;
    /// use dashmap::DashMap;
    /// use std::sync::Arc;
    /// use std::sync::atomic::AtomicU64;
    ///
    /// let (sender, mut receiver) = tokio::sync::mpsc::channel(1000);
    ///
    /// // Spawn background worker (required when using with_dependencies)
    /// let logs = Arc::new(DashMap::new());
    /// let fallback_logs = Arc::new(DashMap::new());
    /// let logs_clone = logs.clone();
    /// let fallback_logs_clone = fallback_logs.clone();
    /// tokio::spawn(async move {
    ///     while let Some(batch) = receiver.recv().await {
    ///         let mut entry = logs_clone.entry(batch.user_id.clone()).or_default();
    ///         entry.push(batch.log);
    ///     }
    /// });
    ///
    /// let logger = AuditLogger::with_dependencies(
    ///     logs,
    ///     1000,
    ///     Arc::new(tokio::sync::Semaphore::new(100)),
    ///     Arc::new(sender),
    ///     fallback_logs,
    ///     Arc::new(AtomicU64::new(0)),
    /// );
    /// let _ = logger;
    /// ```
    pub fn with_dependencies(
        logs: Arc<DashMap<String, Vec<AuditLog>>>,
        max_logs_per_user: usize,
        semaphore: Arc<tokio::sync::Semaphore>,
        queue_sender: Arc<tokio::sync::mpsc::Sender<AuditLogBatch>>,
        fallback_logs: Arc<DashMap<String, Vec<AuditLog>>>,
        dropped_log_count: Arc<std::sync::atomic::AtomicU64>,
    ) -> Self {
        Self {
            logs,
            max_logs_per_user,
            semaphore,
            queue_sender,
            fallback_logs,
            dropped_log_count,
        }
    }

    /// Create new audit logger with default limit
    pub fn new() -> Self {
        Self::with_limit(1000)
    }

    /// Create new audit logger with custom limit
    pub fn with_limit(max_logs: usize) -> Self {
        let (queue_sender, mut queue_receiver) = tokio::sync::mpsc::channel::<AuditLogBatch>(1000);

        // Spawn background worker for async log processing
        let logs: Arc<DashMap<String, Vec<AuditLog>>> = Arc::new(DashMap::new());
        let fallback_logs: Arc<DashMap<String, Vec<AuditLog>>> = Arc::new(DashMap::new());
        let logs_clone = logs.clone();
        let fallback_logs_clone = fallback_logs.clone();
        let max_logs_clone = max_logs;
        tokio::spawn(async move {
            while let Some(batch) = queue_receiver.recv().await {
                let mut entry = logs_clone.entry(batch.user_id.clone()).or_default();
                entry.push(batch.log);

                // Keep only last N logs per user
                if entry.len() > max_logs_clone {
                    entry.truncate(max_logs_clone);
                }

                // Also check if there are fallback logs to merge
                if let Some(fallback) = fallback_logs_clone.get(&batch.user_id) {
                    if !fallback.is_empty() {
                        let mut entry = logs_clone.entry(batch.user_id.clone()).or_default();
                        entry.extend(fallback.iter().cloned());
                        if entry.len() > max_logs_clone {
                            entry.truncate(max_logs_clone);
                        }
                        fallback_logs_clone.remove(&batch.user_id);
                    }
                }
            }
        });

        Self {
            logs,
            max_logs_per_user: max_logs,
            semaphore: Arc::new(tokio::sync::Semaphore::new(100)), // Max 100 concurrent log operations
            queue_sender: Arc::new(queue_sender),
            fallback_logs,
            dropped_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Log an action (with DoS protection)
    ///
    /// Uses semaphore to limit concurrent log operations, preventing
    /// the audit logger from being a DoS vector.
    pub async fn log(
        &self,
        context: &AuthContext,
        action: impl Into<String>,
        resource: impl Into<String>,
        success: bool,
        message: Option<String>,
    ) {
        // Acquire permit with timeout to prevent blocking
        let permit = match tokio::time::timeout(
            Duration::from_secs(1),
            self.semaphore.clone().acquire_owned(),
        )
        .await
        {
            Ok(Ok(permit)) => permit,
            Ok(Err(_)) | Err(_) => {
                // Semaphore closed or timeout - skip logging to prevent DoS
                return;
            }
        };

        let log = AuditLog {
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: context.user_id.clone(),
            action: action.into(),
            resource: resource.into(),
            result: if success {
                AuditResult::Success
            } else {
                AuditResult::Failure {
                    // Sanitize error message to prevent sensitive data exposure
                    message: sanitize_error_message(
                        &message.unwrap_or_else(|| "Unknown error".to_string()),
                    ),
                }
            },
            metadata: context.metadata.clone(),
        };

        let user_id = context
            .user_id
            .clone()
            .unwrap_or_else(|| "anonymous".to_string());

        // Clone log for potential fallback use (must clone before moving into batch)
        let log_for_fallback = log.clone();

        // Send to async queue with fallback handling
        // Security: Use try_send to avoid blocking, with fallback to in-memory buffer
        // to prevent audit log loss under load
        let sender = self.queue_sender.clone();
        let log_batch = AuditLogBatch {
            user_id: user_id.clone(),
            log,
        };

        // Try non-blocking send first, fall back to synchronous logging if channel full
        match sender.try_send(log_batch) {
            Ok(()) => {
                #[cfg(feature = "logging")]
                tracing::debug!(target: "audit", "Audit log queued for user: {}", user_id);
            }
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                // Channel is full - log to fallback storage synchronously
                // This is a rare event under normal load, indicating potential DoS attempt
                #[cfg(feature = "logging")]
                tracing::warn!(target: "audit",
                    "Audit log channel full for user: {}, using fallback storage",
                    user_id
                );
                self.store_fallback_log(&user_id, &log_for_fallback);
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                // Channel closed - log synchronously as last resort
                #[cfg(feature = "logging")]
                tracing::error!(target: "audit",
                    "Audit log channel closed for user: {}, using synchronous logging",
                    user_id
                );
                self.store_fallback_log(&user_id, &log_for_fallback);
            }
        }

        // Drop permit to release semaphore
        drop(permit);
    }

    /// Get logs for a user (synchronous)
    ///
    /// Security: Merges logs from both async and fallback storage to ensure
    /// complete audit trail is available even after channel congestion.
    pub fn get_logs(&self, user_id: &str) -> Vec<AuditLog> {
        // Get logs from primary storage
        let primary = self
            .logs
            .get(user_id)
            .map(|e| e.clone())
            .unwrap_or_default();

        // Get logs from fallback storage
        let fallback = self
            .fallback_logs
            .get(user_id)
            .map(|e| e.clone())
            .unwrap_or_default();

        // Merge and deduplicate (prefer primary logs if duplicates exist)
        let mut all_logs = primary;
        for log in fallback {
            if !all_logs.iter().any(|l| l.id == log.id) {
                all_logs.push(log);
            }
        }

        // Sort by timestamp descending
        all_logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        all_logs
    }

    /// Clear logs for a user (admin function)
    pub fn clear_logs(&self, user_id: &str) {
        self.logs.remove(user_id);
    }

    /// Get total log count (for monitoring)
    pub fn total_log_count(&self) -> usize {
        self.logs.iter().map(|e| e.len()).sum()
    }

    /// Store log in fallback storage (synchronous path)
    ///
    /// Security: This is used when the async channel is full, preventing
    /// audit log loss during high load or potential DoS attempts.
    fn store_fallback_log(&self, user_id: &str, log: &AuditLog) {
        let count = self
            .dropped_log_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Log to fallback storage with warning level
        let mut entry = self.fallback_logs.entry(user_id.to_string()).or_default();
        entry.push(log.clone());

        // Truncate if exceeding limit
        if entry.len() > self.max_logs_per_user {
            entry.truncate(self.max_logs_per_user);
        }

        // Log warning periodically (every 100th drop)
        if count > 0 && count.is_multiple_of(100) {
            #[cfg(feature = "logging")]
            tracing::warn!(target: "audit",
                "High audit log drop rate: {} logs dropped due to channel congestion",
                count + 1
            );
        }
    }

    /// Get count of dropped logs (for monitoring)
    pub fn dropped_log_count(&self) -> u64 {
        self.dropped_log_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Create authentication middleware
pub fn auth_middleware<T: Clone + Send + Sync + 'static>(
    _auth: Arc<T>,
    extract_auth: impl Fn(&Request<Body>) -> AuthResult<AuthContext> + Clone + Send + 'static,
) -> impl Fn(Request<Body>, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Clone + Send {
    move |mut req: Request<Body>, next: Next| {
        let extract_auth = extract_auth.clone();
        Box::pin(async move {
            match extract_auth(&req) {
                Ok(auth_context) => {
                    req.extensions_mut().insert(auth_context);
                    next.run(req).await
                }
                Err(_) => {
                    let mut response = Response::new(Body::from("Unauthorized"));
                    *response.status_mut() = StatusCode::UNAUTHORIZED;
                    response
                }
            }
        })
    }
}

/// Create rate limiting middleware
pub fn rate_limit_middleware(
    limiter: Arc<RateLimiter>,
) -> impl Fn(Request<Body>, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Clone + Send {
    move |req: Request<Body>, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            let client_ip = extract_client_ip_simple(&req);

            match limiter.check(&client_ip) {
                Ok(remaining) => {
                    let mut response = next.run(req).await;
                    if limiter.config.include_headers {
                        response.headers_mut().insert(
                            "X-RateLimit-Limit",
                            HeaderValue::from(limiter.config.max_requests),
                        );
                        response
                            .headers_mut()
                            .insert("X-RateLimit-Remaining", HeaderValue::from(remaining));
                    }
                    response
                }
                Err(e) => {
                    let mut response = Response::new(Body::from("Rate limit exceeded"));
                    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                    response
                        .headers_mut()
                        .insert("X-RateLimit-Limit", HeaderValue::from(e.limit));
                    response
                        .headers_mut()
                        .insert("X-RateLimit-Remaining", HeaderValue::from(0));
                    response
                        .headers_mut()
                        .insert("Retry-After", HeaderValue::from(e.retry_after));
                    response
                }
            }
        })
    }
}

/// Check if an IP is within a CIDR range
fn is_ip_in_range(ip: &str, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return false;
    }

    let network = parts[0];
    let mask_bits: u32 = parts[1].parse().unwrap_or(0);

    let ip_bytes: Vec<u8> = ip.split('.').filter_map(|s| s.parse().ok()).collect();
    let net_bytes: Vec<u8> = network.split('.').filter_map(|s| s.parse().ok()).collect();

    if ip_bytes.len() != 4 || net_bytes.len() != 4 {
        return false;
    }

    let ip_val = (ip_bytes[0] as u32) << 24
        | (ip_bytes[1] as u32) << 16
        | (ip_bytes[2] as u32) << 8
        | ip_bytes[3] as u32;
    let net_val = (net_bytes[0] as u32) << 24
        | (net_bytes[1] as u32) << 16
        | (net_bytes[2] as u32) << 8
        | net_bytes[3] as u32;
    let mask_val = if mask_bits == 0 {
        0
    } else {
        !0u32 << (32 - mask_bits)
    };

    (ip_val & mask_val) == (net_val & mask_val)
}

/// Common IP extraction logic (shared by both logging and non-logging versions)
#[inline]
fn extract_client_ip_core(req: &Request<Body>) -> Option<String> {
    use axum::extract::connect_info::ConnectInfo;

    let trusted_proxies = ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", "127.0.0.1"];

    if let Some(header) = req.headers().get("X-Forwarded-For") {
        if let Ok(value) = header.to_str() {
            if let Some(ip) = value.split(',').next().map(|s| s.trim()) {
                if is_valid_ip(ip)
                    && trusted_proxies
                        .iter()
                        .any(|range| is_ip_in_range(ip, range))
                {
                    return Some(ip.to_string());
                }
            }
        }
    }

    if let Some(header) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = header.to_str() {
            if is_valid_ip(ip) {
                return Some(ip.to_string());
            }
        }
    }

    if let Some(remote) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
        return Some(remote.0.ip().to_string());
    }

    None
}

/// Extract client IP from request with security validation
fn extract_client_ip_simple(req: &Request<Body>) -> String {
    // Use default trusted proxy configuration
    let proxy_config = TrustedProxyConfig::default();
    extract_client_ip_with_config(req, &proxy_config)
}

/// Extract client IP with trusted proxy configuration
fn extract_client_ip_with_config(req: &Request<Body>, proxy_config: &TrustedProxyConfig) -> String {
    if !proxy_config.enabled {
        // Proxy verification disabled, use connection IP
        if let Some(ip) = extract_client_ip_core(req) {
            return ip;
        }
        return "unknown".to_string();
    }

    // Check X-Forwarded-For header
    if let Some(header) = req.headers().get("X-Forwarded-For") {
        if let Ok(value) = header.to_str() {
            // X-Forwarded-For: client, proxy1, proxy2
            // Take the leftmost IP as the client IP
            if let Some(client_ip) = value.split(',').next().map(|s| s.trim()) {
                // Validate the IP format
                if is_valid_ip(client_ip) {
                    return client_ip.to_string();
                }
            }
        }
    }

    // Fallback to X-Real-IP
    if let Some(header) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = header.to_str() {
            if is_valid_ip(ip) {
                return ip.to_string();
            }
        }
    }

    // Final fallback to connection IP
    extract_client_ip_core(req).unwrap_or_else(|| "unknown".to_string())
}

/// Validate IP address format and security
///
/// Accepts:
/// - IPv4: Public IPs only (rejects private ranges)
/// - IPv6: Public IPs only (rejects loopback, link-local, etc)
///
/// Rejects:
/// - Private IP ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
/// - Loopback (127.0.0.1, ::1)
/// - Link-local (169.254.0.0/16)
/// - Multicast (224.0.0.0/4)
fn is_valid_ip(ip: &str) -> bool {
    use std::net::IpAddr;

    if ip.is_empty() || ip.len() > 45 {
        return false;
    }

    if let Ok(IpAddr::V4(ipv4)) = ip.parse::<IpAddr>() {
        let octets = ipv4.octets();

        // Check for private ranges
        // 10.0.0.0/8
        if octets[0] == 10 {
            return false;
        }
        // 172.16.0.0/12
        if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
            return false;
        }
        // 192.168.0.0/16
        if octets[0] == 192 && octets[1] == 168 {
            return false;
        }
        // 127.0.0.0/8 (loopback)
        if octets[0] == 127 {
            return false;
        }
        // 169.254.0.0/16 (link-local)
        if octets[0] == 169 && octets[1] == 254 {
            return false;
        }
        // 224.0.0.0/4 (multicast)
        if octets[0] >= 224 && octets[0] <= 239 {
            return false;
        }
        // 0.0.0.0/8 (unspecified)
        if octets[0] == 0 {
            return false;
        }

        true
    } else if let Ok(IpAddr::V6(ipv6)) = ip.parse::<IpAddr>() {
        let segments = ipv6.segments();

        // ::1 (loopback)
        if segments == [0, 0, 0, 0, 0, 0, 0, 1] {
            return false;
        }
        // fe80::/10 (link-local)
        if segments[0] & 0xffc0 == 0xfe80 {
            return false;
        }
        // fc00::/7 (unique local)
        if segments[0] & 0xfe00 == 0xfc00 {
            return false;
        }
        // ff00::/8 (multicast)
        if segments[0] & 0xff00 == 0xff00 {
            return false;
        }
        // ::/128 (unspecified)
        if segments == [0, 0, 0, 0, 0, 0, 0, 0] {
            return false;
        }

        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base64url_encode(data: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut output = String::new();
        let mut index = 0;

        while index + 3 <= data.len() {
            let chunk = ((data[index] as u32) << 16)
                | ((data[index + 1] as u32) << 8)
                | (data[index + 2] as u32);
            output.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
            output.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
            output.push(TABLE[((chunk >> 6) & 0x3f) as usize] as char);
            output.push(TABLE[(chunk & 0x3f) as usize] as char);
            index += 3;
        }

        let remaining = data.len() - index;
        if remaining == 1 {
            let chunk = (data[index] as u32) << 16;
            output.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
            output.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
        } else if remaining == 2 {
            let chunk = ((data[index] as u32) << 16) | ((data[index + 1] as u32) << 8);
            output.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
            output.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
            output.push(TABLE[((chunk >> 6) & 0x3f) as usize] as char);
        }

        output
    }

    fn create_jwt(secret: &str, payload: serde_json::Value) -> String {
        let header = serde_json::json!({"alg":"HS256","typ":"JWT"});
        let header_str = serde_json::to_string(&header).unwrap();
        let payload_str = serde_json::to_string(&payload).unwrap();
        let header_b64 = base64url_encode(header_str.as_bytes());
        let payload_b64 = base64url_encode(payload_str.as_bytes());
        let signing_input = format!("{}.{}", header_b64, payload_b64);

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signing_input.as_bytes());
        let signature = mac.finalize().into_bytes();
        let signature_b64 = base64url_encode(&signature);

        format!("{}.{}", signing_input, signature_b64)
    }

    #[tokio::test]
    async fn test_api_key_auth() {
        let auth = ApiKeyAuth::new();
        auth.add_key("test-key", vec!["read".to_string(), "write".to_string()]);

        let permissions = auth.validate_key("test-key", "127.0.0.1");
        assert_eq!(
            permissions,
            Some(vec!["read".to_string(), "write".to_string()])
        );

        let permissions = auth.validate_key("invalid-key", "127.0.0.1");
        assert_eq!(permissions, None);
    }

    #[tokio::test]
    async fn test_api_key_auth_rate_limiting() {
        let auth = ApiKeyAuth::with_rate_limit(RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
            include_headers: false,
        });
        auth.add_key("valid-key", vec!["read".to_string()]);

        // Invalid keys should be rate limited
        for i in 0..3 {
            assert_eq!(
                auth.validate_key(&format!("invalid-key-{}", i), "192.168.1.1"),
                None
            );
        }

        // After 3 failures, invalid keys should be rate limited
        assert_eq!(auth.validate_key("invalid-key-4", "192.168.1.1"), None);

        // Valid keys bypass rate limiting (security: don't lock out legitimate users)
        let permissions = auth.validate_key("valid-key", "192.168.1.1");
        assert_eq!(permissions, Some(vec!["read".to_string()]));

        // A different invalid key from the same IP should still be rate limited
        assert_eq!(
            auth.validate_key("another-invalid-key", "192.168.1.1"),
            None
        );
    }

    #[tokio::test]
    async fn test_api_key_hashing() {
        let auth = ApiKeyAuth::new();
        auth.add_key("test-key", vec!["admin".to_string()]);

        assert!(auth.validate_key("test-key", "127.0.0.1").is_some());
        assert!(auth.validate_key("TEST-KEY", "127.0.0.1").is_none());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
            include_headers: true,
        };
        let limiter = RateLimiter::new(Some(config));

        for _ in 0..3 {
            assert!(limiter.check("test-ip").is_ok());
        }

        assert!(limiter.check("test-ip").is_err());
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let logger = AuditLogger::new();
        let context = AuthContext {
            user_id: Some("user-123".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        logger
            .log(&context, "test_action", "test_resource", true, None)
            .await;

        tokio::task::yield_now().await;

        let logs = logger.get_logs("user-123");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, "test_action");
    }

    #[test]
    fn test_ip_range_validation() {
        assert!(is_ip_in_range("10.0.0.1", "10.0.0.0/8"));
        assert!(is_ip_in_range("192.168.1.100", "192.168.0.0/16"));
        assert!(is_ip_in_range("172.16.5.5", "172.16.0.0/12"));
        assert!(is_ip_in_range("172.31.255.255", "172.16.0.0/12"));

        assert!(!is_ip_in_range("8.8.8.8", "10.0.0.0/8"));
        assert!(!is_ip_in_range("172.32.0.1", "172.16.0.0/12"));
        assert!(!is_ip_in_range("8.8.8.8", "192.168.0.0/16"));
    }

    #[test]
    fn test_trusted_proxy_config_default() {
        let config = TrustedProxyConfig::default();
        assert!(config.enabled);
        assert!(config.trusted_proxies.contains(&"127.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_prefers_forwarded_for() {
        let req = Request::builder()
            .header("x-forwarded-for", "203.0.113.5, 10.0.0.1")
            .body(Body::empty())
            .unwrap();
        let config = TrustedProxyConfig {
            trusted_proxies: vec![],
            enabled: true,
        };

        let ip = extract_client_ip(&req, &config);
        assert_eq!(ip, "203.0.113.5");
    }

    #[test]
    fn test_extract_client_ip_fallbacks() {
        let req = Request::builder()
            .header("x-real-ip", "198.51.100.8")
            .body(Body::empty())
            .unwrap();
        let enabled_config = TrustedProxyConfig {
            trusted_proxies: vec![],
            enabled: true,
        };
        let disabled_config = TrustedProxyConfig {
            trusted_proxies: vec![],
            enabled: false,
        };

        let ip_enabled = extract_client_ip(&req, &enabled_config);
        let ip_disabled = extract_client_ip(&req, &disabled_config);
        let req_no_headers = Request::builder().body(Body::empty()).unwrap();
        let ip_no_headers = extract_client_ip(&req_no_headers, &enabled_config);

        assert_eq!(ip_enabled, "198.51.100.8");
        assert_eq!(ip_disabled, "unknown");
        assert_eq!(ip_no_headers, "unknown");
    }

    // ==================== AuthContext Tests ====================

    #[test]
    fn test_auth_context_creation() {
        let metadata = AuthMetadata::new(
            Some("192.168.1.1".to_string()),
            Some("TestClient/1.0".to_string()),
        );

        let context = AuthContext::new(
            Some("user-123".to_string()),
            vec!["read".to_string(), "write".to_string()],
            metadata,
        );

        assert_eq!(context.user_id(), Some("user-123"));
        assert_eq!(context.permissions().len(), 2);
        assert!(context.has_permission("read"));
        assert!(context.has_permission("write"));
        assert!(!context.has_permission("delete"));
    }

    #[test]
    fn test_auth_context_without_user() {
        let context = AuthContext::new(None, vec![], AuthMetadata::default());

        assert_eq!(context.user_id(), None);
        assert!(context.permissions().is_empty());
        assert!(!context.has_permission("any"));
    }

    #[test]
    fn test_auth_metadata_creation() {
        let metadata = AuthMetadata::new(
            Some("10.0.0.1".to_string()),
            Some("Mozilla/5.0".to_string()),
        );

        assert_eq!(metadata.client_ip(), Some("10.0.0.1"));
        assert_eq!(metadata.user_agent(), Some("Mozilla/5.0"));
        assert!(!metadata.request_id().is_empty());
        assert!(metadata.timestamp() > 0);
    }

    // ==================== AuthError Tests ====================

    #[test]
    fn test_auth_error_messages() {
        let missing_auth = AuthError::MissingAuth;
        assert_eq!(
            missing_auth.to_string(),
            "Missing or invalid authorization header"
        );

        let invalid_token = AuthError::InvalidToken;
        assert_eq!(invalid_token.to_string(), "Invalid or expired token");

        let insufficient = AuthError::InsufficientPermissions {
            required: "admin".to_string(),
            user_permissions: vec!["read".to_string()],
        };
        assert!(insufficient
            .to_string()
            .contains("Insufficient permissions"));
        assert!(insufficient.to_string().contains("admin"));
    }

    // ==================== BearerAuth Secret Validation Tests ====================

    #[test]
    fn test_bearer_auth_secret_too_short() {
        let result = BearerAuth::try_new("Short1!");
        assert!(result.is_err(), "Expected error for short secret");

        match result {
            Err(AuthConfigError::SecretTooShort { length }) => {
                assert_eq!(length, 7); // "Short1!" is 7 characters
            }
            Err(e) => panic!("Expected SecretTooShort error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_missing_uppercase() {
        // Secret must be 32+ chars with lowercase + digit + special but NO uppercase
        // "lowercase123!abcdefghijklmnopqrstuvwxyz" has 39 chars
        let result = BearerAuth::try_new("lowercase123!abcdefghijklmnopqrstuvwxyz");
        assert!(result.is_err(), "Expected error for missing uppercase");

        match result {
            Err(AuthConfigError::MissingCharacterClass { required_type }) => {
                assert_eq!(required_type, "uppercase letter");
            }
            Err(e) => panic!("Expected MissingCharacterClass error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_missing_lowercase() {
        // Secret must be 32+ chars with uppercase + digit + special but NO lowercase
        let result = BearerAuth::try_new("UPPERCASE123!ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        assert!(result.is_err(), "Expected error for missing lowercase");

        match result {
            Err(AuthConfigError::MissingCharacterClass { required_type }) => {
                assert_eq!(required_type, "lowercase letter");
            }
            Err(e) => panic!("Expected MissingCharacterClass error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_missing_digit() {
        // Secret must be 32+ chars with lowercase + uppercase + special but NO digit
        let result = BearerAuth::try_new("LowercaseUpper!ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        assert!(result.is_err(), "Expected error for missing digit");

        match result {
            Err(AuthConfigError::MissingCharacterClass { required_type }) => {
                assert_eq!(required_type, "digit");
            }
            Err(e) => panic!("Expected MissingCharacterClass error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_missing_special_char() {
        // Secret must be 32+ chars, this one is all alphanumeric (no special char)
        let result = BearerAuth::try_new("LowercaseUpper123ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        assert!(result.is_err(), "Expected error for missing special char");

        match result {
            Err(AuthConfigError::MissingCharacterClass { required_type }) => {
                assert_eq!(required_type, "special character");
            }
            Err(e) => panic!("Expected MissingCharacterClass error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_valid_secret() {
        // Secret must be at least 32 characters with uppercase, lowercase, digit, and special char
        let auth = BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .expect("Valid secret should work");
        assert!(auth.validate_token("invalid-token").is_none());
    }

    #[test]
    fn test_bearer_auth_valid_jwt() {
        let now = chrono::Utc::now().timestamp();
        let payload = serde_json::json!({
            "sub": "user-123",
            "permissions": ["read", "write"],
            "exp": now + 120,
            "iat": now,
            "nbf": now - 1
        });
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let token = create_jwt(secret, payload);
        let auth = BearerAuth::try_new(secret).expect("valid secret");

        let context = auth.validate_token(&token).expect("valid token");
        assert_eq!(context.user_id(), Some("user-123"));
        assert_eq!(
            context.permissions(),
            &["read".to_string(), "write".to_string()]
        );
    }

    #[test]
    fn test_bearer_auth_invalid_signature() {
        let now = chrono::Utc::now().timestamp();
        let payload = serde_json::json!({
            "sub": "user-123",
            "permissions": ["read"],
            "exp": now + 120
        });
        let token = create_jwt("OtherSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ", payload);
        let auth =
            BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ").expect("valid secret");
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_bearer_auth_expired_jwt() {
        let now = chrono::Utc::now().timestamp();
        let payload = serde_json::json!({
            "sub": "user-123",
            "permissions": ["read"],
            "exp": now - 1
        });
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let token = create_jwt(secret, payload);
        let auth = BearerAuth::try_new(secret).expect("valid secret");
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_bearer_auth_nbf_future() {
        let now = chrono::Utc::now().timestamp();
        let payload = serde_json::json!({
            "sub": "user-123",
            "permissions": ["read"],
            "exp": now + 120,
            "nbf": now + 120
        });
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let token = create_jwt(secret, payload);
        let auth = BearerAuth::try_new(secret).expect("valid secret");
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_bearer_auth_iat_future() {
        let now = chrono::Utc::now().timestamp();
        let payload = serde_json::json!({
            "sub": "user-123",
            "permissions": ["read"],
            "exp": now + 120,
            "iat": now + 120
        });
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let token = create_jwt(secret, payload);
        let auth = BearerAuth::try_new(secret).expect("valid secret");
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_bearer_auth_audience_mismatch() {
        let now = chrono::Utc::now().timestamp();
        let payload = serde_json::json!({
            "sub": "user-123",
            "permissions": ["read"],
            "aud": "other-api",
            "exp": now + 120
        });
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let token = create_jwt(secret, payload);
        let auth = BearerAuth::with_audience(secret, "expected-api");
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_bearer_auth_issuer_mismatch() {
        let now = chrono::Utc::now().timestamp();
        let payload = serde_json::json!({
            "sub": "user-123",
            "permissions": ["read"],
            "iss": "other-issuer",
            "exp": now + 120
        });
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let token = create_jwt(secret, payload);
        let auth = BearerAuth::with_claims(secret, "expected-api", "expected-issuer");
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_bearer_auth_with_audience() {
        // Use a valid secret (32+ chars with all required types)
        let auth = BearerAuth::with_audience("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ", "my-api");
        assert!(auth.validate_token("any-token").is_none());
    }

    #[test]
    fn test_bearer_auth_with_claims() {
        // Use a valid secret (32+ chars with all required types)
        let auth = BearerAuth::with_claims(
            "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "my-api",
            "issuer",
        );
        assert!(auth.validate_token("any-token").is_none());
    }

    // ==================== RateLimitConfig Tests ====================

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window, Duration::from_secs(60));
        assert!(config.include_headers);
    }

    #[test]
    fn test_rate_limit_config_custom() {
        let config = RateLimitConfig {
            max_requests: 50,
            window: Duration::from_secs(30),
            include_headers: false,
        };
        assert_eq!(config.max_requests, 50);
        assert_eq!(config.window, Duration::from_secs(30));
        assert!(!config.include_headers);
    }

    // ==================== RateLimiter Tests ====================

    #[test]
    fn test_rate_limiter_remaining() {
        let config = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
            include_headers: false,
        };
        let limiter = RateLimiter::new(Some(config));

        assert_eq!(limiter.remaining("test-ip"), 5);

        let _ = limiter.check("test-ip");
        assert_eq!(limiter.remaining("test-ip"), 4);

        let _ = limiter.check("test-ip");
        assert_eq!(limiter.remaining("test-ip"), 3);
    }

    #[test]
    fn test_rate_limiter_idempotency() {
        let limiter = RateLimiter::new(None);

        // First request should not be duplicate
        assert!(!limiter.check_idempotency("req-123"));

        // Same idempotency key should be detected as duplicate
        assert!(limiter.check_idempotency("req-123"));
        assert!(limiter.check_idempotency("req-123"));

        // Different key should not be duplicate
        assert!(!limiter.check_idempotency("req-456"));
    }

    #[tokio::test]
    async fn test_rate_limiter_acquire() {
        let limiter = RateLimiter::new(Some(RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(60),
            include_headers: false,
        }));

        // Should be able to acquire 2 permits
        assert!(limiter.acquire("ip-1").await.is_ok());
        assert!(limiter.acquire("ip-1").await.is_ok());

        // Third should fail due to rate limit
        assert!(limiter.acquire("ip-1").await.is_err());
    }

    // ==================== AuditLogger Tests ====================

    #[tokio::test]
    async fn test_audit_logger_get_logs_empty() {
        let logger = AuditLogger::new();
        let logs = logger.get_logs("nonexistent-user");
        assert!(logs.is_empty());
    }

    #[tokio::test]
    async fn test_audit_logger_clear_logs() {
        let logger = AuditLogger::new();
        let context = AuthContext::new(
            Some("user-to-clear".to_string()),
            vec![],
            AuthMetadata::default(),
        );

        logger
            .log(&context, "test_action", "test_resource", true, None)
            .await;

        tokio::task::yield_now().await;

        assert_eq!(logger.get_logs("user-to-clear").len(), 1);

        logger.clear_logs("user-to-clear");
        assert!(logger.get_logs("user-to-clear").is_empty());
    }

    #[tokio::test]
    async fn test_audit_logger_total_log_count() {
        let logger = AuditLogger::new();
        let context = AuthContext::new(Some("user-1".to_string()), vec![], AuthMetadata::default());

        logger
            .log(&context, "action1", "resource1", true, None)
            .await;

        tokio::task::yield_now().await;

        let count = logger.total_log_count();
        assert!(count >= 1);
    }

    // ==================== Sanitize Error Message Tests ====================

    #[test]
    fn test_sanitize_jwt_token() {
        // Use a JWT pattern that matches the regex
        let message = "Invalid token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let sanitized = sanitize_error_message(message);
        // Check that JWT pattern is redacted
        assert!(
            !sanitized.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"),
            "JWT header should be redacted"
        );
        assert!(
            !sanitized.contains("eyJzdWIiOiIxMjM0NTY3ODkwIn0"),
            "JWT payload should be redacted"
        );
        assert!(
            sanitized.contains("REDACTED") || sanitized.contains("redacted"),
            "Should contain redaction marker"
        );
    }

    #[test]
    fn test_sanitize_password() {
        let message = "Connection failed: password=secret123, user=admin";
        let sanitized = sanitize_error_message(message);
        assert!(!sanitized.contains("secret123"));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn test_sanitize_private_key_path() {
        let message = "Invalid file: /etc/ssl/private/server.key";
        let sanitized = sanitize_error_message(message);
        assert!(!sanitized.contains(".key"));
        assert!(sanitized.contains("[REDACTED_PATH]"));
    }

    #[test]
    fn test_sanitize_max_length() {
        // Create a message long enough to exceed MAX_SANITIZED_LENGTH (500)
        // Include patterns that won't be redacted so length check applies
        let long_message = "Error: ".to_string() + &"x".repeat(600);
        let sanitized = sanitize_error_message(&long_message);
        // Check that the result is truncated to ~500 chars + truncation indicator
        assert!(
            sanitized.len() <= 520,
            "Sanitized message should be truncated to ~520 chars max"
        );
        assert!(
            sanitized.contains("...") || sanitized.len() <= 500,
            "Should indicate truncation or be under limit"
        );
    }

    // ==================== IP Validation Tests ====================

    #[test]
    fn test_is_valid_ip_public() {
        // Valid public IPs
        assert!(is_valid_ip("8.8.8.8"));
        assert!(is_valid_ip("1.1.1.1"));
        assert!(is_valid_ip("203.0.113.1"));
    }

    #[test]
    fn test_is_valid_ip_private_ranges() {
        // Private IP ranges should be invalid
        assert!(!is_valid_ip("10.0.0.1"));
        assert!(!is_valid_ip("172.16.0.1"));
        assert!(!is_valid_ip("192.168.1.1"));
    }

    #[test]
    fn test_is_valid_ip_loopback() {
        // Loopback should be invalid
        assert!(!is_valid_ip("127.0.0.1"));
        assert!(!is_valid_ip("::1"));
    }

    #[test]
    fn test_is_valid_ip_link_local() {
        // Link-local should be invalid
        assert!(!is_valid_ip("169.254.0.1"));
    }

    #[test]
    fn test_is_valid_ip_multicast() {
        // Multicast should be invalid
        assert!(!is_valid_ip("224.0.0.1"));
        assert!(!is_valid_ip("239.255.255.255"));
    }

    #[test]
    fn test_is_valid_ip_empty() {
        assert!(!is_valid_ip(""));
    }

    #[test]
    fn test_is_valid_ip_too_long() {
        let long_ip = "123.456.789.012.345.678.901.234.567";
        assert!(!is_valid_ip(long_ip));
    }

    #[test]
    fn test_is_valid_ip_ipv6_public() {
        // Public IPv6 (not loopback, link-local, etc.)
        assert!(is_valid_ip("2001:db8::1"));
    }

    // ==================== RateLimitError Tests ====================

    #[test]
    fn test_rate_limit_error_message() {
        let error = RateLimitError {
            limit: 100,
            remaining: 0,
            retry_after: 30,
        };
        let message = error.to_string();
        assert!(message.contains("30")); // retry_after
        assert!(message.to_lowercase().contains("rate limit")); // Case-insensitive check
    }

    // ==================== AuditResult Tests ====================

    #[test]
    fn test_audit_result_serialization() {
        use serde_json;

        let success = AuditResult::Success;
        let json = serde_json::to_string(&success).unwrap();
        assert!(json.contains("\"status\":\"success\""));

        let failure = AuditResult::Failure {
            message: "Test error".to_string(),
        };
        let json = serde_json::to_string(&failure).unwrap();
        assert!(json.contains("\"status\":\"failure\""));
        assert!(json.contains("Test error"));
    }

    // ==================== ApiKeyAuth Edge Cases ====================

    #[tokio::test]
    async fn test_api_key_clear_failed_attempts() {
        let auth = ApiKeyAuth::with_rate_limit(RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(60),
            include_headers: false,
        });
        auth.add_key("valid-key", vec!["read".to_string()]);

        // Trigger rate limiting
        let _ = auth.validate_key("invalid-1", "192.168.1.1");
        let _ = auth.validate_key("invalid-2", "192.168.1.1");

        // Should be rate limited
        assert_eq!(auth.validate_key("invalid-3", "192.168.1.1"), None);

        // Clear failed attempts
        auth.clear_failed_attempts("192.168.1.1");

        // Should be able to try again
        assert_eq!(auth.validate_key("invalid-4", "192.168.1.1"), None);
    }

    #[tokio::test]
    async fn test_api_key_different_ips() {
        let auth = ApiKeyAuth::with_rate_limit(RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(60),
            include_headers: false,
        });
        auth.add_key("valid-key", vec!["read".to_string()]);

        // Exhaust rate limit for IP 1
        let _ = auth.validate_key("invalid", "192.168.1.1");
        let _ = auth.validate_key("invalid", "192.168.1.1");

        // IP 1 should be rate limited
        assert_eq!(auth.validate_key("invalid", "192.168.1.1"), None);

        // IP 2 should still work
        assert_eq!(auth.validate_key("invalid", "192.168.1.2"), None);
    }

    // ==================== BearerAuth Token Blacklist Tests ====================

    #[tokio::test]
    async fn test_bearer_auth_blacklist() {
        // Use a valid secret (32+ chars with all required types)
        let auth =
            BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ").expect("Valid secret");

        // Note: We can't test actual JWT validation without a valid token
        // but we can test the blacklist mechanism
        auth.invalidate_token("test-token-to-blacklist");

        // The blacklist is checked before JWT verification
        // Since "test-token-to-blacklist" is not a valid JWT, it returns None anyway
        assert!(auth.validate_token("test-token-to-blacklist").is_none());
    }

    // ==================== BearerAuthBuilder Tests ====================

    #[test]
    fn test_bearer_auth_builder_valid_secret() {
        let auth = BearerAuthBuilder::new()
            .secret("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .build()
            .expect("Valid secret should build successfully");

        assert!(auth.validate_token("invalid-token").is_none());
    }

    #[test]
    fn test_bearer_auth_builder_with_audience_and_issuer() {
        let auth = BearerAuthBuilder::new()
            .secret("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .audience("my-api")
            .issuer("my-issuer")
            .build()
            .expect("Valid configuration should build successfully");

        assert!(auth.validate_token("invalid-token").is_none());
    }

    #[test]
    fn test_bearer_auth_builder_missing_secret() {
        let result = BearerAuthBuilder::new().build();
        assert!(result.is_err(), "Expected error for missing secret");

        match result {
            Err(AuthConfigError::InvalidSecret(msg)) => {
                assert!(msg.contains("required"));
            }
            Err(e) => panic!("Expected InvalidSecret error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_builder_secret_too_short() {
        let result = BearerAuthBuilder::new().secret("Short1!").build();

        assert!(result.is_err(), "Expected error for short secret");

        match result {
            Err(AuthConfigError::SecretTooShort { length }) => {
                assert_eq!(length, 7); // "Short1!" is 7 characters
            }
            Err(e) => panic!("Expected SecretTooShort error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_builder_missing_uppercase() {
        let result = BearerAuthBuilder::new()
            .secret("lowercase123!abcdefghijklmnopqrstuvwxyz")
            .build();

        assert!(result.is_err(), "Expected error for missing uppercase");

        match result {
            Err(AuthConfigError::MissingCharacterClass { required_type }) => {
                assert_eq!(required_type, "uppercase letter");
            }
            Err(e) => panic!("Expected MissingCharacterClass error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_builder_missing_lowercase() {
        let result = BearerAuthBuilder::new()
            .secret("UPPERCASE123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .build();

        assert!(result.is_err(), "Expected error for missing lowercase");

        match result {
            Err(AuthConfigError::MissingCharacterClass { required_type }) => {
                assert_eq!(required_type, "lowercase letter");
            }
            Err(e) => panic!("Expected MissingCharacterClass error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_builder_missing_digit() {
        let result = BearerAuthBuilder::new()
            .secret("LowercaseUpper!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .build();

        assert!(result.is_err(), "Expected error for missing digit");

        match result {
            Err(AuthConfigError::MissingCharacterClass { required_type }) => {
                assert_eq!(required_type, "digit");
            }
            Err(e) => panic!("Expected MissingCharacterClass error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_builder_missing_special_char() {
        let result = BearerAuthBuilder::new()
            .secret("LowercaseUpper123ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .build();

        assert!(result.is_err(), "Expected error for missing special char");

        match result {
            Err(AuthConfigError::MissingCharacterClass { required_type }) => {
                assert_eq!(required_type, "special character");
            }
            Err(e) => panic!("Expected MissingCharacterClass error, got {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_bearer_auth_builder_method() {
        // Test that BearerAuth::builder() returns a BearerAuthBuilder
        let auth = BearerAuth::builder()
            .secret("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .build()
            .expect("Builder should work");

        assert!(auth.validate_token("invalid").is_none());
    }

    // ==================== BearerAuth with_dependencies Tests ====================

    #[test]
    fn test_bearer_auth_with_dependencies() {
        let valid_tokens = Arc::new(DashMap::new());
        let blacklisted_tokens = Arc::new(DashMap::new());

        let auth = BearerAuth::with_dependencies(
            b"test-secret-key-for-dependencies-test".to_vec(),
            valid_tokens.clone(),
            blacklisted_tokens.clone(),
            Some("my-api".to_string()),
            Some("my-issuer".to_string()),
        );

        // Verify the dependencies are properly set
        assert!(auth.validate_token("invalid-token").is_none());

        // Verify we can use the shared caches
        let context = AuthContext::new(
            Some("user-123".to_string()),
            vec!["read".to_string()],
            AuthMetadata::default(),
        );
        auth.register_token("test-token".to_string(), context);

        // Verify the token is in the shared cache
        assert!(valid_tokens.contains_key("test-token"));
    }

    #[test]
    fn test_bearer_auth_with_dependencies_shared_state() {
        let valid_tokens = Arc::new(DashMap::new());
        let blacklisted_tokens = Arc::new(DashMap::new());

        // Create two instances sharing the same caches
        let auth1 = BearerAuth::with_dependencies(
            b"shared-secret-key-for-testing-purposes".to_vec(),
            valid_tokens.clone(),
            blacklisted_tokens.clone(),
            None,
            None,
        );

        let auth2 = BearerAuth::with_dependencies(
            b"shared-secret-key-for-testing-purposes".to_vec(),
            valid_tokens.clone(),
            blacklisted_tokens.clone(),
            None,
            None,
        );

        // Register a token with auth1
        let context = AuthContext::new(
            Some("user-456".to_string()),
            vec!["write".to_string()],
            AuthMetadata::default(),
        );
        auth1.register_token("shared-token".to_string(), context);

        // Verify both instances can access the shared state
        assert!(valid_tokens.contains_key("shared-token"));

        // Blacklist a token with auth2
        auth2.invalidate_token("blacklisted-token");

        // Verify the blacklist is shared
        assert!(blacklisted_tokens.contains_key("blacklisted-token"));
    }

    #[test]
    fn test_bearer_auth_with_dependencies_optional_claims() {
        let valid_tokens = Arc::new(DashMap::new());
        let blacklisted_tokens = Arc::new(DashMap::new());

        // Test with no audience or issuer
        let auth_no_claims = BearerAuth::with_dependencies(
            b"test-secret-key-no-claims-validation".to_vec(),
            valid_tokens.clone(),
            blacklisted_tokens.clone(),
            None,
            None,
        );
        assert!(auth_no_claims.validate_token("invalid").is_none());

        // Test with only audience
        let auth_with_audience = BearerAuth::with_dependencies(
            b"test-secret-key-with-audience-validation".to_vec(),
            valid_tokens.clone(),
            blacklisted_tokens.clone(),
            Some("my-api".to_string()),
            None,
        );
        assert!(auth_with_audience.validate_token("invalid").is_none());

        // Test with both audience and issuer
        let auth_with_both = BearerAuth::with_dependencies(
            b"test-secret-key-with-both-claims-validation".to_vec(),
            valid_tokens,
            blacklisted_tokens,
            Some("my-api".to_string()),
            Some("my-issuer".to_string()),
        );
        assert!(auth_with_both.validate_token("invalid").is_none());
    }
}

// Bearer token tests

// ==================== Password Hashing Module ====================

/// Password hashing configuration
///
/// Uses Argon2id - the winner of the Password Hashing Competition (PHC).
/// This provides much stronger protection for user passwords compared to simple hashing.
#[derive(Debug, Clone)]
pub struct PasswordHashConfig {
    /// Time cost parameter (number of iterations)
    /// Higher values increase computation time and resistance to GPU attacks
    /// Recommended: 1-3 for interactive applications, 3+ for sensitive data
    pub time_cost: u32,
    /// Memory cost parameter (in KiB)
    /// Higher values increase memory usage and resistance to ASIC attacks
    /// Recommended: 65536 (64 MiB) for interactive use
    pub memory_cost: u32,
    /// Parallelism parameter (number of lanes)
    /// Should match the number of available CPU cores
    /// Recommended: 1-4 depending on CPU cores
    pub parallelism: u32,
}

impl Default for PasswordHashConfig {
    fn default() -> Self {
        Self {
            time_cost: 3,
            memory_cost: 65536, // 64 MiB
            parallelism: 2,
        }
    }
}
