// Copyright (c) 2026 Kirky.X
//! Shared types for the security module
//!
//! This module contains all common types used across the security module.

use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use std::time::{Duration, Instant};
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
    /// API Key failure tracking: `sdforge:apifailed:{client_ip}`
    ApiFailed,
    /// Bearer token blacklist: `sdforge:bearer:blacklist:{token}`
    BearerBlacklist,
    /// Bearer token valid cache: `sdforge:bearer:valid:{token}`
    BearerValid,
    /// Rate limiting: `sdforge:rl:{key}`
    RateLimit,
    /// Idempotency key cache: `sdforge:idempotency:{key}`
    Idempotency,
}

impl CacheNamespace {
    /// Generate the full cache key with namespace prefix
    pub fn key(&self, suffix: &str) -> String {
        match self {
            CacheNamespace::ApiKey => format!("sdforge:apikey:{suffix}"),
            CacheNamespace::ApiFailed => format!("sdforge:apifailed:{suffix}"),
            CacheNamespace::BearerBlacklist => format!("sdforge:bearer:blacklist:{suffix}"),
            CacheNamespace::BearerValid => format!("sdforge:bearer:valid:{suffix}"),
            CacheNamespace::RateLimit => format!("sdforge:rl:{suffix}"),
            CacheNamespace::Idempotency => format!("sdforge:idempotency:{suffix}"),
        }
    }
}

// =============================================================================
// O(1) Rate Limiting State (Fixed Window Counter)
// =============================================================================

/// Window state for O(1) rate limiting
///
/// Uses fixed window counter algorithm instead of storing all timestamps.
/// This provides O(1) check operations with a small accuracy trade-off
/// at window boundaries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct WindowState {
    /// Current count of requests in this window
    pub(crate) count: u64,
    /// Window start time in seconds since an arbitrary epoch (using Instant::now().elapsed())
    pub(crate) window_start_secs: u64,
}

// =============================================================================
// Serialization Helpers for SyncCache Storage
// =============================================================================

/// Serialize a list of Instants to bytes using bincode
pub(crate) fn serialize_instants(insts: &[Instant]) -> Vec<u8> {
    let as_i64: Vec<i64> = insts.iter().map(|i| i.elapsed().as_secs() as i64).collect();
    bincode::serialize(&as_i64).unwrap_or_default()
}

/// Deserialize a list of Instants from bytes using bincode
pub(crate) fn deserialize_instants(data: &[u8]) -> Vec<Instant> {
    let as_i64: Vec<i64> = match bincode::deserialize(data) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    as_i64
        .iter()
        .map(|&s| Instant::now() - Duration::from_secs(s as u64))
        .collect()
}

/// Serialize a list of permissions (Vec<String>) to bytes
pub(crate) fn serialize_permissions(perms: &[String]) -> Vec<u8> {
    bincode::serialize(perms).unwrap_or_default()
}

/// Deserialize a list of permissions from bytes
pub(crate) fn deserialize_permissions(data: &[u8]) -> Vec<String> {
    bincode::deserialize(data).unwrap_or_default()
}

/// Serialize WindowState to bytes
pub(crate) fn serialize_window_state(state: &WindowState) -> Vec<u8> {
    bincode::serialize(state).unwrap_or_default()
}

/// Deserialize WindowState from bytes
pub(crate) fn deserialize_window_state(data: &[u8]) -> Option<WindowState> {
    bincode::deserialize(data).ok()
}

/// Serialize AuthContext to bytes using bincode
#[allow(dead_code)]
pub(crate) fn serialize_auth_context(ctx: &AuthContext) -> Vec<u8> {
    bincode::serialize(ctx).unwrap_or_default()
}

/// Deserialize AuthContext from bytes using bincode
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
// Rate Limiting Types
// =============================================================================

/// Rate limit configuration
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

/// Rate limit error
#[derive(Debug, Clone)]
pub struct RateLimitError {
    /// Rate limit
    pub limit: u32,
    /// Remaining requests
    pub remaining: u32,
    /// Retry after seconds
    pub retry_after: u64,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rate limit exceeded. Try again in {} seconds",
            self.retry_after
        )
    }
}

impl std::error::Error for RateLimitError {}

// =============================================================================
// Audit Types
// =============================================================================

/// Audit log entry
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
}

impl Serialize for AuditLog {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AuditLog", 7)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("timestamp", &self.timestamp)?;
        s.serialize_field("user_id", &self.user_id)?;
        s.serialize_field("action", &self.action)?;
        s.serialize_field("resource", &self.resource)?;
        s.serialize_field("result", &self.result)?;
        s.serialize_field("metadata", &self.metadata)?;
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
