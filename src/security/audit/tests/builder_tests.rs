// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `AppAuditLoggerBuilder` and `AppAuditLogger::builder()`.

use super::super::*;
use super::make_test_audit_log;

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

#[tokio::test]
async fn test_default_audit_logger() {
    let logger = AppAuditLogger::default();
    assert_eq!(logger.max_logs_per_user, 1000);
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
