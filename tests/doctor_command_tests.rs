mod common;

use std::process::Command;

#[test]
fn doctor_reports_database_path_resolution_details() {
    // Use common::setup_test_env() to ensure proper initialization
    // (.git marker + HOME isolation + auto-init database)
    let temp_dir = common::setup_test_env();

    // Use Cargo-provided environment variable for binary path
    // This works correctly in all test environments (local, CI, llvm-cov, etc.)
    let binary_path = env!("CARGO_BIN_EXE_ie");
    let output = Command::new(binary_path)
        .current_dir(temp_dir.path())
        .env("HOME", "/nonexistent") // Ensure isolation
        .env("USERPROFILE", "/nonexistent") // Windows isolation
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

    // Verify simplified natural language output format
    assert!(
        stdout.contains("Database:"),
        "Should show database location"
    );
    assert!(
        stdout.contains("project.db"),
        "Should show project.db in output"
    );
    assert!(
        stdout.contains("Ancestor directories with databases:"),
        "Should show ancestor directories section"
    );
    assert!(
        stdout.contains("Dashboard:"),
        "Should show dashboard status"
    );

    // Verify database path is shown
    let expected_db_path = temp_dir
        .path()
        .join(".intent-engine")
        .join("project.db")
        .display()
        .to_string();
    assert!(
        stdout.contains(&expected_db_path),
        "Should show the correct database path: {}",
        expected_db_path
    );
}
