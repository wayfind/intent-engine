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
    let binary_path = env!("CARGO_BIN_EXE_intent-engine");

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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");

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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");
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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");
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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");
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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");

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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");

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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");

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
        .args(["task", "find", "--status", "todo"])
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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");
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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");

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

    let binary_path = env!("CARGO_BIN_EXE_intent-engine");

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
