// Copyright (c) 2026 Kirky.X
//! Axiom CLI Tool
//!
//! Command-line interface for the Axiom framework.

mod generator;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;

/// Axiom CLI - Command-line tool for the Axiom framework
#[derive(Parser)]
#[command(name = "cargo-axiom")]
#[command(about = "Command-line tool for the Axiom framework", long_about = None)]
struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

/// Validate protocol parameter
fn validate_protocol(protocol: &str) -> Result<String, String> {
    match protocol {
        "http" | "mcp" | "both" => Ok(protocol.to_string()),
        _ => Err("Protocol must be 'http', 'mcp', or 'both'".to_string()),
    }
}

/// Validate template parameter
fn validate_template(template: &str) -> Result<String, String> {
    match template {
        "basic" | "full" => Ok(template.to_string()),
        _ => Err("Template must be 'basic' or 'full'".to_string()),
    }
}

/// Available commands
#[derive(Subcommand)]
enum Commands {
    /// Create a new Axiom project
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
    /// Initialize Axiom in current directory
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            name,
            protocol,
            features,
            template,
        } => {
            let features_str = features.unwrap_or_default();
            generator::generate_project(&name, &protocol, &features_str, &template)?;
        }
        Commands::Init { protocol, features } => {
            let features_str = features.unwrap_or_default();
            let current_dir = std::env::current_dir()?;
            let project_name = current_dir
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Cannot determine current directory name"))?;
            generator::generate_project(project_name, &protocol, &features_str, "full")?;
        }
        Commands::Generate { template, output } => {
            let output_path = output.as_deref();
            let context = HashMap::new();
            generator::generate_from_template(&template, output_path, context)?;
        }
    }

    Ok(())
}
