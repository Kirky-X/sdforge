// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error category classification for error handling and reporting
///
/// This enum categorizes errors to enable proper error handling strategies,
/// monitoring, and user-facing error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Client errors (4xx) - request was malformed or invalid
    ClientError,
    /// Authentication and authorization errors (401/403)
    AuthError,
    /// Server errors (5xx) - internal processing failure
    ServerError,
    /// Rate limiting errors (429)
    RateLimitError,
    /// Validation errors - input failed business rule validation
    ValidationError,
}

/// Error context information
///
/// Captures contextual information about where and when an error occurred.
/// This information is invaluable for debugging and monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Source file where the error occurred
    pub file: Option<String>,
    /// Line number in the source file
    pub line: Option<u32>,
    /// Function name where the error occurred
    pub function: Option<String>,
    /// Additional contextual information
    pub extra: HashMap<String, String>,
}

impl ErrorContext {
    /// Create a new empty ErrorContext
    pub fn new() -> Self {
        Self {
            file: None,
            line: None,
            function: None,
            extra: HashMap::new(),
        }
    }

    /// Capture the current calling context
    ///
    /// Uses `#[track_caller]` to capture the **caller's** file and line.
    ///
    /// HIGH 修复：此前 `file!()/line!()` 展开于本文件，永远指向
    /// context.rs 自身；`type_name::<()>()` 恒为 `"()"`——"调用者上下文"
    /// 整体失效且 function 字段是常量垃圾值。调用者位置无法经
    /// `track_caller` 获得函数名，`function` 现为 `None`（诚实的缺失
    /// 优于恒错的 `"()"`）。
    ///
    /// # Example
    ///
    /// ```rust
    /// use sdforge::error::ErrorContext;
    /// let context = ErrorContext::current();
    /// ```
    #[track_caller]
    pub fn current() -> Self {
        let loc = std::panic::Location::caller();
        Self {
            file: Some(loc.file().to_string()),
            line: Some(loc.line()),
            function: None,
            extra: HashMap::new(),
        }
    }

    /// Add extra context information
    ///
    /// # Example
    ///
    /// ```rust
    /// use sdforge::error::ErrorContext;
    /// let context = ErrorContext::current()
    ///     .with_extra("user_id".to_string(), "12345".to_string())
    ///     .with_extra("action".to_string(), "delete_user".to_string());
    /// ```
    pub fn with_extra(mut self, key: String, value: String) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}
