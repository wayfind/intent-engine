use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "intent-engine")]
#[command(about = "A command-line database service for tracking strategic intent", long_about = None)]
#[command(version)]
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

    /// Start MCP server for AI assistants (JSON-RPC stdio)
    #[command(name = "mcp-server")]
    McpServer,
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

        /// Task priority (critical, high, medium, low)
        #[arg(long)]
        priority: Option<String>,

        /// Read spec from stdin
        #[arg(long)]
        spec_stdin: bool,
    },

    /// Delete a task
    Del {
        /// Task ID
        id: i64,
    },

    /// List tasks with filters
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by parent ID (use "null" for no parent)
        #[arg(long)]
        parent: Option<String>,
    },

    /// Find tasks with filters (deprecated: use 'list' instead)
    #[command(hide = true)]
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

    /// Complete the current focused task (atomic: check children + update status + clear current)
    /// This command only operates on the current_task_id. It will:
    /// - Check all subtasks are done
    /// - Update the task status to done
    /// - Clear the current_task_id
    ///
    ///   Prerequisites: A task must be set as current (via `current --set <ID>`)
    Done,

    /// Intelligently recommend the next task to work on
    ///
    /// This command uses a context-aware priority model to recommend a single task:
    /// 1. First priority: Subtasks of the current focused task (depth-first)
    /// 2. Second priority: Top-level tasks (breadth-first)
    ///
    /// The command is non-interactive and does not modify task status.
    PickNext {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
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

    /// Search tasks by content using full-text search
    Search {
        /// Search query (supports FTS5 syntax like "bug AND NOT critical")
        query: String,
    },

    /// Add a dependency between tasks
    ///
    /// Creates a dependency where BLOCKED_TASK depends on BLOCKING_TASK.
    /// The BLOCKING_TASK must be completed before BLOCKED_TASK can be started.
    ///
    /// Example: `task depends-on 42 41` means Task 42 depends on Task 41
    /// (Task 41 must be done before Task 42 can start)
    #[command(name = "depends-on")]
    DependsOn {
        /// Task ID that has the dependency (blocked task)
        blocked_task_id: i64,

        /// Task ID that must be completed first (blocking task)
        blocking_task_id: i64,
    },
}

#[derive(Subcommand)]
pub enum EventCommands {
    /// Add a new event
    Add {
        /// Task ID (optional, uses current task if not specified)
        #[arg(long)]
        task_id: Option<i64>,

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

        /// Filter by log type (e.g., "decision", "blocker", "milestone", "note")
        #[arg(long = "type")]
        log_type: Option<String>,

        /// Filter events created within duration (e.g., "7d", "24h", "30m")
        #[arg(long)]
        since: Option<String>,
    },
}
