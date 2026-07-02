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
    /// This uses the `file!()`, `line!()`, and `std::any::type_name` to
    /// automatically capture the caller's location.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sdforge::core::error::ErrorContext;
    /// let context = ErrorContext::current();
    /// ```
    pub fn current() -> Self {
        Self {
            file: Some(file!().to_string()),
            line: Some(line!()),
            function: Some(std::any::type_name::<()>().to_string()),
            extra: HashMap::new(),
        }
    }

    /// Add extra context information
    ///
    /// # Example
    ///
    /// ```rust
    /// use sdforge::core::error::ErrorContext;
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
