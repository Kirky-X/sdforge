//! Error types for the generator module.
//!
//! This module defines specific error types for code generation operations,
//! providing better error messages and easier error handling.

use thiserror::Error;

/// Maximum project name length to prevent DoS attacks
pub const MAX_PROJECT_NAME_LENGTH: usize = 64;

/// Errors that can occur during code generation.
#[derive(Debug, Error)]
pub enum GeneratorError {
    /// Project name validation failed
    #[error("Project name validation failed: {0}")]
    InvalidProjectName(String),

    /// Project name is empty
    #[error("Project name cannot be empty")]
    EmptyProjectName,

    /// Project name exceeds maximum length
    #[error(
        "Project name exceeds maximum length of {} characters",
        MAX_PROJECT_NAME_LENGTH
    )]
    ProjectNameTooLong,

    /// Project name contains invalid characters
    #[error("Project name contains invalid characters. Use only alphanumeric, underscore, hyphen, and dot")]
    InvalidProjectNameCharacters,

    /// Project name contains invalid pattern (e.g., "..", leading/trailing dot)
    #[error("Project name contains invalid pattern")]
    InvalidProjectNamePattern,

    /// Project name contains path separators
    #[error("Project name cannot contain path separators")]
    ProjectNamePathSeparator,

    /// Template directory not found
    #[error("Template directory '{}' not found", .0.display())]
    TemplateNotFound(std::path::PathBuf),

    /// Template file not found
    #[error("Template '{}' not found", .0)]
    TemplateFileNotFound(String),

    /// Output path escapes current directory (path traversal attempt)
    #[error("Output path escapes current directory: {}", .0.display())]
    PathTraversal(std::path::PathBuf),

    /// Output path is invalid
    #[error("Invalid output path: {}", .0)]
    InvalidOutputPath(String),

    /// Failed to get current directory
    #[error("Failed to get current directory: {}", .0)]
    CurrentDirError(String),

    /// Project directory already exists
    #[error("Directory '{}' already exists", .0)]
    DirectoryExists(String),

    /// Failed to create project directory
    #[error("Failed to create project directory: {}", .0.display())]
    CreateDirectoryError(std::path::PathBuf),

    /// Template contains dangerous content
    #[error("Template '{}' contains dangerous pattern: {}", .0, .1)]
    DangerousTemplate(String, String),

    /// Failed to render template
    #[error("Failed to render template '{}': {}", .0, .1)]
    TemplateRenderError(String, String),

    /// Failed to write output file
    #[error("Failed to write output file '{}': {}", .0.display(), .1)]
    WriteFileError(std::path::PathBuf, String),

    /// IO error occurred
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for generator operations.
pub type GeneratorResult<T> = Result<T, GeneratorError>;
