// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `InklogAuditSink` — bridges audit events to inklog structured output.
//!
//! When the `inklog` feature is enabled, this sink forwards every audit
//! log entry to inklog's `LoggerManager` as a structured event. This
//! provides durable, restart-safe audit logging (assuming inklog is
//! configured with a persistent output backend).
//!
//! Requires both `security` and `inklog` features.

use super::AuditSink;
use crate::security::{AuditLog, serialize_audit_logs};
use std::future::Future;
use std::pin::Pin;

/// Audit sink that bridges to inklog's structured logging pipeline.
///
/// Each `write()` call emits the audit log as a JSON-formatted `log::info!`
/// event. inklog's `LoggerManager` (if installed as the `log` backend)
/// captures these events and routes them to the configured output
/// (file, stdout, etc.).
///
/// # Persistence
///
/// Persistence depends entirely on inklog's configuration. If inklog
/// writes to a file, audit logs survive restarts. If inklog only writes
/// to stdout, they do not.
pub struct InklogAuditSink;

impl InklogAuditSink {
    /// Create a new `InklogAuditSink`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for InklogAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditSink for InklogAuditSink {
    fn write<'a>(
        &'a self,
        user_id: &'a str,
        audit_log: AuditLog,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            // Serialize the audit log as JSON and emit via the `log` facade.
            // inklog (when installed as the log backend) captures this.
            let serialized = serialize_audit_logs(&[audit_log]);
            let json_str = String::from_utf8_lossy(&serialized);
            ::log::info!(
                target: "sdforge::audit",
                "audit_log user_id={} event={}",
                user_id,
                json_str,
            );
            Ok(())
        })
    }

    fn read(&self, _user_id: &str) -> Vec<AuditLog> {
        // inklog is a write-only sink from sdforge's perspective —
        // reading back requires querying inklog's storage directly,
        // which is outside the AuditSink contract.
        Vec::new()
    }

    fn clear(&self, _user_id: &str) {
        // No-op: inklog manages its own retention/rotation.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AuditLog, AuditResult, AuthMetadata};

    #[test]
    fn test_inklog_sink_read_returns_empty() {
        let sink = InklogAuditSink::new();
        let logs = sink.read("any_user");
        assert!(logs.is_empty());
    }

    #[test]
    fn test_inklog_sink_clear_is_noop() {
        let sink = InklogAuditSink::new();
        // Should not panic
        sink.clear("any_user");
    }

    #[tokio::test]
    async fn test_inklog_sink_write_succeeds() {
        let sink = InklogAuditSink::new();
        let log = AuditLog {
            id: "test-id".to_string(),
            timestamp: 1234567890,
            user_id: Some("user1".to_string()),
            action: "test_action".to_string(),
            resource: "test_resource".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata {
                client_ip: None,
                user_agent: None,
                request_id: "req-1".to_string(),
                timestamp: 1234567890,
            },
            signature: None,
        };
        let result = sink.write("user1", log).await;
        assert!(result.is_ok());
    }
}
