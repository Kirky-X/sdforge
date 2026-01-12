use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

    let output_dir = Path::new(project_name);

    // Check if output directory already exists
    if output_dir.exists() {
        return Err(anyhow::anyhow!(
            "Directory '{}' already exists",
            project_name
        ));
    }

    // Create output directory
    fs::create_dir_all(output_dir).context("Failed to create project directory")?;

    // Render templates
    render_templates(&template_dir, output_dir, &context)?;

    // Initialize git repository
    initialize_git(output_dir)?;

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
    let mut features = match protocol {
        "http" => "http".to_string(),
        "mcp" => "mcp".to_string(),
        "both" => "http,mcp".to_string(),
        _ => "http".to_string(),
    };

    // Add additional features if provided
    if !additional_features.is_empty() {
        features.push(',');
        features.push_str(additional_features);
    }

    features
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

        // Calculate output path (remove .template extension)
        let output_path = output_dir
            .join(relative_path)
            .to_string_lossy()
            .replace(".template", "");
        let output_path = Path::new(&output_path);

        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read template content
        let template_content = fs::read_to_string(path)?;
        let template_name = relative_path.to_string_lossy().replace('\\', "/");

        // Add template to tera
        tera.add_raw_template(&template_name, &template_content)?;

        // Render template
        let rendered = tera.render(&template_name, &tera_context)?;

        // Write output file
        fs::write(output_path, rendered)?;

        println!("  Created: {}", output_path.display());
    }

    Ok(())
}

/// Initialize git repository with security checks
fn initialize_git(project_dir: &Path) -> Result<()> {
    // Security: Validate and normalize the project directory path
    let canonical_path = match project_dir.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            #[cfg(feature = "logging")]
            tracing::warn!(target: "generator", "Could not canonicalize project dir: {}", e);
            return Ok(()); // Continue without git init
        }
    };

    // Security: Ensure the path is within an allowed directory (prevent path traversal)
    // Allow only paths that don't escape the current working directory
    if !canonical_path.starts_with(std::env::current_dir()?) {
        #[cfg(feature = "logging")]
        tracing::warn!(target: "generator", "Project directory is outside allowed scope");
        return Ok(()); // Continue without git init
    }

    // Run git init with validated path
    std::process::Command::new("git").arg("init").output().ok(); // Ignore errors if git is not available

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
