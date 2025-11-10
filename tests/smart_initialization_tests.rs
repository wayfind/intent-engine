/// Integration tests for smart lazy initialization mechanism
///
/// These tests verify that Intent-Engine correctly infers the project root
/// directory based on common project markers, and initializes in the correct location.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Helper to get the intent-engine binary path
fn get_binary_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let target_dir = Path::new(manifest_dir).join("target").join("debug");
    target_dir.join("intent-engine")
}

/// Helper to create a test directory structure and run a command in a subdirectory
fn run_in_subdirectory<F>(temp_dir: &TempDir, subdir: &str, setup: F) -> std::process::Output
where
    F: FnOnce(&Path),
{
    let root = temp_dir.path();

    // Run setup function to create markers
    setup(root);

    // Create subdirectory
    let subdir_path = root.join(subdir);
    fs::create_dir_all(&subdir_path).expect("Failed to create subdirectory");

    // Run task add command from subdirectory
    Command::new(get_binary_path())
        .current_dir(&subdir_path)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute command")
}

#[test]
fn test_initialization_with_git_marker() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "src/components", |root| {
        // Create .git directory at root
        fs::create_dir(root.join(".git")).expect("Failed to create .git");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be created at the root (where .git is)
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with .git"
    );

    // Should NOT be created in subdirectory
    let wrong_location = temp_dir.path().join("src/components/.intent-engine");
    assert!(
        !wrong_location.exists(),
        ".intent-engine should not exist in subdirectory"
    );
}

#[test]
fn test_initialization_with_cargo_toml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "src/lib", |root| {
        // Create Cargo.toml at root
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"")
            .expect("Failed to create Cargo.toml");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be created at the root (where Cargo.toml is)
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with Cargo.toml"
    );
}

#[test]
fn test_initialization_with_package_json() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "src/utils", |root| {
        // Create package.json at root
        fs::write(root.join("package.json"), "{\"name\": \"test\"}")
            .expect("Failed to create package.json");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be created at the root (where package.json is)
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with package.json"
    );
}

#[test]
fn test_initialization_with_pyproject_toml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "src/main", |root| {
        // Create pyproject.toml at root
        fs::write(root.join("pyproject.toml"), "[project]\nname = \"test\"")
            .expect("Failed to create pyproject.toml");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be created at the root (where pyproject.toml is)
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with pyproject.toml"
    );
}

#[test]
fn test_initialization_with_go_mod() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "pkg/utils", |root| {
        // Create go.mod at root
        fs::write(root.join("go.mod"), "module test\n\ngo 1.21").expect("Failed to create go.mod");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be created at the root (where go.mod is)
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with go.mod"
    );
}

#[test]
fn test_initialization_priority_git_over_cargo_same_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "nested/deep/path", |root| {
        // Create both .git and Cargo.toml at root (same directory)
        // Priority test: .git should win over Cargo.toml
        fs::create_dir(root.join(".git")).expect("Failed to create .git");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"")
            .expect("Failed to create Cargo.toml");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be at root where both markers exist
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with markers"
    );
}

#[test]
fn test_initialization_stops_at_first_marker() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "nested/deep/path", |root| {
        // Create .git at root
        fs::create_dir(root.join(".git")).expect("Failed to create .git");

        // Create Cargo.toml in a subdirectory
        // According to "first match wins", initialization should stop at nested/
        // because that's the first directory (from bottom-up) with a marker
        fs::create_dir_all(root.join("nested")).expect("Failed to create nested dir");
        fs::write(root.join("nested/Cargo.toml"), "[package]\nname = \"test\"")
            .expect("Failed to create Cargo.toml");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be at nested/ (first match going upward)
    let intent_dir = temp_dir.path().join("nested/.intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at nested/ (first marker found)"
    );
}

#[test]
fn test_initialization_fallback_to_cwd_with_warning() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "random/path", |_root| {
        // Don't create any markers - this should trigger fallback
    });

    // Command should still succeed
    assert!(
        output.status.success(),
        "Command should succeed even without markers"
    );

    // Should print warning to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Warning: Could not determine a project root"),
        "Should print warning about not finding project markers"
    );
    assert!(
        stderr.contains("common markers"),
        "Should mention common markers in warning"
    );

    // .intent-engine should be created in the CWD (fallback behavior)
    let intent_dir = temp_dir.path().join("random/path/.intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist in CWD as fallback"
    );
}

#[test]
fn test_initialization_deeply_nested_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "project/src/components/ui/buttons", |root| {
        // Create .git at root
        fs::create_dir(root.join(".git")).expect("Failed to create .git");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be at root, not in deeply nested directory
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root even from deeply nested directory"
    );
}

#[test]
fn test_initialization_with_mercurial() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "src", |root| {
        // Create .hg directory at root
        fs::create_dir(root.join(".hg")).expect("Failed to create .hg");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be created at the root (where .hg is)
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with .hg"
    );
}

#[test]
fn test_initialization_with_maven_pom_xml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "src/main/java", |root| {
        // Create pom.xml at root
        fs::write(
            root.join("pom.xml"),
            "<project><modelVersion>4.0.0</modelVersion></project>",
        )
        .expect("Failed to create pom.xml");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be created at the root (where pom.xml is)
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with pom.xml"
    );
}

#[test]
fn test_initialization_with_gradle() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let output = run_in_subdirectory(&temp_dir, "app/src", |root| {
        // Create build.gradle at root
        fs::write(root.join("build.gradle"), "plugins { id 'java' }")
            .expect("Failed to create build.gradle");
    });

    // Command should succeed
    assert!(output.status.success(), "Command failed: {:?}", output);

    // .intent-engine should be created at the root (where build.gradle is)
    let intent_dir = temp_dir.path().join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with build.gradle"
    );
}

#[test]
fn test_existing_intent_engine_found_and_reused() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create .git marker at root
    fs::create_dir(root.join(".git")).expect("Failed to create .git");

    // First command: initialize from root
    let output1 = Command::new(get_binary_path())
        .current_dir(root)
        .args(["task", "add", "--name", "First task"])
        .output()
        .expect("Failed to execute command");

    assert!(output1.status.success(), "First command failed");

    // .intent-engine should exist at root
    let intent_dir = root.join(".intent-engine");
    assert!(
        intent_dir.exists(),
        "First initialization should create .intent-engine at root"
    );

    // Verify the database file exists
    let db_path = intent_dir.join("project.db");
    assert!(
        db_path.exists(),
        "Database file should exist after initialization"
    );

    // Run another command from a subdirectory
    let subdir = root.join("src/components");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

    let output2 = Command::new(get_binary_path())
        .current_dir(&subdir)
        .args(["task", "add", "--name", "Second task"])
        .output()
        .expect("Failed to execute command");

    assert!(output2.status.success(), "Second command failed");

    // Should reuse existing .intent-engine at root (not create a new one in subdirectory)
    assert!(
        intent_dir.exists(),
        "Existing .intent-engine should still exist at root"
    );

    // The database file should be the same one (modified time should be recent but same file)
    let metadata2 = fs::metadata(&db_path).expect("Failed to get db metadata after second command");
    assert!(
        metadata2.modified().is_ok(),
        "Database file should still exist and be accessible"
    );

    // Verify no .intent-engine was created in subdirectory
    let subdir_intent = subdir.join(".intent-engine");
    assert!(
        !subdir_intent.exists(),
        "Should not create new .intent-engine in subdirectory"
    );
}
