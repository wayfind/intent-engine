/// Comprehensive tests for main.rs to improve code coverage
/// Focuses on error paths and edge cases that are difficult to trigger in normal usage
use assert_cmd::{cargo, Command};
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
    TempDir::new().unwrap()
}

// ============================================================================
// Session Restore Tests
// ============================================================================

#[test]
fn test_session_restore_without_workspace() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("session-restore")
        .arg("--include-events")
        .arg("5");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No Intent-Engine workspace found"));
}

#[test]
fn test_session_restore_with_workspace_path() {
    let temp_dir = setup_test_env();

    // Initialize workspace
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Try session restore with explicit workspace path
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.arg("session-restore")
        .arg("--include-events")
        .arg("3")
        .arg("--workspace")
        .arg(temp_dir.path());

    cmd.assert().success();
}

#[test]
fn test_session_restore_with_nonexistent_workspace_path() {
    let temp_dir = setup_test_env();
    let nonexistent = temp_dir.path().join("nonexistent");

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.arg("session-restore")
        .arg("--workspace")
        .arg(&nonexistent);

    // Should fail to change directory
    cmd.assert().failure();
}

// ============================================================================
// Event Command Error Path Tests
// ============================================================================

#[test]
fn test_event_add_without_data_stdin_flag() {
    let temp_dir = setup_test_env();

    // Initialize and create a task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Try to add event without --data-stdin
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--log-type")
        .arg("note");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail with InvalidInput error
    assert!(
        stderr.contains("--data-stdin is required") || !output.status.success(),
        "Expected error about missing --data-stdin, got: {}",
        stderr
    );
}

#[test]
fn test_event_add_without_current_task_and_without_task_id() {
    let temp_dir = setup_test_env();

    // Initialize workspace but don't set current task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Try to add event without task_id and without current task
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--log-type")
        .arg("note")
        .arg("--data-stdin")
        .write_stdin("test event");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail with error about no current task
    assert!(
        stderr.contains("No current task is set") || !output.status.success(),
        "Expected error about no current task, got: {}",
        stderr
    );
}

// ============================================================================
// Setup Claude Code Tests
// ============================================================================

#[test]
fn test_setup_claude_code_dry_run() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("setup-claude-code")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Would create:"));
}

#[test]
fn test_setup_claude_code_creates_hook() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path()).arg("setup-claude-code");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Setup complete"));

    // Verify hook was created
    let hook_path = temp_dir.path().join(".claude/hooks/session-start.sh");
    assert!(hook_path.exists(), "Hook file should be created");

    // Verify permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&hook_path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(
            mode & 0o111,
            0o111,
            "Hook should be executable: mode={:o}",
            mode
        );
    }
}

#[test]
fn test_setup_claude_code_refuses_to_overwrite_without_force() {
    let temp_dir = setup_test_env();

    // Create hook first
    let hooks_dir = temp_dir.path().join(".claude/hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    let hook_path = hooks_dir.join("session-start.sh");
    fs::write(&hook_path, "existing content").unwrap();

    // Try to setup without --force
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path()).arg("setup-claude-code");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "Should fail when hook exists without --force"
    );
    assert!(
        stderr.contains("already exists") || stderr.contains("--force"),
        "Expected error about existing file, got: {}",
        stderr
    );
}

#[test]
fn test_setup_claude_code_with_force_overwrites() {
    let temp_dir = setup_test_env();

    // Create existing hook
    let hooks_dir = temp_dir.path().join(".claude/hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    let hook_path = hooks_dir.join("session-start.sh");
    fs::write(&hook_path, "old content").unwrap();

    // Setup with --force
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("setup-claude-code")
        .arg("--force");

    cmd.assert().success();

    // Verify content was overwritten
    let content = fs::read_to_string(&hook_path).unwrap();
    assert_ne!(content, "old content", "Content should be updated");
}

#[test]
fn test_setup_claude_code_with_custom_claude_dir() {
    let temp_dir = setup_test_env();
    let custom_dir = temp_dir.path().join("custom-claude");

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("setup-claude-code")
        .arg("--claude-dir")
        .arg(&custom_dir);

    cmd.assert().success();

    let hook_path = custom_dir.join("hooks/session-start.sh");
    assert!(hook_path.exists(), "Hook should be in custom directory");
}

// ============================================================================
// Setup MCP Tests
// ============================================================================

#[test]
fn test_setup_mcp_dry_run() {
    let temp_dir = setup_test_env();
    let config_file = temp_dir.path().join("test-config.json");

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("setup-mcp")
        .arg("--dry-run")
        .arg("--config-path")
        .arg(&config_file);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[DRY RUN]"));
}

#[test]
fn test_setup_mcp_creates_config() {
    let temp_dir = setup_test_env();
    let config_file = temp_dir.path().join("test-config.json");

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("setup-mcp")
        .arg("--config-path")
        .arg(&config_file);

    cmd.assert().success();

    // Verify config was created
    assert!(config_file.exists(), "Config file should be created");

    let content = fs::read_to_string(&config_file).unwrap();
    let config: Value = serde_json::from_str(&content).unwrap();

    assert!(
        config["mcpServers"]["intent-engine"].is_object(),
        "Config should contain intent-engine MCP server"
    );
}

#[test]
fn test_setup_mcp_refuses_to_overwrite_without_force() {
    let temp_dir = setup_test_env();
    let config_file = temp_dir.path().join("test-config.json");

    // Create existing config
    let existing_config = serde_json::json!({
        "mcpServers": {
            "intent-engine": {
                "command": "old-command"
            }
        }
    });
    fs::write(
        &config_file,
        serde_json::to_string_pretty(&existing_config).unwrap(),
    )
    .unwrap();

    // Try to setup without --force
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("setup-mcp")
        .arg("--config-path")
        .arg(&config_file);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("already configured"));
}

#[test]
fn test_setup_mcp_with_force_overwrites() {
    let temp_dir = setup_test_env();
    let config_file = temp_dir.path().join("test-config.json");

    // Create existing config
    let existing_config = serde_json::json!({
        "mcpServers": {
            "intent-engine": {
                "command": "old-command"
            }
        }
    });
    fs::write(
        &config_file,
        serde_json::to_string_pretty(&existing_config).unwrap(),
    )
    .unwrap();

    // Setup with --force
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("setup-mcp")
        .arg("--config-path")
        .arg(&config_file)
        .arg("--force");

    cmd.assert().success();

    // Verify config was updated
    let content = fs::read_to_string(&config_file).unwrap();
    let config: Value = serde_json::from_str(&content).unwrap();
    let command = config["mcpServers"]["intent-engine"]["command"]
        .as_str()
        .unwrap();
    assert_ne!(command, "old-command", "Command should be updated");
}

#[test]
fn test_setup_mcp_creates_backup() {
    let temp_dir = setup_test_env();
    let config_file = temp_dir.path().join("test-config.json");

    // Create existing config
    let existing_config = serde_json::json!({"test": "data"});
    fs::write(
        &config_file,
        serde_json::to_string_pretty(&existing_config).unwrap(),
    )
    .unwrap();

    // Setup with --force to trigger backup
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("setup-mcp")
        .arg("--config-path")
        .arg(&config_file)
        .arg("--force");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Backup created:"));

    // Verify backup exists
    let backup_files: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("test-config.json.backup.")
        })
        .collect();

    assert!(!backup_files.is_empty(), "Backup file should be created");
}

#[test]
fn test_setup_mcp_with_different_targets() {
    let temp_dir = setup_test_env();

    // Test claude-code target
    let config_claude_code = temp_dir.path().join("claude-code.json");
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("setup-mcp")
        .arg("--config-path")
        .arg(&config_claude_code)
        .arg("--target")
        .arg("claude-code")
        .assert()
        .success();

    // Test claude-desktop target
    let config_claude_desktop = temp_dir.path().join("claude-desktop.json");
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("setup-mcp")
        .arg("--config-path")
        .arg(&config_claude_desktop)
        .arg("--target")
        .arg("claude-desktop")
        .assert()
        .success();
}

// ============================================================================
// Doctor Command Error Paths
// ============================================================================

#[test]
fn test_doctor_in_fresh_environment() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path()).arg("doctor");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("System Information"))
        .stdout(predicate::str::contains("SQLite"))
        .stdout(predicate::str::contains("Database Connection"))
        .stdout(predicate::str::contains("Intent Engine Version"));
}

// ============================================================================
// Task Command Edge Cases
// ============================================================================

#[test]
fn test_task_update_with_priority() {
    let temp_dir = setup_test_env();

    // Add a task
    let output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .output()
        .unwrap();

    assert!(output.status.success());

    // Update with priority
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--priority")
        .arg("high");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"priority\": 2"));
}

#[test]
fn test_task_delete() {
    let temp_dir = setup_test_env();

    // Add a task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Delete the task
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("del")
        .arg("1");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("deleted"));
}

#[test]
fn test_task_list_with_parent_filter() {
    let temp_dir = setup_test_env();

    // Add parent task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent task")
        .assert()
        .success();

    // Add child task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Child task")
        .arg("--parent")
        .arg("1")
        .assert()
        .success();

    // List with parent filter
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("list")
        .arg("--parent")
        .arg("1");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Child task"));
}

#[test]
fn test_task_list_with_null_parent() {
    let temp_dir = setup_test_env();

    // Add parent task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent task")
        .assert()
        .success();

    // Add child task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Child task")
        .arg("--parent")
        .arg("1")
        .assert()
        .success();

    // List with null parent filter (only top-level tasks)
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("list")
        .arg("--parent")
        .arg("null");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Parent task"));
    assert!(!stdout.contains("Child task"));
}

#[test]
fn test_task_pick_next_text_format() {
    let temp_dir = setup_test_env();

    // Add a task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Pick next with text format
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("text");

    cmd.assert().success();
}

#[test]
fn test_task_pick_next_json_format() {
    let temp_dir = setup_test_env();

    // Add a task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Pick next with json format
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json");

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The JSON response uses "task" not "recommended_task"
    assert!(stdout.contains("\"task\""));
}

// ============================================================================
// Current Command Tests
// ============================================================================

#[test]
fn test_current_get_when_no_current_task() {
    let temp_dir = setup_test_env();

    // Initialize workspace
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Get current task (should be null)
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path()).arg("current");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": null"));
}

#[test]
fn test_current_set_and_get() {
    let temp_dir = setup_test_env();

    // Add a task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Set current task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("current")
        .arg("--set")
        .arg("1")
        .assert()
        .success();

    // Get current task
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path()).arg("current");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": 1"));
}

// ============================================================================
// Report Command Tests
// ============================================================================

#[test]
fn test_report_with_filters() {
    let temp_dir = setup_test_env();

    // Add tasks
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Generate report with status filter
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("report")
        .arg("--status")
        .arg("todo");

    cmd.assert().success();
}

#[test]
fn test_report_summary_only() {
    let temp_dir = setup_test_env();

    // Add tasks
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Generate summary-only report
    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("report")
        .arg("--summary-only");

    cmd.assert().success();
}
