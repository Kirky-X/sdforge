// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Audit logging implementation
//!
//! This module provides audit logging with DoS protection and async processing.

use crate::cache::SharedCache;
use crate::security::AuditLog;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

mod audit_impl;
// HIGH 修复（#298）：sanitize_error_message 现为无条件 pub(crate)——
// ApiError::internal_* 构造器据此强制脱敏，不再依赖调用方自觉。
pub(crate) use audit_impl::sanitize_error_message;
#[cfg(test)]
pub(crate) use audit_impl::{JWT_PATTERN, PATH_PATTERN, SECRET_PATTERN};

// =============================================================================
// AuditSink trait — abstract audit log storage backend
// =============================================================================

/// Abstract audit log storage backend.
///
/// `AuditSink` decouples *where* audit logs are stored from the
/// `SdForgeAuditLogger`'s DoS-protection machinery (semaphore, queue,
/// merge-lock). The default implementation is an in-memory ring buffer
/// (`SdForgeAuditLogger`'s existing `SharedCache`-backed storage). When the
/// `inklog` feature is enabled, an [`InklogAuditSink`] bridges audit
/// events to inklog's structured output pipeline.
///
/// Uses `Pin<Box<dyn Future>>` instead of `async-trait` to avoid pulling
/// in an extra dependency — mirrors the `RateLimiter` trait pattern.
pub trait AuditSink: Send + Sync {
    /// Write an audit log entry for the given user.
    fn write<'a>(
        &'a self,
        user_id: &'a str,
        log: AuditLog,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

    /// Read all audit logs for the given user.
    fn read(&self, user_id: &str) -> Vec<AuditLog>;

    /// Clear all audit logs for the given user.
    fn clear(&self, user_id: &str);
}

// inklog bridge — available when both `security` and `inklog` features are on.
#[cfg(feature = "inklog")]
mod inklog_sink;
#[cfg(feature = "inklog")]
pub use inklog_sink::InklogAuditSink;

/// Batch of audit logs for async processing.
///
/// Internal struct used to pass user ID and log entry through the async channel.
pub(crate) struct AuditLogBatch {
    user_id: String,
    #[expect(
        dead_code,
        reason = "批次经 fire-and-forget 队列传输，消费端当前不读取该负载"
    )]
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
pub struct SdForgeAuditLogger {
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
    /// 串行化对同一用户日志列表的读-改-写（log() 主写入、fallback 合并、
    /// worker 合并共用）。HIGH 修复：此前 get→push→set 无跨操作互斥，
    /// 同用户并发写互相覆盖导致审计日志静默丢失。
    merge_lock: Arc<std::sync::Mutex<()>>,
}

/// Builder for creating SdForgeAuditLogger with custom configuration.
///
/// This builder allows fine-grained control over audit logger settings
/// including log limits, concurrency, and queue size.
///
/// # Examples
///
/// ```ignore
/// use sdforge::security::SdForgeAuditLogger;
///
/// #[tokio::main]
/// async fn main() {
///     let logger = SdForgeAuditLogger::builder()
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
