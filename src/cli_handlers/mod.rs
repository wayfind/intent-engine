// CLI command handlers module
//
// This module contains all CLI command handling logic extracted from main.rs
// to improve code organization and reduce file size.

pub mod dashboard;
pub mod other;
pub mod task;
pub mod utils;

// Re-export commonly used functions
pub use dashboard::{check_dashboard_status, check_mcp_connections, handle_dashboard_command};
pub use other::{
    check_session_start_hook, handle_current_command, handle_doctor_command, handle_event_command,
    handle_init_command, handle_logs_command, handle_report_command, handle_search_command,
    handle_session_restore, handle_setup,
};
pub use task::handle_task_command;
pub use utils::{get_status_badge, print_task_context, read_stdin};
