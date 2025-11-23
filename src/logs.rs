//! Log Query and Management Module
//!
//! Provides functionality to query, filter, and display application logs.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

/// Log entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub target: Option<String>,
    pub message: String,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
}

/// Log query parameters
#[derive(Debug, Clone)]
pub struct LogQuery {
    pub mode: Option<String>,
    pub level: Option<String>,
    pub since: Option<Duration>,
    pub until: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

impl Default for LogQuery {
    fn default() -> Self {
        Self {
            mode: None,
            level: None,
            since: Some(Duration::hours(24)), // Default: last 24 hours
            until: None,
            limit: Some(100),
        }
    }
}

/// Get log directory path
pub fn log_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("logs")
}

/// Get log file path for a specific mode
pub fn log_file_for_mode(mode: &str) -> Option<PathBuf> {
    let dir = log_dir();
    match mode {
        "dashboard" => Some(dir.join("dashboard.log")),
        "mcp-server" => Some(dir.join("mcp-server.log")),
        "cli" => Some(dir.join("cli.log")),
        _ => None,
    }
}

/// List all available log files
pub fn list_log_files() -> io::Result<Vec<PathBuf>> {
    let dir = log_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut files = vec![];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        // Match both .log files and rotated files like .log.2025-11-23
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".log") || name.contains(".log.") {
                    files.push(path);
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Parse a log line into a LogEntry
pub fn parse_log_line(line: &str, mode: &str) -> Option<LogEntry> {
    // Try JSON format first
    if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
        // Try to get message from fields.message (MCP Server format) or top-level message
        let message = entry
            .get("fields")
            .and_then(|f| f.get("message"))
            .and_then(|m| m.as_str())
            .or_else(|| entry.get("message").and_then(|m| m.as_str()))
            .unwrap_or("")
            .to_string();

        return Some(LogEntry {
            timestamp: entry
                .get("timestamp")
                .and_then(|t| t.as_str())
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            level: entry
                .get("level")
                .and_then(|l| l.as_str())
                .unwrap_or("INFO")
                .to_string(),
            target: entry
                .get("target")
                .and_then(|t| t.as_str())
                .map(String::from),
            message,
            mode: mode.to_string(),
            fields: entry.get("fields").cloned(),
        });
    }

    // Try text format: "2025-11-22T06:54:15.123456789+00:00  INFO target: message"
    // Split by whitespace, skipping empty parts
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 3 {
        if let Ok(timestamp) = DateTime::parse_from_rfc3339(parts[0]) {
            let level = parts[1].to_string();

            // Find the position of the second whitespace to get the rest
            let after_timestamp = line.find(parts[0]).unwrap() + parts[0].len();
            let rest = &line[after_timestamp..].trim_start();
            let after_level = rest.find(parts[1]).unwrap() + parts[1].len();
            let rest = &rest[after_level..].trim_start();

            // Try to extract target from "target: message"
            let (target, message) = if let Some(idx) = rest.find(": ") {
                let (t, m) = rest.split_at(idx);
                (Some(t.to_string()), m[2..].to_string())
            } else {
                (None, rest.to_string())
            };

            return Some(LogEntry {
                timestamp: timestamp.with_timezone(&Utc),
                level,
                target,
                message,
                mode: mode.to_string(),
                fields: None,
            });
        }
    }

    None
}

/// Query logs based on filter criteria
pub fn query_logs(query: &LogQuery) -> io::Result<Vec<LogEntry>> {
    let mut entries = Vec::new();
    let cutoff_time = query
        .since
        .map(|d| Utc::now() - d)
        .unwrap_or_else(|| Utc::now() - Duration::days(365));

    let files = if let Some(mode) = &query.mode {
        // Get all log files (including rotated ones) and filter by mode
        let all_files = list_log_files()?;
        all_files
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|name| name.starts_with(&format!("{}.log", mode)))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        list_log_files()?
    };

    for file_path in files {
        if !file_path.exists() {
            continue;
        }

        let mode = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let file = File::open(&file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if let Some(entry) = parse_log_line(&line, mode) {
                // Filter by timestamp
                if entry.timestamp < cutoff_time {
                    continue;
                }
                if let Some(until) = query.until {
                    if entry.timestamp > until {
                        continue;
                    }
                }

                // Filter by level
                if let Some(ref level) = query.level {
                    if !entry.level.eq_ignore_ascii_case(level) {
                        continue;
                    }
                }

                entries.push(entry);
            }
        }
    }

    // Sort by timestamp
    entries.sort_by_key(|e| e.timestamp);

    // Apply limit
    if let Some(limit) = query.limit {
        entries.truncate(limit);
    }

    Ok(entries)
}

/// Parse duration string like "1h", "24h", "7d"
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (num_str, unit) = if let Some(stripped) = s.strip_suffix('s') {
        (stripped, 's')
    } else if let Some(stripped) = s.strip_suffix('m') {
        (stripped, 'm')
    } else if let Some(stripped) = s.strip_suffix('h') {
        (stripped, 'h')
    } else if let Some(stripped) = s.strip_suffix('d') {
        (stripped, 'd')
    } else {
        return None;
    };

    let num: i64 = num_str.parse().ok()?;

    match unit {
        's' => Some(Duration::seconds(num)),
        'm' => Some(Duration::minutes(num)),
        'h' => Some(Duration::hours(num)),
        'd' => Some(Duration::days(num)),
        _ => None,
    }
}

/// Format log entry as text
pub fn format_entry_text(entry: &LogEntry) -> String {
    if let Some(ref target) = entry.target {
        format!(
            "{} {:5} {:10} {}: {}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            entry.level,
            entry.mode,
            target,
            entry.message
        )
    } else {
        format!(
            "{} {:5} {:10} {}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            entry.level,
            entry.mode,
            entry.message
        )
    }
}

/// Format log entry as JSON
pub fn format_entry_json(entry: &LogEntry) -> String {
    serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_string())
}

/// Follow logs in real-time (like tail -f)
pub fn follow_logs(query: &LogQuery) -> io::Result<()> {
    use std::thread;
    use std::time::Duration as StdDuration;

    let files = if let Some(mode) = &query.mode {
        if let Some(file) = log_file_for_mode(mode) {
            vec![file]
        } else {
            vec![]
        }
    } else {
        list_log_files()?
    };

    let mut positions: Vec<(PathBuf, u64)> = files.iter().map(|f| (f.clone(), 0)).collect();

    // Get initial file sizes
    for (path, pos) in &mut positions {
        if let Ok(metadata) = fs::metadata(path) {
            *pos = metadata.len();
        }
    }

    println!("Following logs... (Ctrl+C to stop)");

    loop {
        for (path, last_pos) in &mut positions {
            if !path.exists() {
                continue;
            }

            let metadata = fs::metadata(&**path)?;
            let current_size = metadata.len();

            if current_size < *last_pos {
                // File was truncated or rotated
                *last_pos = 0;
            }

            if current_size > *last_pos {
                let mut file = File::open(&**path)?;
                file.seek(SeekFrom::Start(*last_pos))?;
                let reader = BufReader::new(file);

                let mode = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                for line in reader.lines() {
                    let line = line?;
                    if let Some(entry) = parse_log_line(&line, mode) {
                        // Apply filters
                        if let Some(ref level) = query.level {
                            if !entry.level.eq_ignore_ascii_case(level) {
                                continue;
                            }
                        }

                        println!("{}", format_entry_text(&entry));
                    }
                }

                *last_pos = current_size;
            }
        }

        thread::sleep(StdDuration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("1h"), Some(Duration::hours(1)));
        assert_eq!(parse_duration("24h"), Some(Duration::hours(24)));
        assert_eq!(parse_duration("7d"), Some(Duration::days(7)));
        assert_eq!(parse_duration("30m"), Some(Duration::minutes(30)));
        assert_eq!(parse_duration("60s"), Some(Duration::seconds(60)));
        assert_eq!(parse_duration("invalid"), None);
    }

    #[test]
    fn test_parse_log_line_text() {
        let line =
            "2025-11-22T06:54:15.123456789+00:00  INFO intent_engine::dashboard: Server started";
        let entry = parse_log_line(line, "dashboard").unwrap();
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.target, Some("intent_engine::dashboard".to_string()));
        assert_eq!(entry.message, "Server started");
    }

    #[test]
    fn test_parse_log_line_json() {
        let line = r#"{"timestamp":"2025-11-22T06:54:15.123456789+00:00","level":"INFO","target":"intent_engine","message":"Test message"}"#;
        let entry = parse_log_line(line, "dashboard").unwrap();
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.message, "Test message");
    }
}
