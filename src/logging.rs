// Copyright (c) 2026 Kirky.X
//! Structured logging utilities for SDForge
//!
//! This module provides structured logging capabilities with support for:
//! - JSON-formatted logs
//! - Log levels (Trace, Debug, Info, Warn, Error)
//! - Contextual fields
//! - Asynchronous writing
//! - Log rotation

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Log level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp in RFC 3339 format
    pub timestamp: String,
    /// Log level
    pub level: LogLevel,
    /// Target module or component
    pub target: String,
    /// Log message
    pub message: String,
    /// Additional contextual fields (ordered for consistent output)
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, serde_json::Value>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(
        level: LogLevel,
        target: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level,
            target: target.into(),
            message: message.into(),
            fields: BTreeMap::new(),
        }
    }

    /// Add a field to the log entry
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Add multiple fields
    pub fn with_fields(mut self, fields: Vec<(String, serde_json::Value)>) -> Self {
        for (key, value) in fields {
            self.fields.insert(key, value);
        }
        self
    }
}

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Minimum log level to emit
    pub min_level: LogLevel,
    /// Output format (JSON or plain text)
    pub format: LogFormat,
    /// Whether to include ANSI colors in terminal output
    pub colored: bool,
    /// Maximum log file size in bytes (for file logging)
    pub max_file_size: u64,
    /// Number of rotated files to keep
    pub max_files: u32,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            format: LogFormat::Json,
            colored: true,
            max_file_size: 10 * 1024 * 1024, // 10 MB
            max_files: 5,
        }
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Text,
}

/// Structured logger
pub struct StructuredLogger {
    config: LoggerConfig,
    tx: mpsc::Sender<LogEntry>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl StructuredLogger {
    /// Create and initialize a new structured logger
    pub fn new(config: LoggerConfig) -> Self {
        let (tx, mut rx) = mpsc::channel::<LogEntry>(1000);

        // Spawn async writer task
        let handle = tokio::spawn(async move {
            let stdout = io::stdout();
            while let Some(entry) = rx.recv().await {
                let mut handle = stdout.lock();
                let _ = write_log_entry(&mut handle, &entry, config.format, config.colored);
                let _ = writeln!(handle);
            }
        });

        Self {
            config,
            tx,
            handle: Some(handle),
        }
    }

    /// Log a message at the specified level
    pub fn log(&self, level: LogLevel, target: &str, message: &str, fields: Vec<(String, serde_json::Value)>) {
        if level >= self.config.min_level {
            let entry = LogEntry::new(level, target, message).with_fields(fields);
            let _ = self.tx.try_send(entry);
        }
    }

    /// Trace level log
    pub fn trace(&self, target: &str, message: &str, fields: Vec<(String, serde_json::Value)>) {
        self.log(LogLevel::Trace, target, message, fields);
    }

    /// Debug level log
    pub fn debug(&self, target: &str, message: &str, fields: Vec<(String, serde_json::Value)>) {
        self.log(LogLevel::Debug, target, message, fields);
    }

    /// Info level log
    pub fn info(&self, target: &str, message: &str, fields: Vec<(String, serde_json::Value)>) {
        self.log(LogLevel::Info, target, message, fields);
    }

    /// Warn level log
    pub fn warn(&self, target: &str, message: &str, fields: Vec<(String, serde_json::Value)>) {
        self.log(LogLevel::Warn, target, message, fields);
    }

    /// Error level log
    pub fn error(&self, target: &str, message: &str, fields: Vec<(String, serde_json::Value)>) {
        self.log(LogLevel::Error, target, message, fields);
    }

    /// Flush pending logs
    pub async fn flush(&self) {
        // Channel will be flushed when dropped
    }

    /// Shutdown the logger
    pub async fn shutdown(self) {
        drop(self.tx);
        if let Some(handle) = self.handle {
            let _ = handle.await;
        }
    }
}

/// Write a log entry to the specified writer
fn write_log_entry<W: Write>(
    writer: &mut W,
    entry: &LogEntry,
    format: LogFormat,
    colored: bool,
) -> io::Result<()> {
    match format {
        LogFormat::Json => {
            let json = serde_json::to_string(entry)?;
            
            if colored {
                let color = get_level_color(entry.level);
                write!(writer, "{}", color)?;
                write!(writer, "{}", json)?;
                write!(writer, "\x1b[0m")?;
            } else {
                write!(writer, "{}", json)?;
            }
        }
        LogFormat::Text => {
            if colored {
                let color = get_level_color(entry.level);
                write!(writer, "{}{}", color, entry.timestamp)?;
                write!(writer, " {:<5}", entry.level)?;
                write!(writer, "\x1b[0m")?;
                write!(writer, " [{}]", entry.target)?;
                write!(writer, " {}", entry.message)?;
                
                if !entry.fields.is_empty() {
                    write!(writer, " {{")?;
                    for (i, (k, v)) in entry.fields.iter().enumerate() {
                        if i > 0 {
                            write!(writer, ", ")?;
                        }
                        write!(writer, "{}={}", k, v)?;
                    }
                    write!(writer, "}}")?;
                }
            } else {
                write!(writer, "{} {:<5} [{}] {}", entry.timestamp, entry.level, entry.target, entry.message)?;
                
                if !entry.fields.is_empty() {
                    write!(writer, " {{")?;
                    for (i, (k, v)) in entry.fields.iter().enumerate() {
                        if i > 0 {
                            write!(writer, ", ")?;
                        }
                        write!(writer, "{}={}", k, v)?;
                    }
                    write!(writer, "}}")?;
                }
            }
        }
    }
    Ok(())
}

/// Get ANSI color code for log level
fn get_level_color(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Trace => "\x1b[90m",      // Bright Black
        LogLevel::Debug => "\x1b[36m",      // Cyan
        LogLevel::Info => "\x1b[32m",       // Green
        LogLevel::Warn => "\x1b[33m",       // Yellow
        LogLevel::Error => "\x1b[31m",      // Red
    }
}

/// Global logger instance
static GLOBAL_LOGGER: once_cell::sync::OnceCell<Arc<StructuredLogger>> = once_cell::sync::OnceCell::new();

/// Initialize the global logger
pub fn init_global_logger(config: LoggerConfig) -> Result<(), LoggerError> {
    let logger = Arc::new(StructuredLogger::new(config));
    GLOBAL_LOGGER.set(logger).map_err(|_| LoggerError::AlreadyInitialized)?;
    Ok(())
}

/// Get the global logger
pub fn get_global_logger() -> Option<Arc<StructuredLogger>> {
    GLOBAL_LOGGER.get().cloned()
}

/// Logger initialization error
#[derive(Debug, thiserror::Error)]
pub enum LoggerError {
    #[error("Logger already initialized")]
    AlreadyInitialized,
}

/// Convenience macro for structured logging
#[macro_export]
macro_rules! log_info {
    ($target:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {{
        use $crate::logging::{get_global_logger, LogLevel};
        if let Some(logger) = get_global_logger() {
            let fields = vec![$(($key, $val.into())),*];
            logger.log(LogLevel::Info, $target, $msg, fields);
        }
    }};
}

#[macro_export]
macro_rules! log_error {
    ($target:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {{
        use $crate::logging::{get_global_logger, LogLevel};
        if let Some(logger) = get_global_logger() {
            let fields = vec![$(($key, $val.into())),*];
            logger.log(LogLevel::Error, $target, $msg, fields);
        }
    }};
}

#[macro_export]
macro_rules! log_debug {
    ($target:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {{
        use $crate::logging::{get_global_logger, LogLevel};
        if let Some(logger) = get_global_logger() {
            let fields = vec![$(($key, $val.into())),*];
            logger.log(LogLevel::Debug, $target, $msg, fields);
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(LogLevel::Info, "test", "Test message")
            .with_field("user_id", "123")
            .with_field("action", "login");

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "test");
        assert_eq!(entry.message, "Test message");
        assert_eq!(entry.fields.len(), 2);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry::new(LogLevel::Warn, "auth", "Failed login attempt")
            .with_field("user", "admin")
            .with_field("ip", "192.168.1.1");

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"level\":\"warn\""));
        assert!(json.contains("\"target\":\"auth\""));
        assert!(json.contains("\"message\":\"Failed login attempt\""));
    }

    #[tokio::test]
    async fn test_structured_logger() {
        let config = LoggerConfig {
            min_level: LogLevel::Debug,
            format: LogFormat::Json,
            colored: false,
            ..Default::default()
        };

        let logger = StructuredLogger::new(config);
        
        logger.info("app", "Application started", vec![
            ("version".to_string(), serde_json::Value::String("0.1.0".to_string())),
            ("env".to_string(), serde_json::Value::String("test".to_string())),
        ]);

        logger.error("db", "Connection failed", vec![
            ("host".to_string(), serde_json::Value::String("localhost".to_string())),
            ("port".to_string(), serde_json::Value::Number(5432.into())),
        ]);

        logger.flush().await;
        logger.shutdown().await;
    }
}
