use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "intent-engine")]
#[command(about = "A command-line database service for tracking strategic intent", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Task management commands
    #[command(subcommand)]
    Task(TaskCommands),

    /// Workspace state management
    Current {
        /// Set the current task ID
        #[arg(long)]
        set: Option<i64>,
    },

    /// Generate analysis and reports
    Report {
        /// Time duration (e.g., "7d", "2h", "30m")
        #[arg(long)]
        since: Option<String>,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by name pattern (FTS5)
        #[arg(long)]
        filter_name: Option<String>,

        /// Filter by spec pattern (FTS5)
        #[arg(long)]
        filter_spec: Option<String>,

        /// Output format
        #[arg(long, default_value = "json")]
        format: String,

        /// Return summary only
        #[arg(long)]
        summary_only: bool,
    },

    /// Event logging commands
    #[command(subcommand)]
    Event(EventCommands),

    /// Check system health and dependencies
    Doctor,
}

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Add a new task
    Add {
        /// Task name
        #[arg(long)]
        name: String,

        /// Parent task ID
        #[arg(long)]
        parent: Option<i64>,

        /// Read spec from stdin
        #[arg(long)]
        spec_stdin: bool,
    },

    /// Get a task by ID
    Get {
        /// Task ID
        id: i64,

        /// Include events summary
        #[arg(long)]
        with_events: bool,
    },

    /// Update a task
    Update {
        /// Task ID
        id: i64,

        /// New task name
        #[arg(long)]
        name: Option<String>,

        /// New parent task ID
        #[arg(long)]
        parent: Option<i64>,

        /// New status
        #[arg(long)]
        status: Option<String>,

        /// Task complexity (1-10)
        #[arg(long)]
        complexity: Option<i32>,

        /// Task priority
        #[arg(long)]
        priority: Option<i32>,

        /// Read spec from stdin
        #[arg(long)]
        spec_stdin: bool,
    },

    /// Delete a task
    Del {
        /// Task ID
        id: i64,
    },

    /// Find tasks with filters
    Find {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by parent ID (use "null" for no parent)
        #[arg(long)]
        parent: Option<String>,
    },

    /// Start a task (atomic: update status + set current)
    Start {
        /// Task ID
        id: i64,

        /// Include events summary
        #[arg(long)]
        with_events: bool,
    },

    /// Complete a task (atomic: check children + update status)
    Done {
        /// Task ID
        id: i64,
    },

    /// Intelligently pick tasks from todo and move to doing
    PickNext {
        /// Maximum number of tasks to pick
        #[arg(long, default_value = "5")]
        max_count: usize,

        /// Maximum total tasks allowed in doing status
        #[arg(long, default_value = "5")]
        capacity: usize,
    },

    /// Create a subtask under current task and switch to it
    SpawnSubtask {
        /// Subtask name
        #[arg(long)]
        name: String,

        /// Read spec from stdin
        #[arg(long)]
        spec_stdin: bool,
    },

    /// Switch to a specific task (atomic: update to doing + set current)
    Switch {
        /// Task ID
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum EventCommands {
    /// Add a new event
    Add {
        /// Task ID
        #[arg(long)]
        task_id: i64,

        /// Log type
        #[arg(long = "type")]
        log_type: String,

        /// Read discussion data from stdin
        #[arg(long)]
        data_stdin: bool,
    },

    /// List events for a task
    List {
        /// Task ID
        #[arg(long)]
        task_id: i64,

        /// Maximum number of events to return
        #[arg(long)]
        limit: Option<i64>,
    },
}
