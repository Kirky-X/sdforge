use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Maximum project name length to prevent DoS attacks
const MAX_PROJECT_NAME_LENGTH: usize = 64;

/// Validate project name to prevent path traversal and other attacks
fn validate_project_name(name: &str) -> Result<()> {
    // Length validation
    if name.is_empty() {
        return Err(anyhow::anyhow!("Project name cannot be empty"));
    }
    if name.len() > MAX_PROJECT_NAME_LENGTH {
        return Err(anyhow::anyhow!(
            "Project name exceeds maximum length of {} characters",
            MAX_PROJECT_NAME_LENGTH
        ));
    }

    // Character whitelist validation (alphanumeric, underscore, hyphen, dot)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(anyhow::anyhow!(
            "Project name contains invalid characters. Use only alphanumeric, underscore, hyphen, and dot"
        ));
    }

    // Prevent special patterns
    if name.contains("..") || name.starts_with('.') || name.ends_with('.') {
        return Err(anyhow::anyhow!("Project name contains invalid pattern"));
    }

    // Prevent path separators
    if name.contains('/') || name.contains('\\') {
        return Err(anyhow::anyhow!(
            "Project name cannot contain path separators"
        ));
    }

    Ok(())
}

/// Template context for rendering
#[derive(Debug, Serialize)]
struct TemplateContext {
    project_name: String,
    protocol: String,
    features: String,
}

/// Generate a new Axiom project
pub fn generate_project(
    project_name: &str,
    protocol: &str,
    features: &str,
    template: &str,
) -> Result<()> {
    // Validate project name first to prevent path traversal
    validate_project_name(project_name)?;

    // Determine features string based on protocol
    let features_str = determine_features(protocol, features);

    // Create template context
    let context = TemplateContext {
        project_name: project_name.to_string(),
        protocol: protocol.to_string(),
        features: features_str.clone(),
    };

    // Get template directory (use current directory)
    let current_dir = std::env::current_dir()?;
    let template_dir = current_dir
        .join("axiom-cli")
        .join("templates")
        .join(template);

    if !template_dir.exists() {
        return Err(anyhow::anyhow!(
            "Template directory '{}' not found",
            template_dir.display()
        ));
    }

    // Create output path and validate it's within current directory
    let output_dir = current_dir.join(project_name);

    // Canonicalize to resolve any ".." or symlinks
    let canonical_output = output_dir
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("Invalid project path: {}", project_name))?;
    let canonical_current = current_dir.canonicalize()?;

    // Ensure the output path is within the current directory
    if !canonical_output.starts_with(&canonical_current) {
        return Err(anyhow::anyhow!(
            "Project path escapes current directory: {}",
            project_name
        ));
    }

    // Check if output directory already exists
    if output_dir.exists() {
        return Err(anyhow::anyhow!(
            "Directory '{}' already exists",
            project_name
        ));
    }

    // Create output directory
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "Failed to create project directory: {}",
            output_dir.display()
        )
    })?;

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

/// Determine features string based on protocol
fn determine_features(protocol: &str, additional_features: &str) -> String {
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

/// Render all templates in a directory
fn render_templates(
    template_dir: &Path,
    output_dir: &Path,
    context: &TemplateContext,
) -> Result<()> {
    // Create tera instance
    let mut tera = tera::Tera::new(&format!("{}/**/*.template", template_dir.display()))?;

    // Convert TemplateContext to tera::Context
    let mut tera_context = tera::Context::new();
    tera_context.insert("project_name", &context.project_name);
    tera_context.insert("protocol", &context.protocol);
    tera_context.insert("features", &context.features);

    // Walk through template directory
    for entry in walkdir::WalkDir::new(template_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Get relative path from template directory
        let relative_path = path.strip_prefix(template_dir)?;

        // Calculate output path (remove .template extension safely)
        let output_path = {
            let mut path = output_dir.join(relative_path);
            // Safe extension removal using Path methods
            if let Some(stem) = path.file_stem() {
                let new_stem = stem.to_string_lossy().replace(".template", "");
                if let Some(parent) = path.parent() {
                    path = parent.join(&new_stem);
                }
            }
            path
        };

        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read template content
        let template_content = fs::read_to_string(path)?;

        // Get template name for validation
        let template_name = relative_path.to_string_lossy().replace('\\', "/");

        // Validate template content for dangerous patterns
        validate_template_content(&template_name, &template_content)?;

        // Add template to tera
        tera.add_raw_template(&template_name, &template_content)?;

        // Render template
        let rendered = tera.render(&template_name, &tera_context)?;

        // Write output file
        fs::write(&output_path, rendered)?;

        println!("  Created: {}", output_path.display());
    }

    Ok(())
}

/// Initialize git repository with security checks
fn initialize_git(project_dir: &Path) -> Result<()> {
    // Security: Validate and normalize the project directory path
    let canonical_path = match project_dir.canonicalize() {
        Ok(path) => path,
        Err(_e) => {
            return Ok(()); // Continue without git init
        }
    };

    // Security: Ensure the path is within an allowed directory (prevent path traversal)
    // Allow only paths that don't escape the current working directory
    if !canonical_path.starts_with(std::env::current_dir()?) {
        return Ok(()); // Continue without git init
    }

    // Run git init with validated path
    let git_init_result = std::process::Command::new("git")
        .arg("init")
        .arg(&project_dir)
        .output();

    match git_init_result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!(target: "generator", "Git init failed: {}", stderr);
            }
        }
        Err(e) => {
            tracing::warn!(target: "generator", "Failed to run git init: {}", e);
        }
    }

    Ok(())
}

/// Validate template content for potentially dangerous operations
/// Prevents template injection attacks
fn validate_template_content(template_name: &str, content: &str) -> Result<()> {
    const DANGEROUS_PATTERNS: &[&str] = &[
        "include::", // File inclusion
        "import::",  // Module import
        "crate::",   // Crate access
        "super::",   // Parent module access
        "self::",    // Current module access
        "std::",     // Standard library (beyond safe subset)
    ];

    for pattern in DANGEROUS_PATTERNS {
        if content.contains(pattern) {
            return Err(anyhow::anyhow!(
                "Template '{}' contains dangerous pattern: {}",
                template_name,
                pattern
            ));
        }
    }

    Ok(())
}

/// Generate code from a single template
pub fn generate_from_template(
    template_name: &str,
    output: Option<&str>,
    context: HashMap<String, String>,
) -> Result<()> {
    let template_dir = Path::new("templates");
    let template_path = template_dir.join(format!("{}.template", template_name));

    if !template_path.exists() {
        return Err(anyhow::anyhow!("Template '{}' not found", template_name));
    }

    // Read template content
    let template_content = fs::read_to_string(&template_path)?;

    // Validate template content for dangerous patterns
    validate_template_content(template_name, &template_content)?;

    // Create tera instance
    let mut tera = tera::Tera::default();
    tera.add_raw_template(template_name, &template_content)?;

    // Convert HashMap to tera::Context
    let mut tera_context = tera::Context::new();
    for (key, value) in context {
        tera_context.insert(key, &value);
    }

    // Render template
    let rendered = tera.render(template_name, &tera_context)?;

    // Write output
    let output_path = output
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}.rs", template_name));
    fs::write(&output_path, rendered)?;

    println!("✓ Generated: {}", output_path);

    Ok(())
}
