//! Tests for Windows console encoding support
//!
//! These tests verify that Chinese and other non-ASCII characters
//! are correctly handled in Windows cmd and PowerShell environments.

#![allow(deprecated)]

use assert_cmd::cargo;
use assert_cmd::Command;
use tempfile::TempDir;

/// Helper to setup test environment with proper UTF-8
fn setup_test_env() -> TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn test_chinese_task_name() {
    let temp_dir = setup_test_env();

    let output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "add", "--name", "测试任务"])
        .output()
        .unwrap();

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("测试任务"),
        "Output should contain Chinese task name. Got: {}",
        stdout
    );
}

#[test]
fn test_chinese_task_name_with_spec() {
    let temp_dir = setup_test_env();

    let output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "add", "--name", "用户认证", "--spec-stdin"])
        .write_stdin("使用 JWT 实现用户认证，支持刷新令牌")
        .output()
        .unwrap();

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("用户认证"),
        "Output should contain task name: {}",
        stdout
    );
    assert!(
        stdout.contains("使用 JWT 实现用户认证"),
        "Output should contain spec: {}",
        stdout
    );
}

#[test]
fn test_chinese_event_data() {
    let temp_dir = setup_test_env();

    // First create a task
    let task_output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "add", "--name", "测试任务"])
        .output()
        .unwrap();
    assert!(task_output.status.success());

    // Extract task ID (assuming it's 1 for the first task)
    let task_id = 1;

    // Set it as current
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["current", "--set", &task_id.to_string()])
        .output()
        .unwrap();

    // Add an event with Chinese data
    let output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["event", "add", "--type", "decision", "--data-stdin"])
        .write_stdin("选择使用 HS256 算法因为不需要密钥轮换")
        .output()
        .unwrap();

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("选择使用 HS256 算法"),
        "Output should contain event data: {}",
        stdout
    );
}

#[test]
fn test_mixed_languages() {
    let temp_dir = setup_test_env();

    let output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "add", "--name", "Implement 用户认证 with JWT"])
        .output()
        .unwrap();

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Implement 用户认证 with JWT"),
        "Output should contain mixed language text: {}",
        stdout
    );
}

#[test]
fn test_special_chinese_characters() {
    let temp_dir = setup_test_env();

    // Test various special Chinese characters
    let special_chars = vec![
        "测试：标点符号",
        "测试「书名号」",
        "测试【方括号】",
        "测试（圆括号）",
        "测试——破折号",
        "测试…省略号",
    ];

    for test_case in special_chars {
        let output = Command::new(cargo::cargo_bin!("ie"))
            .current_dir(temp_dir.path())
            .args(["task", "add", "--name", test_case])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "Command should succeed for: {}",
            test_case
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(test_case),
            "Output should contain special characters: {}. Got: {}",
            test_case,
            stdout
        );
    }
}

#[test]
fn test_emoji_support() {
    let temp_dir = setup_test_env();

    let output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "add", "--name", "测试任务 🎯 完成目标"])
        .output()
        .unwrap();

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("🎯"),
        "Output should contain emoji: {}",
        stdout
    );
}

#[test]
fn test_search_chinese_content() {
    let temp_dir = setup_test_env();

    // Create tasks with Chinese content
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "add", "--name", "实现用户认证"])
        .output()
        .unwrap();

    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "add", "--name", "实现数据库迁移"])
        .output()
        .unwrap();

    // Search for Chinese keywords - verify it doesn't crash with Chinese input
    let output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "search", "用户"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Search should succeed with Chinese query"
    );

    // Verify output is valid JSON (even if empty)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with('[') && stdout.trim().ends_with(']'),
        "Search should return valid JSON array: {}",
        stdout
    );
}

#[test]
fn test_report_with_chinese_tasks() {
    let temp_dir = setup_test_env();

    // Create and complete a task with Chinese name
    let task_output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "add", "--name", "完成中文任务"])
        .output()
        .unwrap();
    assert!(task_output.status.success());

    // Start the task
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "start", "1"])
        .output()
        .unwrap();

    // Complete it
    Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["task", "done"])
        .output()
        .unwrap();

    // Generate full report (without --summary-only to get task details)
    let output = Command::new(cargo::cargo_bin!("ie"))
        .current_dir(temp_dir.path())
        .args(["report"])
        .output()
        .unwrap();

    assert!(output.status.success(), "Report generation should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("完成中文任务"),
        "Report should contain Chinese task name: {}",
        stdout
    );
}

#[cfg(windows)]
#[test]
fn test_console_setup() {
    use intent_engine::windows_console::{
        code_page_name, get_console_code_page, is_console_utf8, setup_windows_console,
    };

    // Setup should succeed
    assert!(
        setup_windows_console().is_ok(),
        "Console setup should succeed"
    );

    // After setup, console should be UTF-8
    let cp = get_console_code_page();
    assert_eq!(cp, 65001, "Console should be set to UTF-8 (65001)");

    // Check UTF-8 detection
    assert!(is_console_utf8(), "Console should be detected as UTF-8");

    // Check code page name
    assert_eq!(code_page_name(65001), "UTF-8");
    assert_eq!(code_page_name(936), "GBK (Simplified Chinese)");
}
