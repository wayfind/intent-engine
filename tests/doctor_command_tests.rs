use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn get_binary_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let target_dir = Path::new(manifest_dir).join("target").join("debug");
    target_dir.join("intent-engine")
}

#[test]
fn doctor_reports_database_path_resolution_details() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    let output = Command::new(get_binary_path())
        .current_dir(temp_dir.path())
        .arg("doctor")
        .output()
        .expect("failed to run doctor command");

    if !output.status.success() {
        panic!(
            "doctor command failed: status={:?}, stdout={}, stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8(output.stdout).expect("doctor output should be utf-8");
    let value: Value = serde_json::from_str(&stdout).expect("doctor output should be valid json");

    let checks = value["checks"]
        .as_array()
        .expect("doctor output must include checks array");
    let db_check = checks
        .iter()
        .find(|check| check["check"] == "Database Path Resolution")
        .expect("doctor output must include database path resolution check");

    assert_eq!(db_check["status"], "✓ INFO");

    let details = &db_check["details"];
    assert_eq!(details["env_var_set"].as_bool(), Some(false));

    let directories = details["directories_checked"]
        .as_array()
        .expect("directories_checked should be an array");
    assert!(
        directories
            .iter()
            .any(|entry| entry["is_selected"].as_bool() == Some(true)
                && entry["has_intent_engine"].as_bool() == Some(true)),
        "expected at least one directory to be selected during traversal"
    );

    let expected_db_path = temp_dir
        .path()
        .join(".intent-engine")
        .join("project.db")
        .display()
        .to_string();
    assert_eq!(
        details["final_database_path"].as_str(),
        Some(expected_db_path.as_str())
    );
    assert_eq!(
        details["resolution_method"].as_str(),
        Some("Upward Directory Traversal")
    );
}
