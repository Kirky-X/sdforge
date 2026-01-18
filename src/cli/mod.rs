// Copyright (c) 2026 Kirky.X
//! CLI module for SDForge

pub mod generator;

#[cfg(test)]
mod tests {
    use super::generator::{GeneratorConfig, ProjectGenerator};
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Test ProjectGenerator creation
    #[test]
    fn test_project_generator_creation() {
        let generator = ProjectGenerator::new();
        let _ = generator;
    }

    /// Test GeneratorConfig default values
    #[test]
    fn test_generator_config_default() {
        let config = GeneratorConfig::default();
        assert!(config.name.is_empty());
        assert!(config.description.is_empty());
        assert!(config.author.is_empty());
        assert!(config.version.is_empty());
    }

    /// Test GeneratorConfig with custom values
    #[test]
    fn test_generator_config_custom() {
        let config = GeneratorConfig {
            name: "my-project".to_string(),
            description: "A test project".to_string(),
            author: "Test Author".to_string(),
            version: "0.1.0".to_string(),
            path: PathBuf::from("/tmp/test"),
            features: vec!["http".to_string(), "mcp".to_string()],
        };

        assert_eq!(config.name, "my-project");
        assert_eq!(config.features.len(), 2);
        assert!(config.features.contains(&"http".to_string()));
    }

    /// Test GeneratorConfig path handling
    #[test]
    fn test_generator_config_path() {
        let temp_dir = TempDir::new().unwrap();
        let config = GeneratorConfig {
            path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        assert_eq!(config.path, temp_dir.path());
    }

    /// Test project name validation logic
    #[test]
    fn test_project_name_patterns() {
        // Valid names
        assert!(is_valid_project_name("my-project"));
        assert!(is_valid_project_name("my_project"));
        assert!(is_valid_project_name("myproject123"));

        // Invalid names
        assert!(!is_valid_project_name("My Project")); // spaces
        assert!(!is_valid_project_name("my-project!")); // special chars
        assert!(!is_valid_project_name("-leading-dash")); // leading dash
    }

    /// Helper function to validate project names
    fn is_valid_project_name(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        if name.starts_with('-') {
            return false;
        }
        if name.contains(char::is_whitespace) {
            return false;
        }
        for c in name.chars() {
            if !c.is_alphanumeric() && c != '-' && c != '_' {
                return false;
            }
        }
        true
    }

    /// Test version format patterns
    #[test]
    fn test_version_format() {
        // Valid versions
        assert!(is_valid_version("0.1.0"));
        assert!(is_valid_version("1.0.0"));
        assert!(is_valid_version("2.10.5"));
        assert!(is_valid_version("0.0.1-alpha"));

        // Invalid versions
        assert!(!is_valid_version("invalid"));
        assert!(!is_valid_version("v1.0.0")); // v prefix not allowed in this format
        assert!(!is_valid_version("1.0")); // needs patch version
    }

    /// Helper function to validate version formats
    fn is_valid_version(version: &str) -> bool {
        if version.is_empty() {
            return false;
        }
        // Simple semver-like check
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return false;
        }
        for part in parts {
            if part.is_empty() {
                return false;
            }
            // Check if it's a number or number with pre-release
            let base = part.split('-').next().unwrap();
            if base.chars().any(|c| !c.is_ascii_digit()) {
                return false;
            }
        }
        true
    }
}
