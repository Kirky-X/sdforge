// Copyright (c) 2026 Kirky.X
//! CLI module for SDForge

pub mod generator;

#[cfg(test)]
mod tests {
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
        // Simple semver-like check - requires major.minor.patch
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
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
