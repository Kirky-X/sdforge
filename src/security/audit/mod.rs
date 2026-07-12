// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Audit logging implementation
//!
//! This module provides audit logging with DoS protection and async processing.

use crate::cache::SharedCache;
use crate::security::AuditLog;
use std::sync::Arc;

mod audit_impl;
pub(crate) use audit_impl::{sanitize_error_message, JWT_PATTERN, SECRET_PATTERN, PATH_PATTERN};

/// Batch of audit logs for async processing.
///
/// Internal struct used to pass user ID and log entry through the async channel.
pub(crate) struct AuditLogBatch {
    user_id: String,
    #[allow(dead_code)] // Field used in queue transfer; direct read access not needed
    log: AuditLog,
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

#[cfg(test)]
mod tests;
