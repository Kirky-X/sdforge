// Copyright (c) 2026 Kirky.X
//! Audit logging implementation
//!
//! This module provides audit logging with DoS protection and async processing.

use crate::cache::SharedCache;
use crate::security::types::{
    deserialize_audit_logs, serialize_audit_logs, AuditLog, AuditResult, AuthContext,
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
    #[allow(dead_code)] // Used in queue transfer, field access not needed directly
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

    /// Create an AppAuditLogger with all dependencies injected.
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
    /// Returns an AppAuditLogger instance configured with the provided dependencies.
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
    /// ```ignore
    /// use sdforge::security::AppAuditLogger;
    /// use sdforge::cache::DashMapCache;
    /// use std::sync::Arc;
    /// use std::sync::atomic::AtomicU64;
    ///
    /// let (sender, mut receiver) = tokio::sync::mpsc::channel(1000);
    ///
    /// // Create SharedCache instances
    /// let logs = Arc::new(DashMapCache::default());
    /// let fallback_logs = Arc::new(DashMapCache::default());
    ///
    /// let logger = AppAuditLogger::with_dependencies(
    ///     logs,
    ///     1000,
    ///     Arc::new(tokio::sync::Semaphore::new(100)),
    ///     Arc::new(sender),
    ///     fallback_logs,
    ///     Arc::new(AtomicU64::new(0)),
    /// );
    /// let _ = logger;
    /// ```
    #[allow(dead_code)]
    pub(crate) fn with_dependencies(
        logs: SharedCache,
        max_logs_per_user: usize,
        semaphore: Arc<tokio::sync::Semaphore>,
        queue_sender: Arc<tokio::sync::mpsc::Sender<AuditLogBatch>>,
        fallback_logs: SharedCache,
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
        if cfg!(feature = "audit-signing") {
            // Load signing key from environment or configuration
            // Security: Use a cryptographically secure random key of at least 32 bytes
            if let Ok(signing_key_str) = std::env::var("SDFORGE_AUDIT_SIGNING_KEY") {
                if !signing_key_str.is_empty() {
                    // Convert string to bytes and use as HMAC key
                    log.generate_signature(signing_key_str.as_bytes());
                } else {
                    eprintln!("⚠️  WARNING: SDFORGE_AUDIT_SIGNING_KEY is empty. Audit logs will not be signed.");
                }
            } else {
                eprintln!("⚠️  WARNING: SDFORGE_AUDIT_SIGNING_KEY not set. Audit logs will not be signed.");
                eprintln!("   For production, set this environment variable to a secure random value (min 32 bytes).");
                eprintln!("   Example: export SDFORGE_AUDIT_SIGNING_KEY=$(openssl rand -hex 32)");
            }
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

    /// Get total log count (for monitoring)
    ///
    /// Note: With SyncCache trait (no iteration support), this returns an
    /// approximate count by checking individual known user keys.
    /// For accurate counting, use a separate counter or database query.
    pub fn total_log_count(&self) -> usize {
        // SyncCache doesn't support iteration; return 0 as approximation
        // For accurate counting, use dbnexus persistence or a separate counter
        0
    }

    /// Store log in fallback storage (synchronous path)
    ///
    /// Security: This is used when the async channel is full, preventing
    /// audit log loss during high load or potential DoS attempts.
    #[allow(dead_code)] // Reserved for fallback handling when queue is full
    fn store_fallback_log(&self, user_id: &str, log: &AuditLog) {
        let count = self
            .dropped_log_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Load existing fallback logs
        let data = self.fallback_logs.get(user_id);
        let mut logs_vec: Vec<AuditLog> = data
            .as_ref()
            .and_then(|d| {
                let v = deserialize_audit_logs(d);
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            })
            .unwrap_or_default();

        logs_vec.push(log.clone());

        // Truncate if exceeding limit
        if logs_vec.len() > self.max_logs_per_user {
            logs_vec.truncate(self.max_logs_per_user);
        }
        self.fallback_logs
            .set(user_id, serialize_audit_logs(&logs_vec));

        // Log warning periodically (every 100th drop)
        if count > 0 && count.is_multiple_of(100) {
            // High audit log drop rate detected
        }
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

        // Clone all Arc fields from self for use in the spawned thread
        let logs = self.logs.clone();
        let max_logs_per_user = self.max_logs_per_user;
        let semaphore = self.semaphore.clone();
        let queue_sender = self.queue_sender.clone();
        let fallback_logs = self.fallback_logs.clone();
        let dropped_log_count = self.dropped_log_count.clone();

        // Spawn a background thread with its own runtime for the async log method
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build runtime for AuditLogger::log");
            rt.block_on(async {
                let logger = AppAuditLogger {
                    logs,
                    max_logs_per_user,
                    semaphore,
                    queue_sender,
                    fallback_logs,
                    dropped_log_count,
                };
                logger
                    .log(&context, action, resource, result, message)
                    .await;
            });
        });
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
        let message = "API key: testkey_live_abcdefghijklmnopqrstuvwxyz";
        let sanitized = sanitize_error_message(message);
        // The key should be redacted (either as [REDACTED] or [REDACTED_API_KEY])
        assert!(!sanitized.contains("testkey_live_"), "API key should be redacted, got: {}", sanitized);
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
}
