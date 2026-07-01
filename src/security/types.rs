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
#[allow(dead_code)]
pub(crate) fn serialize_instants(insts: &[std::time::Instant]) -> Vec<u8> {
    let as_i64: Vec<i64> = insts.iter().map(|i| i.elapsed().as_secs() as i64).collect();
    bincode::serialize(&as_i64).unwrap_or_default()
}

/// Deserialize a list of Instants from bytes using bincode
/// Used for token blacklist expiry tracking
#[allow(dead_code)]
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
/// Reserved as the serialization pair for serialize_auth_context.
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

    // Parse signature: present in signed logs, absent in legacy logs
    let signature = obj
        .get("signature")
        .and_then(|v| v.as_str())
        .map(String::from);

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
        signature,
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
        assert!(!signature.is_empty());
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
        assert!(result.unwrap());
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
        assert!(!result.unwrap());
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
            !result.unwrap(),
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
        assert!(!log.signature().unwrap().is_empty());
    }

    // ============================================================================
    // CacheNamespace Tests
    // ============================================================================

    #[test]
    fn test_cache_namespace_api_key() {
        let ns = CacheNamespace::ApiKey;
        assert_eq!(ns.key("hash123"), "sdforge:apikey:hash123");
    }

    #[test]
    fn test_cache_namespace_bearer_blacklist() {
        let ns = CacheNamespace::BearerBlacklist;
        assert_eq!(ns.key("token_abc"), "sdforge:bearer:blacklist:token_abc");
    }

    #[test]
    fn test_cache_namespace_bearer_valid() {
        let ns = CacheNamespace::BearerValid;
        assert_eq!(ns.key("token_xyz"), "sdforge:bearer:valid:token_xyz");
    }

    #[test]
    fn test_cache_namespace_idempotency() {
        let ns = CacheNamespace::Idempotency;
        assert_eq!(ns.key("idem_key_123"), "sdforge:idempotency:idem_key_123");
    }

    // ============================================================================
    // Serialization Roundtrip Tests
    // ============================================================================

    #[test]
    fn test_serialize_deserialize_permissions_roundtrip() {
        let perms = vec!["read".to_string(), "write".to_string(), "admin".to_string()];
        let bytes = serialize_permissions(&perms);
        let result = deserialize_permissions(&bytes);
        assert_eq!(result, perms);
    }

    #[test]
    fn test_serialize_deserialize_permissions_empty() {
        let perms: Vec<String> = vec![];
        let bytes = serialize_permissions(&perms);
        let result = deserialize_permissions(&bytes);
        assert_eq!(result, perms);
    }

    #[test]
    fn test_deserialize_permissions_invalid_data() {
        let result = deserialize_permissions(b"invalid bincode data");
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_serialize_deserialize_instants_roundtrip() {
        let now = std::time::Instant::now();
        let past = now - std::time::Duration::from_secs(60);
        let instants = vec![now, past];
        let bytes = serialize_instants(&instants);
        let result = deserialize_instants(&bytes);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_serialize_deserialize_instants_empty() {
        let instants: Vec<std::time::Instant> = vec![];
        let bytes = serialize_instants(&instants);
        let result = deserialize_instants(&bytes);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_deserialize_instants_invalid_data() {
        let result = deserialize_instants(b"invalid data");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_serialize_deserialize_auth_context_roundtrip() {
        let ctx = AuthContext {
            user_id: Some("user123".to_string()),
            permissions: vec!["read".to_string()],
            metadata: AuthMetadata {
                client_ip: Some("192.168.1.1".to_string()),
                user_agent: Some("TestAgent".to_string()),
                request_id: "req-123".to_string(),
                timestamp: 1234567890,
            },
        };
        let bytes = serialize_auth_context(&ctx);
        let result = deserialize_auth_context(&bytes);
        assert!(result.is_some());
        let restored = result.unwrap();
        assert_eq!(restored.user_id, Some("user123".to_string()));
        assert_eq!(restored.permissions, vec!["read".to_string()]);
    }

    #[test]
    fn test_deserialize_auth_context_invalid_data() {
        let result = deserialize_auth_context(b"invalid data");
        assert!(result.is_none());
    }

    // ============================================================================
    // parse_audit_log Tests
    // ============================================================================

    #[test]
    fn test_parse_audit_log_valid_success() {
        let value = serde_json::json!({
            "id": "log-1",
            "timestamp": 1234567890i64,
            "user_id": "user1",
            "action": "LOGIN",
            "resource": "/api/auth",
            "result": {"status": "success"},
            "metadata": {
                "client_ip": "10.0.0.1",
                "user_agent": "TestAgent",
                "request_id": "req-1",
                "timestamp": 1234567890i64
            }
        });
        let log = parse_audit_log(&value);
        assert!(log.is_some());
        let log = log.unwrap();
        assert_eq!(log.id, "log-1");
        assert_eq!(log.action, "LOGIN");
        assert!(matches!(log.result, AuditResult::Success));
    }

    #[test]
    fn test_parse_audit_log_valid_failure() {
        let value = serde_json::json!({
            "id": "log-2",
            "timestamp": 1234567890i64,
            "user_id": null,
            "action": "DELETE",
            "resource": "/api/resource/1",
            "result": {"status": "failure", "message": "Not authorized"},
            "metadata": {
                "client_ip": null,
                "user_agent": null,
                "request_id": "req-2",
                "timestamp": 1234567890i64
            }
        });
        let log = parse_audit_log(&value);
        assert!(log.is_some());
        let log = log.unwrap();
        assert!(log.user_id.is_none());
        match log.result {
            AuditResult::Failure { message } => {
                assert_eq!(message, "Not authorized");
            }
            _ => panic!("Expected Failure"),
        }
    }

    #[test]
    fn test_parse_audit_log_invalid_missing_id() {
        let value = serde_json::json!({
            "timestamp": 1234567890i64,
            "action": "LOGIN",
            "resource": "/api/auth",
            "result": {"status": "success"},
            "metadata": {
                "request_id": "req-1",
                "timestamp": 1234567890i64
            }
        });
        assert!(parse_audit_log(&value).is_none());
    }

    #[test]
    fn test_parse_audit_log_invalid_result_status() {
        let value = serde_json::json!({
            "id": "log-3",
            "timestamp": 1234567890i64,
            "action": "LOGIN",
            "resource": "/api/auth",
            "result": {"status": "unknown"},
            "metadata": {
                "request_id": "req-3",
                "timestamp": 1234567890i64
            }
        });
        assert!(parse_audit_log(&value).is_none());
    }

    #[test]
    fn test_parse_audit_log_not_an_object() {
        let value = serde_json::json!("not an object");
        assert!(parse_audit_log(&value).is_none());
    }

    // ============================================================================
    // serialize_audit_logs / deserialize_audit_logs Roundtrip Tests
    // ============================================================================

    #[test]
    fn test_serialize_deserialize_audit_logs_roundtrip() {
        let logs = vec![
            AuditLog {
                id: "log-1".to_string(),
                timestamp: 1234567890,
                user_id: Some("user1".to_string()),
                action: "LOGIN".to_string(),
                resource: "/api/auth".to_string(),
                result: AuditResult::Success,
                metadata: AuthMetadata::default(),
                signature: None,
            },
            AuditLog {
                id: "log-2".to_string(),
                timestamp: 1234567891,
                user_id: Some("user2".to_string()),
                action: "LOGOUT".to_string(),
                resource: "/api/auth".to_string(),
                result: AuditResult::Failure {
                    message: "Timeout".to_string(),
                },
                metadata: AuthMetadata::default(),
                signature: None,
            },
        ];
        let bytes = serialize_audit_logs(&logs);
        let result = deserialize_audit_logs(&bytes);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "log-1");
        assert_eq!(result[1].id, "log-2");
    }

    #[test]
    fn test_deserialize_audit_logs_empty_array() {
        let bytes = b"[]";
        let result = deserialize_audit_logs(bytes);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_deserialize_audit_logs_invalid_json() {
        let bytes = b"not json";
        let result = deserialize_audit_logs(bytes);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_serialize_audit_logs_empty() {
        let logs: Vec<AuditLog> = vec![];
        let bytes = serialize_audit_logs(&logs);
        assert_eq!(bytes, b"[]");
    }

    // ============================================================================
    // AuthContext Tests
    // ============================================================================

    #[test]
    fn test_auth_context_new() {
        let metadata = AuthMetadata::default();
        let ctx = AuthContext::new(
            Some("user1".to_string()),
            vec!["read".to_string(), "write".to_string()],
            metadata,
        );
        assert_eq!(ctx.user_id(), Some("user1"));
        assert_eq!(ctx.permissions().len(), 2);
    }

    #[test]
    fn test_auth_context_has_permission_true() {
        let ctx = AuthContext {
            user_id: Some("user1".to_string()),
            permissions: vec!["read".to_string(), "admin".to_string()],
            metadata: AuthMetadata::default(),
        };
        assert!(ctx.has_permission("read"));
        assert!(ctx.has_permission("admin"));
    }

    #[test]
    fn test_auth_context_has_permission_false() {
        let ctx = AuthContext {
            user_id: Some("user1".to_string()),
            permissions: vec!["read".to_string()],
            metadata: AuthMetadata::default(),
        };
        assert!(!ctx.has_permission("write"));
        assert!(!ctx.has_permission("admin"));
    }

    #[test]
    fn test_auth_context_has_permission_empty() {
        let ctx = AuthContext {
            user_id: None,
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };
        assert!(!ctx.has_permission("anything"));
    }

    #[test]
    fn test_auth_context_accessors() {
        let metadata = AuthMetadata {
            client_ip: Some("10.0.0.1".to_string()),
            user_agent: Some("Agent".to_string()),
            request_id: "req-1".to_string(),
            timestamp: 1234567890,
        };
        let ctx = AuthContext {
            user_id: Some("user1".to_string()),
            permissions: vec!["read".to_string()],
            metadata: metadata.clone(),
        };
        assert_eq!(ctx.user_id(), Some("user1"));
        assert_eq!(ctx.permissions(), &["read".to_string()]);
        assert_eq!(ctx.metadata().client_ip(), Some("10.0.0.1"));
    }

    // ============================================================================
    // AuthMetadata Tests
    // ============================================================================

    #[test]
    fn test_auth_metadata_new() {
        let meta = AuthMetadata::new(Some("10.0.0.1".to_string()), Some("Agent".to_string()));
        assert_eq!(meta.client_ip(), Some("10.0.0.1"));
        assert_eq!(meta.user_agent(), Some("Agent"));
        assert!(!meta.request_id().is_empty());
        assert!(meta.timestamp() > 0);
    }

    #[test]
    fn test_auth_metadata_accessors_with_none() {
        let meta = AuthMetadata {
            client_ip: None,
            user_agent: None,
            request_id: "req-1".to_string(),
            timestamp: 1234567890,
        };
        assert!(meta.client_ip().is_none());
        assert!(meta.user_agent().is_none());
        assert_eq!(meta.request_id(), "req-1");
        assert_eq!(meta.timestamp(), 1234567890);
    }

    #[test]
    fn test_auth_metadata_default() {
        let meta = AuthMetadata::default();
        assert!(meta.client_ip().is_none());
        assert!(meta.user_agent().is_none());
        assert_eq!(meta.request_id(), "");
        assert_eq!(meta.timestamp(), 0);
    }

    // ============================================================================
    // AuthExtractor Tests
    // ============================================================================

    #[test]
    fn test_auth_extractor_new() {
        let ctx = AuthContext {
            user_id: Some("user1".to_string()),
            permissions: vec!["read".to_string()],
            metadata: AuthMetadata::default(),
        };
        let extractor = AuthExtractor::new(ctx.clone());
        assert_eq!(extractor.context().user_id, ctx.user_id);
    }

    #[test]
    fn test_auth_extractor_context() {
        let ctx = AuthContext {
            user_id: Some("extractor_user".to_string()),
            permissions: vec!["admin".to_string()],
            metadata: AuthMetadata::default(),
        };
        let extractor = AuthExtractor::new(ctx.clone());
        let retrieved = extractor.context();
        assert_eq!(retrieved.user_id(), Some("extractor_user"));
    }

    #[test]
    fn test_auth_extractor_into_inner() {
        let ctx = AuthContext {
            user_id: Some("inner_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };
        let extractor = AuthExtractor::new(ctx.clone());
        let inner = extractor.into_inner();
        assert_eq!(inner.user_id, Some("inner_user".to_string()));
    }

    // ============================================================================
    // JwtError Display Tests
    // ============================================================================

    #[test]
    fn test_jwt_error_invalid_format() {
        let err = JwtError::InvalidFormat;
        assert_eq!(format!("{}", err), "Invalid JWT format");
    }

    #[test]
    fn test_jwt_error_base64_decode() {
        let err = JwtError::Base64DecodeError;
        assert_eq!(format!("{}", err), "Failed to decode base64");
    }

    #[test]
    fn test_jwt_error_invalid_signature() {
        let err = JwtError::InvalidSignature;
        assert_eq!(format!("{}", err), "Invalid JWT signature");
    }

    #[test]
    fn test_jwt_error_expired() {
        let err = JwtError::Expired;
        assert_eq!(format!("{}", err), "JWT token expired");
    }

    #[test]
    fn test_jwt_error_not_yet_valid() {
        let err = JwtError::NotYetValid;
        assert_eq!(format!("{}", err), "JWT token not yet valid");
    }

    #[test]
    fn test_jwt_error_invalid_payload() {
        let err = JwtError::InvalidPayload;
        assert_eq!(format!("{}", err), "Invalid JWT payload");
    }

    #[test]
    fn test_jwt_error_clock_skew() {
        let err = JwtError::ClockSkew;
        assert_eq!(format!("{}", err), "Clock skew too large");
    }

    #[test]
    fn test_jwt_error_is_std_error() {
        let err = JwtError::Expired;
        let _: &dyn std::error::Error = &err;
    }

    // ============================================================================
    // AuthConfigError Display Tests
    // ============================================================================

    #[test]
    fn test_auth_config_error_invalid_secret() {
        let err = AuthConfigError::InvalidSecret("too weak".to_string());
        assert!(format!("{}", err).contains("Invalid secret"));
    }

    #[test]
    fn test_auth_config_error_secret_too_short() {
        let err = AuthConfigError::SecretTooShort { length: 10 };
        let msg = format!("{}", err);
        assert!(msg.contains("Secret too short"));
        assert!(msg.contains("10"));
    }

    #[test]
    fn test_auth_config_error_missing_character_class() {
        let err = AuthConfigError::MissingCharacterClass {
            required_type: "uppercase letter",
        };
        let msg = format!("{}", err);
        assert!(msg.contains("uppercase letter"));
    }

    #[test]
    fn test_auth_config_error_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = AuthConfigError::from(io_err);
        let msg = format!("{}", err);
        assert!(msg.contains("Configuration I/O error"));
    }

    // ============================================================================
    // AuditResult Custom Serialize Tests
    // ============================================================================

    #[test]
    fn test_audit_result_serialize_success() {
        let result = AuditResult::Success;
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""status":"success""#));
    }

    #[test]
    fn test_audit_result_serialize_failure() {
        let result = AuditResult::Failure {
            message: "Access denied".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""status":"failure""#));
        assert!(json.contains("Access denied"));
    }

    #[test]
    fn test_audit_log_serialize_with_audit_result() {
        let log = AuditLog {
            id: "ser-test".to_string(),
            timestamp: 1234567890,
            user_id: Some("user1".to_string()),
            action: "TEST".to_string(),
            resource: "/test".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };
        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains(r#""status":"success""#));
    }

    #[test]
    fn test_audit_result_clone() {
        let result = AuditResult::Failure {
            message: "clone test".to_string(),
        };
        let cloned = result.clone();
        match cloned {
            AuditResult::Failure { message } => {
                assert_eq!(message, "clone test");
            }
            _ => panic!("Expected Failure"),
        }
    }

    #[test]
    fn test_audit_result_debug() {
        let result = AuditResult::Success;
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("Success"));
    }

    // ============================================================================
    // AuthError Display Tests
    // ============================================================================

    #[test]
    fn test_auth_error_missing_auth() {
        let err = AuthError::MissingAuth;
        assert_eq!(
            format!("{}", err),
            "Missing or invalid authorization header"
        );
    }

    #[test]
    fn test_auth_error_invalid_token() {
        let err = AuthError::InvalidToken;
        assert_eq!(format!("{}", err), "Invalid or expired token");
    }

    #[test]
    fn test_auth_error_insufficient_permissions() {
        let err = AuthError::InsufficientPermissions {
            required: "admin".to_string(),
            user_permissions: vec!["read".to_string()],
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Insufficient permissions"));
        assert!(msg.contains("admin"));
    }

    // ============================================================================
    // Additional CacheNamespace Tests (Copy, Clone, PartialEq, Debug)
    // ============================================================================

    #[test]
    fn test_cache_namespace_copy() {
        let ns = CacheNamespace::ApiKey;
        let ns_copy = ns;
        assert_eq!(ns, ns_copy);
    }

    #[test]
    fn test_cache_namespace_clone() {
        let ns = CacheNamespace::BearerBlacklist;
        let ns_copy = ns;
        assert_eq!(ns, ns_copy);
    }

    #[test]
    fn test_cache_namespace_partial_eq() {
        let ns1 = CacheNamespace::BearerValid;
        let ns2 = CacheNamespace::BearerValid;
        let ns3 = CacheNamespace::ApiKey;
        assert_eq!(ns1, ns2);
        assert_ne!(ns1, ns3);
    }

    #[test]
    fn test_cache_namespace_debug() {
        let ns = CacheNamespace::Idempotency;
        let debug_str = format!("{:?}", ns);
        assert!(debug_str.contains("Idempotency"));
    }

    // ============================================================================
    // Additional Serialization Tests (Special Cases)
    // ============================================================================

    #[test]
    fn test_serialize_permissions_special_chars() {
        let perms = vec![
            "read:admin".to_string(),
            "write@user".to_string(),
            "delete#1".to_string(),
        ];
        let bytes = serialize_permissions(&perms);
        let result = deserialize_permissions(&bytes);
        assert_eq!(result, perms);
    }

    #[test]
    fn test_deserialize_permissions_empty_data() {
        let result = deserialize_permissions(b"");
        assert_eq!(result, Vec::<String>::new());
    }

    // ============================================================================
    // Additional AuthContext Tests (All Fields, Empty Fields)
    // ============================================================================

    #[test]
    fn test_auth_context_with_all_fields() {
        let ctx = AuthContext {
            user_id: Some("admin_user".to_string()),
            permissions: vec![
                "read".to_string(),
                "write".to_string(),
                "delete".to_string(),
                "admin".to_string(),
            ],
            metadata: AuthMetadata {
                client_ip: Some("10.20.30.40".to_string()),
                user_agent: Some("Mozilla/5.0".to_string()),
                request_id: "full-req-123".to_string(),
                timestamp: 1700000000,
            },
        };
        assert_eq!(ctx.user_id(), Some("admin_user"));
        assert_eq!(ctx.permissions().len(), 4);
        assert!(ctx.has_permission("admin"));
        assert_eq!(ctx.metadata().client_ip(), Some("10.20.30.40"));
    }

    #[test]
    fn test_auth_context_empty_fields() {
        let ctx = AuthContext {
            user_id: None,
            permissions: vec![],
            metadata: AuthMetadata {
                client_ip: None,
                user_agent: None,
                request_id: "".to_string(),
                timestamp: 0,
            },
        };
        assert_eq!(ctx.user_id(), None);
        assert!(ctx.permissions().is_empty());
        assert!(!ctx.has_permission("anything"));
    }

    // ============================================================================
    // Additional AuthConfigError Tests
    // ============================================================================

    #[test]
    fn test_auth_config_error_display() {
        let err = AuthConfigError::InvalidSecret("test error".to_string());
        let msg = format!("{}", err);
        assert_eq!(msg, "Invalid secret: test error");
    }

    // ============================================================================
    // Edge Case Tests for Serialization
    // ============================================================================

    #[test]
    fn test_serialize_instants_empty() {
        let instants: Vec<std::time::Instant> = vec![];
        let bytes = serialize_instants(&instants);
        let result = deserialize_instants(&bytes);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_serialize_auth_context_empty() {
        let ctx = AuthContext::new(None, vec![], AuthMetadata::default());
        let bytes = serialize_auth_context(&ctx);
        let result = deserialize_auth_context(&bytes);
        assert!(result.is_some());
        let restored = result.unwrap();
        assert_eq!(restored.user_id, None);
        assert!(restored.permissions.is_empty());
    }

    #[test]
    fn test_serialize_auth_context_roundtrip() {
        let ctx = AuthContext {
            user_id: Some("roundtrip_user".to_string()),
            permissions: vec!["test".to_string()],
            metadata: AuthMetadata {
                client_ip: Some("127.0.0.1".to_string()),
                user_agent: None,
                request_id: "test-roundtrip".to_string(),
                timestamp: 9999999999,
            },
        };
        let bytes = serialize_auth_context(&ctx);
        let result = deserialize_auth_context(&bytes);
        assert!(result.is_some());
        let restored = result.unwrap();
        assert_eq!(restored.user_id, ctx.user_id);
        assert_eq!(restored.permissions, ctx.permissions);
    }

    // ============================================================================
    // AuthMetadata Additional Tests
    // ============================================================================

    #[test]
    fn test_auth_metadata_accessors() {
        let meta = AuthMetadata {
            client_ip: Some("192.168.0.1".to_string()),
            user_agent: Some("TestBrowser".to_string()),
            request_id: "accessor-test".to_string(),
            timestamp: 1234567890,
        };
        assert_eq!(meta.client_ip(), Some("192.168.0.1"));
        assert_eq!(meta.user_agent(), Some("TestBrowser"));
        assert_eq!(meta.request_id(), "accessor-test");
        assert_eq!(meta.timestamp(), 1234567890);
    }

    // ============================================================================
    // CacheNamespace Additional Tests
    // ============================================================================

    #[test]
    fn test_cache_namespace_key_empty_suffix() {
        let ns = CacheNamespace::ApiKey;
        let key = ns.key("");
        assert_eq!(key, "sdforge:apikey:");
    }

    #[test]
    fn test_cache_namespace_key_special_suffix() {
        let ns = CacheNamespace::BearerValid;
        let key = ns.key("token/special?chars=here");
        assert_eq!(key, "sdforge:bearer:valid:token/special?chars=here");
    }

    // ============================================================================
    // Additional Permission Tests
    // ============================================================================

    #[test]
    fn test_permissions_large_list() {
        let perms: Vec<String> = (0..1000).map(|i| format!("perm_{}", i)).collect();
        let bytes = serialize_permissions(&perms);
        let result = deserialize_permissions(&bytes);
        assert_eq!(result.len(), 1000);
        assert_eq!(result[0], "perm_0");
        assert_eq!(result[999], "perm_999");
    }
}
