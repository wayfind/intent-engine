//! Logging System Rotation and Cleanup Tests
//!
//! Tests for Phase 2: Log rotation and cleanup functionality
//! - Daily rotation mechanism
//! - Old log file cleanup
//! - Retention period handling
//! - Edge cases (empty directory, non-existent directory)

mod common;

use serial_test::serial;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime};

/// Get log directory path
fn get_log_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("logs")
}

/// Get dashboard log path
#[allow(dead_code)]
fn dashboard_log_path() -> PathBuf {
    get_log_dir().join("dashboard.log")
}

/// Stop any running dashboard instance
fn stop_dashboard() {
    Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("stop")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok();
    thread::sleep(Duration::from_millis(500));
}

/// Create a fake old log file with specified age (days)
fn create_old_log_file(name: &str, age_days: u64) -> PathBuf {
    let log_dir = get_log_dir();
    fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    let file_path = log_dir.join(name);
    let mut file = File::create(&file_path).expect("Failed to create log file");
    writeln!(file, "Test log content").expect("Failed to write to log file");

    // Set modification time to simulate old file
    let mtime = SystemTime::now() - Duration::from_secs(age_days * 24 * 60 * 60);
    filetime::set_file_mtime(&file_path, filetime::FileTime::from_system_time(mtime))
        .expect("Failed to set mtime");

    file_path
}

#[test]
#[serial]
fn test_daily_rotation_creates_dated_files() {
    // Setup
    stop_dashboard();

    let log_dir = get_log_dir();
    if log_dir.exists() {
        fs::remove_dir_all(&log_dir).ok();
    }

    // Start dashboard in daemon mode
    let status = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Failed to start dashboard");

    assert!(status.success(), "Dashboard should start successfully");
    thread::sleep(Duration::from_secs(3));

    // The daily rotation creates files with date suffix directly
    // e.g., dashboard.log.2025-11-23 (no intermediate dashboard.log)
    // We verify the mechanism is in place by checking dated files exist

    let log_dir_path = get_log_dir();
    let log_files: Vec<_> = fs::read_dir(&log_dir_path)
        .expect("Failed to read log directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("dashboard.log.")
        })
        .collect();

    assert!(
        !log_files.is_empty(),
        "At least one dated dashboard log file should be created"
    );

    // Verify the log file can be read (content may be empty if no logs yet)
    let log_file = log_files.first().expect("No log files found");
    let _log_content = fs::read_to_string(log_file.path()).expect("Failed to read log file");

    // Note: Log content may be empty if dashboard just started and hasn't logged anything yet
    // The important verification is that the dated log file exists and is readable

    // Cleanup
    stop_dashboard();
}

#[test]
#[serial]
fn test_cleanup_old_logs_deletes_old_files() {
    // Setup
    stop_dashboard();

    let log_dir = get_log_dir();
    if log_dir.exists() {
        fs::remove_dir_all(&log_dir).ok();
    }
    fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    // Create fake old log files with different ages
    let old_10d = create_old_log_file("dashboard.log.2025-11-12", 10); // 10 days old
    let old_8d = create_old_log_file("dashboard.log.2025-11-14", 8); // 8 days old
    let old_5d = create_old_log_file("dashboard.log.2025-11-17", 5); // 5 days old

    assert!(old_10d.exists(), "10-day old file should exist");
    assert!(old_8d.exists(), "8-day old file should exist");
    assert!(old_5d.exists(), "5-day old file should exist");

    // Start dashboard (should trigger cleanup with default 7-day retention)
    let status = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Failed to start dashboard");

    assert!(status.success(), "Dashboard should start successfully");
    thread::sleep(Duration::from_secs(3));

    // Verify cleanup: files older than 7 days should be deleted
    assert!(
        !old_10d.exists(),
        "10-day old file should be deleted (older than 7 days)"
    );
    assert!(
        !old_8d.exists(),
        "8-day old file should be deleted (older than 7 days)"
    );
    assert!(
        old_5d.exists(),
        "5-day old file should be kept (within 7-day retention)"
    );

    // Cleanup
    stop_dashboard();
}

#[test]
#[serial]
fn test_cleanup_respects_custom_retention_period() {
    // Setup
    stop_dashboard();

    let log_dir = get_log_dir();
    if log_dir.exists() {
        fs::remove_dir_all(&log_dir).ok();
    }
    fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    // Create fake old log files
    let old_4d = create_old_log_file("dashboard.log.2025-11-18", 4); // 4 days old
    let old_2d = create_old_log_file("dashboard.log.2025-11-20", 2); // 2 days old

    assert!(old_4d.exists(), "4-day old file should exist");
    assert!(old_2d.exists(), "2-day old file should exist");

    // Start dashboard with custom 3-day retention
    let status = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .env("IE_LOG_RETENTION_DAYS", "3") // Custom retention
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Failed to start dashboard");

    assert!(status.success(), "Dashboard should start successfully");
    thread::sleep(Duration::from_secs(3));

    // Verify cleanup with 3-day retention
    assert!(
        !old_4d.exists(),
        "4-day old file should be deleted (older than 3 days)"
    );
    assert!(
        old_2d.exists(),
        "2-day old file should be kept (within 3-day retention)"
    );

    // Cleanup
    stop_dashboard();
}

#[test]
#[serial]
fn test_cleanup_handles_empty_directory() {
    // Setup
    stop_dashboard();

    let log_dir = get_log_dir();
    if log_dir.exists() {
        fs::remove_dir_all(&log_dir).ok();
    }
    // Don't create the directory - let dashboard handle it

    // Start dashboard (should handle non-existent log directory)
    let status = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Failed to start dashboard");

    assert!(status.success(), "Dashboard should start successfully");
    thread::sleep(Duration::from_secs(3));

    // Verify log directory and dated file created
    assert!(log_dir.exists(), "Log directory should be created");

    // Check for dated log files (daily rotation format)
    let log_files: Vec<_> = fs::read_dir(log_dir)
        .expect("Failed to read log directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("dashboard.log.")
        })
        .collect();

    assert!(
        !log_files.is_empty(),
        "At least one dated log file should be created"
    );

    // Cleanup
    stop_dashboard();
}

#[test]
#[serial]
fn test_cleanup_only_removes_rotated_log_files() {
    // Setup
    stop_dashboard();

    let log_dir = get_log_dir();
    if log_dir.exists() {
        fs::remove_dir_all(&log_dir).ok();
    }
    fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    // Create old rotated log file (should be deleted)
    let old_log = create_old_log_file("dashboard.log.2025-11-10", 12);

    // Create other old files (should NOT be deleted)
    let other_file = log_dir.join("other-file.txt");
    let mut file = File::create(&other_file).expect("Failed to create file");
    writeln!(file, "Other content").expect("Failed to write");
    let mtime = SystemTime::now() - Duration::from_secs(12 * 24 * 60 * 60);
    filetime::set_file_mtime(&other_file, filetime::FileTime::from_system_time(mtime))
        .expect("Failed to set mtime");

    assert!(old_log.exists(), "Old log file should exist");
    assert!(other_file.exists(), "Other file should exist");

    // Start dashboard
    let status = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Failed to start dashboard");

    assert!(status.success(), "Dashboard should start successfully");
    thread::sleep(Duration::from_secs(3));

    // Verify: old log deleted, other file kept
    assert!(!old_log.exists(), "Old rotated log file should be deleted");
    assert!(
        other_file.exists(),
        "Other file should not be deleted (doesn't match .log.* pattern)"
    );

    // Cleanup
    stop_dashboard();
    fs::remove_file(other_file).ok();
}
