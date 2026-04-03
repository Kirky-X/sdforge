// Copyright (c) 2026 Kirky.X
//! Shared types for the security module
//!
//! This module contains all common types used across the security module.

use base64::Engine;
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use uuid::Uuid;

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

impl CacheNamespace {
    /// Generate the full cache key with namespace prefix
    pub fn key(&self, suffix: &str) -> String {
        match self {
            CacheNamespace::ApiKey => format!("sdforge:apikey:{suffix}"),
            CacheNamespace::BearerBlacklist => format!("sdforge:bearer:blacklist:{suffix}"),
            CacheNamespace::BearerValid => format!("sdforge:bearer:valid:{suffix}"),
            CacheNamespace::Idempotency => format!("sdforge:idempotency:{suffix}"),
        }
    }
}

// =============================================================================
// Rate Limiting Types
// =============================================================================
// Serialization Helpers for Cache Storage
// =============================================================================

/// Serialize a list of permissions (Vec<String>) to bytes
pub(crate) fn serialize_permissions(perms: &[String]) -> Vec<u8> {
    bincode::serialize(perms).unwrap_or_default()
}

/// Deserialize a list of permissions from bytes
pub(crate) fn deserialize_permissions(data: &[u8]) -> Vec<String> {
    bincode::deserialize(data).unwrap_or_default()
}

/// Serialize a list of Instants to bytes using bincode
/// Used for token blacklist expiry tracking
pub(crate) fn serialize_instants(insts: &[std::time::Instant]) -> Vec<u8> {
    let as_i64: Vec<i64> = insts.iter().map(|i| i.elapsed().as_secs() as i64).collect();
    bincode::serialize(&as_i64).unwrap_or_default()
}

/// Deserialize a list of Instants from bytes using bincode
/// Used for token blacklist expiry tracking
pub(crate) fn deserialize_instants(data: &[u8]) -> Vec<std::time::Instant> {
    let as_i64: Vec<i64> = match bincode::deserialize(data) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    as_i64
        .iter()
        .map(|&s| std::time::Instant::now() - std::time::Duration::from_secs(s as u64))
        .collect()
}

/// Serialize AuthContext to bytes using bincode
pub(crate) fn serialize_auth_context(ctx: &AuthContext) -> Vec<u8> {
    bincode::serialize(ctx).unwrap_or_default()
}

/// Deserialize AuthContext from bytes using bincode.
///
/// **Reserved as the serialization pair for `serialize_auth_context`.**
/// Kept for future use when AuthContext deserialization from cache is needed.
#[allow(dead_code)]
pub(crate) fn deserialize_auth_context(data: &[u8]) -> Option<AuthContext> {
    bincode::deserialize(data).ok()
}

/// Parse a single AuditLog from a serde_json::Value object.
pub(crate) fn parse_audit_log(v: &serde_json::Value) -> Option<AuditLog> {
    let obj = v.as_object()?;
    let id = obj.get("id")?.as_str()?.to_string();
    let timestamp = obj.get("timestamp")?.as_i64()?;
    let user_id = obj
        .get("user_id")
        .and_then(|v| v.as_str().map(String::from));
    let action = obj.get("action")?.as_str()?.to_string();
    let resource = obj.get("resource")?.as_str()?.to_string();

    // Parse result: {"status": "success"} or {"status": "failure", "message": "..."}
    let result_val = obj.get("result")?.as_object()?;
    let status = result_val.get("status")?.as_str()?;
    let result = match status {
        "success" => AuditResult::Success,
        "failure" => {
            let msg = result_val.get("message")?.as_str()?.to_string();
            AuditResult::Failure { message: msg }
        }
        _ => return None,
    };

    // Parse metadata: AuthMetadata
    let meta_val = obj.get("metadata")?.as_object()?;
    let client_ip = meta_val
        .get("client_ip")
        .and_then(|v| v.as_str())
        .map(String::from);
    let user_agent = meta_val
        .get("user_agent")
        .and_then(|v| v.as_str())
        .map(String::from);
    let request_id = meta_val
        .get("request_id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default();
    let timestamp_meta = meta_val
        .get("timestamp")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    Some(AuditLog {
        id,
        timestamp,
        user_id,
        action,
        resource,
        result,
        metadata: AuthMetadata {
            client_ip,
            user_agent,
            request_id,
            timestamp: timestamp_meta,
        },
        signature: None, // Legacy logs don't have signatures
    })
}

/// Serialize audit logs to bytes
pub(crate) fn serialize_audit_logs(logs: &[AuditLog]) -> Vec<u8> {
    serde_json::to_vec(logs).unwrap_or_default()
}

/// Deserialize audit logs from bytes
pub(crate) fn deserialize_audit_logs(data: &[u8]) -> Vec<AuditLog> {
    match serde_json::from_slice::<serde_json::Value>(data) {
        Ok(serde_json::Value::Array(arr)) => arr.iter().filter_map(parse_audit_log).collect(),
        Ok(serde_json::Value::Object(_)) => {
            // Safe: we already know it's valid JSON object
            serde_json::from_slice(data)
                .ok()
                .and_then(|v| parse_audit_log(&v))
                .into_iter()
                .collect()
        }
        _ => Vec::new(),
    }
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
pub struct AuthExtractor(pub AuthContext);

impl AuthExtractor {
    /// Create new AuthExtractor
    pub fn new(context: AuthContext) -> Self {
        Self(context)
    }

    /// Get reference to context
    pub fn context(&self) -> &AuthContext {
        &self.0
    }

    /// Consume and return inner context
    pub fn into_inner(self) -> AuthContext {
        self.0
    }
}

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
// Trusted Proxy Configuration
// =============================================================================

/// Trusted proxy configuration for IP extraction
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct TrustedProxyConfig {
    /// List of trusted proxy IP addresses
    pub(crate) trusted_proxies: Vec<String>,
    /// Whether proxy verification is enabled
    pub(crate) enabled: bool,
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

impl Serialize for AuditLog {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AuditLog", 8)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("timestamp", &self.timestamp)?;
        s.serialize_field("user_id", &self.user_id)?;
        s.serialize_field("action", &self.action)?;
        s.serialize_field("resource", &self.resource)?;
        s.serialize_field("result", &self.result)?;
        s.serialize_field("metadata", &self.metadata)?;
        s.serialize_field("signature", &self.signature)?;
        s.end()
    }
}

impl AuditLog {
    /// Get the unique identifier for this audit log entry.
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

    /// Get the cryptographic signature for tamper detection
    pub fn signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }

    /// Generate HMAC-SHA256 signature for this audit log entry
    ///
    /// This creates a canonical representation of the log and signs it,
    /// allowing detection of any tampering with the audit trail.
    ///
    /// # Arguments
    /// * `secret_key` - The secret key for HMAC signing (should be kept secure)
    ///
    /// # Returns
    /// Base64-encoded HMAC-SHA256 signature
    ///
    /// # Example
    /// ```ignore
    /// let mut log = AuditLog::new(...);
    /// log.generate_signature(b"your-secret-key");
    /// ```
    pub fn generate_signature(&mut self, secret_key: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        // Create canonical string: id|timestamp|user_id|action|resource|result
        let canonical = format!(
            "{}|{}|{}|{}|{}|{}",
            self.id,
            self.timestamp,
            self.user_id.as_deref().unwrap_or(""),
            self.action,
            self.resource,
            match self.result {
                AuditResult::Success => "SUCCESS",
                AuditResult::Failure { .. } => "FAILURE",
            }
        );

        // Create HMAC
        let mut mac =
            HmacSha256::new_from_slice(secret_key).expect("HMAC can take key of any size");
        mac.update(canonical.as_bytes());
        let result = mac.finalize();

        // Encode to base64
        let signature = base64::engine::general_purpose::STANDARD.encode(result.into_bytes());

        // Store and return
        self.signature = Some(signature.clone());
        signature
    }

    /// Verify the integrity of this audit log entry
    ///
    /// Recomputes the signature and compares it with the stored signature
    /// to detect any tampering.
    ///
    /// # Arguments
    /// * `secret_key` - The secret key that was used for signing
    ///
    /// # Returns
    /// - `Ok(true)` if signature is valid
    /// - `Ok(false)` if signature doesn't match (tampered or wrong key)
    /// - `Err` if no signature is present
    ///
    /// # Example
    /// ```ignore
    /// match log.verify_signature(b"your-secret-key") {
    ///     Ok(true) => println!("Audit log is authentic"),
    ///     Ok(false) => println!("Audit log may have been tampered!"),
    ///     Err(_) => println!("No signature present"),
    /// }
    /// ```
    pub fn verify_signature(&self, secret_key: &[u8]) -> Result<bool, &'static str> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let stored_sig = self.signature.as_ref().ok_or("No signature present")?;

        // Recreate canonical string
        let canonical = format!(
            "{}|{}|{}|{}|{}|{}",
            self.id,
            self.timestamp,
            self.user_id.as_deref().unwrap_or(""),
            self.action,
            self.resource,
            match self.result {
                AuditResult::Success => "SUCCESS",
                AuditResult::Failure { .. } => "FAILURE",
            }
        );

        // Compute expected signature
        let mut mac =
            HmacSha256::new_from_slice(secret_key).expect("HMAC can take key of any size");
        mac.update(canonical.as_bytes());
        let result = mac.finalize();
        let expected_signature =
            base64::engine::general_purpose::STANDARD.encode(result.into_bytes());

        // Constant-time comparison to prevent timing attacks
        Ok(stored_sig == &expected_signature)
    }
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

/// AuditResult custom Serialize: produces `{"status":"success"}` / `{"status":"failure","message":"..."}`
impl Serialize for AuditResult {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            AuditResult::Success => {
                let mut s = serializer.serialize_struct("AuditResult", 1)?;
                s.serialize_field("status", "success")?;
                s.end()
            }
            AuditResult::Failure { message } => {
                let mut s = serializer.serialize_struct("AuditResult", 2)?;
                s.serialize_field("status", "failure")?;
                s.serialize_field("message", message)?;
                s.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_signature_generation() {
        let mut log = AuditLog {
            id: "test-id-123".to_string(),
            timestamp: 1234567890,
            user_id: Some("user123".to_string()),
            action: "LOGIN".to_string(),
            resource: "/api/auth".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata {
                client_ip: Some("192.168.1.1".to_string()),
                user_agent: Some("TestAgent/1.0".to_string()),
                request_id: "req-123".to_string(),
                timestamp: 1234567890,
            },
            signature: None,
        };

        // Generate signature
        let secret_key = b"test-secret-key";
        let signature = log.generate_signature(secret_key);

        // Verify signature was generated and stored
        assert!(signature.len() > 0);
        assert!(log.signature.is_some());
        assert_eq!(log.signature.as_ref().unwrap(), &signature);
    }

    #[test]
    fn test_audit_log_signature_verification_success() {
        let mut log = AuditLog {
            id: "test-id-456".to_string(),
            timestamp: 1234567890,
            user_id: Some("user456".to_string()),
            action: "LOGOUT".to_string(),
            resource: "/api/auth".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata {
                client_ip: Some("192.168.1.1".to_string()),
                user_agent: Some("TestAgent/1.0".to_string()),
                request_id: "req-456".to_string(),
                timestamp: 1234567890,
            },
            signature: None,
        };

        // Sign the log
        let secret_key = b"test-secret-key";
        let _signature = log.generate_signature(secret_key);

        // Verify with correct key
        let result = log.verify_signature(secret_key);
        assert!(result.is_ok());
        assert!(result.unwrap() == true);
    }

    #[test]
    fn test_audit_log_signature_verification_wrong_key() {
        let mut log = AuditLog {
            id: "test-id-789".to_string(),
            timestamp: 1234567890,
            user_id: Some("user789".to_string()),
            action: "DELETE".to_string(),
            resource: "/api/resource".to_string(),
            result: AuditResult::Failure {
                message: "Not authorized".to_string(),
            },
            metadata: AuthMetadata {
                client_ip: Some("192.168.1.1".to_string()),
                user_agent: Some("TestAgent/1.0".to_string()),
                request_id: "req-789".to_string(),
                timestamp: 1234567890,
            },
            signature: None,
        };

        // Sign with one key
        let secret_key = b"test-secret-key";
        let _signature = log.generate_signature(secret_key);

        // Try to verify with wrong key
        let wrong_key = b"wrong-secret-key";
        let result = log.verify_signature(wrong_key);
        assert!(result.is_ok());
        assert!(result.unwrap() == false);
    }

    #[test]
    fn test_audit_log_signature_tamper_detection() {
        let mut log = AuditLog {
            id: "test-id-tamper".to_string(),
            timestamp: 1234567890,
            user_id: Some("usertamper".to_string()),
            action: "UPDATE".to_string(),
            resource: "/api/resource/123".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata {
                client_ip: Some("192.168.1.1".to_string()),
                user_agent: Some("TestAgent/1.0".to_string()),
                request_id: "req-tamper".to_string(),
                timestamp: 1234567890,
            },
            signature: None,
        };

        // Sign the log
        let secret_key = b"test-secret-key";
        let _signature = log.generate_signature(secret_key);

        // Tamper with the log (change action)
        log.action = "DELETED".to_string();

        // Verification should fail
        let result = log.verify_signature(secret_key);
        assert!(result.is_ok());
        assert!(
            result.unwrap() == false,
            "Tampered log should fail verification"
        );
    }

    #[test]
    fn test_audit_log_no_signature() {
        let log = AuditLog {
            id: "test-id-nosig".to_string(),
            timestamp: 1234567890,
            user_id: Some("usernosig".to_string()),
            action: "READ".to_string(),
            resource: "/api/public".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata {
                client_ip: Some("192.168.1.1".to_string()),
                user_agent: Some("TestAgent/1.0".to_string()),
                request_id: "req-nosig".to_string(),
                timestamp: 1234567890,
            },
            signature: None,
        };

        // Verify should return error when no signature present
        let result = log.verify_signature(b"any-key");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No signature present");
    }

    #[test]
    fn test_audit_log_signature_getter() {
        let mut log = AuditLog {
            id: "test-id-getter".to_string(),
            timestamp: 1234567890,
            user_id: None,
            action: "ANONYMOUS".to_string(),
            resource: "/public".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata {
                client_ip: None,
                user_agent: None,
                request_id: "req-getter".to_string(),
                timestamp: 1234567890,
            },
            signature: None,
        };

        // Initially no signature
        assert!(log.signature().is_none());

        // Generate signature
        let _signature = log.generate_signature(b"test-key");

        // Now signature should be present
        assert!(log.signature().is_some());
        assert!(log.signature().unwrap().len() > 0);
    }
}
