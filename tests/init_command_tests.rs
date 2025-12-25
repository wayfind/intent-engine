mod common;

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

/// Helper to get the binary command
fn ie_command() -> Command {
    common::ie_command()
}

#[test]
fn test_init_creates_directory_and_database() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create a .git marker
    fs::create_dir(project_dir.join(".git")).unwrap();

    let output = ie_command()
        .current_dir(project_dir)
        .arg("init")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("Intent-Engine initialized successfully"));

    // Verify directory and database were created
    let intent_dir = project_dir.join(".intent-engine");
    assert!(intent_dir.exists());
    assert!(intent_dir.is_dir());

    let db_path = intent_dir.join("project.db");
    assert!(db_path.exists());
    assert!(db_path.is_file());
}

#[test]
fn test_init_fails_when_already_exists() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create a .git marker
    fs::create_dir(project_dir.join(".git")).unwrap();

    // Initialize once
    ie_command()
        .current_dir(project_dir)
        .arg("init")
        .assert()
        .success();

    // Try to initialize again (should fail)
    let output = ie_command()
        .current_dir(project_dir)
        .arg("init")
        .assert()
        .failure();

    // Error message could be in stdout or stderr
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Check if there's JSON error in output
    if let Ok(json) = serde_json::from_str::<Value>(&stdout) {
        assert!(json["error"]
            .as_str()
            .unwrap()
            .contains(".intent-engine already exists"));
    } else {
        // Check raw output (stdout or stderr)
        assert!(
            combined.contains(".intent-engine already exists"),
            "Expected error message not found. stdout: {}, stderr: {}",
            stdout,
            stderr
        );
    }
}

#[test]
fn test_init_force_reinitializes() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create a .git marker
    fs::create_dir(project_dir.join(".git")).unwrap();

    // Initialize once
    ie_command()
        .current_dir(project_dir)
        .arg("init")
        .assert()
        .success();

    // Write a test file to verify re-initialization
    let test_file = project_dir.join(".intent-engine").join("test.txt");
    fs::write(&test_file, "test content").unwrap();
    assert!(test_file.exists());

    // Re-initialize with --force
    let output = ie_command()
        .current_dir(project_dir)
        .arg("init")
        .arg("--force")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["success"], true);

    // Verify database still exists (re-initialized)
    let db_path = project_dir.join(".intent-engine").join("project.db");
    assert!(db_path.exists());
}

#[test]
fn test_init_with_custom_directory() {
    let temp_dir = TempDir::new().unwrap();
    let custom_dir = temp_dir.path().join("custom-location");
    fs::create_dir(&custom_dir).unwrap();

    let output = ie_command()
        .arg("init")
        .arg("--at")
        .arg(custom_dir.to_str().unwrap())
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["root"].as_str().unwrap().contains("custom-location"));

    // Verify directory created at custom location
    let intent_dir = custom_dir.join(".intent-engine");
    assert!(intent_dir.exists());
    assert!(intent_dir.is_dir());
}

#[test]
fn test_init_with_nonexistent_directory_fails() {
    let output = ie_command()
        .arg("init")
        .arg("--at")
        .arg("/nonexistent/path/xyz123")
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Check if there's JSON error in output
    if let Ok(json) = serde_json::from_str::<Value>(&stdout) {
        assert!(json["error"]
            .as_str()
            .unwrap()
            .contains("Directory does not exist"));
    } else {
        // Check raw output (stdout or stderr)
        assert!(
            combined.contains("Directory does not exist"),
            "Expected error message not found. stdout: {}, stderr: {}",
            stdout,
            stderr
        );
    }
}

#[test]
fn test_init_in_current_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create project structure with nested directories
    fs::create_dir(project_dir.join(".git")).unwrap();
    let src_dir = project_dir.join("src");
    fs::create_dir(&src_dir).unwrap();

    // Run init from nested directory
    let output = ie_command()
        .current_dir(&src_dir)
        .arg("init")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Verify .intent-engine is created in current directory (src/), not at project root
    assert_eq!(json["success"], true);

    let src_intent_dir = src_dir.join(".intent-engine");
    assert!(src_intent_dir.exists());

    let parent_intent_dir = project_dir.join(".intent-engine");
    assert!(!parent_intent_dir.exists());
}
