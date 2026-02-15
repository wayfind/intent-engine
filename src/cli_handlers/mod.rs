// CLI command handlers module
//
// This module contains CLI command handling logic:
// Core: plan, log, search, status, task
// System: init, dashboard, doctor

pub mod config_commands;
pub mod dashboard;
pub mod log_command;
pub mod other;
pub mod plan_command;
pub mod status_command;
pub mod suggestions_commands;
pub mod task_commands;
pub mod utils;

// Re-export commonly used functions
pub use config_commands::handle_config_command;
pub use dashboard::{check_dashboard_status, check_mcp_connections, handle_dashboard_command};
pub use log_command::handle_log;
pub use other::{
    handle_doctor_command,
    handle_init_command,
    handle_search_command,
    // Deprecated handlers (kept for potential MCP or Dashboard use):
    // handle_current_command, handle_event_command, handle_report_command,
    // handle_session_restore, handle_setup, handle_logs_command, check_session_start_hook
};
pub use plan_command::{execute_and_print as execute_plan_and_print, print_plan_result};
pub use status_command::handle_status;
pub use task_commands::handle_task_command;
pub use utils::{
    get_status_badge, merge_metadata, parse_metadata, print_events_summary, print_task_context,
    print_task_summary, print_task_tree, read_stdin, status_icon,
};
