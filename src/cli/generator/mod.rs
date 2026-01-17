// Copyright (c) 2026 Kirky.X
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

// 只暴露必要的公共 API
pub use error::GeneratorError;
pub use project::generate_project;
pub use template::generate_from_template;
