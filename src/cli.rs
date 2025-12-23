use clap::{Parser, Subcommand};

#[derive(Parser, Clone)]
#[command(name = "intent-engine")]
#[command(about = "A command-line database service for tracking strategic intent", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Enable verbose output (-v)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error output (-q)
    #[arg(short, long)]
    pub quiet: bool,

    /// Output logs in JSON format
    #[arg(long)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// Create or update task structures declaratively
    #[command(long_about = include_str!("../docs/help/plan.md"))]
    Plan {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Record events (decisions, blockers, milestones, notes)
    ///
    /// Quick event logging for the current focused task or a specific task.
    ///
    /// Examples:
    ///   ie log decision "Chose JWT authentication"
    ///   ie log blocker "API rate limit hit" --task 42
    ///   ie log milestone "MVP complete"
    ///   ie log note "Consider caching optimization"
    Log {
        /// Event type: decision, blocker, milestone, note
        #[arg(value_enum)]
        event_type: LogEventType,

        /// Event message (markdown supported)
        message: String,

        /// Target task ID (optional, uses current focused task if not specified)
        #[arg(long)]
        task: Option<i64>,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Unified search across tasks and events
    ///
    /// Smart keyword detection:
    ///   - Query with ONLY status keywords (todo, doing, done) → filters by status
    ///   - Query with other words → uses FTS5 full-text search
    ///
    /// Status filter examples (returns tasks with matching status):
    ///   ie search "todo doing"     # All unfinished tasks (AI session start)
    ///   ie search "todo"           # Only todo tasks
    ///   ie search "done"           # Only completed tasks
    ///
    /// FTS5 search examples (full-text search):
    ///   ie search "JWT authentication"
    ///   ie search "API AND client"
    ///   ie search "blocker" --events --no-tasks
    Search {
        /// Search query: status keywords (todo/doing/done) or FTS5 syntax
        query: String,

        /// Search in tasks (default: true)
        #[arg(long, default_value = "true")]
        tasks: bool,

        /// Search in events (default: true)
        #[arg(long, default_value = "true")]
        events: bool,

        /// Maximum number of results (default: 20)
        #[arg(long)]
        limit: Option<i64>,

        /// Result offset for pagination (default: 0)
        #[arg(long)]
        offset: Option<i64>,
    },

    /// Initialize a new Intent-Engine project
    ///
    /// Creates a .intent-engine directory with database in the current working directory.
    ///
    /// Examples:
    ///   ie init                    # Initialize in current directory
    ///   ie init --at /my/project   # Initialize at specific directory
    Init {
        /// Custom directory to initialize (default: current directory)
        #[arg(long)]
        at: Option<String>,

        /// Re-initialize even if .intent-engine already exists
        #[arg(long)]
        force: bool,
    },

    /// Dashboard management commands
    #[command(subcommand)]
    Dashboard(DashboardCommands),

    /// Check system health and dependencies
    Doctor,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LogEventType {
    Decision,
    Blocker,
    Milestone,
    Note,
}

impl LogEventType {
    pub fn as_str(&self) -> &str {
        match self {
            LogEventType::Decision => "decision",
            LogEventType::Blocker => "blocker",
            LogEventType::Milestone => "milestone",
            LogEventType::Note => "note",
        }
    }
}

#[derive(Subcommand, Clone)]
pub enum DashboardCommands {
    /// Start the Dashboard server
    Start {
        /// Port to bind (default: 11391)
        #[arg(long)]
        port: Option<u16>,

        /// Auto-open browser
        #[arg(long)]
        browser: bool,

        /// Start in daemon mode (background)
        #[arg(long)]
        daemon: bool,
    },

    /// Stop the Dashboard server
    Stop {
        /// Stop all running dashboards
        #[arg(long)]
        all: bool,
    },

    /// Show Dashboard status
    Status {
        /// Show all instances
        #[arg(long)]
        all: bool,
    },

    /// List registered projects
    List,

    /// Open Dashboard in browser
    Open,
}
