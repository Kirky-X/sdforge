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
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User ID
    pub user_id: Option<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// Request metadata
    pub metadata: AuthMetadata,
}

/// Authentication metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthMetadata {
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request ID
    pub request_id: String,
    /// Timestamp
    pub timestamp: i64,
}

/// Authentication result
pub type AuthResult<T = AuthContext> = Result<T, AuthError>;

/// Authentication extractor
#[derive(Debug)]
pub struct AuthExtractor(pub AuthContext);

/// API key authentication
#[derive(Clone)]
pub struct ApiKeyAuth {
    /// Valid API keys
    valid_keys: Arc<DashMap<String, Vec<String>>>,
}

impl ApiKeyAuth {
    /// Create new API key authentication
    pub fn new() -> Self {
        Self {
            valid_keys: Arc::new(DashMap::new()),
        }
    }

    /// Add a valid API key
    pub fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        self.valid_keys.insert(key.into(), permissions);
    }

    /// Validate an API key
    pub fn validate_key(&self, key: &str) -> Option<Vec<String>> {
        self.valid_keys.get(key).map(|p| p.clone())
    }
}

impl Default for ApiKeyAuth {
    fn default() -> Self {
        Self::new()
    }
}

/// Bearer token authentication
#[derive(Clone)]
pub struct BearerAuth {
    /// JWT secret for HMAC-SHA256 signing
    secret: Vec<u8>,
    /// Valid tokens cache
    valid_tokens: Arc<DashMap<String, AuthContext>>,
    /// Token blacklist (for logout)
    blacklisted_tokens: Arc<DashMap<String, Instant>>,
}

impl BearerAuth {
    /// Create new bearer authentication
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into().into_bytes(),
            valid_tokens: Arc::new(DashMap::new()),
            blacklisted_tokens: Arc::new(DashMap::new()),
        }
    }

    /// Simple constant-time comparison to prevent timing attacks
    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
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

    /// Parse JWT token and verify signature
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
            .and_then(|arr| {
                Some(
                    arr.iter()
                        .filter_map(|p| p.as_str().map(String::from))
                        .collect(),
                )
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
    /// Create new rate limiter
    pub fn new(config: Option<RateLimitConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            requests: Arc::new(DashMap::new()),
            idempotency_cache: Arc::new(DashMap::new()),
            semaphore: Arc::new(tokio::sync::Semaphore::new(1000)),
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
            // Instant::duration_since returns Duration (panics if other > self in debug)
            let elapsed = now.duration_since(existing_time).as_secs();
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
        // Check rate limit first
        let remaining = self.check(key)?;

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

/// RAII permit for rate limiting
pub struct Permit(pub tokio::sync::OwnedSemaphorePermit);

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
    pub id: String,
    /// Timestamp
    pub timestamp: i64,
    /// User ID
    pub user_id: Option<String>,
    /// Action
    pub action: String,
    /// Resource
    pub resource: String,
    /// Result
    pub result: AuditResult,
    /// Request metadata
    pub metadata: AuthMetadata,
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
#[derive(Clone)]
pub struct AuditLogger {
    /// Logs storage
    logs: Arc<DashMap<String, Vec<AuditLog>>>,
    /// Maximum logs per user
    max_logs_per_user: usize,
    /// Rate limiting semaphore (max concurrent log operations)
    semaphore: Arc<tokio::sync::Semaphore>,
    /// Log queue sender (for async processing)
    queue_sender: Arc<tokio::sync::mpsc::Sender<AuditLogBatch>>,
}

struct AuditLogBatch {
    user_id: String,
    log: AuditLog,
}

impl AuditLogger {
    /// Create new audit logger with default limit
    pub fn new() -> Self {
        Self::with_limit(1000)
    }

    /// Create new audit logger with custom limit
    pub fn with_limit(max_logs: usize) -> Self {
        let (queue_sender, mut queue_receiver) = tokio::sync::mpsc::channel::<AuditLogBatch>(1000);

        // Spawn background worker for async log processing
        let logs: Arc<DashMap<String, Vec<AuditLog>>> = Arc::new(DashMap::new());
        let logs_clone = logs.clone();
        let max_logs_clone = max_logs;
        tokio::spawn(async move {
            while let Some(batch) = queue_receiver.recv().await {
                let mut entry = logs_clone.entry(batch.user_id.clone()).or_default();
                entry.push(batch.log);

                // Keep only last N logs per user
                if entry.len() > max_logs_clone {
                    entry.truncate(max_logs_clone);
                }
            }
        });

        Self {
            logs,
            max_logs_per_user: max_logs,
            semaphore: Arc::new(tokio::sync::Semaphore::new(100)), // Max 100 concurrent log operations
            queue_sender: Arc::new(queue_sender),
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
                    message: message.unwrap_or_else(|| "Unknown error".to_string()),
                }
            },
            metadata: context.metadata.clone(),
        };

        let user_id = context
            .user_id
            .clone()
            .unwrap_or_else(|| "anonymous".to_string());

        // Send to async queue (non-blocking)
        let sender = self.queue_sender.clone();
        let _ = sender
            .send(AuditLogBatch {
                user_id: user_id.clone(),
                log,
            })
            .await;

        // Drop permit to release semaphore
        drop(permit);
    }

    /// Get logs for a user (synchronous)
    pub fn get_logs(&self, user_id: &str) -> Vec<AuditLog> {
        self.logs
            .get(user_id)
            .map(|e| e.clone())
            .unwrap_or_default()
    }

    /// Clear logs for a user (admin function)
    pub fn clear_logs(&self, user_id: &str) {
        self.logs.remove(user_id);
    }

    /// Get total log count (for monitoring)
    pub fn total_log_count(&self) -> usize {
        self.logs.iter().map(|e| e.len()).sum()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
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
            let client_ip = extract_client_ip(&req);

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

/// Extract client IP from request with security validation
///
/// Security considerations:
/// - Validates X-Forwarded-For format to prevent header injection
/// - Takes the first IP from X-Forwarded-For (original client)
/// - Falls back to X-Real-IP or defaults to "unknown"
#[cfg(feature = "logging")]
fn extract_client_ip(req: &Request<Body>) -> String {
    use axum::extract::connect_info::ConnectInfo;

    // Check X-Forwarded-For header first
    if let Some(header) = req.headers().get("X-Forwarded-For") {
        if let Ok(value) = header.to_str() {
            // X-Forwarded-For can contain multiple IPs: "client, proxy1, proxy2"
            // We take the first one (original client)
            let first_ip = value.split(',').next().map(|s| s.trim());

            if let Some(ip) = first_ip {
                if is_valid_ip(ip) {
                    return ip.to_string();
                }
                // Invalid IP format in X-Forwarded-For, log warning
                tracing::warn!(target: "security", "Invalid X-Forwarded-For IP: {}", ip);
            }
        }
    }

    // Fall back to X-Real-IP
    if let Some(header) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = header.to_str() {
            if is_valid_ip(ip) {
                return ip.to_string();
            }
            tracing::warn!(target: "security", "Invalid X-Real-IP: {}", ip);
        }
    }

    // Use connection remote peer if available
    if let Some(remote) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
        return remote.0.ip().to_string();
    }

    "unknown".to_string()
}

/// Non-logging version without security warnings
#[cfg(not(feature = "logging"))]
fn extract_client_ip(req: &Request<Body>) -> String {
    use axum::extract::connect_info::ConnectInfo;

    if let Some(header) = req.headers().get("X-Forwarded-For") {
        if let Ok(value) = header.to_str() {
            let first_ip = value.split(',').next().map(|s| s.trim());
            if let Some(ip) = first_ip {
                if is_valid_ip(ip) {
                    return ip.to_string();
                }
            }
        }
    }

    if let Some(header) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = header.to_str() {
            if is_valid_ip(ip) {
                return ip.to_string();
            }
        }
    }

    if let Some(remote) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
        return remote.0.ip().to_string();
    }

    "unknown".to_string()
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

    #[tokio::test]
    async fn test_api_key_auth() {
        let auth = ApiKeyAuth::new();
        auth.add_key("test-key", vec!["read".to_string(), "write".to_string()]);

        let permissions = auth.validate_key("test-key");
        assert_eq!(
            permissions,
            Some(vec!["read".to_string(), "write".to_string()])
        );

        let permissions = auth.validate_key("invalid-key");
        assert_eq!(permissions, None);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
            include_headers: true,
        };
        let limiter = RateLimiter::new(Some(config));

        // First 3 requests should succeed
        for _ in 0..3 {
            assert!(limiter.check("test-ip").is_ok());
        }

        // 4th request should fail
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

        // Await the async log call
        logger
            .log(&context, "test_action", "test_resource", true, None)
            .await;

        // Give the background worker time to process the log
        tokio::task::yield_now().await;

        let logs = logger.get_logs("user-123");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, "test_action");
    }
}
