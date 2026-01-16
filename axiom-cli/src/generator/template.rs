// Copyright (c) 2026 Kirky.X
//! Template rendering utilities.
//!
//! Provides functions for rendering templates using the Tera template engine
//! with security validation and proper error handling.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tera::{Context, Tera};

use crate::generator::error::{GeneratorError, GeneratorResult};
use crate::generator::validator::validate_template_content;

/// Template context for rendering.
///
/// This struct holds the data that will be inserted into templates.
#[derive(Debug, Serialize)]
pub(crate) struct TemplateContext {
    /// Name of the project
    pub project_name: String,
    /// Communication protocol (http, mcp, both)
    pub protocol: String,
    /// Features to enable
    pub features: String,
}

impl TemplateContext {
    /// Create a new template context.
    ///
    /// # Arguments
    ///
    /// * `project_name` - Name of the project
    /// * `protocol` - Communication protocol
    /// * `features` - Features string
    pub fn new(project_name: &str, protocol: &str, features: &str) -> Self {
        Self {
            project_name: project_name.to_string(),
            protocol: protocol.to_string(),
            features: features.to_string(),
        }
    }
}

/// Calculate the output path for a template file.
///
/// Removes the `.template` extension from the filename.
///
/// # Arguments
///
/// * `template_path` - Path to the template file
/// * `output_dir` - Output directory
///
/// # Returns
///
/// The calculated output path
fn calculate_output_path(template_path: &Path, output_dir: &Path) -> PathBuf {
    let relative_path = template_path
        .strip_prefix(template_path.parent().unwrap())
        .unwrap();
    let mut output_path = output_dir.join(relative_path);

    // Safe extension removal using Path methods
    if let Some(stem) = output_path.file_stem() {
        let new_stem = stem.to_string_lossy().replace(".template", "");
        if let Some(parent) = output_path.parent() {
            output_path = parent.join(&new_stem);
        }
    }

    output_path
}

/// Render a single template file.
///
/// # Arguments
///
/// * `tera` - Tera instance
/// * `template_path` - Path to the template file
/// * `output_path` - Path to write the rendered template
/// * `context` - Template context
///
/// # Returns
///
/// `Ok(())` on success, `Err(GeneratorError)` on failure
fn render_single_template(
    tera: &mut Tera,
    template_path: &Path,
    output_path: &Path,
    context: &Context,
) -> GeneratorResult<()> {
    // Read template content
    let template_content = fs::read_to_string(template_path)?;

    // Get template name for validation
    let template_name = template_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Validate template content for dangerous patterns
    validate_template_content(&template_name, &template_content)?;

    // Add template to tera
    tera.add_raw_template(&template_name, &template_content)
        .map_err(|e| GeneratorError::TemplateRenderError(template_name.clone(), e.to_string()))?;

    // Render template
    let rendered = tera
        .render(&template_name, context)
        .map_err(|e| GeneratorError::TemplateRenderError(template_name.clone(), e.to_string()))?;

    // Write output file
    fs::write(output_path, rendered)
        .map_err(|e| GeneratorError::WriteFileError(output_path.to_path_buf(), e.to_string()))?;

    Ok(())
}

/// Render all templates in a directory.
///
/// Walks through the template directory, renders each template,
/// and writes the output to the corresponding location.
///
/// # Arguments
///
/// * `template_dir` - Directory containing templates
/// * `output_dir` - Output directory for rendered templates
/// * `context` - Template context with variables
///
/// # Returns
///
/// `Ok(())` on success, `Err(GeneratorError)` on failure
pub(crate) fn render_templates(
    template_dir: &Path,
    output_dir: &Path,
    context: &TemplateContext,
) -> GeneratorResult<()> {
    // Create tera instance
    let tera_pattern = format!("{}/**/*.template", template_dir.display());
    let mut tera = Tera::new(&tera_pattern).map_err(|e| {
        GeneratorError::TemplateRenderError(template_dir.display().to_string(), e.to_string())
    })?;

    // Convert TemplateContext to tera::Context
    let mut tera_context = Context::new();
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

        // Calculate output path
        let output_path = calculate_output_path(path, output_dir);

        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|_e| GeneratorError::CreateDirectoryError(parent.to_path_buf()))?;
        }

        // Render and write template
        render_single_template(&mut tera, path, &output_path, &tera_context)?;

        println!("  Created: {}", output_path.display());
    }

    Ok(())
}

/// Generate code from a single template.
///
/// # Arguments
///
/// * `template_name` - Name of the template (without extension)
/// * `output` - Optional output path (defaults to `{template_name}.rs` in current directory)
/// * `context` - HashMap of template variables
///
/// # Returns
///
/// `Ok(())` on success, `Err(GeneratorError)` on failure
pub fn generate_from_template(
    template_name: &str,
    output: Option<&str>,
    context: HashMap<String, String>,
) -> GeneratorResult<()> {
    let template_dir = Path::new("templates");
    let template_path = template_dir.join(format!("{}.template", template_name));

    if !template_path.exists() {
        return Err(GeneratorError::TemplateFileNotFound(
            template_name.to_string(),
        ));
    }

    // Read template content
    let template_content = fs::read_to_string(&template_path)?;

    // Validate template content for dangerous patterns
    validate_template_content(template_name, &template_content)?;

    // Create tera instance
    let mut tera = Tera::default();
    tera.add_raw_template(template_name, &template_content)
        .map_err(|e| {
            GeneratorError::TemplateRenderError(template_name.to_string(), e.to_string())
        })?;

    // Convert HashMap to tera::Context
    let mut tera_context = Context::new();
    for (key, value) in context {
        tera_context.insert(&key, &value);
    }

    // Render template
    let rendered = tera.render(template_name, &tera_context).map_err(|e| {
        GeneratorError::TemplateRenderError(template_name.to_string(), e.to_string())
    })?;

    // Validate and sanitize output path to prevent path traversal attacks
    let output_path = match output {
        Some(path) => validate_output_path(path)?,
        None => std::env::current_dir()
            .map_err(|e| GeneratorError::CurrentDirError(e.to_string()))?
            .join(format!("{}.rs", template_name)),
    };

    // Write output
    fs::write(&output_path, rendered)
        .map_err(|e| GeneratorError::WriteFileError(output_path.to_path_buf(), e.to_string()))?;

    println!("✓ Generated: {}", output_path.display());

    Ok(())
}

/// Validate and get output path.
///
/// This is a convenience function that handles the common case
/// of validating an output path.
///
/// # Arguments
///
/// * `output` - Optional output path as string slice
/// * `default_filename` - Default filename if output is None
///
/// # Returns
///
/// The validated output path
pub fn validate_output_path(output: &str) -> GeneratorResult<PathBuf> {
    crate::generator::validator::validate_output_path(Path::new(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_output_path() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("test.template");
        let output_dir = temp_dir.path();

        let output_path = calculate_output_path(&template_path, output_dir);

        assert_eq!(output_path.file_stem().unwrap().to_string_lossy(), "test");
    }

    #[test]
    fn test_render_templates() {
        let temp_dir = TempDir::new().unwrap();
        let template_dir = temp_dir.path().join("templates");
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&template_dir).unwrap();

        // Create a template file
        let template_file = template_dir.join("test.template");
        let mut file = File::create(&template_file).unwrap();
        writeln!(file, "fn main() {{ println!(\"Hello, {{}}\"); }}").unwrap();

        let context = TemplateContext::new("test-project", "http", "database");

        assert!(render_templates(&template_dir, &output_dir, &context).is_ok());

        // Check output file was created
        let output_file = output_dir.join("test");
        assert!(output_file.exists());
    }

    #[test]
    fn test_generate_from_template() {
        let temp_dir = TempDir::new().unwrap();
        let template_dir = temp_dir.path().join("templates");
        fs::create_dir_all(&template_dir).unwrap();

        // Create a template file
        let template_file = template_dir.join("hello.template");
        let mut file = File::create(&template_file).unwrap();
        writeln!(file, "fn greet() {{ println!(\"Hello\"); }}").unwrap();

        let context: HashMap<String, String> = HashMap::new();

        // Set temp dir as working directory for template lookup
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = generate_from_template("hello", None, context);

        // Restore original directory
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
    }
}
