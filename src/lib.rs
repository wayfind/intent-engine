pub mod cli;
pub mod dashboard;
pub mod db;
pub mod dependencies;
pub mod error;
pub mod events;
pub mod logging;
pub mod logs;
pub mod mcp;
pub mod notifications;
pub mod plan;
pub mod priority;
pub mod project;
pub mod report;
pub mod search;
pub mod session_restore;
pub mod setup;
pub mod tasks;
pub mod windows_console;
pub mod workspace;

#[cfg(test)]
pub mod test_utils;
