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
            },
            ApplicationMode::Dashboard => Self {
                level: Level::INFO,
                color: false, // Background service
                show_timestamps: true,
                show_target: true,
                json_format: false,
                enable_spans: true, // Good for debugging dashboard
            },
            ApplicationMode::Cli => Self {
                level: Level::INFO,
                color: true,
                show_timestamps: false,
                show_target: false,
                json_format: false,
                enable_spans: false,
            },
            ApplicationMode::Test => Self {
                level: Level::DEBUG,
                color: false,
                show_timestamps: true,
                show_target: true,
                json_format: false,
                enable_spans: true,
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
    // Create environment filter from config
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("intent_engine={}", config.level)));

    let registry = Registry::default().with(env_filter);

    if config.json_format {
        // JSON format for machine processing
        let json_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(config.enable_spans)
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(io::stdout);

        json_layer.with_subscriber(registry).init();
    } else {
        // Human-readable format
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();

        assert_eq!(config.level, Level::INFO);
        assert!(config.color);
        assert!(!config.show_timestamps);
        assert!(!config.show_target);
        assert!(!config.json_format);
        assert!(!config.enable_spans);
    }

    #[test]
    fn test_logging_config_for_mode_mcp_server() {
        let config = LoggingConfig::for_mode(ApplicationMode::McpServer);

        assert_eq!(config.level, Level::INFO);
        assert!(!config.color); // MCP should be clean
        assert!(config.show_timestamps);
        assert!(config.show_target);
        assert!(config.json_format); // Machine-readable
        assert!(!config.enable_spans); // Avoid noise
    }

    #[test]
    fn test_logging_config_for_mode_dashboard() {
        let config = LoggingConfig::for_mode(ApplicationMode::Dashboard);

        assert_eq!(config.level, Level::INFO);
        assert!(!config.color); // Background service
        assert!(config.show_timestamps);
        assert!(config.show_target);
        assert!(!config.json_format);
        assert!(config.enable_spans); // Good for debugging
    }

    #[test]
    fn test_logging_config_for_mode_cli() {
        let config = LoggingConfig::for_mode(ApplicationMode::Cli);

        assert_eq!(config.level, Level::INFO);
        assert!(config.color);
        assert!(!config.show_timestamps);
        assert!(!config.show_target);
        assert!(!config.json_format);
        assert!(!config.enable_spans);
    }

    #[test]
    fn test_logging_config_for_mode_test() {
        let config = LoggingConfig::for_mode(ApplicationMode::Test);

        assert_eq!(config.level, Level::DEBUG);
        assert!(!config.color);
        assert!(config.show_timestamps);
        assert!(config.show_target);
        assert!(!config.json_format);
        assert!(config.enable_spans);
    }

    #[test]
    fn test_logging_config_from_args_default() {
        let config = LoggingConfig::from_args(false, false, false);

        assert_eq!(config.level, Level::INFO);
        assert!(!config.show_timestamps);
        assert!(!config.show_target);
        assert!(!config.json_format);
        assert!(!config.enable_spans);
    }

    #[test]
    fn test_logging_config_from_args_verbose() {
        let config = LoggingConfig::from_args(false, true, false);

        assert_eq!(config.level, Level::DEBUG);
        assert!(config.show_timestamps);
        assert!(config.show_target);
        assert!(!config.json_format);
        assert!(config.enable_spans);
    }

    #[test]
    fn test_logging_config_from_args_quiet() {
        let config = LoggingConfig::from_args(true, false, false);

        assert_eq!(config.level, Level::ERROR);
        assert!(!config.color); // Quiet mode disables color
        assert!(!config.show_timestamps);
        assert!(!config.show_target);
        assert!(!config.json_format);
        assert!(!config.enable_spans);
    }

    #[test]
    fn test_logging_config_from_args_json() {
        let config = LoggingConfig::from_args(false, false, true);

        assert_eq!(config.level, Level::INFO);
        assert!(!config.color); // JSON mode disables color
        assert!(config.show_timestamps);
        assert!(!config.show_target);
        assert!(config.json_format);
        assert!(!config.enable_spans);
    }

    #[test]
    fn test_logging_config_from_args_verbose_json() {
        let config = LoggingConfig::from_args(false, true, true);

        assert_eq!(config.level, Level::DEBUG);
        assert!(!config.color); // JSON mode disables color
        assert!(config.show_timestamps);
        assert!(config.show_target);
        assert!(config.json_format);
        assert!(config.enable_spans);
    }

    #[test]
    fn test_application_mode_variants() {
        // Just verify all modes can be created
        let modes = [
            ApplicationMode::McpServer,
            ApplicationMode::Dashboard,
            ApplicationMode::Cli,
            ApplicationMode::Test,
        ];

        for mode in modes {
            let config = LoggingConfig::for_mode(mode);
            // Each mode should produce a valid config
            assert!(matches!(
                config.level,
                Level::ERROR | Level::WARN | Level::INFO | Level::DEBUG | Level::TRACE
            ));
        }
    }
}
