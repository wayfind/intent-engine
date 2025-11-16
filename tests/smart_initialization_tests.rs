/// Integration tests for smart lazy initialization mechanism
///
/// These tests verify that Intent-Engine correctly infers the project root
/// directory based on common project markers, and initializes in the correct location.
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

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

    // Use Cargo-provided environment variable for binary path
    // This works correctly in all test environments (local, CI, llvm-cov, etc.)
    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Run task add command from subdirectory
    Command::new(binary_path)
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

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // First command: initialize from root
    let output1 = Command::new(binary_path)
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

    let output2 = Command::new(binary_path)
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

// ============================================================================
// EDGE CASE TESTS: Symlinks, Special Files, and Boundary Conditions
// ============================================================================

#[test]
fn test_initialization_with_symlinked_git_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create actual .git directory
    let real_git = root.join("real_git");
    fs::create_dir_all(&real_git).expect("Failed to create real git dir");

    // Create symlink to .git
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&real_git, root.join(".git")).expect("Failed to create symlink");
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_dir;
        symlink_dir(&real_git, root.join(".git")).expect("Failed to create symlink");
    }

    // Run command from subdirectory
    let subdir = root.join("src");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

    let binary_path = env!("CARGO_BIN_EXE_ie");
    let output = Command::new(binary_path)
        .current_dir(&subdir)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute command");

    // Should succeed with symlinked marker
    assert!(
        output.status.success(),
        "Command should succeed with symlinked .git"
    );

    // .intent-engine should be created at root
    let intent_dir = root.join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with symlinked .git"
    );
}

#[test]
fn test_initialization_with_git_as_file_submodule() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create .git as a file (simulating Git submodule)
    fs::write(
        root.join(".git"),
        "gitdir: ../main-project/.git/modules/submodule",
    )
    .expect("Failed to create .git file");

    // Run command from subdirectory
    let subdir = root.join("src");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

    let binary_path = env!("CARGO_BIN_EXE_ie");
    let output = Command::new(binary_path)
        .current_dir(&subdir)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute command");

    // Should succeed - .git as file is valid marker
    assert!(
        output.status.success(),
        "Command should succeed with .git as file (submodule case)"
    );

    // .intent-engine should be created at root
    let intent_dir = root.join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with .git file"
    );
}

#[test]
fn test_initialization_with_empty_marker_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create empty package.json (invalid JSON, but exists)
    fs::write(root.join("package.json"), "").expect("Failed to create empty package.json");

    // Run command from subdirectory
    let subdir = root.join("src");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

    let binary_path = env!("CARGO_BIN_EXE_ie");
    let output = Command::new(binary_path)
        .current_dir(&subdir)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute command");

    // Should succeed - only checks existence, not validity
    assert!(
        output.status.success(),
        "Command should succeed even with empty marker file"
    );

    // .intent-engine should be created at root
    let intent_dir = root.join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with empty marker file"
    );
}

#[test]
fn test_initialization_in_nested_monorepo_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create monorepo structure
    fs::create_dir(root.join(".git")).expect("Failed to create .git");

    // Backend service with Cargo.toml
    fs::create_dir_all(root.join("backend/src")).expect("Failed to create backend/src");
    fs::write(
        root.join("backend/Cargo.toml"),
        "[package]\nname = \"backend\"",
    )
    .expect("Failed to create backend Cargo.toml");

    // Frontend service with package.json
    fs::create_dir_all(root.join("frontend/src")).expect("Failed to create frontend/src");
    fs::write(
        root.join("frontend/package.json"),
        r#"{"name": "frontend"}"#,
    )
    .expect("Failed to create frontend package.json");

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Test from backend: should find Cargo.toml in backend/ first (nearest marker)
    let output_backend = Command::new(binary_path)
        .current_dir(root.join("backend/src"))
        .args(["task", "add", "--name", "Backend task"])
        .output()
        .expect("Failed to execute backend command");

    assert!(
        output_backend.status.success(),
        "Backend command should succeed"
    );

    // .intent-engine should be in backend/ (nearest marker)
    let backend_intent = root.join("backend/.intent-engine");
    assert!(
        backend_intent.exists(),
        ".intent-engine should exist in backend/ (nearest marker)"
    );

    // Test from frontend: should find package.json in frontend/ first
    let output_frontend = Command::new(binary_path)
        .current_dir(root.join("frontend/src"))
        .args(["task", "add", "--name", "Frontend task"])
        .output()
        .expect("Failed to execute frontend command");

    assert!(
        output_frontend.status.success(),
        "Frontend command should succeed"
    );

    // .intent-engine should be in frontend/ (nearest marker)
    let frontend_intent = root.join("frontend/.intent-engine");
    assert!(
        frontend_intent.exists(),
        ".intent-engine should exist in frontend/ (nearest marker)"
    );

    // Should NOT be at monorepo root (even though .git exists there)
    let root_intent = root.join(".intent-engine");
    assert!(
        !root_intent.exists(),
        ".intent-engine should not exist at monorepo root"
    );
}

#[test]
fn test_initialization_with_multiple_markers_different_levels() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create .git at root
    fs::create_dir(root.join(".git")).expect("Failed to create .git");

    // Create nested structure with Cargo.toml
    fs::create_dir_all(root.join("rust-project/nested/deep"))
        .expect("Failed to create nested dirs");
    fs::write(
        root.join("rust-project/Cargo.toml"),
        "[package]\nname = \"test\"",
    )
    .expect("Failed to create Cargo.toml");

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Run from deeply nested directory
    let output = Command::new(binary_path)
        .current_dir(root.join("rust-project/nested/deep"))
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command should succeed");

    // Should initialize in rust-project/ (first marker found going up)
    let rust_project_intent = root.join("rust-project/.intent-engine");
    assert!(
        rust_project_intent.exists(),
        ".intent-engine should be in rust-project/ (first marker found)"
    );

    // Should NOT be at root (even though .git is there)
    let root_intent = root.join(".intent-engine");
    assert!(
        !root_intent.exists(),
        ".intent-engine should not be at root"
    );
}

#[test]
fn test_concurrent_initialization_attempts() {
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create .git marker
    fs::create_dir(root.join(".git")).expect("Failed to create .git");

    // Create subdirectory for testing
    let subdir = root.join("src");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

    // Use barrier to synchronize thread starts
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];

    let binary_path = env!("CARGO_BIN_EXE_ie");

    for i in 0..3 {
        let barrier_clone = Arc::clone(&barrier);
        let subdir_clone = subdir.clone();
        let binary = binary_path.to_string();

        let handle = thread::spawn(move || {
            // Wait for all threads to be ready
            barrier_clone.wait();

            // Add small stagger to reduce exact simultaneity
            thread::sleep(Duration::from_millis(i * 10));

            // Execute command
            Command::new(binary)
                .current_dir(&subdir_clone)
                .args(["task", "add", "--name", &format!("Concurrent task {}", i)])
                .output()
                .expect("Failed to execute command")
        });

        handles.push(handle);
    }

    // Collect results
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // At least one command should succeed
    let success_count = results.iter().filter(|r| r.status.success()).count();

    // Print debug info if all failed
    if success_count == 0 {
        for (i, result) in results.iter().enumerate() {
            eprintln!("Thread {} status: {:?}", i, result.status);
            eprintln!(
                "Thread {} stderr: {}",
                i,
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }

    assert!(
        success_count >= 1,
        "At least one concurrent initialization should succeed, but got {} successes",
        success_count
    );

    // .intent-engine should exist
    let intent_dir = root.join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist after concurrent initialization"
    );

    // Database should be valid and accessible
    let db_path = intent_dir.join("project.db");
    assert!(
        db_path.exists(),
        "Database should exist after concurrent initialization"
    );

    // Verify database is accessible by trying to read from it
    // This ensures the concurrent operations didn't corrupt it
    let output = Command::new(binary_path)
        .current_dir(&subdir)
        .args(["task", "list", "todo"])
        .output()
        .expect("Failed to execute find command");

    assert!(
        output.status.success(),
        "Database should be accessible after concurrent operations"
    );
}

#[test]
fn test_initialization_with_symlinked_marker_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create actual Cargo.toml
    let real_cargo = root.join("real_Cargo.toml");
    fs::write(&real_cargo, "[package]\nname = \"test\"").expect("Failed to create real Cargo.toml");

    // Create symlink to Cargo.toml
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&real_cargo, root.join("Cargo.toml")).expect("Failed to create symlink");
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_file;
        symlink_file(&real_cargo, root.join("Cargo.toml")).expect("Failed to create symlink");
    }

    // Run command from subdirectory
    let subdir = root.join("src");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

    let binary_path = env!("CARGO_BIN_EXE_ie");
    let output = Command::new(binary_path)
        .current_dir(&subdir)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command should succeed with symlinked Cargo.toml"
    );

    let intent_dir = root.join(".intent-engine");
    assert!(
        intent_dir.exists(),
        ".intent-engine should exist at root with symlinked marker file"
    );
}

#[test]
fn test_partial_initialization_state_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create .git marker
    fs::create_dir(root.join(".git")).expect("Failed to create .git");

    // Create .intent-engine directory but NO database (corrupted/partial state)
    let intent_dir = root.join(".intent-engine");
    fs::create_dir_all(&intent_dir).expect("Failed to create .intent-engine");

    // Verify database doesn't exist yet
    let db_path = intent_dir.join("project.db");
    assert!(!db_path.exists(), "Database should not exist initially");

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Run command - behavior depends on SQLite
    let output = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute command");

    // SQLite will create the database file on connect, then run migrations
    // So this should actually succeed as SQLite handles the "missing DB" case
    if output.status.success() {
        // Database was created successfully
        assert!(
            db_path.exists(),
            "Database should be created by SQLite on connect"
        );
    } else {
        // Or command failed - which is also acceptable behavior
        // (means SQLite didn't auto-create, which is fine)
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Error should be about database, not about project not found
        assert!(
            !stderr.contains("NOT_A_PROJECT"),
            "Should not be a 'not a project' error"
        );
    }
}

#[test]
fn test_invalid_database_fails_appropriately() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create .git marker
    fs::create_dir(root.join(".git")).expect("Failed to create .git");

    // Create .intent-engine directory and INVALID database file
    let intent_dir = root.join(".intent-engine");
    fs::create_dir_all(&intent_dir).expect("Failed to create .intent-engine");

    let db_path = intent_dir.join("project.db");
    fs::write(&db_path, "This is not a valid SQLite database")
        .expect("Failed to create invalid db");

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Run command - should fail with database error
    let output = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute command");

    // Should fail (corrupted database)
    assert!(
        !output.status.success(),
        "Command should fail with corrupted database"
    );

    // Error should indicate database problem
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("DATABASE_ERROR")
            || stderr.contains("database")
            || stderr.contains("Database"),
        "Error should indicate database problem"
    );
}

/// Test for nested projects bug: Child project should NOT use parent's database
///
/// This test verifies a critical bug scenario:
/// - Parent project has .git and .intent_engine
/// - Child project has .git but NO .intent_engine
/// - When running from child project, it should create its OWN database
/// - It should NOT use the parent project's database
///
/// Expected behavior:
/// - Child project should detect its own .git marker
/// - Child project should initialize its own .intent_engine
/// - Tasks added in child should NOT appear in parent's database
#[test]
fn test_nested_projects_should_not_share_database() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Setup parent project with .git and .intent_engine
    fs::create_dir(root.join(".git")).expect("Failed to create parent .git");

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Initialize parent project
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to initialize parent");

    assert!(
        parent_init.status.success(),
        "Parent initialization failed: {:?}",
        String::from_utf8_lossy(&parent_init.stderr)
    );

    // Verify parent has .intent_engine
    let parent_intent = root.join(".intent-engine");
    assert!(parent_intent.exists(), "Parent should have .intent_engine");

    // Setup child project INSIDE parent with its own .git
    let child_dir = root.join("child-project");
    fs::create_dir_all(&child_dir).expect("Failed to create child dir");
    fs::create_dir(child_dir.join(".git")).expect("Failed to create child .git");

    // Initialize child project
    let child_init = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to initialize child");

    assert!(
        child_init.status.success(),
        "Child initialization failed: {:?}",
        String::from_utf8_lossy(&child_init.stderr)
    );

    // *** CRITICAL CHECKS ***

    // Child should have its OWN .intent_engine
    let child_intent = child_dir.join(".intent-engine");
    assert!(
        child_intent.exists(),
        "Child project should have its own .intent_engine directory"
    );

    // Child should have its own database
    let child_db = child_intent.join("project.db");
    assert!(
        child_db.exists(),
        "Child project should have its own database file"
    );

    // Parent should still have its database
    let parent_db = parent_intent.join("project.db");
    assert!(
        parent_db.exists(),
        "Parent project should still have its database"
    );

    // Verify databases are DIFFERENT files
    assert_ne!(
        parent_db.canonicalize().unwrap(),
        child_db.canonicalize().unwrap(),
        "Parent and child should have DIFFERENT database files"
    );

    // List tasks in parent - should only have "Parent task"
    let parent_list = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list parent tasks");

    assert!(parent_list.status.success(), "Parent list should succeed");
    let parent_output = String::from_utf8_lossy(&parent_list.stdout);
    assert!(
        parent_output.contains("Parent task"),
        "Parent should have 'Parent task'"
    );
    assert!(
        !parent_output.contains("Child task"),
        "Parent should NOT have 'Child task' (databases should be isolated)"
    );

    // List tasks in child - should only have "Child task"
    let child_list = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "list"])
        .output()
        .expect("Failed to list child tasks");

    assert!(child_list.status.success(), "Child list should succeed");
    let child_output = String::from_utf8_lossy(&child_list.stdout);
    assert!(
        child_output.contains("Child task"),
        "Child should have 'Child task'"
    );
    assert!(
        !child_output.contains("Parent task"),
        "Child should NOT have 'Parent task' (databases should be isolated)"
    );
}

/// Test for nested projects in subdirectory: should use child's database, not parent's
///
/// This tests a variant where we run from a subdirectory of the child project
#[test]
fn test_nested_projects_from_child_subdirectory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Setup parent project
    fs::create_dir(root.join(".git")).expect("Failed to create parent .git");

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Initialize parent
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to initialize parent");

    assert!(parent_init.status.success(), "Parent init failed");

    // Setup child project with its own .git
    let child_dir = root.join("child-project");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::create_dir(child_dir.join(".git")).expect("Failed to create child .git");

    // Create child subdirectory
    let child_subdir = child_dir.join("src/components");
    fs::create_dir_all(&child_subdir).expect("Failed to create child subdir");

    // Run from child subdirectory
    let child_init = Command::new(binary_path)
        .current_dir(&child_subdir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to run from child subdir");

    assert!(
        child_init.status.success(),
        "Child subdir init failed: {:?}",
        String::from_utf8_lossy(&child_init.stderr)
    );

    // .intent_engine should be in child project root (where child's .git is)
    let child_intent = child_dir.join(".intent-engine");
    assert!(
        child_intent.exists(),
        "Child should have .intent_engine at its root (where .git is)"
    );

    // Should NOT create .intent_engine in subdirectory
    let subdir_intent = child_subdir.join(".intent-engine");
    assert!(
        !subdir_intent.exists(),
        "Should not create .intent_engine in subdirectory"
    );

    // Verify task isolation
    let parent_list = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list parent");

    let parent_output = String::from_utf8_lossy(&parent_list.stdout);
    assert!(
        !parent_output.contains("Child task"),
        "Parent should NOT see child tasks"
    );
}

// ============================================================================
// COMPREHENSIVE NESTED PROJECT TEST MATRIX
// Based on: tests/nested_project_test_matrix.md
// ============================================================================

/// Test Matrix #2: Parent has .git+.intent, Child has NO markers
/// Expected: Child should use Parent's .intent (no boundary to separate them)
#[test]
fn test_matrix_2_parent_has_all_child_has_none() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Setup: Parent with .git and .intent
    fs::create_dir(root.join(".git")).expect("Failed to create .git");
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success(), "Parent init should succeed");

    // Child directory with NO markers
    let child_dir = root.join("child");
    fs::create_dir_all(&child_dir).expect("Failed to create child");

    // Run from child
    let child_run = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to run in child");
    assert!(child_run.status.success(), "Child run should succeed");

    // Child should NOT have its own .intent
    assert!(
        !child_dir.join(".intent-engine").exists(),
        "Child should not create .intent (no boundary)"
    );

    // Parent should have .intent
    assert!(
        root.join(".intent-engine").exists(),
        "Parent should have .intent"
    );

    // Both tasks should be in parent's database
    let list_output = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list");
    let output_str = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        output_str.contains("Parent task") && output_str.contains("Child task"),
        "Both tasks should be in parent's database"
    );
}

/// Test Matrix #3: Parent has .git but NO .intent, Child has .git but NO .intent
/// Expected: Child creates its own .intent (boundary at child's .git)
#[test]
fn test_matrix_3_both_have_git_no_intent() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Setup: Parent with .git only
    fs::create_dir(root.join(".git")).expect("Failed to create parent .git");

    // Child with its own .git
    let child_dir = root.join("child-project");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::create_dir(child_dir.join(".git")).expect("Failed to create child .git");

    // Run from child
    let child_init = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to init child");
    assert!(child_init.status.success(), "Child init should succeed");

    // Child should have its own .intent
    let child_intent = child_dir.join(".intent-engine");
    assert!(
        child_intent.exists(),
        "Child should create own .intent (has .git boundary)"
    );

    // Parent should NOT have .intent yet
    assert!(
        !root.join(".intent-engine").exists(),
        "Parent should not have .intent"
    );

    // Run from parent
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success(), "Parent init should succeed");

    // Now parent should have .intent
    let parent_intent = root.join(".intent-engine");
    assert!(parent_intent.exists(), "Parent should now have .intent");

    // Verify isolation
    let parent_db = parent_intent.join("project.db");
    let child_db = child_intent.join("project.db");
    assert_ne!(
        parent_db.canonicalize().unwrap(),
        child_db.canonicalize().unwrap(),
        "Databases should be different files"
    );
}

/// Test Matrix #4: Parent has NO .git but HAS .intent, Child has .git
/// Expected: Child creates its own .intent (boundary at child's .git)
#[test]
fn test_matrix_4_parent_intent_only_child_has_git() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Setup: Parent with .intent but no marker
    // Initialize parent first (will create .intent at CWD due to no markers)
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to init parent");
    assert!(
        parent_init.status.success(),
        "Parent init should succeed: {:?}",
        String::from_utf8_lossy(&parent_init.stderr)
    );

    // Parent should now have .intent (created at CWD fallback)
    let parent_intent = root.join(".intent-engine");
    assert!(
        parent_intent.exists(),
        "Parent should have .intent after initialization"
    );

    // Child with .git marker
    let child_dir = root.join("child-project");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::create_dir(child_dir.join(".git")).expect("Failed to create child .git");

    // Run from child
    let child_init = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to init child");
    assert!(child_init.status.success(), "Child init should succeed");

    // Child should create its own .intent (has .git boundary)
    let child_intent = child_dir.join(".intent-engine");
    assert!(
        child_intent.exists(),
        "Child should create own .intent (has .git boundary)"
    );

    // Verify isolation - child should NOT have parent's task
    let child_list = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "list"])
        .output()
        .expect("Failed to list child");
    let child_output = String::from_utf8_lossy(&child_list.stdout);
    assert!(
        !child_output.contains("Parent task"),
        "Child should not see parent tasks"
    );
}

/// Test Matrix #5: Parent has nothing, Child has .git
/// Expected: Child creates .intent at its .git boundary
#[test]
fn test_matrix_5_parent_empty_child_has_git() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Parent has nothing (no markers, no .intent)
    // Child has .git
    let child_dir = root.join("child-project");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::create_dir(child_dir.join(".git")).expect("Failed to create child .git");

    // Run from child
    let child_init = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to init child");
    assert!(child_init.status.success(), "Child init should succeed");

    // Child should have .intent
    assert!(
        child_dir.join(".intent-engine").exists(),
        "Child should create .intent at .git boundary"
    );

    // Parent should NOT have .intent
    assert!(
        !root.join(".intent-engine").exists(),
        "Parent should not have .intent"
    );
}

/// Test Matrix #6: Both parent and child already have .intent
/// Expected: Child uses its own .intent
#[test]
fn test_matrix_6_both_have_intent() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Setup parent with .git and .intent
    fs::create_dir(root.join(".git")).expect("Failed to create parent .git");
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success());

    // Setup child with .git and .intent
    let child_dir = root.join("child-project");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::create_dir(child_dir.join(".git")).expect("Failed to create child .git");
    let child_init = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to init child");
    assert!(child_init.status.success());

    // Verify both have .intent
    assert!(root.join(".intent-engine").exists());
    assert!(child_dir.join(".intent-engine").exists());

    // Add another task to child
    let child_add = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task 2"])
        .output()
        .expect("Failed to add to child");
    assert!(child_add.status.success());

    // Child should only see its own tasks
    let child_list = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "list"])
        .output()
        .expect("Failed to list child");
    let child_output = String::from_utf8_lossy(&child_list.stdout);
    assert!(child_output.contains("Child task"));
    assert!(child_output.contains("Child task 2"));
    assert!(!child_output.contains("Parent task"));

    // Parent should only see its own task
    let parent_list = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list parent");
    let parent_output = String::from_utf8_lossy(&parent_list.stdout);
    assert!(parent_output.contains("Parent task"));
    assert!(!parent_output.contains("Child task"));
}

/// Test Matrix #7: Parent has .intent but no marker, Child has no marker
/// Expected: Child uses parent's .intent (no boundary)
#[test]
fn test_matrix_7_parent_intent_only_child_nothing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Parent: manually create .intent (unusual case)
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success());

    // Child: no markers
    let child_dir = root.join("subdir");
    fs::create_dir_all(&child_dir).expect("Failed to create child");

    // Run from child
    let child_add = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to add in child");
    assert!(child_add.status.success());

    // Child should NOT create .intent
    assert!(
        !child_dir.join(".intent-engine").exists(),
        "Child should not create .intent"
    );

    // Both tasks should be in parent's database
    let list_output = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list");
    let output_str = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        output_str.contains("Parent task") && output_str.contains("Child task"),
        "Both tasks should be in parent's database"
    );
}

/// Test Matrix #8: Neither parent nor child have markers
/// Expected: Creates .intent at CWD with fallback warning
#[test]
fn test_matrix_8_both_have_nothing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // No markers anywhere
    let child_dir = root.join("some/deep/path");
    fs::create_dir_all(&child_dir).expect("Failed to create deep path");

    // Run from deep path
    let output = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Task"])
        .output()
        .expect("Failed to run");
    assert!(output.status.success());

    // Should create .intent at the run location (CWD fallback)
    let intent_at_deep = child_dir.join(".intent-engine");
    assert!(
        intent_at_deep.exists(),
        ".intent should be created at CWD (fallback)"
    );

    // Should print warning about no markers
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Warning") || stderr.contains("warning"),
        "Should warn about no project markers"
    );
}

// ============================================================================
// MULTI-LEVEL NESTING TESTS (3+ levels deep)
// ============================================================================

/// Test N1: Grandparent(.git+.intent) -> Parent(.git) -> Child(.git)
/// Expected: Child creates own .intent, Parent creates own .intent
#[test]
fn test_multi_level_n1_all_have_git() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Grandparent: .git + .intent
    fs::create_dir(root.join(".git")).expect("Failed to create GP .git");
    let gp_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "GP task"])
        .output()
        .expect("Failed to init GP");
    assert!(gp_init.status.success());

    // Parent: .git only
    let parent_dir = root.join("parent");
    fs::create_dir_all(&parent_dir).expect("Failed to create parent");
    fs::create_dir(parent_dir.join(".git")).expect("Failed to create parent .git");

    // Child: .git only
    let child_dir = parent_dir.join("child");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::create_dir(child_dir.join(".git")).expect("Failed to create child .git");

    // Initialize child
    let child_init = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to init child");
    assert!(child_init.status.success());

    // Child should have own .intent (boundary at child's .git)
    assert!(
        child_dir.join(".intent-engine").exists(),
        "Child should have own .intent"
    );

    // Initialize parent
    let parent_init = Command::new(binary_path)
        .current_dir(&parent_dir)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success());

    // Parent should have own .intent (boundary at parent's .git)
    assert!(
        parent_dir.join(".intent-engine").exists(),
        "Parent should have own .intent"
    );

    // All three should have separate databases
    assert!(root.join(".intent-engine").exists());
    assert!(parent_dir.join(".intent-engine").exists());
    assert!(child_dir.join(".intent-engine").exists());

    // Verify isolation - each sees only their own task
    let gp_list = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list GP");
    let gp_output = String::from_utf8_lossy(&gp_list.stdout);
    assert!(gp_output.contains("GP task"));
    assert!(!gp_output.contains("Parent task"));
    assert!(!gp_output.contains("Child task"));
}

/// Test N2: Grandparent(.git+.intent) -> Parent(nothing) -> Child(.git)
/// Expected: Child creates own .intent, Parent uses GP's .intent
#[test]
fn test_multi_level_n2_skip_middle_generation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Grandparent: .git + .intent
    fs::create_dir(root.join(".git")).expect("Failed to create GP .git");
    let gp_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "GP task"])
        .output()
        .expect("Failed to init GP");
    assert!(gp_init.status.success());

    // Parent: nothing
    let parent_dir = root.join("parent");
    fs::create_dir_all(&parent_dir).expect("Failed to create parent");

    // Child: .git
    let child_dir = parent_dir.join("child");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::create_dir(child_dir.join(".git")).expect("Failed to create child .git");

    // Add task from parent (should use GP's .intent)
    let parent_add = Command::new(binary_path)
        .current_dir(&parent_dir)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to add parent task");
    assert!(parent_add.status.success());

    // Parent should NOT have .intent (uses GP's)
    assert!(
        !parent_dir.join(".intent-engine").exists(),
        "Parent should not have .intent (no boundary)"
    );

    // Add task from child
    let child_add = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to add child task");
    assert!(child_add.status.success());

    // Child should have own .intent (has .git boundary)
    assert!(
        child_dir.join(".intent-engine").exists(),
        "Child should have own .intent"
    );

    // GP should see GP + Parent tasks (same database)
    let gp_list = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list GP");
    let gp_output = String::from_utf8_lossy(&gp_list.stdout);
    assert!(gp_output.contains("GP task"));
    assert!(gp_output.contains("Parent task"));
    assert!(!gp_output.contains("Child task"));

    // Child should only see own task
    let child_list = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "list"])
        .output()
        .expect("Failed to list child");
    let child_output = String::from_utf8_lossy(&child_list.stdout);
    assert!(child_output.contains("Child task"));
    assert!(!child_output.contains("GP task"));
    assert!(!child_output.contains("Parent task"));
}

/// Test N3: Different marker types across levels
/// GP(.git) -> Parent(package.json) -> Child(Cargo.toml)
#[test]
fn test_multi_level_n3_different_markers() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Grandparent: .git + .intent
    fs::create_dir(root.join(".git")).expect("Failed to create .git");
    let gp_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "GP task"])
        .output()
        .expect("Failed to init GP");
    assert!(gp_init.status.success());

    // Parent: package.json (Node.js project)
    let parent_dir = root.join("nodejs-service");
    fs::create_dir_all(&parent_dir).expect("Failed to create parent");
    fs::write(parent_dir.join("package.json"), r#"{"name": "service"}"#)
        .expect("Failed to create package.json");

    // Child: Cargo.toml (Rust project)
    let child_dir = parent_dir.join("rust-lib");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::write(child_dir.join("Cargo.toml"), "[package]\nname = \"lib\"")
        .expect("Failed to create Cargo.toml");

    // Initialize parent
    let parent_init = Command::new(binary_path)
        .current_dir(&parent_dir)
        .args(["task", "add", "--name", "Node task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success());

    // Parent should have own .intent (package.json boundary)
    assert!(
        parent_dir.join(".intent-engine").exists(),
        "Parent should have own .intent"
    );

    // Initialize child
    let child_init = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Rust task"])
        .output()
        .expect("Failed to init child");
    assert!(child_init.status.success());

    // Child should have own .intent (Cargo.toml boundary)
    assert!(
        child_dir.join(".intent-engine").exists(),
        "Child should have own .intent"
    );

    // All three should be isolated
    assert!(root.join(".intent-engine").exists());
    assert!(parent_dir.join(".intent-engine").exists());
    assert!(child_dir.join(".intent-engine").exists());
}

/// Test N4: Deep nesting with no middle boundaries
/// GP(.git+.intent) -> Parent(nothing) -> Child(nothing)
/// Expected: Both parent and child use GP's .intent
#[test]
fn test_multi_level_n4_no_middle_boundaries() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Grandparent: .git + .intent
    fs::create_dir(root.join(".git")).expect("Failed to create .git");
    let gp_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "GP task"])
        .output()
        .expect("Failed to init GP");
    assert!(gp_init.status.success());

    // Parent: nothing
    let parent_dir = root.join("src");
    fs::create_dir_all(&parent_dir).expect("Failed to create parent");

    // Child: nothing
    let child_dir = parent_dir.join("components");
    fs::create_dir_all(&child_dir).expect("Failed to create child");

    // Add tasks from different levels
    let parent_add = Command::new(binary_path)
        .current_dir(&parent_dir)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to add parent");
    assert!(parent_add.status.success());

    let child_add = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to add child");
    assert!(child_add.status.success());

    // Neither parent nor child should have .intent
    assert!(
        !parent_dir.join(".intent-engine").exists(),
        "Parent should not have .intent"
    );
    assert!(
        !child_dir.join(".intent-engine").exists(),
        "Child should not have .intent"
    );

    // All tasks should be in GP's database
    let gp_list = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list");
    let gp_output = String::from_utf8_lossy(&gp_list.stdout);
    assert!(gp_output.contains("GP task"));
    assert!(gp_output.contains("Parent task"));
    assert!(gp_output.contains("Child task"));
}

// ============================================================================
// SIBLING PROJECT ISOLATION TESTS
// ============================================================================

/// Test S1: Two sibling projects should have separate databases
/// Parent(.git+.intent) -> Sibling1(.git) + Sibling2(.git)
#[test]
fn test_siblings_s1_both_have_git() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Parent with .git + .intent
    fs::create_dir(root.join(".git")).expect("Failed to create parent .git");
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success());

    // Sibling 1 with .git
    let sib1_dir = root.join("project-a");
    fs::create_dir_all(&sib1_dir).expect("Failed to create sib1");
    fs::create_dir(sib1_dir.join(".git")).expect("Failed to create sib1 .git");

    // Sibling 2 with .git
    let sib2_dir = root.join("project-b");
    fs::create_dir_all(&sib2_dir).expect("Failed to create sib2");
    fs::create_dir(sib2_dir.join(".git")).expect("Failed to create sib2 .git");

    // Initialize sibling 1
    let sib1_init = Command::new(binary_path)
        .current_dir(&sib1_dir)
        .args(["task", "add", "--name", "Project A task"])
        .output()
        .expect("Failed to init sib1");
    assert!(sib1_init.status.success());

    // Initialize sibling 2
    let sib2_init = Command::new(binary_path)
        .current_dir(&sib2_dir)
        .args(["task", "add", "--name", "Project B task"])
        .output()
        .expect("Failed to init sib2");
    assert!(sib2_init.status.success());

    // Each sibling should have own .intent
    assert!(
        sib1_dir.join(".intent-engine").exists(),
        "Sibling 1 should have own .intent"
    );
    assert!(
        sib2_dir.join(".intent-engine").exists(),
        "Sibling 2 should have own .intent"
    );

    // Verify isolation - sib1 should only see its task
    let sib1_list = Command::new(binary_path)
        .current_dir(&sib1_dir)
        .args(["task", "list"])
        .output()
        .expect("Failed to list sib1");
    let sib1_output = String::from_utf8_lossy(&sib1_list.stdout);
    assert!(sib1_output.contains("Project A task"));
    assert!(!sib1_output.contains("Project B task"));
    assert!(!sib1_output.contains("Parent task"));

    // Sib2 should only see its task
    let sib2_list = Command::new(binary_path)
        .current_dir(&sib2_dir)
        .args(["task", "list"])
        .output()
        .expect("Failed to list sib2");
    let sib2_output = String::from_utf8_lossy(&sib2_list.stdout);
    assert!(sib2_output.contains("Project B task"));
    assert!(!sib2_output.contains("Project A task"));
    assert!(!sib2_output.contains("Parent task"));

    // Parent should only see its task
    let parent_list = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list parent");
    let parent_output = String::from_utf8_lossy(&parent_list.stdout);
    assert!(parent_output.contains("Parent task"));
    assert!(!parent_output.contains("Project A task"));
    assert!(!parent_output.contains("Project B task"));
}

/// Test S2: Three siblings with mixed markers
/// Parent(.git) -> Sib1(.git) + Sib2(package.json) + Sib3(nothing)
#[test]
fn test_siblings_s2_mixed_markers() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Parent with .git + .intent
    fs::create_dir(root.join(".git")).expect("Failed to create parent .git");
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success());

    // Sibling 1: .git
    let sib1_dir = root.join("rust-service");
    fs::create_dir_all(&sib1_dir).expect("Failed to create sib1");
    fs::create_dir(sib1_dir.join(".git")).expect("Failed to create sib1 .git");

    // Sibling 2: package.json
    let sib2_dir = root.join("node-service");
    fs::create_dir_all(&sib2_dir).expect("Failed to create sib2");
    fs::write(sib2_dir.join("package.json"), r#"{"name": "service"}"#)
        .expect("Failed to create package.json");

    // Sibling 3: nothing
    let sib3_dir = root.join("docs");
    fs::create_dir_all(&sib3_dir).expect("Failed to create sib3");

    // Initialize all siblings
    let sib1_init = Command::new(binary_path)
        .current_dir(&sib1_dir)
        .args(["task", "add", "--name", "Rust task"])
        .output()
        .expect("Failed to init sib1");
    assert!(sib1_init.status.success());

    let sib2_init = Command::new(binary_path)
        .current_dir(&sib2_dir)
        .args(["task", "add", "--name", "Node task"])
        .output()
        .expect("Failed to init sib2");
    assert!(sib2_init.status.success());

    let sib3_init = Command::new(binary_path)
        .current_dir(&sib3_dir)
        .args(["task", "add", "--name", "Docs task"])
        .output()
        .expect("Failed to init sib3");
    assert!(sib3_init.status.success());

    // Sib1 and Sib2 should have own .intent (have boundaries)
    assert!(
        sib1_dir.join(".intent-engine").exists(),
        "Sib1 should have own .intent"
    );
    assert!(
        sib2_dir.join(".intent-engine").exists(),
        "Sib2 should have own .intent"
    );

    // Sib3 should NOT have .intent (no boundary, uses parent's)
    assert!(
        !sib3_dir.join(".intent-engine").exists(),
        "Sib3 should not have .intent"
    );

    // Verify: Sib3's task should be in parent's database
    let parent_list = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list parent");
    let parent_output = String::from_utf8_lossy(&parent_list.stdout);
    assert!(parent_output.contains("Parent task"));
    assert!(parent_output.contains("Docs task"));
    assert!(!parent_output.contains("Rust task"));
    assert!(!parent_output.contains("Node task"));
}

// ============================================================================
// EDGE CASES AND SPECIAL SCENARIOS
// ============================================================================

/// Test E1: Run from very deep subdirectory (5+ levels)
/// Should find correct project boundary
#[test]
fn test_edge_e1_very_deep_subdirectory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Create .git at root
    fs::create_dir(root.join(".git")).expect("Failed to create .git");

    // Create very deep path
    let deep_path = root.join("src/components/ui/buttons/primary/variants");
    fs::create_dir_all(&deep_path).expect("Failed to create deep path");

    // Initialize from deep path
    let init = Command::new(binary_path)
        .current_dir(&deep_path)
        .args(["task", "add", "--name", "Deep task"])
        .output()
        .expect("Failed to init deep");
    assert!(init.status.success());

    // .intent should be at root (where .git is), not in deep path
    assert!(
        root.join(".intent-engine").exists(),
        ".intent should be at project root"
    );
    assert!(
        !deep_path.join(".intent-engine").exists(),
        ".intent should NOT be in deep path"
    );

    // Verify from root
    let list = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "list"])
        .output()
        .expect("Failed to list");
    let output = String::from_utf8_lossy(&list.stdout);
    assert!(output.contains("Deep task"));
}

/// Test E3: Parent with .intent but no marker, Child with marker
/// Child should create own .intent (not use parent's orphaned one)
#[test]
fn test_edge_e3_orphaned_parent_intent() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Parent: Initialize without marker (creates .intent at CWD)
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Orphan task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success());

    // Now parent has .intent but no marker
    assert!(root.join(".intent-engine").exists());

    // Child: has .git marker
    let child_dir = root.join("child-project");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::create_dir(child_dir.join(".git")).expect("Failed to create .git");

    // Initialize child
    let child_init = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to init child");
    assert!(child_init.status.success());

    // Child should create own .intent (has .git boundary)
    assert!(
        child_dir.join(".intent-engine").exists(),
        "Child should create own .intent despite parent having one"
    );

    // Verify isolation
    let child_list = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "list"])
        .output()
        .expect("Failed to list child");
    let child_output = String::from_utf8_lossy(&child_list.stdout);
    assert!(child_output.contains("Child task"));
    assert!(!child_output.contains("Orphan task"));
}

/// Test E4: Multiple markers in same directory - uses highest priority
/// Directory has both .git and package.json
#[test]
fn test_edge_e4_multiple_markers_same_dir() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let binary_path = env!("CARGO_BIN_EXE_ie");

    // Create both .git (priority 1) and package.json (priority 3)
    fs::create_dir(root.join(".git")).expect("Failed to create .git");
    fs::write(root.join("package.json"), r#"{"name": "test"}"#)
        .expect("Failed to create package.json");

    // Create child with Cargo.toml
    let child_dir = root.join("rust-subproject");
    fs::create_dir_all(&child_dir).expect("Failed to create child");
    fs::write(child_dir.join("Cargo.toml"), "[package]\nname = \"sub\"")
        .expect("Failed to create Cargo.toml");

    // Initialize parent
    let parent_init = Command::new(binary_path)
        .current_dir(root)
        .args(["task", "add", "--name", "Parent task"])
        .output()
        .expect("Failed to init parent");
    assert!(parent_init.status.success());

    // Initialize child
    let child_init = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "add", "--name", "Child task"])
        .output()
        .expect("Failed to init child");
    assert!(child_init.status.success());

    // Child should have own .intent (Cargo.toml is a boundary)
    assert!(
        child_dir.join(".intent-engine").exists(),
        "Child should have own .intent"
    );

    // Both should be isolated
    let child_list = Command::new(binary_path)
        .current_dir(&child_dir)
        .args(["task", "list"])
        .output()
        .expect("Failed to list child");
    let child_output = String::from_utf8_lossy(&child_list.stdout);
    assert!(child_output.contains("Child task"));
    assert!(!child_output.contains("Parent task"));
}
