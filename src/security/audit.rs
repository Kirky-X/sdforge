// Copyright (c) 2026 Kirky.X
//! Audit logging implementation
//!
//! This module provides audit logging with DoS protection and async processing.

use crate::cache::SharedCache;
use crate::security::types::{
    deserialize_audit_logs, serialize_audit_logs, AuditLog, AuditResult, AuthContext, AuthMetadata,
};
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Batch of audit logs for async processing.
///
/// Internal struct used to pass user ID and log entry through the async channel.
pub(crate) struct AuditLogBatch {
    user_id: String,
    #[allow(dead_code)] // Field used in queue transfer; direct read access not needed
    log: AuditLog,
}

// =============================================================================
// Global State - Pre-compiled Regex Patterns for Sanitization
// =============================================================================
// These are immutable, thread-safe, pre-compiled regex patterns.
// Using Lazy initialization ensures they are compiled only once at first use,
// providing optimal performance while maintaining safety guarantees.
//
// Design Rationale:
// - Immutable after initialization (no mutable global state)
// - Thread-safe access via once_cell::Lazy
// - Performance optimization (avoid re-compilation on each call)
// - No dependency injection needed (these are pure functions with no side effects)
// =============================================================================

/// Pattern to match JWT tokens (three base64url-encoded segments separated by dots)
///
/// Used to detect and redact JWT tokens from error messages to prevent token leakage.
/// Format: header.payload.signature (each segment is base64url encoded)
static JWT_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r#"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+"#).unwrap()
});

/// Pattern to match sensitive key-value pairs (passwords, secrets, tokens, keys)
///
/// Matches patterns like:
/// - `password=secret123`
/// - `token: abcdef`
/// - `api_key: xyz789`
///
/// Limited repetition (1-100 chars) prevents ReDoS attacks.
static SECRET_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    // Limited repetition to prevent ReDoS attacks
    // Maximum 100 characters after the separator
    regex::Regex::new(r#"(?i)(password|secret|token|key|auth|bearer)\s*[:=]\s*[^,\s}\]]{1,100}"#)
        .unwrap()
});

/// Pattern to match certificate/key file paths
///
/// Matches paths ending with common certificate extensions:
/// - `.pem` - Privacy Enhanced Mail certificate
/// - `.key` - Private key file
/// - `.crt` - Certificate file
/// - `.p12` - PKCS#12 archive
/// - `.jks` - Java KeyStore
static PATH_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r#"/[a-zA-Z0-9/_.-]+\.(pem|key|crt|p12|jks)"#).unwrap());

/// Pattern to match API keys in error messages (case-insensitive key=value form)
static API_KEY_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['\"]?[A-Za-z0-9]{20,}['\"]?"#)
        .unwrap()
});

/// Pattern to match credit card numbers (16 digits, optional separators)
static CREDIT_CARD_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r#"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b"#).unwrap());

/// Pattern to match US Social Security Numbers
static SSN_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r#"\b\d{3}[-\s]?\d{2}[-\s]?\d{4}\b"#).unwrap());

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
    result = API_KEY_PATTERN
        .replace_all(&result, "[REDACTED_API_KEY]")
        .to_string();

    // Remove credit card numbers
    result = CREDIT_CARD_PATTERN
        .replace_all(&result, "[REDACTED_CREDIT_CARD]")
        .to_string();

    // Remove SSN numbers
    result = SSN_PATTERN
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

/// Audit logger with DoS protection
///
/// Security features:
/// - Semaphore-based rate limiting to prevent log flooding
/// - Per-user log count limits
/// - Async processing to avoid blocking main threads
/// - Fallback storage when async channel is full (prevents log loss)
///
#[derive(Clone)]
pub struct AppAuditLogger {
    /// Logs storage via SyncCache (keyed by user_id)
    logs: SharedCache,
    /// Maximum logs per user
    max_logs_per_user: usize,
    /// Rate limiting semaphore (max concurrent log operations)
    semaphore: Arc<tokio::sync::Semaphore>,
    /// Log queue sender (for async processing)
    queue_sender: Arc<tokio::sync::mpsc::Sender<AuditLogBatch>>,
    /// Fallback storage for when channel is full (synchronous path) via SyncCache
    fallback_logs: SharedCache,
    /// Counter for dropped logs (monitoring)
    dropped_log_count: Arc<std::sync::atomic::AtomicU64>,
    /// Counter for total logs successfully stored (monitoring)
    total_log_count: Arc<std::sync::atomic::AtomicU64>,
}

impl Default for AppAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AppAuditLogger {
    /// Create a new AppAuditLoggerBuilder for custom configuration.
    ///
    /// This is the recommended way to create an AppAuditLogger when you need
    /// to customize any of the default settings.
    ///
    /// # Returns
    ///
    /// Returns a new AppAuditLoggerBuilder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use sdforge::security::AppAuditLogger;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let logger = AppAuditLogger::builder()
    ///         .max_logs_per_user(500)
    ///         .max_concurrent_ops(50)
    ///         .queue_size(2000)
    ///         .build();
    ///     let _ = logger;
    /// }
    /// ```
    pub fn builder() -> AppAuditLoggerBuilder {
        AppAuditLoggerBuilder::new()
    }

    /// Create new audit logger with default limit
    pub fn new() -> Self {
        Self::with_limit(1000)
    }

    /// Create new audit logger with custom limit
    pub fn with_limit(max_logs: usize) -> Self {
        let (queue_sender, mut queue_receiver) = tokio::sync::mpsc::channel::<AuditLogBatch>(1000);

        // Spawn background worker for async log processing
        let logs: SharedCache = Arc::new(crate::cache::DashMapCache::new());
        let fallback_logs: SharedCache = Arc::new(crate::cache::DashMapCache::new());
        let logs_clone = logs.clone();
        let fallback_logs_clone = fallback_logs.clone();
        let max_logs_clone = max_logs;
        tokio::spawn(async move {
            // Primary storage is done synchronously by log() — this worker only
            // handles draining the queue and merging fallback logs.
            while let Some(batch) = queue_receiver.recv().await {
                let key = &batch.user_id;
                // Drain the queue: primary log was already stored by log().
                // Just handle any fallback logs for this user.
                if let Some(fallback_data) = fallback_logs_clone.get(key) {
                    let fallback: Vec<AuditLog> = deserialize_audit_logs(&fallback_data);
                    if !fallback.is_empty() {
                        let data = logs_clone.get(key);
                        let mut logs_vec: Vec<AuditLog> = data
                            .as_ref()
                            .map(|d| deserialize_audit_logs(d))
                            .unwrap_or_default();
                        logs_vec.extend(fallback);
                        if logs_vec.len() > max_logs_clone {
                            logs_vec.truncate(max_logs_clone);
                        }
                        logs_clone.set(key, serialize_audit_logs(&logs_vec));
                        fallback_logs_clone.delete(key);
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
            total_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
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

        let mut log = AuditLog {
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
            signature: None, // Will be signed if a signing key is configured
        };

        // Generate cryptographic signature for tamper detection (if enabled)
        // In production, you should configure a signing key via secure configuration
        // Security: Use a cryptographically secure random key of at least 32 bytes
        if let Ok(signing_key_str) = std::env::var("SDFORGE_AUDIT_SIGNING_KEY") {
            if !signing_key_str.is_empty() {
                // Convert string to bytes and use as HMAC key
                log.generate_signature(signing_key_str.as_bytes());
            } else {
                eprintln!("⚠️  WARNING: SDFORGE_AUDIT_SIGNING_KEY is empty. Audit logs will not be signed.");
            }
        } else {
            eprintln!(
                "⚠️  WARNING: SDFORGE_AUDIT_SIGNING_KEY not set. Audit logs will not be signed."
            );
            eprintln!("   For production, set this environment variable to a secure random value (min 32 bytes).");
            eprintln!("   Example: export SDFORGE_AUDIT_SIGNING_KEY=$(openssl rand -hex 32)");
        }

        let user_id = context
            .user_id
            .clone()
            .unwrap_or_else(|| "anonymous".to_string());

        // === Synchronous write to primary storage ===
        // This ensures logs are immediately visible via get_logs() without
        // relying on the async worker being scheduled. Critical for tests and
        // for any code that reads logs immediately after logging.
        let key = &user_id;
        let data = self.logs.get(key);
        let mut logs_vec: Vec<AuditLog> = data
            .as_ref()
            .map(|d| deserialize_audit_logs(d))
            .unwrap_or_default();
        logs_vec.push(log.clone());
        if logs_vec.len() > self.max_logs_per_user {
            logs_vec.truncate(self.max_logs_per_user);
        }
        let bytes = serialize_audit_logs(&logs_vec);
        self.logs.set(key, bytes);

        // Increment the total log counter for accurate monitoring.
        // (Previously total_log_count() silently returned 0, hiding real activity.)
        self.total_log_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Also merge any pending fallback logs synchronously (worker will also do this)
        if let Some(fallback_data) = self.fallback_logs.get(key) {
            let fallback: Vec<AuditLog> = deserialize_audit_logs(&fallback_data);
            if !fallback.is_empty() {
                let mut merged = logs_vec;
                merged.extend(fallback);
                if merged.len() > self.max_logs_per_user {
                    merged.truncate(self.max_logs_per_user);
                }
                self.logs.set(key, serialize_audit_logs(&merged));
                self.fallback_logs.delete(key);
            }
        }

        // Send to async queue for potential downstream consumers.
        // The primary storage is already done above. This is fire-and-forget
        // for background processing that may have been relying on the queue.
        let sender = self.queue_sender.clone();
        let log_batch = AuditLogBatch {
            user_id: user_id.clone(),
            log,
        };

        // Try non-blocking send — primary storage is already complete above
        match sender.try_send(log_batch) {
            Ok(()) => {
                // Audit log queued successfully
            }
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                // Channel is full — primary storage already done above (synchronous path)
                // dropped_log_count is NOT incremented since we stored successfully
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                // Channel closed — primary storage already done above
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
            .map(|data| deserialize_audit_logs(&data))
            .unwrap_or_default();

        // Get logs from fallback storage
        let fallback = self
            .fallback_logs
            .get(user_id)
            .map(|data| deserialize_audit_logs(&data))
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
        self.logs.delete(user_id);
    }

    /// Log an API key rotation event.
    ///
    /// # Arguments
    ///
    /// * `key_id` - The ID of the API key being rotated
    /// * `old_version` - The version being rotated from
    /// * `new_version` - The version being rotated to
    /// * `success` - Whether the rotation was successful
    /// * `message` - Optional message with additional details
    pub async fn log_key_rotation(
        &self,
        _key_id: &str,
        _old_version: &str,
        _new_version: &str,
        success: bool,
        message: Option<String>,
    ) {
        let context = AuthContext {
            user_id: Some("system".to_string()),
            permissions: vec![], // System rotation has no specific permissions
            metadata: AuthMetadata {
                client_ip: None,
                user_agent: None,
                request_id: format!("rotation_{}", chrono::Utc::now().timestamp()),
                timestamp: chrono::Utc::now().timestamp(),
            },
        };

        self.log(&context, "key_rotation", "api_key", success, message)
            .await;
    }

    /// Get total log count (for monitoring)
    ///
    /// Note: With SyncCache trait (no iteration support), this returns an
    /// approximate count by checking individual known user keys.
    /// For accurate counting, use a separate counter or database query.
    pub fn total_log_count(&self) -> usize {
        // Returns the actual count of logs successfully stored via log().
        // (Previously returned a hardcoded 0, hiding real audit activity.)
        self.total_log_count
            .load(std::sync::atomic::Ordering::Relaxed) as usize
    }

    /// Get count of dropped logs (for monitoring)
    pub fn dropped_log_count(&self) -> u64 {
        self.dropped_log_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Implement AuditLogger trait for AppAuditLogger
impl crate::security::traits::AuditLogger for AppAuditLogger {
    fn log(&self, log: AuditLog) {
        // Build an AuthContext from the audit log for the async log method
        let context = AuthContext {
            user_id: log.user_id.clone(),
            permissions: vec![],
            metadata: log.metadata.clone(),
        };
        let result = matches!(log.result, AuditResult::Success);
        let message = match &log.result {
            AuditResult::Success => None,
            AuditResult::Failure { message } => Some(message.clone()),
        };
        let action = log.action.clone();
        let resource = log.resource.clone();

        // Clone all Arc fields from self for use in the spawned task
        let logs = self.logs.clone();
        let max_logs_per_user = self.max_logs_per_user;
        let semaphore = self.semaphore.clone();
        let queue_sender = self.queue_sender.clone();
        let fallback_logs = self.fallback_logs.clone();
        let dropped_log_count = self.dropped_log_count.clone();

        // Reuse the current tokio runtime instead of spawning a new OS thread + runtime per log call.
        // Previous implementation used std::thread::spawn + tokio::runtime::Builder which had
        // unacceptable overhead (thread creation + runtime construction per audit log) and
        // could panic the process on resource exhaustion.
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn(async move {
                    let logger = AppAuditLogger {
                        logs,
                        max_logs_per_user,
                        semaphore,
                        queue_sender,
                        fallback_logs,
                        dropped_log_count,
                        total_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                    };
                    logger
                        .log(&context, action, resource, result, message)
                        .await;
                });
            }
            Err(_) => {
                // No tokio runtime available — emit to stderr as fallback
                // (better than silently dropping the audit log)
                eprintln!(
                    "[AuditLogger] WARNING: no tokio runtime available, audit log dropped for action={}"
                , action);
            }
        }
    }
}

/// Builder for creating AppAuditLogger with custom configuration.
///
/// This builder allows fine-grained control over audit logger settings
/// including log limits, concurrency, and queue size.
///
/// # Examples
///
/// ```ignore
/// use sdforge::security::AppAuditLogger;
///
/// #[tokio::main]
/// async fn main() {
///     let logger = AppAuditLogger::builder()
///         .max_logs_per_user(500)
///         .max_concurrent_ops(50)
///         .queue_size(2000)
///         .build();
///     let _ = logger;
/// }
/// ```
pub struct AppAuditLoggerBuilder {
    /// Maximum number of logs to retain per user
    max_logs_per_user: usize,
    /// Maximum number of concurrent log operations (semaphore permits)
    max_concurrent_ops: usize,
    /// Size of the async log processing queue
    queue_size: usize,
}

impl Default for AppAuditLoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppAuditLoggerBuilder {
    /// Create a new AppAuditLoggerBuilder with default settings.
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
    /// use sdforge::security::AppAuditLoggerBuilder;
    ///
    /// let builder = AppAuditLoggerBuilder::new();
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
    /// use sdforge::security::AppAuditLoggerBuilder;
    ///
    /// let builder = AppAuditLoggerBuilder::new().max_logs_per_user(500);
    /// let _ = builder;
    /// ```
    pub fn max_logs_per_user(mut self, max_logs: usize) -> Self {
        self.max_logs_per_user = max_logs;
        self
    }

    /// Set the maximum number of concurrent log operations.
    ///
    /// This controls the semaphore permit count, limiting how many
    /// log operations can run simultaneously to prevent DoS.
    ///
    /// # Arguments
    ///
    /// * `max_concurrent` - Maximum concurrent log operations (default: 100).
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
    /// use sdforge::security::AppAuditLoggerBuilder;
    ///
    /// let builder = AppAuditLoggerBuilder::new().max_concurrent_ops(50);
    /// let _ = builder;
    /// ```
    pub fn max_concurrent_ops(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent_ops = max_concurrent;
        self
    }

    /// Set the size of the async log processing queue.
    ///
    /// Larger queues allow more buffering during high load but use more memory.
    ///
    /// # Arguments
    ///
    /// * `queue_size` - Size of the async log processing queue (default: 1000).
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
    /// use sdforge::security::AppAuditLoggerBuilder;
    ///
    /// let builder = AppAuditLoggerBuilder::new().queue_size(2000);
    /// let _ = builder;
    /// ```
    pub fn queue_size(mut self, queue_size: usize) -> Self {
        self.queue_size = queue_size;
        self
    }

    /// Build an AppAuditLogger instance using the configured settings.
    ///
    /// This method spawns a background worker task for async log processing.
    /// Ensure tokio runtime is available when calling this method.
    ///
    /// # Returns
    ///
    /// Returns a fully configured AppAuditLogger instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use sdforge::security::AppAuditLoggerBuilder;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let logger = AppAuditLoggerBuilder::new()
    ///         .max_logs_per_user(500)
    ///         .max_concurrent_ops(50)
    ///         .queue_size(2000)
    ///         .build();
    ///     let _ = logger;
    /// }
    /// ```
    pub fn build(self) -> AppAuditLogger {
        let (queue_sender, mut queue_receiver) =
            tokio::sync::mpsc::channel::<AuditLogBatch>(self.queue_size);

        // Spawn background worker for async log processing
        let logs: SharedCache = Arc::new(crate::cache::DashMapCache::new());
        let fallback_logs: SharedCache = Arc::new(crate::cache::DashMapCache::new());
        let logs_clone = logs.clone();
        let fallback_logs_clone = fallback_logs.clone();
        let max_logs_clone = self.max_logs_per_user;
        tokio::spawn(async move {
            // Primary storage is done synchronously by log() — this worker only
            // handles draining the queue and merging fallback logs.
            while let Some(batch) = queue_receiver.recv().await {
                let key = &batch.user_id;
                // Drain the queue: primary log was already stored by log().
                // Just handle any fallback logs for this user.
                if let Some(fallback_data) = fallback_logs_clone.get(key) {
                    let fallback: Vec<AuditLog> = deserialize_audit_logs(&fallback_data);
                    if !fallback.is_empty() {
                        let data = logs_clone.get(key);
                        let mut logs_vec: Vec<AuditLog> = data
                            .as_ref()
                            .map(|d| deserialize_audit_logs(d))
                            .unwrap_or_default();
                        logs_vec.extend(fallback);
                        if logs_vec.len() > max_logs_clone {
                            logs_vec.truncate(max_logs_clone);
                        }
                        logs_clone.set(key, serialize_audit_logs(&logs_vec));
                        fallback_logs_clone.delete(key);
                    }
                }
            }
        });

        AppAuditLogger {
            logs,
            max_logs_per_user: self.max_logs_per_user,
            semaphore: Arc::new(tokio::sync::Semaphore::new(self.max_concurrent_ops)),
            queue_sender: Arc::new(queue_sender),
            fallback_logs,
            dropped_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_error_message() {
        // JWT must have three parts (header.payload.signature) to match the pattern
        let message = "Error with JWT: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let sanitized = sanitize_error_message(message);
        assert!(!sanitized.contains("eyJ"));
        assert!(sanitized.contains("[REDACTED_JWT]"));
    }

    #[test]
    fn test_sanitize_error_message_removes_secrets() {
        // Test secret pattern removal
        let message = "Failed with password=secret123 and token: abc456";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("password=[REDACTED]"));
        assert!(sanitized.contains("token=[REDACTED]"));
    }

    #[test]
    fn test_sanitize_error_message_removes_api_keys() {
        // Test API key removal (20+ characters)
        // Note: API keys are caught by the secret pattern first, then by API key pattern
        let message = "API key: sk_live_REDACTED_TEST_KEY_PLACEHOLDER";
        let sanitized = sanitize_error_message(message);
        // The key should be redacted (either as [REDACTED] or [REDACTED_API_KEY])
        assert!(
            !sanitized.contains("sk_live_"),
            "API key should be redacted, got: {}",
            sanitized
        );
    }

    #[test]
    fn test_sanitize_error_message_removes_credit_cards() {
        // Test credit card number removal
        let message = "Card number: 1234-5678-9012-3456";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED_CREDIT_CARD]"));
    }

    #[test]
    fn test_sanitize_error_message_removes_ssn() {
        // Test SSN removal
        let message = "SSN: 123-45-6789";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED_SSN]"));
    }

    #[test]
    fn test_sanitize_error_message_removes_certificate_paths() {
        // Test certificate path removal
        let message = "Cannot load /etc/ssl/certs/server.pem";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED_PATH]"));
    }

    #[test]
    fn test_sanitize_error_message_truncation() {
        // Test truncation of long messages
        let long_message = "x".repeat(1000);
        let sanitized = sanitize_error_message(&long_message);
        assert!(sanitized.len() <= 520); // 500 + "...[TRUNCATED]"
        assert!(sanitized.ends_with("...[TRUNCATED]"));
    }

    #[test]
    fn test_global_regex_patterns_are_initialized() {
        // Verify global regex patterns are properly initialized
        assert!(JWT_PATTERN.is_match("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxw"));
        assert!(SECRET_PATTERN.is_match("password=secret123"));
        assert!(PATH_PATTERN.is_match("/path/to/cert.pem"));
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("test_user".to_string()),
            permissions: vec![],
            metadata: crate::security::types::AuthMetadata::default(),
        };

        logger
            .log(&context, "test_action", "test_resource", true, None)
            .await;

        let logs = logger.get_logs("test_user");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action(), "test_action");
    }

    #[tokio::test]
    async fn test_get_logs_empty() {
        let logger = AppAuditLogger::new();
        let logs = logger.get_logs("nonexistent_user");
        assert_eq!(logs.len(), 0);
    }

    #[tokio::test]
    async fn test_clear_logs() {
        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("test_user".to_string()),
            permissions: vec![],
            metadata: crate::security::types::AuthMetadata::default(),
        };

        logger
            .log(&context, "test_action", "test_resource", true, None)
            .await;
        assert_eq!(logger.get_logs("test_user").len(), 1);

        logger.clear_logs("test_user");
        assert_eq!(logger.get_logs("test_user").len(), 0);
    }

    // ============================================================================
    // AppAuditLoggerBuilder Tests
    // ============================================================================

    #[test]
    fn test_builder_new_default_values() {
        let builder = AppAuditLoggerBuilder::new();
        assert_eq!(builder.max_logs_per_user, 1000);
        assert_eq!(builder.max_concurrent_ops, 100);
        assert_eq!(builder.queue_size, 1000);
    }

    #[test]
    fn test_builder_default_trait() {
        let builder = AppAuditLoggerBuilder::default();
        assert_eq!(builder.max_logs_per_user, 1000);
        assert_eq!(builder.max_concurrent_ops, 100);
        assert_eq!(builder.queue_size, 1000);
    }

    #[test]
    fn test_builder_max_logs_per_user() {
        let builder = AppAuditLoggerBuilder::new().max_logs_per_user(500);
        assert_eq!(builder.max_logs_per_user, 500);
    }

    #[test]
    fn test_builder_max_concurrent_ops() {
        let builder = AppAuditLoggerBuilder::new().max_concurrent_ops(50);
        assert_eq!(builder.max_concurrent_ops, 50);
    }

    #[test]
    fn test_builder_queue_size() {
        let builder = AppAuditLoggerBuilder::new().queue_size(2000);
        assert_eq!(builder.queue_size, 2000);
    }

    #[test]
    fn test_builder_chaining() {
        let builder = AppAuditLoggerBuilder::new()
            .max_logs_per_user(500)
            .max_concurrent_ops(50)
            .queue_size(2000);
        assert_eq!(builder.max_logs_per_user, 500);
        assert_eq!(builder.max_concurrent_ops, 50);
        assert_eq!(builder.queue_size, 2000);
    }

    #[tokio::test]
    async fn test_builder_build() {
        let logger = AppAuditLoggerBuilder::new()
            .max_logs_per_user(500)
            .max_concurrent_ops(50)
            .queue_size(2000)
            .build();
        assert_eq!(logger.max_logs_per_user, 500);
        assert_eq!(logger.dropped_log_count(), 0);
    }

    // ============================================================================
    // AppAuditLogger::builder() Tests
    // ============================================================================

    #[test]
    fn test_audit_logger_builder_method() {
        let builder = AppAuditLogger::builder();
        assert_eq!(builder.max_logs_per_user, 1000);
        assert_eq!(builder.max_concurrent_ops, 100);
        assert_eq!(builder.queue_size, 1000);
    }

    // ============================================================================
    // AppAuditLogger::log_key_rotation Tests
    // ============================================================================

    #[tokio::test]
    async fn test_log_key_rotation_success() {
        let logger = AppAuditLogger::with_limit(10);
        logger
            .log_key_rotation("key-123", "v1", "v2", true, None)
            .await;

        let logs = logger.get_logs("system");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action(), "key_rotation");
        assert_eq!(logs[0].resource(), "api_key");
    }

    #[tokio::test]
    async fn test_log_key_rotation_failure() {
        let logger = AppAuditLogger::with_limit(10);
        logger
            .log_key_rotation(
                "key-456",
                "v1",
                "v2",
                false,
                Some("Rotation failed".to_string()),
            )
            .await;

        let logs = logger.get_logs("system");
        assert_eq!(logs.len(), 1);
        assert!(matches!(logs[0].result(), AuditResult::Failure { .. }));
    }

    // ============================================================================
    // AppAuditLogger::total_log_count Tests
    // ============================================================================

    #[tokio::test]
    async fn test_total_log_count_returns_zero() {
        let logger = AppAuditLogger::new();
        assert_eq!(logger.total_log_count(), 0);
    }

    #[tokio::test]
    async fn test_total_log_count_after_logging() {
        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("test_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };
        logger
            .log(&context, "action1", "resource1", true, None)
            .await;
        // total_log_count now returns the actual count (previously always 0).
        assert_eq!(logger.total_log_count(), 1);
    }

    // ============================================================================
    // AppAuditLogger::dropped_log_count Tests
    // ============================================================================

    #[tokio::test]
    async fn test_dropped_log_count_initial_value() {
        let logger = AppAuditLogger::new();
        assert_eq!(logger.dropped_log_count(), 0);
    }

    #[tokio::test]
    async fn test_dropped_log_count_after_logging() {
        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("test_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };
        logger.log(&context, "action", "resource", true, None).await;
        // Channel is not full, so dropped count should still be 0
        assert_eq!(logger.dropped_log_count(), 0);
    }

    // ============================================================================
    // AppAuditLogger::log() with Failure + Error Sanitization Tests
    // ============================================================================

    #[tokio::test]
    async fn test_log_failure_with_message() {
        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("test_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        logger
            .log(
                &context,
                "delete_resource",
                "/api/resource/123",
                false,
                Some("Permission denied".to_string()),
            )
            .await;

        let logs = logger.get_logs("test_user");
        assert_eq!(logs.len(), 1);
        match logs[0].result() {
            AuditResult::Failure { message } => {
                assert_eq!(message, "Permission denied");
            }
            _ => panic!("Expected Failure result"),
        }
    }

    #[tokio::test]
    async fn test_log_failure_sanitizes_sensitive_data() {
        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("test_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        // Error message with password should be sanitized
        logger
            .log(
                &context,
                "auth",
                "/api/login",
                false,
                Some("Failed with password=secret123".to_string()),
            )
            .await;

        let logs = logger.get_logs("test_user");
        assert_eq!(logs.len(), 1);
        match logs[0].result() {
            AuditResult::Failure { message } => {
                assert!(
                    !message.contains("secret123"),
                    "Password should be sanitized"
                );
                assert!(
                    message.contains("[REDACTED]"),
                    "Should contain redacted marker"
                );
            }
            _ => panic!("Expected Failure result"),
        }
    }

    #[tokio::test]
    async fn test_log_failure_default_message() {
        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("test_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        logger
            .log(&context, "action", "resource", false, None)
            .await;

        let logs = logger.get_logs("test_user");
        match logs[0].result() {
            AuditResult::Failure { message } => {
                assert_eq!(message, "Unknown error");
            }
            _ => panic!("Expected Failure result"),
        }
    }

    // ============================================================================
    // AuditLogger Trait Implementation Tests
    // ============================================================================

    #[test]
    fn test_audit_logger_trait_log() {
        use crate::security::traits::AuditLogger;

        // Use a runtime to ensure tokio tasks are properly scheduled
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let logger = AppAuditLogger::with_limit(10);
            let log = AuditLog {
                id: "trait-test-id".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
                user_id: Some("trait_user".to_string()),
                action: "trait_action".to_string(),
                resource: "trait_resource".to_string(),
                result: AuditResult::Success,
                metadata: AuthMetadata::default(),
                signature: None,
            };

            // Call the trait method - it spawns a background thread
            AuditLogger::log(&logger, log);

            // Give the background thread time to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let logs = logger.get_logs("trait_user");
            assert_eq!(logs.len(), 1);
            assert_eq!(logs[0].action(), "trait_action");
        });
    }

    #[test]
    fn test_audit_logger_trait_log_failure() {
        use crate::security::traits::AuditLogger;

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let logger = AppAuditLogger::with_limit(10);
            let log = AuditLog {
                id: "trait-fail-id".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
                user_id: Some("trait_fail_user".to_string()),
                action: "trait_fail_action".to_string(),
                resource: "trait_fail_resource".to_string(),
                result: AuditResult::Failure {
                    message: "Trait test failure".to_string(),
                },
                metadata: AuthMetadata::default(),
                signature: None,
            };

            AuditLogger::log(&logger, log);

            // Give the background thread time to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let logs = logger.get_logs("trait_fail_user");
            assert_eq!(logs.len(), 1);
            match logs[0].result() {
                AuditResult::Failure { message } => {
                    assert_eq!(message, "Trait test failure");
                }
                _ => panic!("Expected Failure result"),
            }
        });
    }

    // ============================================================================
    // Multiple Log Entries and Truncation Tests
    // ============================================================================

    #[tokio::test]
    async fn test_multiple_log_entries_per_user() {
        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("multi_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        for i in 0..5 {
            logger
                .log(
                    &context,
                    format!("action_{}", i),
                    format!("resource_{}", i),
                    true,
                    None,
                )
                .await;
        }

        let logs = logger.get_logs("multi_user");
        assert_eq!(logs.len(), 5);
    }

    #[tokio::test]
    async fn test_log_truncation_when_exceeding_limit() {
        let logger = AppAuditLogger::with_limit(3);
        let context = AuthContext {
            user_id: Some("trunc_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        for i in 0..5 {
            logger
                .log(
                    &context,
                    format!("action_{}", i),
                    format!("resource_{}", i),
                    true,
                    None,
                )
                .await;
        }

        let logs = logger.get_logs("trunc_user");
        // Should be truncated to max_logs_per_user (3)
        assert_eq!(logs.len(), 3);
    }

    #[tokio::test]
    async fn test_multiple_users_independent_logs() {
        let logger = AppAuditLogger::with_limit(10);

        let context1 = AuthContext {
            user_id: Some("user_a".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };
        let context2 = AuthContext {
            user_id: Some("user_b".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        logger
            .log(&context1, "action_a", "resource_a", true, None)
            .await;
        logger
            .log(&context2, "action_b", "resource_b", true, None)
            .await;

        let logs_a = logger.get_logs("user_a");
        let logs_b = logger.get_logs("user_b");

        assert_eq!(logs_a.len(), 1);
        assert_eq!(logs_a[0].action(), "action_a");
        assert_eq!(logs_b.len(), 1);
        assert_eq!(logs_b[0].action(), "action_b");
    }

    #[tokio::test]
    async fn test_log_with_anonymous_user() {
        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: None,
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        logger
            .log(&context, "anonymous_action", "resource", true, None)
            .await;

        let logs = logger.get_logs("anonymous");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action(), "anonymous_action");
    }

    #[tokio::test]
    async fn test_default_audit_logger() {
        let logger = AppAuditLogger::default();
        assert_eq!(logger.max_logs_per_user, 1000);
    }

    // ============================================================================
    // Additional Boundary Tests for sanitize_error_message
    // ============================================================================

    #[test]
    fn test_sanitize_jwt_token() {
        let message = "Authentication failed for JWT: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED_JWT]"));
        assert!(!sanitized.contains("eyJ"));
    }

    #[test]
    fn test_sanitize_password_in_error() {
        let message = "Connection failed: password=my_secret_123";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("password=[REDACTED]"));
        assert!(!sanitized.contains("my_secret_123"));
    }

    #[test]
    fn test_sanitize_secret_in_json() {
        let message = r#"Error: password="secret123" token="abc456""#;
        let sanitized = sanitize_error_message(message);
        assert!(
            !sanitized.contains("secret123"),
            "Password should be redacted, got: {}",
            sanitized
        );
        assert!(
            !sanitized.contains("abc456"),
            "Token should be redacted, got: {}",
            sanitized
        );
    }

    #[test]
    fn test_sanitize_certificate_path() {
        let message = "Failed to load certificate from /etc/ssl/certs/server.pem";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED_PATH]"));
        assert!(!sanitized.contains("server.pem"));
    }

    #[test]
    fn test_sanitize_multiple_secrets() {
        let message = "Auth failed with password=secret123 and token: abcdef123456 and api_key: sk_test_12345678901234567890";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("password=[REDACTED]"));
        assert!(sanitized.contains("token=[REDACTED]"));
        assert!(!sanitized.contains("secret123"));
        assert!(!sanitized.contains("abcdef123456"));
    }

    #[test]
    fn test_sanitize_no_secrets() {
        let message = "Database connection timeout after 30 seconds";
        let sanitized = sanitize_error_message(message);
        assert_eq!(sanitized, message);
    }

    #[test]
    fn test_sanitize_empty_message() {
        let message = "";
        let sanitized = sanitize_error_message(message);
        assert_eq!(sanitized, "");
    }

    #[test]
    fn test_sanitize_bearer_token() {
        let message = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0In0.signature validation failed";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED_JWT]"));
        assert!(!sanitized.contains("eyJ"));
    }

    // ============================================================================
    // AuditLog Structure Tests
    // ============================================================================

    #[test]
    fn test_audit_log_creation() {
        let log = AuditLog {
            id: "test-id-123".to_string(),
            timestamp: 1234567890,
            user_id: Some("user_123".to_string()),
            action: "login".to_string(),
            resource: "/auth/login".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };

        assert_eq!(log.id(), "test-id-123");
        assert_eq!(log.timestamp(), 1234567890);
        assert_eq!(log.user_id(), Some("user_123"));
        assert_eq!(log.action(), "login");
        assert_eq!(log.resource(), "/auth/login");
        assert!(matches!(log.result(), AuditResult::Success));
    }

    #[test]
    fn test_audit_log_serialization() {
        let log = AuditLog {
            id: "test-id-456".to_string(),
            timestamp: 1234567890,
            user_id: Some("user_456".to_string()),
            action: "logout".to_string(),
            resource: "/auth/logout".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };

        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"id\":\"test-id-456\""));
        assert!(json.contains("\"action\":\"logout\""));
        assert!(json.contains("\"status\":\"success\""));
    }

    #[test]
    fn test_audit_log_deserialization() {
        let json = r#"[{
            "id": "test-id-789",
            "timestamp": 1234567890,
            "user_id": "user_789",
            "action": "delete",
            "resource": "/api/resource/123",
            "result": {"status": "failure", "message": "Permission denied"},
            "metadata": {
                "client_ip": "192.168.1.1",
                "user_agent": "Mozilla/5.0",
                "request_id": "req-123",
                "timestamp": 1234567890
            },
            "signature": null
        }]"#;

        let logs = deserialize_audit_logs(json.as_bytes());
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].id(), "test-id-789");
        assert_eq!(logs[0].action(), "delete");
        match logs[0].result() {
            AuditResult::Failure { message } => {
                assert_eq!(message, "Permission denied");
            }
            _ => panic!("Expected Failure result"),
        }
    }

    #[test]
    fn test_audit_log_serialization_roundtrip() {
        let original = AuditLog {
            id: "test-id-roundtrip".to_string(),
            timestamp: 1234567890,
            user_id: Some("user_roundtrip".to_string()),
            action: "update".to_string(),
            resource: "/api/resource/456".to_string(),
            result: AuditResult::Failure {
                message: "Validation failed".to_string(),
            },
            metadata: AuthMetadata {
                client_ip: Some("10.0.0.1".to_string()),
                user_agent: Some("TestClient/1.0".to_string()),
                request_id: "req-456".to_string(),
                timestamp: 1234567890,
            },
            signature: Some("test-signature".to_string()),
        };

        let serialized = serialize_audit_logs(std::slice::from_ref(&original));
        let deserialized = deserialize_audit_logs(&serialized);

        assert_eq!(deserialized.len(), 1);
        let log = &deserialized[0];
        assert_eq!(log.id(), original.id());
        assert_eq!(log.timestamp(), original.timestamp());
        assert_eq!(log.user_id(), original.user_id());
        assert_eq!(log.action(), original.action());
        assert_eq!(log.resource(), original.resource());
        assert_eq!(log.metadata().request_id, original.metadata().request_id);
    }

    // ============================================================================
    // Edge Case Tests for sanitize_error_message
    // ============================================================================

    #[test]
    fn test_sanitize_very_long_jwt_token() {
        let long_payload = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".to_string()
            + ".eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ"
            + &"A".repeat(600)
            + ".SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

        let message = format!("Auth error: {}", long_payload);
        let sanitized = sanitize_error_message(&message);

        assert!(sanitized.contains("[REDACTED_JWT]") || sanitized.contains("...[TRUNCATED]"));
        assert!(!sanitized.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }

    #[test]
    fn test_sanitize_token_in_url() {
        let message =
            "Request failed: https://api.example.com/data?token=secret_token_123&user=test";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("token=[REDACTED]"));
        assert!(!sanitized.contains("secret_token_123"));
    }

    // ============================================================================
    // Signing key environment variable tests
    //
    // The log() method checks SDFORGE_AUDIT_SIGNING_KEY env var to optionally
    // sign audit logs. These tests use #[serial] to safely set/unset the env
    // var without interfering with other tests.
    // ============================================================================

    fn make_test_audit_log(user_id: &str, action: &str) -> AuditLog {
        AuditLog {
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: Some(user_id.to_string()),
            action: action.to_string(),
            resource: "test_resource".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_log_with_signing_key_generates_signature() {
        // Set a non-empty signing key — log.generate_signature should be called
        // and the resulting log should have a non-None signature.
        std::env::set_var("SDFORGE_AUDIT_SIGNING_KEY", "test_signing_key_min_32_bytes_long!!!");

        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("signing_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        logger
            .log(&context, "signed_action", "resource", true, None)
            .await;

        let logs = logger.get_logs("signing_user");
        assert_eq!(logs.len(), 1);
        assert!(
            logs[0].signature.is_some(),
            "Audit log should have a signature when signing key is set"
        );

        // Clean up
        std::env::remove_var("SDFORGE_AUDIT_SIGNING_KEY");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_log_with_empty_signing_key_warns() {
        // Set an empty signing key — the empty-key warning branch should be
        // taken and the log should NOT have a signature.
        std::env::set_var("SDFORGE_AUDIT_SIGNING_KEY", "");

        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("empty_key_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        logger
            .log(&context, "unsigned_action", "resource", true, None)
            .await;

        let logs = logger.get_logs("empty_key_user");
        assert_eq!(logs.len(), 1);
        assert!(
            logs[0].signature.is_none(),
            "Audit log should NOT have a signature when signing key is empty"
        );

        // Clean up
        std::env::remove_var("SDFORGE_AUDIT_SIGNING_KEY");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_log_without_signing_key_warns() {
        // Ensure the env var is NOT set — the "not set" warning branch should
        // be taken and the log should NOT have a signature.
        std::env::remove_var("SDFORGE_AUDIT_SIGNING_KEY");

        let logger = AppAuditLogger::with_limit(10);
        let context = AuthContext {
            user_id: Some("no_key_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        logger
            .log(&context, "unsigned_action", "resource", true, None)
            .await;

        let logs = logger.get_logs("no_key_user");
        assert_eq!(logs.len(), 1);
        assert!(
            logs[0].signature.is_none(),
            "Audit log should NOT have a signature when signing key is not set"
        );
    }

    // ============================================================================
    // log() fallback merge tests
    //
    // log() synchronously merges fallback logs into primary storage before
    // sending to the queue. These tests manually populate fallback_logs to
    // exercise that merge path (lines 324-333).
    // ============================================================================

    #[tokio::test]
    async fn test_log_merges_fallback_logs_synchronously() {
        let logger = AppAuditLogger::with_limit(100);

        // Manually populate fallback_logs with a serialized AuditLog
        let fallback_log = AuditLog {
            id: "fallback-log-1".to_string(),
            timestamp: chrono::Utc::now().timestamp() - 100,
            user_id: Some("merge_user".to_string()),
            action: "fallback_action".to_string(),
            resource: "fallback_resource".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };
        logger.fallback_logs.set(
            "merge_user",
            serialize_audit_logs(std::slice::from_ref(&fallback_log)),
        );

        // Verify fallback was stored
        assert!(logger.fallback_logs.get("merge_user").is_some());

        // Now call log() for the same user — this should merge the fallback
        // into primary storage and delete the fallback.
        let context = AuthContext {
            user_id: Some("merge_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };
        logger
            .log(&context, "new_action", "new_resource", true, None)
            .await;

        // The fallback should have been merged and deleted
        assert!(
            logger.fallback_logs.get("merge_user").is_none(),
            "Fallback logs should be deleted after merge"
        );

        // Primary storage should contain both the fallback log and the new log
        let logs = logger.get_logs("merge_user");
        assert_eq!(logs.len(), 2, "Should have 2 logs after merge");

        // Verify both logs are present
        let actions: Vec<&str> = logs.iter().map(|l| l.action()).collect();
        assert!(actions.contains(&"fallback_action"), "Fallback action should be present");
        assert!(actions.contains(&"new_action"), "New action should be present");
    }

    #[tokio::test]
    async fn test_log_fallback_merge_respects_max_logs() {
        // When merging fallback + primary exceeds max_logs_per_user, the
        // merged result should be truncated.
        let logger = AppAuditLogger::with_limit(2); // Very small limit

        // Populate primary with 1 log
        let primary_log = AuditLog {
            id: "primary-1".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: Some("trunc_user".to_string()),
            action: "primary_action".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };
        logger
            .logs
            .set("trunc_user", serialize_audit_logs(&[primary_log]));

        // Populate fallback with 2 logs
        let fallback_logs = vec![
            AuditLog {
                id: "fallback-1".to_string(),
                timestamp: chrono::Utc::now().timestamp() - 200,
                user_id: Some("trunc_user".to_string()),
                action: "fb1".to_string(),
                resource: "res".to_string(),
                result: AuditResult::Success,
                metadata: AuthMetadata::default(),
                signature: None,
            },
            AuditLog {
                id: "fallback-2".to_string(),
                timestamp: chrono::Utc::now().timestamp() - 100,
                user_id: Some("trunc_user".to_string()),
                action: "fb2".to_string(),
                resource: "res".to_string(),
                result: AuditResult::Success,
                metadata: AuthMetadata::default(),
                signature: None,
            },
        ];
        logger
            .fallback_logs
            .set("trunc_user", serialize_audit_logs(&fallback_logs));

        // Call log() which merges fallback into primary
        let context = AuthContext {
            user_id: Some("trunc_user".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };
        logger.log(&context, "new", "res", true, None).await;

        // After merge + truncate, should have at most max_logs_per_user (2)
        let logs = logger.get_logs("trunc_user");
        assert!(
            logs.len() <= 2,
            "Logs should be truncated to max_logs_per_user (2), got {}",
            logs.len()
        );
    }

    // ============================================================================
    // get_logs dedup tests
    //
    // get_logs merges primary and fallback, deduplicating by log ID.
    // These tests populate both stores with overlapping IDs (lines 386-389).
    // ============================================================================

    #[tokio::test]
    async fn test_get_logs_deduplicates_by_id() {
        let logger = AppAuditLogger::with_limit(100);

        // Create a log that appears in BOTH primary and fallback (same ID)
        let shared_log = AuditLog {
            id: "shared-id-1".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: Some("dedup_user".to_string()),
            action: "shared_action".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };

        // A log that only appears in fallback
        let fallback_only = AuditLog {
            id: "fallback-only-1".to_string(),
            timestamp: chrono::Utc::now().timestamp() - 50,
            user_id: Some("dedup_user".to_string()),
            action: "fallback_only_action".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };

        // Populate primary with the shared log
        logger
            .logs
            .set("dedup_user", serialize_audit_logs(std::slice::from_ref(&shared_log)));

        // Populate fallback with both the shared log and the fallback-only log
        logger.fallback_logs.set(
            "dedup_user",
            serialize_audit_logs(&[shared_log, fallback_only]),
        );

        // get_logs should deduplicate: shared log appears once, fallback-only appears once
        let logs = logger.get_logs("dedup_user");
        assert_eq!(
            logs.len(),
            2,
            "Should have 2 logs after dedup (shared + fallback-only), got {}",
            logs.len()
        );

        // Verify the shared log appears only once
        let shared_count = logs.iter().filter(|l| l.id() == "shared-id-1").count();
        assert_eq!(shared_count, 1, "Shared log should appear exactly once");

        // Verify the fallback-only log appears
        let fallback_count = logs.iter().filter(|l| l.id() == "fallback-only-1").count();
        assert_eq!(fallback_count, 1, "Fallback-only log should appear once");
    }

    #[tokio::test]
    async fn test_get_logs_with_only_fallback() {
        let logger = AppAuditLogger::with_limit(100);

        // Populate only fallback (no primary)
        let fallback_log = AuditLog {
            id: "fb-only-2".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: Some("fb_user".to_string()),
            action: "fb_action".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };
        logger
            .fallback_logs
            .set("fb_user", serialize_audit_logs(&[fallback_log]));

        let logs = logger.get_logs("fb_user");
        assert_eq!(logs.len(), 1, "Should return fallback log when primary is empty");
        assert_eq!(logs[0].id(), "fb-only-2");
    }

    // ============================================================================
    // Worker fallback merge tests
    //
    // The spawned worker task in with_limit() and build() drains the queue and
    // merges any remaining fallback logs. Since log() already merges fallback
    // synchronously, the worker's fallback merge is only triggered when
    // fallback data exists at the time the worker processes a batch. These
    // tests manually inject fallback data and send a batch to trigger the
    // worker's merge path.
    // ============================================================================

    #[tokio::test]
    async fn test_worker_merges_fallback_from_queue() {
        let logger = AppAuditLogger::with_limit(100);

        // Populate fallback_logs for "worker_user"
        let fallback_log = AuditLog {
            id: "worker-fb-1".to_string(),
            timestamp: chrono::Utc::now().timestamp() - 100,
            user_id: Some("worker_user".to_string()),
            action: "worker_fb_action".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };
        logger
            .fallback_logs
            .set("worker_user", serialize_audit_logs(&[fallback_log]));

        // Populate primary with an existing log
        let primary_log = AuditLog {
            id: "worker-primary-1".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: Some("worker_user".to_string()),
            action: "worker_primary_action".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };
        logger
            .logs
            .set("worker_user", serialize_audit_logs(&[primary_log]));

        // Send a batch to the queue to trigger the worker's fallback merge
        let batch = AuditLogBatch {
            user_id: "worker_user".to_string(),
            log: make_test_audit_log("worker_user", "queued_action"),
        };
        let _ = logger.queue_sender.send(batch).await;

        // Wait for the worker to process the batch
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // The worker should have merged the fallback into primary and deleted it
        assert!(
            logger.fallback_logs.get("worker_user").is_none(),
            "Worker should have deleted fallback after merging"
        );

        // Primary should contain the merged logs (primary + fallback)
        let logs = logger.get_logs("worker_user");
        let actions: Vec<&str> = logs.iter().map(|l| l.action()).collect();
        assert!(
            actions.contains(&"worker_fb_action"),
            "Merged logs should contain the fallback action"
        );
    }

    #[tokio::test]
    async fn test_worker_no_fallback_does_nothing() {
        // When there's no fallback data, the worker should just drain the
        // queue without modifying primary storage.
        let logger = AppAuditLogger::with_limit(100);

        // Populate primary only (no fallback)
        let primary_log = AuditLog {
            id: "no-fb-primary".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: Some("no_fb_user".to_string()),
            action: "primary_action".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };
        logger
            .logs
            .set("no_fb_user", serialize_audit_logs(&[primary_log]));

        // Send a batch
        let batch = AuditLogBatch {
            user_id: "no_fb_user".to_string(),
            log: make_test_audit_log("no_fb_user", "queued"),
        };
        let _ = logger.queue_sender.send(batch).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Primary should be unchanged (worker found no fallback to merge)
        let logs = logger.get_logs("no_fb_user");
        assert_eq!(logs.len(), 1, "Primary should have 1 log (no merge occurred)");
    }

    // ============================================================================
    // Builder build() worker tests
    //
    // AppAuditLoggerBuilder::build() also spawns a worker with the same
    // fallback merge logic. This test exercises the builder's worker path.
    // ============================================================================

    #[tokio::test]
    async fn test_builder_build_worker_merges_fallback() {
        let logger = AppAuditLogger::builder()
            .max_logs_per_user(100)
            .max_concurrent_ops(10)
            .queue_size(100)
            .build();

        // Populate fallback for the builder-created logger
        let fallback_log = AuditLog {
            id: "builder-fb-1".to_string(),
            timestamp: chrono::Utc::now().timestamp() - 100,
            user_id: Some("builder_user".to_string()),
            action: "builder_fb_action".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };
        logger
            .fallback_logs
            .set("builder_user", serialize_audit_logs(&[fallback_log]));

        // Send a batch to trigger the worker
        let batch = AuditLogBatch {
            user_id: "builder_user".to_string(),
            log: make_test_audit_log("builder_user", "builder_queued"),
        };
        let _ = logger.queue_sender.send(batch).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Worker should have merged and deleted fallback
        assert!(
            logger.fallback_logs.get("builder_user").is_none(),
            "Builder worker should have deleted fallback after merging"
        );

        let logs = logger.get_logs("builder_user");
        let actions: Vec<&str> = logs.iter().map(|l| l.action()).collect();
        assert!(
            actions.contains(&"builder_fb_action"),
            "Merged logs should contain the fallback action"
        );
    }

    // ============================================================================
    // AuditLogger trait no-runtime path test
    //
    // The AuditLogger trait impl's log() method checks for a tokio runtime. If
    // none is available, it falls back to printing to stderr (lines 498-503).
    // This test calls the trait method from a plain std::thread (no runtime).
    // ============================================================================

    #[tokio::test]
    async fn test_audit_logger_trait_no_runtime_path() {
        use crate::security::traits::AuditLogger as AuditLoggerTrait;

        // with_limit() calls tokio::spawn(), so we need a runtime.
        // The spawned thread below has NO runtime, testing the no-runtime fallback path.
        let logger = AppAuditLogger::with_limit(10);
        let log = make_test_audit_log("no_rt_user", "no_runtime_action");

        // Spawn a plain OS thread with NO tokio runtime. The trait impl's
        // log() should detect the missing runtime and print to stderr instead
        // of panicking.
        let logger_clone = logger.clone();
        let handle = std::thread::spawn(move || {
            // This calls the AuditLogger trait method, not the async log()
            AuditLoggerTrait::log(&logger_clone, log);
        });

        // The thread should complete without panicking
        handle.join().expect("Thread should not panic");

        // No logs should be stored (they were dropped to stderr)
        let logs = logger.get_logs("no_rt_user");
        assert_eq!(
            logs.len(),
            0,
            "No logs should be stored when no runtime is available"
        );
    }

    #[tokio::test]
    async fn test_audit_logger_trait_with_runtime_spawns_task() {
        use crate::security::traits::AuditLogger as AuditLoggerTrait;

        let logger = AppAuditLogger::with_limit(10);
        let log = make_test_audit_log("rt_user", "runtime_action");

        // Call the trait method from within a tokio runtime — it should
        // spawn a task that calls the async log() method.
        AuditLoggerTrait::log(&logger, log);

        // Wait for the spawned task to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // The log should have been stored
        let logs = logger.get_logs("rt_user");
        assert_eq!(logs.len(), 1, "Log should be stored when runtime is available");
        assert_eq!(logs[0].action(), "runtime_action");
    }
}
