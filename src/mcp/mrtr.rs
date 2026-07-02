// Copyright (c) 2026 Kirky.X
//! Multi Round-Trip Requests (MRTR) support for MCP 2026-07-28.
//!
//! MRTR allows a tool to pause execution and request additional input from
//! the client before completing. This is useful for tools that need user
//! confirmation, additional parameters, or human-in-the-loop interaction.
//!
//! # Flow
//!
//! 1. Client calls a tool
//! 2. Tool returns `InputRequiredResult` with a session ID
//! 3. Client provides input via `resume_session()`
//! 4. Tool completes and returns the final `CallToolResult`
//!
//! Sessions have a 300-second timeout. After timeout, the session is
//! cancelled and the original tool call returns an error.

use rmcp::model::{CallToolResult, Content, ErrorData};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Default session timeout: 300 seconds (per MCP 2026-07-28 spec).
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(300);

/// Result returned when a tool needs additional input.
#[derive(Debug, Clone)]
pub struct InputRequiredResult {
    /// Unique session identifier
    pub session_id: String,
    /// Message describing what input is needed
    pub message: String,
    /// Optional schema for the expected input
    pub input_schema: Option<Value>,
}

impl InputRequiredResult {
    /// Create a new input-required result.
    pub fn new(session_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            message: message.into(),
            input_schema: None,
        }
    }

    /// Create with an input schema.
    pub fn with_schema(mut self, schema: Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Convert to a `CallToolResult` for the client.
    pub fn to_call_tool_result(&self) -> CallToolResult {
        let structured = serde_json::json!({
            "inputRequired": {
                "sessionId": self.session_id,
                "message": self.message,
                "inputSchema": self.input_schema,
            }
        });
        CallToolResult {
            content: vec![Content::text(self.message.clone())],
            structured_content: Some(structured),
            is_error: Some(false),
            meta: None,
        }
    }
}

/// State of an MRTR session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// Waiting for client input
    Pending,
    /// Client provided input, tool can resume
    Resumed,
    /// Session timed out
    Timeout,
    /// Session cancelled
    Cancelled,
    /// Session completed
    Completed,
}

/// An MRTR session tracking a paused tool execution.
#[derive(Debug)]
pub struct MrtrSession {
    /// Unique session ID
    pub id: String,
    /// Tool name that initiated the session
    pub tool_name: String,
    /// Current state
    pub state: SessionState,
    /// When the session was created
    pub created_at: Instant,
    /// Session timeout duration
    pub timeout: Duration,
    /// The input provided by the client (when resumed)
    pub resume_input: Option<Value>,
}

impl MrtrSession {
    /// Create a new pending session.
    pub fn new(id: impl Into<String>, tool_name: impl Into<String>) -> Self {
        Self::with_timeout(id, tool_name, DEFAULT_SESSION_TIMEOUT)
    }

    /// Create a new session with a custom timeout.
    pub fn with_timeout(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            id: id.into(),
            tool_name: tool_name.into(),
            state: SessionState::Pending,
            created_at: Instant::now(),
            timeout,
            resume_input: None,
        }
    }

    /// Check if the session has timed out.
    pub fn is_timed_out(&self) -> bool {
        self.state == SessionState::Pending && self.created_at.elapsed() >= self.timeout
    }

    /// Check if the session is still pending.
    pub fn is_pending(&self) -> bool {
        self.state == SessionState::Pending && !self.is_timed_out()
    }

    /// Resume the session with client input.
    pub fn resume(&mut self, input: Value) {
        self.state = SessionState::Resumed;
        self.resume_input = Some(input);
    }

    /// Mark the session as completed.
    pub fn complete(&mut self) {
        self.state = SessionState::Completed;
    }

    /// Cancel the session.
    pub fn cancel(&mut self) {
        self.state = SessionState::Cancelled;
    }

    /// Mark the session as timed out.
    pub fn mark_timeout(&mut self) {
        self.state = SessionState::Timeout;
    }

    /// Get the resume input (if resumed).
    pub fn resume_input(&self) -> Option<&Value> {
        self.resume_input.as_ref()
    }

    /// Get the elapsed time since creation.
    pub fn elapsed(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get the remaining time before timeout.
    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.elapsed())
    }
}

/// Manager for MRTR sessions.
///
/// This struct tracks all active MRTR sessions and provides methods to
/// create, resume, and clean up sessions. It is thread-safe via `Mutex`.
#[derive(Debug, Clone)]
pub struct MrtrSessionManager {
    sessions: Arc<Mutex<HashMap<String, MrtrSession>>>,
}

impl Default for MrtrSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MrtrSessionManager {
    /// Create a new empty session manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new MRTR session for a tool.
    ///
    /// Returns `ErrorData::invalid_params` if a session with the given
    /// `session_id` already exists. Previously this method silently
    /// overwrote the existing session via `HashMap::insert`, which combined
    /// with the nanosecond-timestamp ID generator could cause session data
    /// loss when two sessions were created within the same nanosecond on
    /// high-frequency clocks.
    pub fn create_session(
        &self,
        session_id: impl Into<String>,
        tool_name: impl Into<String>,
    ) -> Result<InputRequiredResult, ErrorData> {
        let session_id = session_id.into();
        let tool_name_str = tool_name.into();

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| ErrorData::internal_error("session manager lock poisoned", None))?;

        if sessions.contains_key(&session_id) {
            return Err(ErrorData::invalid_params(
                format!(
                    "MRTR session '{}' already exists; choose a unique session id",
                    session_id
                ),
                None,
            ));
        }

        let session = MrtrSession::new(session_id.clone(), tool_name_str.clone());
        let result = InputRequiredResult::new(
            session_id.clone(),
            format!("Tool '{}' requires additional input", tool_name_str),
        );

        sessions.insert(session_id, session);

        Ok(result)
    }

    /// Resume a session with client input.
    pub fn resume_session(&self, session_id: &str, input: Value) -> Result<(), ErrorData> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| ErrorData::internal_error("session manager lock poisoned", None))?;

        let session = sessions.get_mut(session_id).ok_or_else(|| {
            ErrorData::invalid_params(format!("session not found: {}", session_id), None)
        })?;

        if session.is_timed_out() {
            session.mark_timeout();
            return Err(ErrorData::invalid_params(
                format!("session {} has timed out", session_id),
                None,
            ));
        }

        if session.state != SessionState::Pending {
            return Err(ErrorData::invalid_params(
                format!(
                    "session {} is not pending (state: {:?})",
                    session_id, session.state
                ),
                None,
            ));
        }

        session.resume(input);
        Ok(())
    }

    /// Get the resume input for a session.
    pub fn get_resume_input(&self, session_id: &str) -> Result<Option<Value>, ErrorData> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| ErrorData::internal_error("session manager lock poisoned", None))?;

        let session = sessions.get(session_id).ok_or_else(|| {
            ErrorData::invalid_params(format!("session not found: {}", session_id), None)
        })?;

        Ok(session.resume_input().cloned())
    }

    /// Mark a session as completed.
    pub fn complete_session(&self, session_id: &str) -> Result<(), ErrorData> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| ErrorData::internal_error("session manager lock poisoned", None))?;

        let session = sessions.get_mut(session_id).ok_or_else(|| {
            ErrorData::invalid_params(format!("session not found: {}", session_id), None)
        })?;

        session.complete();
        Ok(())
    }

    /// Cancel a session.
    pub fn cancel_session(&self, session_id: &str) -> Result<(), ErrorData> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| ErrorData::internal_error("session manager lock poisoned", None))?;

        let session = sessions.get_mut(session_id).ok_or_else(|| {
            ErrorData::invalid_params(format!("session not found: {}", session_id), None)
        })?;

        session.cancel();
        Ok(())
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: &str) -> Option<SessionState> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(session_id).map(|s| s.state.clone()))
    }

    /// Clean up expired (timed-out) sessions.
    pub fn cleanup_expired(&self) -> usize {
        let mut sessions = match self.sessions.lock() {
            Ok(s) => s,
            Err(_) => return 0,
        };
        let before = sessions.len();
        sessions.retain(|_, session| {
            if session.is_timed_out() && session.state == SessionState::Pending {
                session.mark_timeout();
                false // remove timed-out pending sessions
            } else {
                true
            }
        });
        before - sessions.len()
    }

    /// Get the number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.lock().map(|s| s.len()).unwrap_or(0)
    }

    /// Clear all sessions.
    pub fn clear(&self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.clear();
        }
    }
}

/// Generate a unique session ID.
pub fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("mrtr-{}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_required_result_new() {
        let result = InputRequiredResult::new("session-1", "Need more input");
        assert_eq!(result.session_id, "session-1");
        assert_eq!(result.message, "Need more input");
        assert!(result.input_schema.is_none());
    }

    #[test]
    fn test_input_required_result_with_schema() {
        let schema = serde_json::json!({"type": "object"});
        let result =
            InputRequiredResult::new("session-1", "Need input").with_schema(schema.clone());
        assert_eq!(result.input_schema, Some(schema));
    }

    #[test]
    fn test_input_required_result_to_call_tool_result() {
        let result = InputRequiredResult::new("session-1", "Need input");
        let call_result = result.to_call_tool_result();
        assert!(!call_result.content.is_empty());
        assert!(call_result.structured_content.is_some());
        let structured = call_result.structured_content.unwrap();
        assert!(structured["inputRequired"]["sessionId"].is_string());
    }

    #[test]
    fn test_session_state_equality() {
        assert_eq!(SessionState::Pending, SessionState::Pending);
        assert_ne!(SessionState::Pending, SessionState::Resumed);
    }

    #[test]
    fn test_mrtr_session_new() {
        let session = MrtrSession::new("s1", "my_tool");
        assert_eq!(session.id, "s1");
        assert_eq!(session.tool_name, "my_tool");
        assert_eq!(session.state, SessionState::Pending);
        assert!(session.resume_input.is_none());
    }

    #[test]
    fn test_mrtr_session_with_timeout() {
        let session = MrtrSession::with_timeout("s1", "tool", Duration::from_millis(100));
        assert_eq!(session.timeout, Duration::from_millis(100));
    }

    #[test]
    fn test_mrtr_session_is_pending() {
        let session = MrtrSession::new("s1", "tool");
        assert!(session.is_pending());
    }

    #[test]
    fn test_mrtr_session_is_timed_out() {
        let session = MrtrSession::with_timeout("s1", "tool", Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(2));
        assert!(session.is_timed_out());
        assert!(!session.is_pending());
    }

    #[test]
    fn test_mrtr_session_resume() {
        let mut session = MrtrSession::new("s1", "tool");
        session.resume(serde_json::json!({"input": "value"}));
        assert_eq!(session.state, SessionState::Resumed);
        assert!(session.resume_input.is_some());
        assert_eq!(session.resume_input().unwrap()["input"], "value");
    }

    #[test]
    fn test_mrtr_session_complete() {
        let mut session = MrtrSession::new("s1", "tool");
        session.complete();
        assert_eq!(session.state, SessionState::Completed);
    }

    #[test]
    fn test_mrtr_session_cancel() {
        let mut session = MrtrSession::new("s1", "tool");
        session.cancel();
        assert_eq!(session.state, SessionState::Cancelled);
    }

    #[test]
    fn test_mrtr_session_mark_timeout() {
        let mut session = MrtrSession::new("s1", "tool");
        session.mark_timeout();
        assert_eq!(session.state, SessionState::Timeout);
    }

    #[test]
    fn test_mrtr_session_remaining() {
        let session = MrtrSession::with_timeout("s1", "tool", Duration::from_secs(300));
        let remaining = session.remaining();
        assert!(remaining <= Duration::from_secs(300));
        assert!(remaining > Duration::from_secs(290));
    }

    #[test]
    fn test_mrtr_session_elapsed() {
        let session = MrtrSession::new("s1", "tool");
        let elapsed = session.elapsed();
        assert!(elapsed < Duration::from_millis(100));
    }

    #[test]
    fn test_session_manager_default() {
        let manager = MrtrSessionManager::default();
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_session_manager_create_session() {
        let manager = MrtrSessionManager::new();
        let result = manager.create_session("s1", "my_tool").unwrap();
        assert_eq!(result.session_id, "s1");
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_session_manager_resume_session() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        manager
            .resume_session("s1", serde_json::json!({"input": "value"}))
            .unwrap();
        assert_eq!(manager.get_session("s1"), Some(SessionState::Resumed));
    }

    #[test]
    fn test_session_manager_resume_nonexistent() {
        let manager = MrtrSessionManager::new();
        let result = manager.resume_session("nonexistent", serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_session_manager_get_resume_input() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        manager
            .resume_session("s1", serde_json::json!({"data": 42}))
            .unwrap();
        let input = manager.get_resume_input("s1").unwrap();
        assert_eq!(input, Some(serde_json::json!({"data": 42})));
    }

    #[test]
    fn test_session_manager_complete_session() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        manager.resume_session("s1", serde_json::json!({})).unwrap();
        manager.complete_session("s1").unwrap();
        assert_eq!(manager.get_session("s1"), Some(SessionState::Completed));
    }

    #[test]
    fn test_session_manager_cancel_session() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        manager.cancel_session("s1").unwrap();
        assert_eq!(manager.get_session("s1"), Some(SessionState::Cancelled));
    }

    #[test]
    fn test_session_manager_resume_already_resumed() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        manager.resume_session("s1", serde_json::json!({})).unwrap();
        let result = manager.resume_session("s1", serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_session_manager_cleanup_expired() {
        let manager = MrtrSessionManager::new();
        // Create a session with very short timeout
        {
            let session = MrtrSession::with_timeout("s1", "tool", Duration::from_millis(1));
            manager
                .sessions
                .lock()
                .unwrap()
                .insert("s1".to_string(), session);
        }
        std::thread::sleep(Duration::from_millis(5));
        let cleaned = manager.cleanup_expired();
        assert_eq!(cleaned, 1);
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_session_manager_cleanup_no_expired() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        let cleaned = manager.cleanup_expired();
        assert_eq!(cleaned, 0);
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_session_manager_clear() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        manager.create_session("s2", "tool").unwrap();
        manager.clear();
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_session_manager_get_session_nonexistent() {
        let manager = MrtrSessionManager::new();
        assert_eq!(manager.get_session("nonexistent"), None);
    }

    #[test]
    fn test_session_manager_clone() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        let cloned = manager.clone();
        assert_eq!(cloned.session_count(), 1);
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();
        assert!(id1.starts_with("mrtr-"));
        assert!(id2.starts_with("mrtr-"));
        // IDs should be unique (nanosecond precision)
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_default_session_timeout() {
        assert_eq!(DEFAULT_SESSION_TIMEOUT, Duration::from_secs(300));
    }

    #[test]
    fn test_session_manager_multiple_sessions() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool1").unwrap();
        manager.create_session("s2", "tool2").unwrap();
        manager.create_session("s3", "tool3").unwrap();
        assert_eq!(manager.session_count(), 3);
    }

    #[test]
    fn test_session_manager_resume_after_complete() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        manager.resume_session("s1", serde_json::json!({})).unwrap();
        manager.complete_session("s1").unwrap();
        // Should not be able to resume a completed session
        let result = manager.resume_session("s1", serde_json::json!({}));
        assert!(result.is_err());
    }

    /// Verify that resuming a session that has timed out returns a timeout
    /// error and marks the session state as Timeout.
    /// This covers the `is_timed_out()` branch in `resume_session`.
    #[test]
    fn test_session_manager_resume_timed_out_session() {
        let manager = MrtrSessionManager::new();
        // Insert a session with a very short timeout directly so we can
        // trigger the timeout path without waiting 300 seconds.
        {
            let session = MrtrSession::with_timeout("s1", "tool", Duration::from_millis(1));
            manager
                .sessions
                .lock()
                .unwrap()
                .insert("s1".to_string(), session);
        }
        // Wait for the session to exceed its timeout.
        std::thread::sleep(Duration::from_millis(5));

        let result = manager.resume_session("s1", serde_json::json!({"input": "value"}));
        assert!(result.is_err(), "Resuming a timed-out session should error");
        let err = result.unwrap_err();
        // The error message should mention the session id and the timeout.
        let err_str = err.to_string();
        assert!(
            err_str.contains("timed out"),
            "Error should mention timeout, got: {}",
            err_str
        );
        assert!(
            err_str.contains("s1"),
            "Error should mention session id, got: {}",
            err_str
        );
        // The session state should have been marked as Timeout.
        assert_eq!(
            manager.get_session("s1"),
            Some(SessionState::Timeout),
            "Session should be marked as Timeout after resume on expired session"
        );
    }

    /// Verify that `is_pending` returns false for a Cancelled session even if
    /// it hasn't timed out (covers the `state == Pending` check in is_pending).
    #[test]
    fn test_mrtr_session_is_pending_false_for_cancelled() {
        let mut session = MrtrSession::new("s1", "tool");
        session.cancel();
        assert!(
            !session.is_pending(),
            "Cancelled session should not be pending"
        );
    }

    /// Verify that `is_pending` returns false for a Completed session.
    #[test]
    fn test_mrtr_session_is_pending_false_for_completed() {
        let mut session = MrtrSession::new("s1", "tool");
        session.complete();
        assert!(
            !session.is_pending(),
            "Completed session should not be pending"
        );
    }

    /// Verify that `is_pending` returns false for a Resumed session.
    #[test]
    fn test_mrtr_session_is_pending_false_for_resumed() {
        let mut session = MrtrSession::new("s1", "tool");
        session.resume(serde_json::json!({}));
        assert!(
            !session.is_pending(),
            "Resumed session should not be pending"
        );
    }

    /// Verify that `is_pending` returns false for an already-timed-out session
    /// (state set to Timeout via mark_timeout).
    #[test]
    fn test_mrtr_session_is_pending_false_for_timeout_state() {
        let mut session = MrtrSession::new("s1", "tool");
        session.mark_timeout();
        assert!(
            !session.is_pending(),
            "Timeout session should not be pending"
        );
    }

    /// Verify that `is_timed_out` returns false when the session is not in
    /// Pending state (e.g. Resumed), even if the timeout has elapsed.
    #[test]
    fn test_mrtr_session_is_timed_out_false_for_non_pending() {
        let mut session = MrtrSession::with_timeout("s1", "tool", Duration::from_millis(1));
        session.resume(serde_json::json!({}));
        std::thread::sleep(Duration::from_millis(2));
        assert!(
            !session.is_timed_out(),
            "Non-pending session should not report timed out"
        );
    }

    /// Verify that `remaining()` returns zero (saturating) when the session
    /// has exceeded its timeout.
    #[test]
    fn test_mrtr_session_remaining_zero_after_timeout() {
        let session = MrtrSession::with_timeout("s1", "tool", Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(
            session.remaining(),
            Duration::ZERO,
            "Remaining should be zero after timeout"
        );
    }

    /// Verify that `cleanup_expired` does not remove sessions that have timed
    /// out but are no longer Pending (e.g. already Resumed).
    #[test]
    fn test_session_manager_cleanup_keeps_non_pending_expired() {
        let manager = MrtrSessionManager::new();
        {
            let mut session = MrtrSession::with_timeout("s1", "tool", Duration::from_millis(1));
            session.resume(serde_json::json!({}));
            manager
                .sessions
                .lock()
                .unwrap()
                .insert("s1".to_string(), session);
        }
        std::thread::sleep(Duration::from_millis(5));
        let cleaned = manager.cleanup_expired();
        assert_eq!(cleaned, 0, "Should not clean non-pending session");
        assert_eq!(manager.session_count(), 1);
    }

    /// Verify that `resume_input()` returns None for a session that was never
    /// resumed (i.e. no input was provided).
    #[test]
    fn test_mrtr_session_resume_input_none_when_not_resumed() {
        let session = MrtrSession::new("s1", "tool");
        assert!(session.resume_input().is_none());
    }

    /// Verify that `get_resume_input` returns None for a session that exists
    /// but has not been resumed.
    #[test]
    fn test_session_manager_get_resume_input_none() {
        let manager = MrtrSessionManager::new();
        manager.create_session("s1", "tool").unwrap();
        let input = manager.get_resume_input("s1").unwrap();
        assert_eq!(input, None);
    }

    /// Verify that `get_resume_input` errors for a non-existent session.
    #[test]
    fn test_session_manager_get_resume_input_nonexistent() {
        let manager = MrtrSessionManager::new();
        let result = manager.get_resume_input("nonexistent");
        assert!(result.is_err());
    }

    /// Verify that `complete_session` errors for a non-existent session.
    #[test]
    fn test_session_manager_complete_nonexistent() {
        let manager = MrtrSessionManager::new();
        let result = manager.complete_session("nonexistent");
        assert!(result.is_err());
    }

    /// Verify that `cancel_session` errors for a non-existent session.
    #[test]
    fn test_session_manager_cancel_nonexistent() {
        let manager = MrtrSessionManager::new();
        let result = manager.cancel_session("nonexistent");
        assert!(result.is_err());
    }

    /// Verify that `InputRequiredResult::to_call_tool_result` sets is_error to
    /// Some(false) and includes the message as text content.
    #[test]
    fn test_input_required_result_to_call_tool_result_fields() {
        let result = InputRequiredResult::new("session-1", "Need input");
        let call_result = result.to_call_tool_result();
        assert_eq!(call_result.is_error, Some(false));
        assert_eq!(call_result.content.len(), 1);
        // meta should be None
        assert!(call_result.meta.is_none());
    }

    /// Verify that `InputRequiredResult::with_schema` chains correctly.
    #[test]
    fn test_input_required_result_with_schema_chain() {
        let result = InputRequiredResult::new("session-1", "Need input")
            .with_schema(serde_json::json!({"type": "string"}))
            .with_schema(serde_json::json!({"type": "object"}));
        // The second with_schema should override the first.
        assert_eq!(
            result.input_schema,
            Some(serde_json::json!({"type": "object"}))
        );
    }
}
