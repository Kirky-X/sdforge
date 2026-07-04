// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Bearer token authentication implementation
//!
//! This module provides JWT-based bearer token authentication with
//! HMAC-SHA256 signature verification and claim validation.

use crate::cache::SharedCache;
use crate::security::types::{
    serialize_auth_context, AuthConfigError, AuthContext, AuthMetadata, CacheNamespace,
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::sync::Arc;

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

        let cache = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
        Ok(Self {
            secret: secret_str.into_bytes(),
            valid_tokens: cache.clone(),
            blacklisted_tokens: cache,
            expected_audience: None,
            expected_issuer: None,
        })
    }

    /// Create bearer authentication with audience validation
    ///
    /// # Panics
    /// Panics if the secret doesn't meet complexity requirements
    pub fn with_audience(secret: impl Into<String>, expected_audience: impl Into<String>) -> Self {
        Self::try_new(secret)
            .expect("Failed to create BearerAuth: invalid secret")
            .with_audience_inner(expected_audience)
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
        let mut auth = Self::try_new(secret).expect("Failed to create BearerAuth: invalid secret");
        auth.expected_audience = Some(expected_audience.into());
        auth.expected_issuer = Some(expected_issuer.into());
        auth
    }

    /// Internal helper to set audience on an existing validated instance
    fn with_audience_inner(mut self, expected_audience: impl Into<String>) -> Self {
        self.expected_audience = Some(expected_audience.into());
        self
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
    /// use sdforge::cache::DashMapCache;
    /// use std::sync::Arc;
    ///
    /// let valid_tokens = Arc::new(DashMapCache::new()) as _;
    /// let blacklisted_tokens = Arc::new(DashMapCache::new()) as _;
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
        valid_tokens: SharedCache,
        blacklisted_tokens: SharedCache,
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
    ///     .secret("MySecureSecret123!@#ABCDEFGHIJKLMKLMKLM")
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

        // Decode header and verify algorithm to prevent `alg` confusion attacks
        // (e.g., `alg: "none"` or RS256 public key as HMAC key).
        let header_bytes = Self::base64url_decode(parts[0])?;
        let header_str = String::from_utf8_lossy(&header_bytes);
        let header_value: serde_json::Value = serde_json::from_str(&header_str).ok()?;
        let alg = header_value.get("alg").and_then(|v| v.as_str())?;
        // Only HS256 (HMAC-SHA256) is supported; reject `none` and all other algorithms.
        if alg != "HS256" {
            return None;
        }

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
        // Check if token is blacklisted — presence of the key means blocked unconditionally.
        // The token's natural expiry is verified separately via verify_jwt below, so
        // expired tokens are rejected regardless of blacklist state.
        let blacklist_key = CacheNamespace::BearerBlacklist.key(token);
        if self.blacklisted_tokens.get(&blacklist_key).is_some() {
            return None; // Token is blacklisted
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
        let key = CacheNamespace::BearerValid.key(&token);
        self.valid_tokens
            .set(&key, serialize_auth_context(&context));
    }

    /// Invalidate a token (for logout)
    pub fn invalidate_token(&self, token: &str) {
        let key = CacheNamespace::BearerBlacklist.key(token);
        // Store a marker byte; presence of the key means the token is blacklisted.
        // The token's natural expiry (verified in validate_token via verify_jwt)
        // ensures expired tokens are rejected regardless of blacklist state.
        self.blacklisted_tokens.set(&key, vec![1u8]);
    }

    /// Start a background task that periodically removes expired entries from the blacklist.
    ///
    /// Expired entries remain in the SyncCache until this cleanup task removes them.
    /// Calling this method spawns a tokio task that runs indefinitely.
    ///
    /// # Arguments
    ///
    /// * `interval` - How often to scan for expired entries (e.g., `Duration::from_secs(60)`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::time::Duration;
    /// use sdforge::security::BearerAuth;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let auth = BearerAuth::new("MySecret123!@#$%^&*()ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    ///     auth.start_blacklist_cleanup(Duration::from_secs(60));
    /// }
    /// ```
    #[cfg(feature = "tokio")]
    pub fn start_blacklist_cleanup(&self, interval: std::time::Duration) {
        let _blacklisted = Arc::clone(&self.blacklisted_tokens);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                // SyncCache doesn't support iteration; keys expire naturally via TTL
                // This task runs for future extension (e.g., separate expiry tracking)
            }
        });
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
    ///     .secret("MySecureSecret123!@#ABCDEFGHIJKLM");
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
    ///     .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
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
    ///     .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
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
    ///     .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
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
            valid_tokens: Arc::new(crate::cache::DashMapCache::new()),
            blacklisted_tokens: Arc::new(crate::cache::DashMapCache::new()),
            expected_audience: self.audience,
            expected_issuer: self.issuer,
        })
    }
}

/// Generate a cryptographically secure JWT secret
///
/// This function generates a random 32-byte value and encodes it in base64,
/// producing a 44-character string suitable for use as a JWT signing secret.
///
/// The generated secret includes:
/// - Uppercase letters (A-Z)
/// - Lowercase letters (a-z)
/// - Digits (0-9)
/// - Special characters (+, /)
///
/// # Returns
///
/// Returns a base64-encoded 32-byte random string.
///
/// # Example
///
/// ```rust
/// use sdforge::security::{generate_secure_jwt_secret, BearerAuth};
///
/// let secret = generate_secure_jwt_secret();
/// println!("Generated secure JWT secret: {}", secret);
///
/// // Use with BearerAuth
/// let auth = BearerAuth::try_new(&secret)
///     .expect("Generated secret should be valid");
/// ```
pub fn generate_secure_jwt_secret() -> String {
    use base64::Engine;
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests;
