use clap::{Parser, Subcommand};

const LONG_ABOUT: &str = r#"
Intent-Engine - AI Long-Term Task Memory System

What does it offer beyond Claude Code's built-in TodoWrite?
  ✅ Cross-session persistence (never lost)
  ✅ Hierarchical task trees (parent-child, dependencies)
  ✅ Decision logs (why you made choices)
  ✅ Web Dashboard (visual management)

When to use ie instead of TodoWrite?
  • Would be a shame to lose it → use ie
  • Use once and discard → use TodoWrite

AI Workflow:
  ie status   ← Run at session start to restore context
  ie plan     ← Declarative task management (create/update/complete)
  ie log      ← Record decisions, blockers, milestones
  ie search   ← Search tasks and event history

Key Rules:
  • status:doing requires spec (description)
  • status:done requires all children complete first
  • parent_id:null creates independent root task

Documentation:
  • User Guide: docs/help/user-guide.md
  • System Prompt: docs/system_prompt.md
  • Interface Spec: docs/spec-03-interface-current.md
"#;

#[derive(Parser, Clone)]
#[command(name = "intent-engine")]
#[command(
    about = "AI Long-Term Task Memory - Cross-session persistence, hierarchical tasks, decision logs"
)]
#[command(long_about = LONG_ABOUT)]
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

        /// Filter by start time (e.g., "7d", "1w", "2025-01-01")
        #[arg(long)]
        since: Option<String>,

        /// Filter by end time (e.g., "1d", "2025-12-31")
        #[arg(long)]
        until: Option<String>,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
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

    /// Show current task context (focus spotlight)
    ///
    /// Displays the focused task with its complete context:
    /// - Current task details (full info)
    /// - Ancestors chain (full info)
    /// - Siblings (id + name + status)
    /// - Descendants (id + name + status + parent_id)
    ///
    /// Examples:
    ///   ie status              # Show current focused task context
    ///   ie status 42           # Show task 42's context (without changing focus)
    ///   ie status -e           # Include event history
    Status {
        /// Task ID to inspect (optional, defaults to current focused task)
        task_id: Option<i64>,

        /// Include event history
        #[arg(short = 'e', long)]
        with_events: bool,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// CRUD operations on individual tasks
    ///
    /// Direct task manipulation commands for creating, reading, updating,
    /// and deleting tasks, plus workflow shortcuts (start, done, next).
    ///
    /// Examples:
    ///   ie task create "Implement auth" --description "JWT-based auth"
    ///   ie task get 42 --with-context
    ///   ie task list --status todo
    ///   ie task start 42
    ///   ie task done
    ///   ie task next
    #[command(subcommand)]
    Task(TaskCommands),

    /// Configuration management (key-value store)
    ///
    /// Manage configuration settings stored in the project database.
    /// Used for LLM endpoints, API keys, and other project-level settings.
    ///
    /// Examples:
    ///   ie config set llm.endpoint "http://localhost:8080/v1/chat/completions"
    ///   ie config get llm.api_key
    ///   ie config list --prefix llm
    ///   ie config unset llm.model
    #[command(subcommand)]
    Config(ConfigCommands),
}

#[derive(Subcommand, Clone)]
pub enum ConfigCommands {
    /// Set a configuration value
    ///
    /// Examples:
    ///   ie config set llm.endpoint "http://localhost:8080/v1/chat/completions"
    ///   ie config set llm.api_key "sk-your-key"
    Set {
        /// Configuration key (e.g., llm.endpoint)
        key: String,

        /// Configuration value
        value: String,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Get a configuration value
    ///
    /// Sensitive values (api_key, secret) are automatically masked.
    ///
    /// Examples:
    ///   ie config get llm.endpoint
    ///   ie config get llm.api_key    # Shows masked value
    Get {
        /// Configuration key
        key: String,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// List configuration entries
    ///
    /// Examples:
    ///   ie config list
    ///   ie config list --prefix llm
    List {
        /// Filter by key prefix (e.g., "llm" shows all llm.* keys)
        #[arg(long)]
        prefix: Option<String>,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Remove a configuration entry
    ///
    /// Examples:
    ///   ie config unset llm.model
    Unset {
        /// Configuration key to remove
        key: String,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand, Clone)]
pub enum TaskCommands {
    /// Create a new task
    ///
    /// Examples:
    ///   ie task create "Implement auth"
    ///   ie task create "Add tests" --description "Unit + integration tests" --parent 42
    ///   ie task create "Fix bug" --status doing --priority 1
    Create {
        /// Task name
        name: String,

        /// Task description/spec (markdown supported)
        #[arg(short, long)]
        description: Option<String>,

        /// Parent task ID (0 = root task, omit = auto-parent to current focus)
        #[arg(short, long)]
        parent: Option<i64>,

        /// Initial status (default: todo)
        #[arg(short, long, default_value = "todo")]
        status: String,

        /// Priority (1=critical, 2=high, 3=medium, 4=low)
        #[arg(long)]
        priority: Option<i32>,

        /// Task owner (default: human)
        #[arg(long, default_value = "human")]
        owner: String,

        /// Metadata key=value pairs (e.g., --metadata type=epic --metadata tag=auth)
        #[arg(long)]
        metadata: Vec<String>,

        /// IDs of tasks that block this task (this task depends on them)
        #[arg(long = "blocked-by")]
        blocked_by: Vec<i64>,

        /// IDs of tasks that this task blocks (they depend on this task)
        #[arg(long)]
        blocks: Vec<i64>,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Get task details
    ///
    /// Examples:
    ///   ie task get 42
    ///   ie task get 42 --with-events
    ///   ie task get 42 --with-context
    Get {
        /// Task ID
        id: i64,

        /// Include event history
        #[arg(short = 'e', long)]
        with_events: bool,

        /// Include full context (ancestors, siblings, children, dependencies)
        #[arg(short = 'c', long)]
        with_context: bool,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Update an existing task
    ///
    /// Examples:
    ///   ie task update 42 --name "New name"
    ///   ie task update 42 --description "Updated spec" --priority 1
    ///   ie task update 42 --status doing
    ///   ie task update 42 --metadata type=epic --metadata "key="  (delete key)
    Update {
        /// Task ID
        id: i64,

        /// New task name
        #[arg(long)]
        name: Option<String>,

        /// New description/spec
        #[arg(short, long)]
        description: Option<String>,

        /// New status (todo, doing, done)
        #[arg(short, long)]
        status: Option<String>,

        /// New priority (1=critical, 2=high, 3=medium, 4=low)
        #[arg(long)]
        priority: Option<i32>,

        /// New active form text
        #[arg(long)]
        active_form: Option<String>,

        /// New owner
        #[arg(long)]
        owner: Option<String>,

        /// New parent task ID (0 = make root task)
        #[arg(long)]
        parent: Option<i64>,

        /// Metadata key=value pairs to merge (key= to delete)
        #[arg(long)]
        metadata: Vec<String>,

        /// Add dependency: this task is blocked by these task IDs
        #[arg(long = "add-blocked-by")]
        add_blocked_by: Vec<i64>,

        /// Add dependency: this task blocks these task IDs
        #[arg(long = "add-blocks")]
        add_blocks: Vec<i64>,

        /// Remove dependency: remove blocked-by relationship
        #[arg(long = "rm-blocked-by")]
        rm_blocked_by: Vec<i64>,

        /// Remove dependency: remove blocks relationship
        #[arg(long = "rm-blocks")]
        rm_blocks: Vec<i64>,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// List tasks with optional filters
    ///
    /// Examples:
    ///   ie task list
    ///   ie task list --status todo
    ///   ie task list --parent 42
    ///   ie task list --tree
    List {
        /// Filter by status (todo, doing, done)
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by parent ID (0 = root tasks only)
        #[arg(short, long)]
        parent: Option<i64>,

        /// Sort by (id, priority, time, focus_aware)
        #[arg(long)]
        sort: Option<String>,

        /// Maximum number of results
        #[arg(long)]
        limit: Option<i64>,

        /// Result offset for pagination
        #[arg(long)]
        offset: Option<i64>,

        /// Show as hierarchical tree
        #[arg(long)]
        tree: bool,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Delete a task
    ///
    /// Examples:
    ///   ie task delete 42
    ///   ie task delete 42 --cascade
    Delete {
        /// Task ID
        id: i64,

        /// Also delete all descendant tasks
        #[arg(long)]
        cascade: bool,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Start working on a task (sets status to doing and focuses it)
    ///
    /// Examples:
    ///   ie task start 42
    ///   ie task start 42 --description "Starting with validation layer"
    Start {
        /// Task ID
        id: i64,

        /// Update description before starting
        #[arg(short, long)]
        description: Option<String>,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Mark a task as done
    ///
    /// Examples:
    ///   ie task done         # Complete current focused task
    ///   ie task done 42      # Focus task 42 then complete it
    Done {
        /// Task ID (optional, defaults to current focused task)
        id: Option<i64>,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Suggest the next task to work on
    ///
    /// Uses context-aware priority: subtasks of focused task first,
    /// then top-level tasks, based on priority ordering.
    ///
    /// Examples:
    ///   ie task next
    Next {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
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
