/// Tests for `ie init` CLI command to improve main.rs coverage
/// Focuses on the handle_init_command() function and its various branches
mod common;

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Init Command Error Path Tests
// ============================================================================

#[tokio::test]
async fn test_init_at_nonexistent_directory() {
    let mut cmd = common::ie_command();
    cmd.arg("init").arg("--at").arg("/nonexistent/path/12345");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Directory does not exist"));
}

#[tokio::test]
async fn test_init_at_file_not_directory() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("somefile.txt");
    fs::write(&file_path, "test").unwrap();

    let mut cmd = common::ie_command();
    cmd.arg("init").arg("--at").arg(file_path.to_str().unwrap());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Path is not a directory"));
}

#[tokio::test]
async fn test_init_already_exists_without_force() {
    let temp_dir = TempDir::new().unwrap();
    let intent_dir = temp_dir.path().join(".intent-engine");
    fs::create_dir_all(&intent_dir).unwrap();

    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains(".intent-engine already exists"))
        .stderr(predicate::str::contains("Use --force to re-initialize"));
}

// ============================================================================
// Init Command Success Path Tests
// ============================================================================

#[tokio::test]
async fn test_init_success_new_directory() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Intent-Engine initialized successfully",
        ))
        .stdout(predicate::str::contains(r#""success": true"#));

    // Verify directory was created
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(intent_dir.exists());
    assert!(intent_dir.join("project.db").exists());
}

#[tokio::test]
async fn test_init_success_with_force() {
    let temp_dir = TempDir::new().unwrap();
    let intent_dir = temp_dir.path().join(".intent-engine");
    fs::create_dir_all(&intent_dir).unwrap();

    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--force");

    cmd.assert().success().stdout(predicate::str::contains(
        "Intent-Engine initialized successfully",
    ));

    // Verify directory still exists
    assert!(intent_dir.exists());
    assert!(intent_dir.join("project.db").exists());
}
