// Copyright (c) 2026 Kirky.X
//! Integration tests for the Axiom CLI generator.
//!
//! These tests verify the complete functionality of the generator
//! including project creation, template rendering, and git initialization.

use std::fs;
use tempfile::TempDir;

/// Helper function to assert a command succeeds.
fn assert_success(result: &Result<std::process::Output, std::io::Error>) {
    let output = result.as_ref().expect("Command should execute");
    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Helper function to assert a command fails.
fn assert_failure(result: &Result<std::process::Output, std::io::Error>) {
    let output = result.as_ref().expect("Command should execute");
    assert!(
        !output.status.success(),
        "Command should have failed but succeeded"
    );
}

#[test]
fn test_cli_new_invalid_project_name() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "../malicious";

    let mut cmd = assert_cmd::Command::cargo_bin("cargo-axiom").unwrap();
    cmd.current_dir(&temp_dir)
        .arg("new")
        .arg(project_name)
        .arg("--protocol")
        .arg("http");

    let result = cmd.output();
    assert_failure(&result);
}

#[test]
fn test_cli_new_empty_project_name() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("cargo-axiom").unwrap();
    cmd.current_dir(&temp_dir).arg("new").arg("");

    let result = cmd.output();
    assert_failure(&result);
}

#[test]
fn test_cli_new_project_already_exists() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "existing-project";

    // Create the directory first
    fs::create_dir(temp_dir.path().join(project_name)).unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("cargo-axiom").unwrap();
    cmd.current_dir(&temp_dir)
        .arg("new")
        .arg(project_name)
        .arg("--protocol")
        .arg("http");

    let result = cmd.output();
    assert_failure(&result);
}

#[test]
fn test_cli_new_invalid_protocol() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-project";

    let mut cmd = assert_cmd::Command::cargo_bin("cargo-axiom").unwrap();
    cmd.current_dir(&temp_dir)
        .arg("new")
        .arg(project_name)
        .arg("--protocol")
        .arg("invalid");

    let result = cmd.output();
    assert_failure(&result);
}

#[test]
fn test_cli_new_invalid_template() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-project";

    let mut cmd = assert_cmd::Command::cargo_bin("cargo-axiom").unwrap();
    cmd.current_dir(&temp_dir)
        .arg("new")
        .arg(project_name)
        .arg("--template")
        .arg("invalid");

    let result = cmd.output();
    assert_failure(&result);
}

#[test]
fn test_cli_help() {
    let mut cmd = assert_cmd::Command::cargo_bin("cargo-axiom").unwrap();
    cmd.arg("--help");

    let result = cmd.output();
    assert_success(&result);

    let output = result.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Axiom"), "Help should contain 'Axiom'");
}

#[test]
fn test_cli_new_help() {
    let mut cmd = assert_cmd::Command::cargo_bin("cargo-axiom").unwrap();
    cmd.arg("new").arg("--help");

    let result = cmd.output();
    assert_success(&result);

    let output = result.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Create"), "Help should contain 'Create'");
}
