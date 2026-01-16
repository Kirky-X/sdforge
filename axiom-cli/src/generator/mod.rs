//! Axiom Code Generator
//!
//! A code generation tool for the Axiom framework. Provides project scaffolding,
//! template rendering, and git initialization with security checks.
//!
//! # Modules
//!
//! - [`error`](error) - Error types for the generator
//! - [`validator`](validator) - Validation utilities for security
//! - [`template`](template) - Template rendering with Tera
//! - [`git`](git) - Git repository initialization
//! - [`project`](project) - Project generation entry point

#![allow(dead_code, unused_imports)]

pub mod error;
pub mod git;
pub mod project;
pub mod template;
pub mod validator;

pub use error::{GeneratorError, GeneratorResult};
pub use git::initialize_git;
pub use project::{determine_features, generate_project};
pub use template::{generate_from_template, render_templates, TemplateContext};
pub use validator::{
    validate_output_path, validate_project_directory_does_not_exist, validate_project_name,
    validate_template_content, validate_template_directory,
};
