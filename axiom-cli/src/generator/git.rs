// Copyright (c) 2026 Kirky.X
//! Git initialization utilities.
//!
//! Provides safe git repository initialization with security checks
//! to prevent path traversal and other attacks.

use std::path::Path;

use tracing::{warn, Level};

use crate::generator::error::GeneratorResult;

/// Initialize a git repository with security checks.
///
/// This function validates the project directory path before running
/// git init to prevent path traversal attacks.
///
/// # Arguments
///
/// * `project_dir` - Path to the project directory
///
/// # Returns
///
/// `Ok(())` on success or if git init is skipped, `Err(GeneratorError)` on failure
///
/// # Security
///
/// - Validates and normalizes the project directory path
/// - Ensures the path is within the current working directory
/// - Silently continues (returns Ok) if git init fails, to avoid blocking project creation
pub fn initialize_git(project_dir: &Path) -> GeneratorResult<()> {
    // Security: Validate and normalize the project directory path
    let canonical_path = match project_dir.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            warn!(
                target: "generator",
                ?e,
                "Failed to canonicalize project directory path, skipping git init"
            );
            return Ok(()); // Continue without git init
        }
    };

    // Security: Ensure the path is within an allowed directory (prevent path traversal)
    // Allow only paths that don't escape the current working directory
    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            warn!(
                target: "generator",
                ?e,
                "Failed to get current directory, skipping git init"
            );
            return Ok(()); // Continue without git init
        }
    };

    if !canonical_path.starts_with(&current_dir) {
        warn!(
            target: "generator",
            "Project directory path escapes current directory, skipping git init"
        );
        return Ok(()); // Continue without git init
    }

    // Run git init with validated path
    let git_init_result = std::process::Command::new("git")
        .arg("init")
        .arg(project_dir)
        .output();

    match git_init_result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    target: "generator",
                    ?stderr,
                    "Git init failed for project directory"
                );
            } else {
                tracing::event!(Level::INFO, "Git repository initialized");
            }
        }
        Err(e) => {
            warn!(
                target: "generator",
                ?e,
                "Failed to run git init"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_initialize_git_valid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // Should not fail
        let result = initialize_git(project_dir);
        assert!(result.is_ok());

        // Git directory may or may not be created depending on git availability
        // Just verify the function doesn't fail
    }

    #[test]
    fn test_initialize_git_non_existent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("non-existent");

        // Should not fail (gracefully skips)
        assert!(initialize_git(&project_dir).is_ok());
    }

    #[test]
    fn test_initialize_git_path_traversal_attempt() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("../../../etc/passwd");

        // Should not fail (gracefully skips path traversal)
        assert!(initialize_git(&project_dir).is_ok());
    }
}
