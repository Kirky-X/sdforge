// Trybuild compile-fail tests for macro error handling
// These tests verify that the macros properly reject invalid inputs at compile time

use std::path::PathBuf;

#[cfg(test)]
mod compile_fail_tests {
    use super::PathBuf;

    #[test]
    fn test_missing_name_compile_fail() {
        let t = trybuild::TestCases::new();
        let test_file: PathBuf = ["tests", "compile_fail", "missing_name.rs"].iter().collect();
        t.compile_fail(test_file);
    }

    #[test]
    fn test_missing_version_compile_fail() {
        let t = trybuild::TestCases::new();
        let test_file: PathBuf = ["tests", "compile_fail", "missing_version.rs"].iter().collect();
        t.compile_fail(test_file);
    }

    #[test]
    fn test_unknown_attribute_compile_fail() {
        let t = trybuild::TestCases::new();
        let test_file: PathBuf = ["tests", "compile_fail", "unknown_attribute.rs"].iter().collect();
        t.compile_fail(test_file);
    }

    #[test]
    fn test_missing_prefix_compile_fail() {
        let t = trybuild::TestCases::new();
        let test_file: PathBuf = ["tests", "compile_fail", "missing_prefix.rs"].iter().collect();
        t.compile_fail(test_file);
    }

    #[test]
    fn test_module_unknown_attribute_compile_fail() {
        let t = trybuild::TestCases::new();
        let test_file: PathBuf = ["tests", "compile_fail", "module_unknown_attribute.rs"].iter().collect();
        t.compile_fail(test_file);
    }
}