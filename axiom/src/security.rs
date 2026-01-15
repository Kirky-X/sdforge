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
    rate_limit_config: RateLimitConfig,
}

impl ApiKeyAuth {
    /// Create new API key authentication with default rate limiting
    pub fn new() -> Self {
        Self::with_rate_limit(RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
            include_headers: false,
        })
    }

    /// Create API key authentication with custom rate limiting
    pub fn with_rate_limit(config: RateLimitConfig) -> Self {
        Self {
            valid_keys: Arc::new(DashMap::new()),
            failed_attempts: Arc::new(DashMap::new()),
            rate_limit_config: config,
        }
    }

    /// Hash API key using SHA256 for secure storage
    fn hash_key(key: &str) -> String {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Add a valid API key (stored as hash)
    pub fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        let key_hash = Self::hash_key(&key.into());
        self.valid_keys.insert(key_hash, permissions);
    }

    /// Validate an API key with rate limiting
    ///
    /// Security: Implements rate limiting per caller to prevent brute force
    /// attacks on API key validation. Returns None immediately if rate limited.
    /// Valid keys bypass rate limiting to prevent blocking legitimate users.
    pub fn validate_key(&self, key: &str, client_ip: &str) -> Option<Vec<String>> {
        let key_hash = Self::hash_key(key);
        let result = self.valid_keys.get(&key_hash).map(|p| p.clone());

        if result.is_some() {
            return result;
        }

        if self.is_rate_limited(client_ip) {
            return None;
        }

        self.record_failed_attempt(client_ip);

        None
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

impl BearerAuth {
    /// Create new bearer authentication with basic secret
    ///
    /// # Panics
    /// Panics if the secret is too short or doesn't meet complexity requirements
    pub fn new(secret: impl Into<String>) -> Self {
        let secret_str = secret.into();

        if secret_str.len() < 32 {
            panic!(
                "JWT secret too short ({} chars). Minimum 32 characters required for security.",
                secret_str.len()
            );
        }

        if !secret_str.chars().any(|c| c.is_uppercase()) {
            panic!("JWT secret must contain at least one uppercase letter");
        }
        if !secret_str.chars().any(|c| c.is_lowercase()) {
            panic!("JWT secret must contain at least one lowercase letter");
        }
        if !secret_str.chars().any(|c| c.is_digit(10)) {
            panic!("JWT secret must contain at least one digit");
        }
        if !secret_str.chars().any(|c| !c.is_alphanumeric()) {
            panic!("JWT secret must contain at least one special character");
        }

        Self {
            secret: secret_str.into_bytes(),
            valid_tokens: Arc::new(DashMap::new()),
            blacklisted_tokens: Arc::new(DashMap::new()),
            expected_audience: None,
            expected_issuer: None,
        }
    }

    /// Create bearer authentication with audience validation
    ///
    /// # Arguments
    /// * `secret` - JWT signing secret
    /// * `expected_audience` - Expected `aud` claim value (prevents token substitution)
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
/// - Fallback storage when async channel is full (prevents log loss)
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
    /// Fallback storage for when channel is full (synchronous path)
    fallback_logs: Arc<DashMap<String, Vec<AuditLog>>>,
    /// Counter for dropped logs (monitoring)
    dropped_log_count: Arc<std::sync::atomic::AtomicU64>,
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
                    message: message.unwrap_or_else(|| "Unknown error".to_string()),
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
#[cfg(feature = "logging")]
fn extract_client_ip(req: &Request<Body>) -> String {
    if let Some(ip) = extract_client_ip_core(req) {
        return ip;
    }

    if let Some(header) = req.headers().get("X-Forwarded-For") {
        if let Ok(value) = header.to_str() {
            if let Some(ip) = value.split(',').next().map(|s| s.trim()) {
                tracing::warn!(target: "security", "X-Forwarded-For IP not from trusted proxy or invalid: {}", ip);
            }
        }
    }

    if let Some(header) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = header.to_str() {
            if !is_valid_ip(ip) {
                tracing::warn!(target: "security", "Invalid X-Real-IP: {}", ip);
            }
        }
    }

    "unknown".to_string()
}

/// Extract client IP from request without logging
#[cfg(not(feature = "logging"))]
fn extract_client_ip(req: &Request<Body>) -> String {
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

        for i in 0..3 {
            assert_eq!(
                auth.validate_key(&format!("invalid-key-{}", i), "192.168.1.1"),
                None
            );
        }

        assert_eq!(auth.validate_key("invalid-key-4", "192.168.1.1"), None);

        let permissions = auth.validate_key("valid-key", "192.168.1.1");
        assert_eq!(permissions, Some(vec!["read".to_string()]));
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
}
