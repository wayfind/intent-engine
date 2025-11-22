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
    /// Task management commands
    #[command(subcommand)]
    Task(TaskCommands),

    /// Workspace state management
    Current {
        /// Set the current task ID (for backward compatibility)
        #[arg(long)]
        set: Option<i64>,

        #[command(subcommand)]
        command: Option<CurrentAction>,
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

    /// Create or update task structures declaratively
    ///
    /// Reads a JSON plan from stdin and creates/updates tasks atomically.
    /// Supports hierarchical nesting and name-based dependencies.
    ///
    /// Example:
    ///   echo '{"tasks": [{"name": "Task A", "children": [{"name": "Task B"}]}]}' | ie plan
    Plan {
        /// Show what would be created without actually doing it
        #[arg(long)]
        dry_run: bool,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Event logging commands
    #[command(subcommand)]
    Event(EventCommands),

    /// Unified search across tasks and events
    Search {
        /// Search query (supports FTS5 syntax like "JWT AND authentication")
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
    },

    /// Check system health and dependencies
    Doctor,

    /// Start MCP server for AI assistants (JSON-RPC stdio)
    #[command(name = "mcp-server")]
    McpServer,

    /// Restore session context for AI agents (Focus Restoration - Phase 1)
    ///
    /// This command returns all context needed to restore work continuity:
    /// - Current focused task
    /// - Parent task and siblings progress
    /// - Child tasks status
    /// - Recent events (decisions, blockers, notes)
    /// - Suggested next commands
    ///
    /// Designed for SessionStart hooks to inject context at the beginning of new sessions.
    #[command(name = "session-restore")]
    SessionRestore {
        /// Number of recent events to include (default: 3)
        #[arg(long, default_value = "3")]
        include_events: usize,

        /// Workspace path (default: current directory)
        #[arg(long)]
        workspace: Option<String>,
    },

    /// Dashboard management commands
    #[command(subcommand)]
    Dashboard(DashboardCommands),

    /// Unified setup command for AI tool integrations
    ///
    /// This command provides a unified interface for setting up intent-engine integration
    /// with various AI assistant tools. It handles both hook installation and MCP server
    /// configuration in one step.
    ///
    /// Features:
    /// - User-level or project-level installation
    /// - Atomic setup with rollback on failure
    /// - Built-in connectivity testing
    /// - Diagnosis mode for troubleshooting
    Setup {
        /// Target tool to configure (claude-code, gemini-cli, codex)
        #[arg(long)]
        target: Option<String>,

        /// Installation scope: user (default, recommended), project, or both
        #[arg(long, default_value = "user")]
        scope: String,

        /// Show what would be done without actually doing it
        #[arg(long)]
        dry_run: bool,

        /// Overwrite existing configuration
        #[arg(long)]
        force: bool,

        /// Run diagnosis on existing setup instead of installing
        #[arg(long)]
        diagnose: bool,

        /// Custom config file path (advanced)
        #[arg(long)]
        config_path: Option<String>,

        /// Project directory for INTENT_ENGINE_PROJECT_DIR env var
        #[arg(long)]
        project_dir: Option<String>,
    },

    // ========================================
    // Hybrid Commands - High-Frequency Aliases
    // ========================================
    // These are convenience aliases for the most common operations.
    // They provide a verb-centric, streamlined UX for frequent actions.
    // See: docs/architecture-03-cli-hybrid-command.md
    /// Add a new task (alias for 'task add')
    #[command(alias = "a")]
    Add {
        /// Task name
        name: String,

        /// Detailed specification (markdown)
        #[arg(long)]
        spec: Option<String>,

        /// Parent task ID
        #[arg(long)]
        parent: Option<i64>,

        /// Priority (critical, high, medium, low)
        #[arg(long)]
        priority: Option<String>,
    },

    /// Start a task and set focus (alias for 'task start')
    #[command(alias = "s")]
    Start {
        /// Task ID
        id: i64,

        /// Include events summary
        #[arg(long, default_value = "true")]
        with_events: bool,
    },

    /// Complete the current focused task (alias for 'task done')
    #[command(alias = "d")]
    Done,

    /// Record an event for current task (alias for 'event add')
    Log {
        /// Event type (decision, blocker, milestone, note)
        #[arg(value_parser = ["decision", "blocker", "milestone", "note"])]
        event_type: String,

        /// Event data/description
        data: String,

        /// Task ID (optional, uses current task if not specified)
        #[arg(long)]
        task_id: Option<i64>,
    },

    /// Get the next recommended task (alias for 'task pick-next')
    #[command(alias = "n")]
    Next {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// List tasks with filters (alias for 'task list')
    ///
    /// Examples:
    ///   ie ls              # List all tasks
    ///   ie ls todo         # List todo tasks
    ///   ie ls doing        # List doing tasks
    ///   ie ls done         # List done tasks
    #[command(alias = "ls")]
    List {
        /// Filter by status (todo, doing, done)
        status: Option<String>,

        /// Filter by parent ID (use "null" for no parent)
        #[arg(long)]
        parent: Option<String>,
    },

    /// Get task context (alias for 'task context')
    #[command(alias = "ctx")]
    Context {
        /// Task ID (optional, uses current task if omitted)
        task_id: Option<i64>,
    },

    /// Get task details (alias for 'task get')
    Get {
        /// Task ID
        id: i64,

        /// Include events summary
        #[arg(long)]
        with_events: bool,
    },
}

#[derive(Subcommand, Clone)]
pub enum CurrentAction {
    /// Set the current task (low-level atomic command, prefer 'ie task start')
    #[command(hide = true)]
    Set {
        /// Task ID to set as current
        task_id: i64,
    },

    /// Clear the current task (low-level atomic command, prefer 'ie task done')
    #[command(hide = true)]
    Clear,
}

#[derive(Subcommand, Clone)]
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
    ///
    /// Examples:
    ///   ie task list              # List all tasks
    ///   ie task list todo         # List todo tasks
    ///   ie task list doing        # List doing tasks
    ///   ie task list done         # List done tasks
    List {
        /// Filter by status (todo, doing, done)
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

    /// Get task context (ancestors, siblings, children)
    ///
    /// Shows the full family tree of a task to understand its strategic context.
    /// If no task ID is provided, uses the current focused task.
    Context {
        /// Task ID (optional, uses current task if omitted)
        task_id: Option<i64>,
    },
}

#[derive(Subcommand, Clone)]
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

    /// List events for a task (or globally if task_id not specified)
    List {
        /// Task ID (optional, lists all events if not specified)
        #[arg(long)]
        task_id: Option<i64>,

        /// Maximum number of events to return (default: 50)
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

#[derive(Subcommand, Clone)]
pub enum DashboardCommands {
    /// Start the Dashboard web server for current project
    Start {
        /// Custom port (default: 11391)
        #[arg(long)]
        port: Option<u16>,

        /// Run in foreground (default: daemon mode)
        #[arg(long)]
        foreground: bool,

        /// Automatically open browser (default: false, use --browser to enable)
        #[arg(long)]
        browser: bool,
    },

    /// Stop the Dashboard server
    Stop {
        /// Stop all Dashboard instances
        #[arg(long)]
        all: bool,
    },

    /// Show Dashboard status
    Status {
        /// Show status for all projects
        #[arg(long)]
        all: bool,
    },

    /// List all registered Dashboard instances
    List,

    /// Open Dashboard in browser
    Open,
}
