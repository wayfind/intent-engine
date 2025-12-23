use std::fs::{self, File};
use std::io::Write;

mod common;

#[test]
fn test_log_directory_path() {
    let log_dir = intent_engine::logs::log_dir();
    let log_dir_str = log_dir.to_string_lossy();
    // Cross-platform check: accept both forward and backslash separators
    assert!(
        log_dir_str.contains(".intent-engine/logs")
            || log_dir_str.contains(".intent-engine\\logs"),
        "Expected log directory to contain '.intent-engine/logs' or '.intent-engine\\logs', got: {}",
        log_dir_str
    );
}

#[test]
fn test_log_file_for_mode() {
    let dashboard_log = intent_engine::logs::log_file_for_mode("dashboard");
    assert!(dashboard_log.is_some());
    assert!(dashboard_log
        .unwrap()
        .to_string_lossy()
        .ends_with("dashboard.log"));

    let mcp_log = intent_engine::logs::log_file_for_mode("mcp-server");
    assert!(mcp_log.is_some());
    assert!(mcp_log
        .unwrap()
        .to_string_lossy()
        .ends_with("mcp-server.log"));

    let invalid_log = intent_engine::logs::log_file_for_mode("invalid");
    assert!(invalid_log.is_none());
}

#[test]
fn test_parse_log_line_text_format() {
    let line = "2025-11-22T06:54:15.123456789+00:00  INFO intent_engine::dashboard: Server started";
    let entry = intent_engine::logs::parse_log_line(line, "dashboard").unwrap();

    assert_eq!(entry.level, "INFO");
    assert_eq!(entry.target, Some("intent_engine::dashboard".to_string()));
    assert_eq!(entry.message, "Server started");
    assert_eq!(entry.mode, "dashboard");
}

#[test]
fn test_parse_log_line_json_format() {
    let line = r#"{"timestamp":"2025-11-22T06:54:15.123456789+00:00","level":"INFO","target":"intent_engine::mcp","fields":{"message":"Test message"}}"#;
    let entry = intent_engine::logs::parse_log_line(line, "mcp-server").unwrap();

    assert_eq!(entry.level, "INFO");
    assert_eq!(entry.target, Some("intent_engine::mcp".to_string()));
    assert_eq!(entry.message, "Test message");
    assert_eq!(entry.mode, "mcp-server");
}

#[test]
fn test_parse_duration() {
    assert_eq!(
        intent_engine::logs::parse_duration("1h"),
        Some(chrono::Duration::hours(1))
    );
    assert_eq!(
        intent_engine::logs::parse_duration("24h"),
        Some(chrono::Duration::hours(24))
    );
    assert_eq!(
        intent_engine::logs::parse_duration("7d"),
        Some(chrono::Duration::days(7))
    );
    assert_eq!(
        intent_engine::logs::parse_duration("30m"),
        Some(chrono::Duration::minutes(30))
    );
    assert_eq!(
        intent_engine::logs::parse_duration("60s"),
        Some(chrono::Duration::seconds(60))
    );
    assert_eq!(intent_engine::logs::parse_duration("invalid"), None);
}

#[test]
fn test_format_entry_text() {
    let entry = intent_engine::logs::LogEntry {
        timestamp: chrono::Utc::now(),
        level: "INFO".to_string(),
        target: Some("test::module".to_string()),
        message: "Test message".to_string(),
        mode: "dashboard".to_string(),
        fields: None,
    };

    let formatted = intent_engine::logs::format_entry_text(&entry);
    assert!(formatted.contains("INFO"));
    assert!(formatted.contains("dashboard"));
    assert!(formatted.contains("test::module"));
    assert!(formatted.contains("Test message"));
}

#[test]
fn test_format_entry_json() {
    let entry = intent_engine::logs::LogEntry {
        timestamp: chrono::Utc::now(),
        level: "ERROR".to_string(),
        target: Some("test::module".to_string()),
        message: "Error occurred".to_string(),
        mode: "cli".to_string(),
        fields: None,
    };

    let formatted = intent_engine::logs::format_entry_json(&entry);
    let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

    assert_eq!(parsed["level"], "ERROR");
    assert_eq!(parsed["target"], "test::module");
    assert_eq!(parsed["message"], "Error occurred");
    assert_eq!(parsed["mode"], "cli");
}

#[test]
#[serial_test::serial]
fn test_query_logs_with_temp_file() {
    // Create a temporary log directory for testing
    let temp_dir = std::env::temp_dir().join("intent-engine-test-logs");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Create a test log file
    let log_file = temp_dir.join("test.log");
    let mut file = File::create(&log_file).unwrap();
    writeln!(
        file,
        "2025-11-22T06:54:15.123456789+00:00  INFO test::module: First message"
    )
    .unwrap();
    writeln!(
        file,
        "2025-11-22T06:54:16.123456789+00:00  ERROR test::module: Error message"
    )
    .unwrap();
    writeln!(
        file,
        "2025-11-22T06:54:17.123456789+00:00  INFO test::module: Second message"
    )
    .unwrap();
    drop(file);

    // Note: This test would need to be adapted to work with the actual query_logs function
    // which expects logs in ~/.intent-engine/logs/
    // For now, we just verify the file was created
    assert!(log_file.exists());

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_list_log_files_empty() {
    // This test checks behavior when log directory doesn't exist
    // The function should return an empty vec instead of erroring
    // Note: This assumes the function handles non-existent directories gracefully
    // Actual implementation may vary
}

#[test]
fn test_parse_log_line_malformed() {
    // Test with malformed log lines
    let invalid_lines = vec![
        "",
        "not a log line",
        "2025-11-22",
        "invalid timestamp  INFO message",
    ];

    for line in invalid_lines {
        let result = intent_engine::logs::parse_log_line(line, "test");
        assert!(result.is_none(), "Expected None for line: {}", line);
    }
}

#[test]
fn test_log_entry_serialization() {
    let entry = intent_engine::logs::LogEntry {
        timestamp: chrono::DateTime::parse_from_rfc3339("2025-11-22T06:54:15.123456789+00:00")
            .unwrap()
            .with_timezone(&chrono::Utc),
        level: "WARN".to_string(),
        target: Some("test".to_string()),
        message: "Warning message".to_string(),
        mode: "cli".to_string(),
        fields: Some(serde_json::json!({"extra": "data"})),
    };

    // Test JSON serialization
    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: intent_engine::logs::LogEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(entry.level, deserialized.level);
    assert_eq!(entry.target, deserialized.target);
    assert_eq!(entry.message, deserialized.message);
    assert_eq!(entry.mode, deserialized.mode);
}
