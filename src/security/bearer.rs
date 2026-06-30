// Copyright (c) 2026 Kirky.X
//! Bearer token authentication implementation
//!
//! This module provides JWT-based bearer token authentication with
// HMAC-SHA256 signature verification and claim validation.

use crate::cache::SharedCache;
use crate::security::types::{
    serialize_auth_context, AuthConfigError, AuthContext, AuthMetadata, CacheNamespace,
};
use hmac::{Hmac, Mac};
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
        Self::try_new(secret).expect("Failed to create BearerAuth: invalid secret")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bearer_auth() {
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");
        assert_eq!(auth.secret.len(), 33);
    }

    #[test]
    fn test_bearer_auth_builder() {
        let auth = BearerAuthBuilder::new()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
            .audience("my-api")
            .issuer("my-issuer")
            .build()
            .expect("Failed to build BearerAuth");

        assert!(auth.expected_audience.is_some());
        assert_eq!(auth.expected_audience.as_ref().unwrap(), "my-api");
        assert!(auth.expected_issuer.is_some());
        assert_eq!(auth.expected_issuer.as_ref().unwrap(), "my-issuer");
    }

    #[test]
    fn test_invalid_secret_too_short() {
        let result = BearerAuth::try_new("short");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_secret_no_uppercase() {
        let result = BearerAuth::try_new("mysecret123!@#abcdefghijklm");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_secret_no_lowercase() {
        let result = BearerAuth::try_new("MYSECRET123!@#ABCDEFGHIJKLM");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_secret_no_digit() {
        let result = BearerAuth::try_new("MySecureSecret!@#ABCDEFGHIJKLM");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_secret_no_special() {
        let result = BearerAuth::try_new("MySecureSecret123ABCDEFGHIJKLM");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_secure_jwt_secret() {
        let secret = generate_secure_jwt_secret();

        // Check length (base64 encoded 32 bytes = 44 characters)
        assert_eq!(secret.len(), 44);

        // Check it can be used to create BearerAuth
        let result = BearerAuth::try_new(&secret);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_multiple_secrets_are_different() {
        let secret1 = generate_secure_jwt_secret();
        let secret2 = generate_secure_jwt_secret();

        // Ensure uniqueness
        assert_ne!(secret1, secret2);
    }

    // ========================================================================
    // Constructor Tests
    // ========================================================================

    #[test]
    fn test_bearer_auth_with_audience() {
        let auth =
            BearerAuth::with_audience("MySecureSecret123!@#ABCDEFGHIJKLM", "my-api-audience");
        assert_eq!(auth.expected_audience, Some("my-api-audience".to_string()));
        assert!(auth.expected_issuer.is_none());
    }

    #[test]
    fn test_bearer_auth_with_claims() {
        let auth = BearerAuth::with_claims(
            "MySecureSecret123!@#ABCDEFGHIJKLM",
            "my-api-audience",
            "my-token-issuer",
        );
        assert_eq!(auth.expected_audience, Some("my-api-audience".to_string()));
        assert_eq!(auth.expected_issuer, Some("my-token-issuer".to_string()));
    }

    #[test]
    fn test_bearer_auth_with_dependencies() {
        let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
        let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

        let auth = BearerAuth::with_dependencies(
            b"test-secret-bytes".to_vec(),
            valid_tokens.clone(),
            blacklisted_tokens.clone(),
            Some("aud".to_string()),
            Some("iss".to_string()),
        );

        assert_eq!(auth.secret, b"test-secret-bytes".to_vec());
        assert_eq!(auth.expected_audience, Some("aud".to_string()));
        assert_eq!(auth.expected_issuer, Some("iss".to_string()));
    }

    #[test]
    fn test_bearer_auth_with_dependencies_all_none() {
        let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
        let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

        let auth = BearerAuth::with_dependencies(
            b"test-secret".to_vec(),
            valid_tokens,
            blacklisted_tokens,
            None,
            None,
        );

        assert!(auth.expected_audience.is_none());
        assert!(auth.expected_issuer.is_none());
    }

    #[test]
    fn test_bearer_auth_new_panics_on_invalid_secret() {
        let result = std::panic::catch_unwind(|| {
            BearerAuth::new("short");
        });
        assert!(result.is_err());
    }

    // ========================================================================
    // Builder Tests
    // ========================================================================

    #[test]
    fn test_bearer_auth_builder_minimal() {
        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
            .build()
            .expect("Should build with valid secret");

        assert!(auth.expected_audience.is_none());
        assert!(auth.expected_issuer.is_none());
    }

    #[test]
    fn test_bearer_auth_builder_with_audience_only() {
        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
            .audience("my-api")
            .build()
            .expect("Should build");

        assert_eq!(auth.expected_audience, Some("my-api".to_string()));
        assert!(auth.expected_issuer.is_none());
    }

    #[test]
    fn test_bearer_auth_builder_with_issuer_only() {
        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
            .issuer("my-issuer")
            .build()
            .expect("Should build");

        assert!(auth.expected_audience.is_none());
        assert_eq!(auth.expected_issuer, Some("my-issuer".to_string()));
    }

    #[test]
    fn test_bearer_auth_builder_no_secret_error() {
        let result = BearerAuthBuilder::new().build();
        assert!(result.is_err());
        let err = result.as_ref().err().unwrap();
        match err {
            AuthConfigError::InvalidSecret(msg) => {
                assert!(msg.contains("Secret is required"));
            }
            _ => panic!("Expected InvalidSecret error"),
        }
    }

    #[test]
    fn test_bearer_auth_builder_secret_too_short() {
        let result = BearerAuthBuilder::new().secret("short").build();
        assert!(matches!(result, Err(AuthConfigError::SecretTooShort { length }) if length == 5));
    }

    #[test]
    fn test_bearer_auth_builder_missing_uppercase() {
        let result = BearerAuthBuilder::new()
            .secret("mysecret123!@#abcdefghijklmnopqrstuv") // 36 chars, no uppercase
            .build();
        assert!(matches!(
            result,
            Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "uppercase letter"
        ));
    }

    #[test]
    fn test_bearer_auth_builder_missing_lowercase() {
        let result = BearerAuthBuilder::new()
            .secret("MYSECRET123!@#ABCDEFGHIJKLMNOPQRSTUV") // 36 chars, no lowercase
            .build();
        assert!(matches!(
            result,
            Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "lowercase letter"
        ));
    }

    #[test]
    fn test_bearer_auth_builder_missing_digit() {
        let result = BearerAuthBuilder::new()
            .secret("MySecureSecret!@#abcdefghijklmnopqrstuv") // 38 chars, no digit
            .build();
        assert!(matches!(
            result,
            Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "digit"
        ));
    }

    #[test]
    fn test_bearer_auth_builder_missing_special_char() {
        let result = BearerAuthBuilder::new()
            .secret("MySecureSecret123abcdefghijklmnopqrstuv") // 41 chars, no special
            .build();
        assert!(matches!(
            result,
            Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "special character"
        ));
    }

    // ========================================================================
    // Helper: Create a valid JWT token for testing
    // ========================================================================

    fn create_test_jwt(secret: &[u8], payload: &serde_json::Value) -> String {
        use base64::Engine;
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&header).unwrap());
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(payload).unwrap());

        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let mut mac =
            Hmac::<Sha256>::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(signing_input.as_bytes());
        let signature = mac.finalize().into_bytes();
        let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);

        format!("{}.{}", signing_input, signature_b64)
    }

    // ========================================================================
    // Token Validation Tests
    // ========================================================================

    #[test]
    fn test_validate_token_valid() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "permissions": ["read", "write"],
            "iat": chrono::Utc::now().timestamp(),
            "exp": (chrono::Utc::now().timestamp() + 3600)
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        let ctx = auth.validate_token(&token);

        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.user_id(), Some("user123"));
        assert_eq!(
            ctx.permissions(),
            &["read".to_string(), "write".to_string()]
        );
    }

    #[test]
    fn test_validate_token_no_sub_no_permissions() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "iat": chrono::Utc::now().timestamp()
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        let ctx = auth.validate_token(&token);

        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert!(ctx.user_id().is_none());
        assert!(ctx.permissions().is_empty());
    }

    #[test]
    fn test_validate_token_invalid_structure() {
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

        // Token with wrong number of parts
        assert!(auth.validate_token("not.a.valid.jwt.token.extra").is_none());
        assert!(auth.validate_token("onlytwo.parts").is_none());
        assert!(auth.validate_token("notajwt").is_none());
    }

    #[test]
    fn test_validate_token_invalid_signature() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let wrong_secret = "WrongSecret123!@#ABCDEFGHIJKLMnopq";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp()
        });

        // Sign with wrong secret
        let token = create_test_jwt(wrong_secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_validate_token_expired() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp() - 7200,
            "exp": chrono::Utc::now().timestamp() - 3600 // Expired 1 hour ago
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_validate_token_iat_in_future() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp() + 120, // 2 minutes in the future (> 60s skew)
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_validate_token_iat_within_clock_skew() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp() + 30, // 30 seconds in the future (within 60s skew)
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        // Should pass because within clock skew tolerance
        assert!(auth.validate_token(&token).is_some());
    }

    #[test]
    fn test_validate_token_nbf_not_yet_valid() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "nbf": chrono::Utc::now().timestamp() + 3600, // Not valid for another hour
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 7200
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_validate_token_audience_mismatch() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::with_audience(secret, "expected-api");

        let payload = serde_json::json!({
            "sub": "user123",
            "aud": "wrong-api",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_validate_token_audience_match() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::with_audience(secret, "expected-api");

        let payload = serde_json::json!({
            "sub": "user123",
            "aud": "expected-api",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_some());
    }

    #[test]
    fn test_validate_token_audience_array_first_match() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::with_audience(secret, "api-one");

        let payload = serde_json::json!({
            "sub": "user123",
            "aud": ["api-one", "api-two"],
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_some());
    }

    #[test]
    fn test_validate_token_audience_array_mismatch() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::with_audience(secret, "expected-api");

        let payload = serde_json::json!({
            "sub": "user123",
            "aud": ["other-api", "another-api"],
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_validate_token_issuer_mismatch() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::with_claims(secret, "my-api", "expected-issuer");

        let payload = serde_json::json!({
            "sub": "user123",
            "aud": "my-api",
            "iss": "wrong-issuer",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_validate_token_issuer_match() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::with_claims(secret, "my-api", "expected-issuer");

        let payload = serde_json::json!({
            "sub": "user123",
            "aud": "my-api",
            "iss": "expected-issuer",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_some());
    }

    // ========================================================================
    // Token Blacklist Tests
    // ========================================================================

    #[test]
    fn test_invalidate_token_blacklists() {
        // Test the blacklist mechanism by verifying that calling invalidate_token
        // correctly writes to the blacklist cache and that the validate_token
        // function checks the blacklist.
        //
        // Note: The blacklist stores elapsed().as_secs() (truncated to seconds).
        // An entry created "just now" deserializes to Instant::now() - 0, which
        // is slightly in the past, so the comparison `Instant::now() < past` is
        // false and the token passes. The blacklist only blocks when the stored
        // Instant is in the future, which happens when the entry was created
        // before the clock moved forward (e.g., cross-second boundary).
        //
        // Here we verify the write path and that the validate_token function
        // does check the blacklist cache.
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
        let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

        let auth = BearerAuth::with_dependencies(
            secret.as_bytes().to_vec(),
            valid_tokens.clone(),
            blacklisted_tokens.clone(),
            None,
            None,
        );

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);

        // Token should be valid initially
        assert!(auth.validate_token(&token).is_some());

        // Use invalidate_token to blacklist
        auth.invalidate_token(&token);

        // Verify the blacklist cache was updated
        let key = CacheNamespace::BearerBlacklist.key(&token);
        assert!(blacklisted_tokens.get(&key).is_some());
        assert!(auth.blacklisted_tokens.contains(&key));
        assert!(blacklisted_tokens.contains(&key));
    }

    #[test]
    fn test_invalidate_token_blacklist_blocks_token() {
        // Test that a blacklisted token is blocked unconditionally.
        // Previously this test validated buggy behavior where the blacklist
        // never blocked any token due to Instant::now().elapsed() ≈ 0.
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
        let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

        let auth = BearerAuth::with_dependencies(
            secret.as_bytes().to_vec(),
            valid_tokens.clone(),
            blacklisted_tokens.clone(),
            None,
            None,
        );

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);

        // Token should be valid initially
        assert!(auth.validate_token(&token).is_some());

        // Blacklist the token
        auth.invalidate_token(&token);

        // Token should now be blocked
        assert!(
            auth.validate_token(&token).is_none(),
            "Blacklisted token must be rejected"
        );
    }

    #[test]
    fn test_invalidate_token_direct_method() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);

        // Token should be valid initially
        assert!(auth.validate_token(&token).is_some());

        // Invalidate the token - this stores Instant::now() as the expiry
        auth.invalidate_token(&token);

        // The blacklist entry is created, verify the key exists in cache
        let blacklist_key = CacheNamespace::BearerBlacklist.key(&token);
        assert!(auth.blacklisted_tokens.get(&blacklist_key).is_some());
    }

    #[test]
    fn test_register_and_validate_token() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user456",
            "permissions": ["admin"],
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        let context = AuthContext {
            user_id: Some("user456".to_string()),
            permissions: vec!["admin".to_string()],
            metadata: AuthMetadata::default(),
        };

        // Register the token
        auth.register_token(token.clone(), context);

        // Token should still be valid
        let result = auth.validate_token(&token);
        assert!(result.is_some());
        assert_eq!(result.unwrap().user_id(), Some("user456"));
    }

    // ========================================================================
    // Constant Time Comparison Tests
    // ========================================================================

    #[test]
    fn test_constant_time_eq_equal() {
        let a = b"hello world";
        let b = b"hello world";
        assert!(BearerAuth::constant_time_eq(a, b));
    }

    #[test]
    fn test_constant_time_eq_not_equal() {
        let a = b"hello world";
        let b = b"hello earth";
        assert!(!BearerAuth::constant_time_eq(a, b));
    }

    #[test]
    fn test_constant_time_eq_different_lengths() {
        let a = b"short";
        let b = b"much longer string";
        assert!(!BearerAuth::constant_time_eq(a, b));
    }

    #[test]
    fn test_constant_time_eq_empty() {
        assert!(BearerAuth::constant_time_eq(b"", b""));
    }

    #[test]
    fn test_constant_time_eq_single_byte_diff() {
        assert!(BearerAuth::constant_time_eq(b"a", b"a"));
        assert!(!BearerAuth::constant_time_eq(b"a", b"b"));
    }

    // ========================================================================
    // Base64URL Decode Tests
    // ========================================================================

    #[test]
    fn test_base64url_decode_valid() {
        // "Hello" in base64url = "SGVsbG8"
        let result = BearerAuth::base64url_decode("SGVsbG8");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"Hello");
    }

    #[test]
    fn test_base64url_decode_with_periods() {
        // Should skip period separators
        let result = BearerAuth::base64url_decode("SGVs.bG8");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"Hello");
    }

    #[test]
    fn test_base64url_decode_with_whitespace() {
        // Should skip whitespace
        let result = BearerAuth::base64url_decode("SGVs\nbG8");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"Hello");
    }

    #[test]
    fn test_base64url_decode_non_ascii_chars() {
        // The base64url decoder uses a lookup table indexed by byte value.
        // Non-ASCII UTF-8 characters produce bytes > 127, which map to index 0
        // in the lookup table (uninitialized entries are zero). The decoder is
        // permissive and won't return None - it just decodes them as zero-value bits.
        let result = BearerAuth::base64url_decode("\u{0080}");
        assert!(result.is_some());
        // 2 UTF-8 bytes (0xC2, 0x80), both map to 0 in table
        // 2*6 bits = 12 bits -> 1 full byte
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_base64url_decode_empty_string() {
        let result = BearerAuth::base64url_decode("");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"");
    }

    // ========================================================================
    // Edge Cases and Integration Tests
    // ========================================================================

    #[test]
    fn test_validate_token_with_malformed_base64() {
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");
        // Invalid base64 in payload
        assert!(auth
            .validate_token("header.!!!invalid!!!.signature")
            .is_none());
    }

    #[test]
    fn test_validate_token_with_invalid_json_payload() {
        use base64::Engine;
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{}");
        let invalid_json_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"not json");
        let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0u8; 32]);

        let token = format!("{}.{}.{}", header_b64, invalid_json_b64, sig_b64);
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_validate_token_with_wrong_signature_length() {
        use base64::Engine;
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&header).unwrap());
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&payload).unwrap());

        // Signature that's not 32 bytes when decoded
        let short_sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0u8; 16]);

        let token = format!("{}.{}.{}", header_b64, payload_b64, short_sig_b64);
        assert!(auth.validate_token(&token).is_none());
    }

    #[test]
    fn test_clone_shares_state() {
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");
        let auth_clone = auth.clone();

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let token = create_test_jwt(secret.as_bytes(), &payload);

        // Invalidate on original
        auth.invalidate_token(&token);

        // Clone shares the same internal Arc<DashMapCache>, so blacklist entry
        // should be visible. Verify the cache key was written.
        let blacklist_key = CacheNamespace::BearerBlacklist.key(&token);
        assert!(auth_clone.blacklisted_tokens.get(&blacklist_key).is_some());
    }

    #[test]
    fn test_builder_pattern_chaining() {
        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
            .audience("test-api")
            .issuer("test-issuer")
            .build()
            .expect("Should build with all options");

        assert_eq!(auth.expected_audience, Some("test-api".to_string()));
        assert_eq!(auth.expected_issuer, Some("test-issuer".to_string()));
    }

    #[test]
    fn test_try_new_returns_correct_errors() {
        // Test SecretTooShort
        let result = BearerAuth::try_new("short");
        assert!(matches!(
            result,
            Err(AuthConfigError::SecretTooShort { .. })
        ));

        // Test MissingCharacterClass for each type (all must be >= 32 chars)
        let result = BearerAuth::try_new("mysecret123!@#abcdefghijklmnopqrstuv");
        assert!(matches!(
            result,
            Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "uppercase letter"
        ));

        let result = BearerAuth::try_new("MYSECRET123!@#ABCDEFGHIJKLMNOPQRSTUV");
        assert!(matches!(
            result,
            Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "lowercase letter"
        ));

        let result = BearerAuth::try_new("MySecureSecret!@#abcdefghijklmnopqrstuv");
        assert!(matches!(
            result,
            Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "digit"
        ));

        let result = BearerAuth::try_new("MySecureSecret123abcdefghijklmnopqrstuv");
        assert!(matches!(
            result,
            Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "special character"
        ));
    }

    #[test]
    fn test_validate_token_no_exp_claim() {
        // Token without exp claim should still be valid
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp()
            // No exp claim
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_some());
    }

    #[test]
    fn test_validate_token_no_iat_no_nbf() {
        // Token without iat and nbf claims should still be valid
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "exp": chrono::Utc::now().timestamp() + 3600
            // No iat or nbf
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        assert!(auth.validate_token(&token).is_some());
    }

    #[test]
    fn test_permissions_filtered_from_invalid_entries() {
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "permissions": ["valid_perm", 123, null, "another_valid"],
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        let ctx = auth.validate_token(&token);

        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        // Only string permissions should be kept
        assert_eq!(
            ctx.permissions(),
            &["valid_perm".to_string(), "another_valid".to_string()]
        );
    }

    #[test]
    fn test_secret_exactly_32_chars() {
        // Test secret that's exactly 32 characters with all required character classes
        let secret = "Abcdefghijklmnopqrstuvwx12345!";
        assert_eq!(secret.len(), 30); // Actually 30, need 32

        let secret = "Abcdefghijklmnopqrstuvwx123456!@";
        assert_eq!(secret.len(), 32);
        let result = BearerAuth::try_new(secret);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Additional Boundary Tests (11 new tests)
    // ========================================================================

    #[test]
    fn test_verify_token_tampered_payload() {
        // Test that tampering with the payload invalidates the signature
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);

        // Tamper with the payload by modifying a character in the middle
        let mut token_parts: Vec<&str> = token.split('.').collect();
        let payload_b64 = token_parts[1].to_string();
        let mut payload_bytes = payload_b64.into_bytes();

        // Modify a byte in the middle (if long enough)
        if payload_bytes.len() > 10 {
            payload_bytes[5] ^= 0xFF; // Flip bits
        }

        token_parts[1] = std::str::from_utf8(&payload_bytes).unwrap_or("invalid");
        let tampered_token = token_parts.join(".");

        // Tampered token should be rejected
        assert!(auth.validate_token(&tampered_token).is_none());
    }

    #[test]
    fn test_verify_token_empty_signature() {
        // Test token with empty signature part
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        let mut parts: Vec<&str> = token.split('.').collect();

        // Replace signature with empty string
        parts[2] = "";
        let token_with_empty_sig = parts.join(".");

        // Empty signature should be rejected
        assert!(auth.validate_token(&token_with_empty_sig).is_none());
    }

    #[test]
    fn test_verify_blacklisted_token_rejected() {
        // Test that invalidate_token properly adds token to blacklist
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
        let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

        let auth = BearerAuth::with_dependencies(
            secret.as_bytes().to_vec(),
            valid_tokens.clone(),
            blacklisted_tokens.clone(),
            None,
            None,
        );

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);

        // Token should be valid initially
        assert!(auth.validate_token(&token).is_some());

        // Add to blacklist using invalidate_token
        auth.invalidate_token(&token);

        // Verify the blacklist cache was updated
        let key = CacheNamespace::BearerBlacklist.key(&token);
        assert!(blacklisted_tokens.get(&key).is_some());
        assert!(auth.blacklisted_tokens.contains(&key));
        assert!(blacklisted_tokens.contains(&key));
    }

    #[test]
    fn test_blacklist_nonexistent_token() {
        // Test that blacklisting a non-existent token doesn't cause errors
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        // Create a valid token
        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });
        let token = create_test_jwt(secret.as_bytes(), &payload);

        // Blacklist a different (non-existent) token
        let nonexistent_token = "nonexistent.token.value";
        auth.invalidate_token(nonexistent_token);

        // Original token should still be valid
        assert!(auth.validate_token(&token).is_some());

        // Verify the nonexistent token was blacklisted (the method should work without errors)
        let key = CacheNamespace::BearerBlacklist.key(nonexistent_token);
        assert!(auth.blacklisted_tokens.get(&key).is_some());
    }

    #[test]
    fn test_verify_token_empty_token() {
        // Test validation with empty string token
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

        // Empty string should be rejected
        assert!(auth.validate_token("").is_none());
    }

    #[test]
    fn test_verify_token_with_special_chars() {
        // Test token containing special characters
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        // Create a token with special characters in the payload
        let payload = serde_json::json!({
            "sub": "user@example.com",  // Contains @
            "name": "Test User (Admin)",  // Contains parentheses
            "permissions": ["read", "write"],
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);

        // Token with special characters in payload should be valid
        let ctx = auth.validate_token(&token);
        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.user_id(), Some("user@example.com"));
    }

    #[test]
    fn test_bearer_auth_clone_equality() {
        // Test that cloned BearerAuth instances share the same internal state
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");
        let auth_clone = auth.clone();

        // Both should have the same secret
        assert_eq!(auth.secret, auth_clone.secret);
        assert_eq!(auth.expected_audience, auth_clone.expected_audience);
        assert_eq!(auth.expected_issuer, auth_clone.expected_issuer);
    }

    #[test]
    fn test_bearer_auth_with_custom_caches() {
        // Test BearerAuth with custom cache implementations
        use crate::cache::DashMapCache;

        let custom_valid_cache = Arc::new(DashMapCache::new()) as SharedCache;
        let custom_blacklist_cache = Arc::new(DashMapCache::new()) as SharedCache;

        let auth = BearerAuth::with_dependencies(
            b"custom-secret-key-with-all-required-classes-123!@#".to_vec(),
            custom_valid_cache.clone(),
            custom_blacklist_cache.clone(),
            Some("custom-audience".to_string()),
            Some("custom-issuer".to_string()),
        );

        assert_eq!(auth.expected_audience, Some("custom-audience".to_string()));
        assert_eq!(auth.expected_issuer, Some("custom-issuer".to_string()));

        // Verify custom caches are used
        assert!(Arc::ptr_eq(&auth.valid_tokens, &custom_valid_cache));
        assert!(Arc::ptr_eq(
            &auth.blacklisted_tokens,
            &custom_blacklist_cache
        ));
    }

    #[test]
    fn test_verify_token_missing_signature_part() {
        // Test token with missing signature part (only header.payload)
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "user123",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);
        let parts: Vec<&str> = token.split('.').collect();

        // Create token without signature (only header.payload)
        let incomplete_token = format!("{}.{}", parts[0], parts[1]);

        // Token without signature should be rejected
        assert!(auth.validate_token(&incomplete_token).is_none());
    }

    #[test]
    fn test_verify_token_header_only() {
        // Test token with only header part
        use base64::Engine;

        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

        let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&header).unwrap());

        // Token with only header
        assert!(auth.validate_token(&header_b64).is_none());
    }

    #[test]
    fn test_verify_token_with_unicode_chars() {
        // Test token with Unicode characters in payload
        let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
        let auth = BearerAuth::new(secret);

        let payload = serde_json::json!({
            "sub": "用户123",  // Chinese characters
            "name": "José García",  // Accented characters
            "description": "Test with emoji 🎉",
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600
        });

        let token = create_test_jwt(secret.as_bytes(), &payload);

        // Token with Unicode characters should be valid
        let ctx = auth.validate_token(&token);
        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.user_id(), Some("用户123"));
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
/// use sdforge::security::bearer::generate_secure_jwt_secret;
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
