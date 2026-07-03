// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! SDForge CLI Tool
//!
//! Command-line interface for the SDForge framework.
//!
//! This binary is only compiled when the `cli` feature is enabled.

#[cfg(feature = "cli")]
mod cli;

#[cfg(feature = "cli")]
use anyhow::Result;
#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};
#[cfg(feature = "cli")]
use std::collections::HashMap;

/// SDForge CLI - Command-line tool for the SDForge framework
#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(name = "sdforge")]
#[command(about = "Command-line tool for the SDForge framework", long_about = None)]
struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

/// Validate protocol parameter
#[cfg(feature = "cli")]
fn validate_protocol(protocol: &str) -> Result<String, String> {
    match protocol {
        "http" | "mcp" | "both" => Ok(protocol.to_string()),
        _ => Err("Protocol must be 'http', 'mcp', or 'both'".to_string()),
    }
}

/// Validate template parameter
#[cfg(feature = "cli")]
fn validate_template(template: &str) -> Result<String, String> {
    match template {
        "basic" | "full" => Ok(template.to_string()),
        _ => Err("Template must be 'basic' or 'full'".to_string()),
    }
}

/// Available commands
#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// Create a new SDForge project
    New {
        /// Project name (must be a valid Rust identifier)
        name: String,
        /// Protocol to use (http, mcp, both)
        #[arg(long, default_value = "http", value_parser = validate_protocol)]
        protocol: String,
        /// Additional features (comma-separated)
        #[arg(long)]
        features: Option<String>,
        /// Template to use (basic, full)
        #[arg(long, default_value = "basic", value_parser = validate_template)]
        template: String,
    },
    /// Initialize SDForge in current directory
    Init {
        /// Protocol to use (http, mcp, both)
        #[arg(long, default_value = "http", value_parser = validate_protocol)]
        protocol: String,
        /// Additional features (comma-separated)
        #[arg(long)]
        features: Option<String>,
    },
    /// Generate code from templates
    Generate {
        /// Template name
        template: String,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
}

#[cfg(feature = "cli")]
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            name,
            protocol,
            features,
            template,
        } => {
            let features_str = features.unwrap_or_default();
            cli::generator::generate_project(&name, &protocol, &features_str, &template)?;
        }
        Commands::Init { protocol, features } => {
            let features_str = features.unwrap_or_default();
            let current_dir = std::env::current_dir()?;
            let project_name = current_dir
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Cannot determine current directory name"))?;
            cli::generator::generate_project(project_name, &protocol, &features_str, "full")?;
        }
        Commands::Generate { template, output } => {
            let output_path = output.as_deref();
            let context = HashMap::new();
            cli::generator::generate_from_template(&template, output_path, context)?;
        }
    }

    Ok(())
}

/// Main entry point when CLI feature is not enabled.
/// This function exists to satisfy Cargo's requirement that a binary crate has a main function.
/// When `cli` feature is disabled, running this binary does nothing.
#[cfg(not(feature = "cli"))]
fn main() {
    panic!("CLI feature is not enabled. To use the SDForge CLI, enable the `cli` feature with: cargo build --features cli");
}
