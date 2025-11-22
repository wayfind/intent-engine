//! Intent-Engine Logging System
//!
//! Provides structured logging with configurable levels and output formats.
//! Uses tracing crate for structured logging with spans and events.

use std::io;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

/// Logging configuration options
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Minimum log level to output
    pub level: Level,
    /// Enable colored output
    pub color: bool,
    /// Show timestamps
    pub show_timestamps: bool,
    /// Show target/module name
    pub show_target: bool,
    /// Enable JSON format for machine parsing
    pub json_format: bool,
    /// Enable span events for tracing
    pub enable_spans: bool,
    /// Output to file instead of stdout (for daemon mode)
    pub file_output: Option<std::path::PathBuf>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            color: true,
            show_timestamps: false,
            show_target: false,
            json_format: false,
            enable_spans: false,
            file_output: None,
        }
    }
}

impl LoggingConfig {
    /// Create config for different application modes
    pub fn for_mode(mode: ApplicationMode) -> Self {
        match mode {
            ApplicationMode::McpServer => Self {
                level: Level::INFO,
                color: false, // MCP output should be clean
                show_timestamps: true,
                show_target: true,
                json_format: true,   // Machine-readable for MCP
                enable_spans: false, // Avoid noise in JSON-RPC
                file_output: None,
            },
            ApplicationMode::Dashboard => Self {
                level: Level::INFO,
                color: false, // Background service
                show_timestamps: true,
                show_target: true,
                json_format: false,
                enable_spans: true, // Good for debugging dashboard
                file_output: None,
            },
            ApplicationMode::Cli => Self {
                level: Level::INFO,
                color: true,
                show_timestamps: false,
                show_target: false,
                json_format: false,
                enable_spans: false,
                file_output: None,
            },
            ApplicationMode::Test => Self {
                level: Level::DEBUG,
                color: false,
                show_timestamps: true,
                show_target: true,
                json_format: false,
                enable_spans: true,
                file_output: None,
            },
        }
    }

    /// Create config from CLI arguments
    pub fn from_args(quiet: bool, verbose: bool, json: bool) -> Self {
        let level = if verbose {
            Level::DEBUG
        } else if quiet {
            Level::ERROR
        } else {
            Level::INFO
        };

        Self {
            level,
            color: !quiet && !json && atty::is(atty::Stream::Stdout),
            show_timestamps: verbose || json,
            show_target: verbose,
            json_format: json,
            enable_spans: verbose,
            file_output: None,
        }
    }
}

/// Application modes with different logging requirements
#[derive(Debug, Clone, Copy)]
pub enum ApplicationMode {
    /// MCP server mode - clean, structured output
    McpServer,
    /// Dashboard server mode - detailed for debugging
    Dashboard,
    /// CLI mode - user-friendly output
    Cli,
    /// Test mode - maximum detail for testing
    Test,
}

/// Initialize the logging system
pub fn init_logging(config: LoggingConfig) -> io::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("intent_engine={}", config.level)));

    let registry = Registry::default().with(env_filter);

    if let Some(log_file) = config.file_output {
        let file_appender = tracing_appender::rolling::never(
            log_file.parent().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Invalid log file path")
            })?,
            log_file.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Invalid log file name")
            })?,
        );

        if config.json_format {
            let json_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(config.enable_spans)
                .with_span_events(FmtSpan::CLOSE)
                .with_writer(file_appender);
            json_layer.with_subscriber(registry).init();
        } else {
            let fmt_layer = fmt::layer()
                .with_target(config.show_target)
                .with_level(true)
                .with_ansi(false)
                .with_writer(file_appender);

            if config.show_timestamps {
                fmt_layer
                    .with_timer(fmt::time::ChronoUtc::rfc_3339())
                    .with_subscriber(registry)
                    .init();
            } else {
                fmt_layer.with_subscriber(registry).init();
            }
        }
    } else if config.json_format {
        let json_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(config.enable_spans)
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(io::stdout);
        json_layer.with_subscriber(registry).init();
    } else {
        let fmt_layer = fmt::layer()
            .with_target(config.show_target)
            .with_level(true)
            .with_ansi(config.color)
            .with_writer(io::stdout);

        if config.show_timestamps {
            fmt_layer
                .with_timer(fmt::time::ChronoUtc::rfc_3339())
                .with_subscriber(registry)
                .init();
        } else {
            fmt_layer.with_subscriber(registry).init();
        }
    }

    Ok(())
}

/// Initialize logging from environment variables
pub fn init_from_env() -> io::Result<()> {
    let _level = match std::env::var("IE_LOG_LEVEL").as_deref() {
        Ok("error") => Level::ERROR,
        Ok("warn") => Level::WARN,
        Ok("info") => Level::INFO,
        Ok("debug") => Level::DEBUG,
        Ok("trace") => Level::TRACE,
        _ => Level::INFO,
    };

    let json = std::env::var("IE_LOG_JSON").as_deref() == Ok("true");
    let verbose = std::env::var("IE_LOG_VERBOSE").as_deref() == Ok("true");
    let quiet = std::env::var("IE_LOG_QUIET").as_deref() == Ok("true");

    let config = LoggingConfig::from_args(quiet, verbose, json);
    init_logging(config)
}

/// Clean up old log files based on retention policy
///
/// Scans the log directory and removes files older than the specified retention period.
/// Only removes files matching the pattern `.log.YYYY-MM-DD` (rotated log files).
///
/// # Arguments
/// * `log_dir` - Directory containing log files
/// * `retention_days` - Number of days to retain logs (default: 7)
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use intent_engine::logging::cleanup_old_logs;
///
/// let log_dir = Path::new("/home/user/.intent-engine/logs");
/// cleanup_old_logs(log_dir, 7).ok();
/// ```
pub fn cleanup_old_logs(log_dir: &std::path::Path, retention_days: u32) -> io::Result<()> {
    use std::fs;
    use std::time::SystemTime;

    if !log_dir.exists() {
        return Ok(()); // Nothing to clean if directory doesn't exist
    }

    let now = SystemTime::now();
    let retention_duration = std::time::Duration::from_secs(retention_days as u64 * 24 * 60 * 60);

    let mut cleaned_count = 0;
    let mut cleaned_size: u64 = 0;

    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process rotated log files (containing .log. followed by a date)
        // Examples: dashboard.log.2025-11-22, mcp-server.log.2025-11-21
        let path_str = path.to_string_lossy();
        if !path_str.contains(".log.") || !path.is_file() {
            continue;
        }

        let metadata = entry.metadata()?;
        let modified = metadata.modified()?;

        if let Ok(age) = now.duration_since(modified) {
            if age > retention_duration {
                let size = metadata.len();
                match fs::remove_file(&path) {
                    Ok(_) => {
                        cleaned_count += 1;
                        cleaned_size += size;
                        tracing::info!(
                            "Cleaned up old log file: {} (age: {} days, size: {} bytes)",
                            path.display(),
                            age.as_secs() / 86400,
                            size
                        );
                    },
                    Err(e) => {
                        tracing::warn!("Failed to remove old log file {}: {}", path.display(), e);
                    },
                }
            }
        }
    }

    if cleaned_count > 0 {
        tracing::info!(
            "Log cleanup completed: removed {} files, freed {} bytes",
            cleaned_count,
            cleaned_size
        );
    }

    Ok(())
}

/// Log macros for common intent-engine operations
#[macro_export]
macro_rules! log_project_operation {
    ($operation:expr, $project_path:expr) => {
        tracing::info!(
            operation = $operation,
            project_path = %$project_path.display(),
            "Project operation"
        );
    };
    ($operation:expr, $project_path:expr, $details:expr) => {
        tracing::info!(
            operation = $operation,
            project_path = %$project_path.display(),
            details = $details,
            "Project operation"
        );
    };
}

#[macro_export]
macro_rules! log_mcp_operation {
    ($operation:expr, $method:expr) => {
        tracing::debug!(
            operation = $operation,
            mcp_method = $method,
            "MCP operation"
        );
    };
    ($operation:expr, $method:expr, $details:expr) => {
        tracing::debug!(
            operation = $operation,
            mcp_method = $method,
            details = $details,
            "MCP operation"
        );
    };
}

#[macro_export]
macro_rules! log_dashboard_operation {
    ($operation:expr) => {
        tracing::info!(operation = $operation, "Dashboard operation");
    };
    ($operation:expr, $details:expr) => {
        tracing::info!(
            operation = $operation,
            details = $details,
            "Dashboard operation"
        );
    };
}

#[macro_export]
macro_rules! log_task_operation {
    ($operation:expr, $task_id:expr) => {
        tracing::info!(operation = $operation, task_id = $task_id, "Task operation");
    };
    ($operation:expr, $task_id:expr, $details:expr) => {
        tracing::info!(
            operation = $operation,
            task_id = $task_id,
            details = $details,
            "Task operation"
        );
    };
}

#[macro_export]
macro_rules! log_registry_operation {
    ($operation:expr, $count:expr) => {
        tracing::debug!(
            operation = $operation,
            project_count = $count,
            "Registry operation"
        );
    };
}

/// Utility macro for structured error logging
#[macro_export]
macro_rules! log_error {
    ($error:expr, $context:expr) => {
        tracing::error!(
            error = %$error,
            context = $context,
            "Operation failed"
        );
    };
}

/// Utility macro for structured warning logging
#[macro_export]
macro_rules! log_warning {
    ($message:expr) => {
        tracing::warn!($message);
    };
    ($message:expr, $details:expr) => {
        tracing::warn!(message = $message, details = $details, "Warning");
    };
}

/// Get log file path for a given application mode
pub fn log_file_path(mode: ApplicationMode) -> std::path::PathBuf {
    let home = dirs::home_dir().expect("Failed to get home directory");
    let log_dir = home.join(".intent-engine").join("logs");

    // Create log directory if it doesn't exist
    std::fs::create_dir_all(&log_dir).ok();

    match mode {
        ApplicationMode::Dashboard => log_dir.join("dashboard.log"),
        ApplicationMode::McpServer => log_dir.join("mcp-server.log"),
        ApplicationMode::Cli => log_dir.join("cli.log"),
        ApplicationMode::Test => log_dir.join("test.log"),
    }
}
