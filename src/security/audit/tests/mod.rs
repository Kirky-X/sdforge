// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Audit module test suites.
//!
//! Tests are organized by responsibility:
//! - `audit_logger_tests`: core logger functionality (`sanitize_error_message`,
//!   `AppAuditLogger` logging/get/clear, key rotation, total/dropped counts,
//!   failure sanitization, trait impl, multi-user, truncation, serialization,
//!   signing key, fallback merge, dedup, worker drain, channel/semaphore edges)
//! - `builder_tests`: `AppAuditLoggerBuilder` and `AppAuditLogger::builder()`
//!   construction, defaults, chaining, `build()`, and builder worker fallback merge

mod audit_logger_tests;
mod builder_tests;

use super::*;

// ============================================================================
// Shared test helpers — accessible by all sub-modules via `super::`
// ============================================================================

/// Build an `AuditLog` with sensible defaults for tests.
pub(super) fn make_test_audit_log(user_id: &str, action: &str) -> AuditLog {
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
