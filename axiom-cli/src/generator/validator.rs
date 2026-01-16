//! Validation utilities for code generation.
//!
//! Provides validation functions for project names, template content,
//! and output paths to ensure security and correctness.

use std::path::{Path, PathBuf};

use crate::generator::error::{GeneratorError, GeneratorResult};

/// Validate project name to prevent path traversal and other attacks.
///
/// # Arguments
///
/// * `name` - The project name to validate
///
/// # Returns
///
/// `Ok(())` if valid, `Err(GeneratorError)` with details otherwise
///
/// # Security
///
/// This function prevents:
/// - Empty project names
/// - Names exceeding maximum length (DoS protection)
/// - Invalid characters (path traversal attempts)
/// - Special patterns like ".."
/// - Path separators
pub(crate) fn validate_project_name(name: &str) -> GeneratorResult<()> {
    // Length validation
    if name.is_empty() {
        return Err(GeneratorError::EmptyProjectName);
    }
    if name.len() > crate::generator::error::MAX_PROJECT_NAME_LENGTH {
        return Err(GeneratorError::ProjectNameTooLong);
    }

    // Character whitelist validation (alphanumeric, underscore, hyphen, dot)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(GeneratorError::InvalidProjectNameCharacters);
    }

    // Prevent special patterns
    if name.contains("..") || name.starts_with('.') || name.ends_with('.') {
        return Err(GeneratorError::InvalidProjectNamePattern);
    }

    // Prevent path separators
    if name.contains('/') || name.contains('\\') {
        return Err(GeneratorError::ProjectNamePathSeparator);
    }

    Ok(())
}

/// Validate that a path is within the current directory.
///
/// This prevents path traversal attacks by ensuring the canonical path
/// starts with the current working directory.
///
/// # Arguments
///
/// * `path` - The path to validate
///
/// # Returns
///
/// `Ok(PathBuf)` with the canonical path if valid, `Err(GeneratorError)` otherwise
pub(crate) fn validate_output_path(path: &Path) -> GeneratorResult<PathBuf> {
    let canonical_path = std::fs::canonicalize(path)
        .map_err(|e| GeneratorError::InvalidOutputPath(e.to_string()))?;

    // Ensure output path is within current directory
    let current_dir = std::env::current_dir()
        .map_err(|e| GeneratorError::CurrentDirError(e.to_string()))?
        .canonicalize()
        .map_err(|e| GeneratorError::CurrentDirError(e.to_string()))?;

    if !canonical_path.starts_with(&current_dir) {
        return Err(GeneratorError::PathTraversal(canonical_path));
    }

    Ok(canonical_path)
}

/// Dangerous patterns to detect in template content.
///
/// These patterns could be used for template injection attacks.
const DANGEROUS_PATTERNS: &[&str] = &[
    "include::", // File inclusion
    "import::",  // Module import
    "crate::",   // Crate access
    "super::",   // Parent module access
    "self::",    // Current module access
    "std::",     // Standard library (beyond safe subset)
    "env!",      // Environment variable access
    "file!",     // File path leak
    "line!",     // Line number leak
    "column!",   // Column number leak
];

/// Validate template content for potentially dangerous operations.
///
/// This prevents template injection attacks by detecting dangerous
/// patterns that could execute arbitrary code or access sensitive information.
///
/// # Arguments
///
/// * `template_name` - Name of the template being validated
/// * `content` - The template content to check
///
/// # Returns
///
/// `Ok(())` if content is safe, `Err(GeneratorError)` if dangerous patterns found
pub(crate) fn validate_template_content(template_name: &str, content: &str) -> GeneratorResult<()> {
    for pattern in DANGEROUS_PATTERNS {
        if content.contains(pattern) {
            return Err(GeneratorError::DangerousTemplate(
                template_name.to_string(),
                pattern.to_string(),
            ));
        }
    }

    Ok(())
}

/// Check if a template directory exists.
///
/// # Arguments
///
/// * `template_dir` - Path to the template directory
///
/// # Returns
///
/// `Ok(())` if exists, `Err(GeneratorError)` otherwise
pub(crate) fn validate_template_directory(template_dir: &Path) -> GeneratorResult<()> {
    if !template_dir.exists() {
        return Err(GeneratorError::TemplateNotFound(template_dir.to_path_buf()));
    }
    Ok(())
}

/// Check if a project directory already exists.
///
/// # Arguments
///
/// * `project_dir` - Path to the potential project directory
///
/// # Returns
///
/// `Ok(())` if does not exist, `Err(GeneratorError)` if exists
pub(crate) fn validate_project_directory_does_not_exist(project_dir: &Path) -> GeneratorResult<()> {
    if project_dir.exists() {
        return Err(GeneratorError::DirectoryExists(
            project_dir.to_string_lossy().to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_project_name_valid() {
        assert!(validate_project_name("my-project").is_ok());
        assert!(validate_project_name("my_project_v2").is_ok());
        assert!(validate_project_name("project.123").is_ok());
        assert!(validate_project_name("a").is_ok());
    }

    #[test]
    fn test_validate_project_name_empty() {
        assert!(validate_project_name("").is_err());
    }

    #[test]
    fn test_validate_project_name_too_long() {
        let long_name = "a".repeat(65);
        assert!(validate_project_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_project_name_invalid_chars() {
        assert!(validate_project_name("project/name").is_err());
        assert!(validate_project_name("project\\name").is_err());
        assert!(validate_project_name("project name").is_err());
        assert!(validate_project_name("project@name").is_err());
    }

    #[test]
    fn test_validate_project_name_invalid_patterns() {
        assert!(validate_project_name("..").is_err());
        assert!(validate_project_name("../malicious").is_err());
        assert!(validate_project_name(".hidden").is_err());
        assert!(validate_project_name("hidden.").is_err());
    }

    #[test]
    fn test_validate_template_content_safe() {
        let safe_content = r#"
fn main() {
    println!("Hello, {}!", name);
}
"#;
        assert!(validate_template_content("test.rs", safe_content).is_ok());
    }

    #[test]
    fn test_validate_template_content_dangerous() {
        let dangerous_content = r#"
fn main() {
    let path = std::env::current_dir();
}
"#;
        assert!(validate_template_content("test.rs", dangerous_content).is_err());
    }

    #[test]
    fn test_validate_template_content_dangerous_patterns() {
        let patterns = [
            "include::",
            "import::",
            "crate::",
            "super::",
            "self::",
            "std::",
            "env!",
            "file!",
            "line!",
            "column!",
        ];

        for pattern in patterns {
            let content = format!("let x = {};", pattern);
            assert!(
                validate_template_content("test.rs", &content).is_err(),
                "Pattern '{}' should be detected as dangerous",
                pattern
            );
        }
    }
}
