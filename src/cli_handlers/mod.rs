// CLI command handlers module (Simplified for v0.10.1+)
//
// This module contains CLI command handling logic for the simplified 6-command structure:
// Core: plan, log, search
// System: init, dashboard, doctor

pub mod dashboard;
pub mod other;
pub mod utils;
// Removed modules:
// - pub mod guide;  // Removed: Help content moved to --help
// - pub mod task;   // Removed: No longer needed with simplified CLI

// Re-export commonly used functions (simplified)
pub use dashboard::{check_dashboard_status, check_mcp_connections, handle_dashboard_command};
pub use other::{
    handle_doctor_command,
    handle_init_command,
    handle_search_command,
    // Deprecated handlers (kept for potential MCP or Dashboard use):
    // handle_current_command, handle_event_command, handle_report_command,
    // handle_session_restore, handle_setup, handle_logs_command, check_session_start_hook
};
pub use utils::{get_status_badge, print_task_context, read_stdin};
