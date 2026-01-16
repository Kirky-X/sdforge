// Copyright (c) 2026 Kirky.X
//! Project generation utilities.
//!
//! Provides the main entry point for creating new Axiom projects
//! with template rendering and git initialization.

use crate::generator::error::{GeneratorError, GeneratorResult};
use crate::generator::git::initialize_git;
use crate::generator::template::{render_templates, TemplateContext};
use crate::generator::validator::{
    validate_output_path, validate_project_directory_does_not_exist, validate_project_name,
    validate_template_directory,
};
use std::fs;

/// Determine features string based on protocol.
///
/// # Arguments
///
/// * `protocol` - Communication protocol (http, mcp, both)
/// * `additional_features` - Additional features to include
///
/// # Returns
///
/// The features string for Cargo.toml
pub(crate) fn determine_features(protocol: &str, additional_features: &str) -> String {
    let base = match protocol {
        "http" | "mcp" | "both" => protocol.to_string(),
        _ => "http".to_string(),
    };

    if additional_features.is_empty() {
        base
    } else {
        format!("{},{}", base, additional_features)
    }
}

/// Generate a new Axiom project from a template.
///
/// This is the main function for creating new projects. It validates
/// all inputs, renders templates, and initializes a git repository.
///
/// # Arguments
///
/// * `project_name` - Name of the project to create
/// * `protocol` - Communication protocol: "http", "mcp", or "both"
/// * `features` - Comma-separated list of additional features
/// * `template` - Template to use: "basic" or "full"
///
/// # Returns
///
/// `Ok(())` on successful project creation, `Err(GeneratorError)` on failure
///
/// # Example
///
/// ```ignore
/// use axiom_generator::generate_project;
///
/// generate_project("my-project", "http", "database,auth", "basic")
///     .expect("Failed to create project");
/// ```
pub fn generate_project(
    project_name: &str,
    protocol: &str,
    features: &str,
    template: &str,
) -> GeneratorResult<()> {
    // Validate project name first to prevent path traversal
    validate_project_name(project_name)?;

    // Determine features string based on protocol
    let features_str = determine_features(protocol, features);

    // Create template context
    let context = TemplateContext::new(project_name, protocol, &features_str);

    // Get template directory (use current directory)
    let current_dir =
        std::env::current_dir().map_err(|e| GeneratorError::CurrentDirError(e.to_string()))?;
    let template_dir = current_dir
        .join("axiom-cli")
        .join("templates")
        .join(template);

    // Validate template directory exists
    validate_template_directory(&template_dir)?;

    // Create output path and validate it's within current directory
    let output_dir = current_dir.join(project_name);

    // Validate output path is within current directory
    validate_output_path(&output_dir)?;

    // Check if output directory already exists
    validate_project_directory_does_not_exist(&output_dir)?;

    // Create output directory
    fs::create_dir_all(&output_dir)
        .map_err(|_e| GeneratorError::CreateDirectoryError(output_dir.to_path_buf()))?;

    // Render templates
    render_templates(&template_dir, &output_dir, &context)?;

    // Initialize git repository
    initialize_git(&output_dir)?;

    println!("✓ Project '{}' created successfully!", project_name);
    println!("  Template: {}", template);
    println!("  Protocol: {}", protocol);
    println!("  Features: {}", features_str);
    println!("\nNext steps:");
    println!("  cd {}", project_name);
    println!("  cargo build");
    println!("  cargo run");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_determine_features_http() {
        assert_eq!(determine_features("http", ""), "http");
        assert_eq!(determine_features("http", "database"), "http,database");
    }

    #[test]
    fn test_determine_features_mcp() {
        assert_eq!(determine_features("mcp", ""), "mcp");
        assert_eq!(determine_features("mcp", "auth"), "mcp,auth");
    }

    #[test]
    fn test_determine_features_both() {
        assert_eq!(determine_features("both", ""), "both");
        assert_eq!(determine_features("both", "logging"), "both,logging");
    }

    #[test]
    fn test_determine_features_unknown() {
        assert_eq!(determine_features("unknown", ""), "http");
        assert_eq!(determine_features("unknown", "test"), "http,test");
    }

    #[test]
    fn test_generate_project_invalid_name() {
        assert!(generate_project("", "http", "", "basic").is_err());
        assert!(generate_project("../malicious", "http", "", "basic").is_err());
        assert!(generate_project("project/name", "http", "", "basic").is_err());
    }

    #[test]
    fn test_generate_project_name_too_long() {
        let long_name = "a".repeat(65);
        assert!(generate_project(&long_name, "http", "", "basic").is_err());
    }
}
