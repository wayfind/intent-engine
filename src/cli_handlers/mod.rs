// CLI command handlers module
//
// This module contains CLI command handling logic:
// Core: plan, log, search, task
// System: init, dashboard, doctor

pub mod config_commands;
pub mod dashboard;
pub mod other;
pub mod suggestions_commands;
pub mod task_commands;
pub mod utils;

// Re-export commonly used functions
pub use config_commands::handle_config_command;
pub use dashboard::{check_dashboard_status, check_mcp_connections, handle_dashboard_command};
pub use other::{
    handle_doctor_command,
    handle_init_command,
    handle_search_command,
    // Deprecated handlers (kept for potential MCP or Dashboard use):
    // handle_current_command, handle_event_command, handle_report_command,
    // handle_session_restore, handle_setup, handle_logs_command, check_session_start_hook
};
pub use task_commands::handle_task_command;
pub use utils::{get_status_badge, print_task_context, read_stdin};
