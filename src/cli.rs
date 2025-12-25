use clap::{Parser, Subcommand};

const LONG_ABOUT: &str = r#"
Intent-Engine - AI 长期任务记忆系统

比 Claude Code 内置的 TodoWrite 多了什么？
  ✅ 跨 session 持久化（永远不会丢失）
  ✅ 层级任务树（父子关系、依赖）
  ✅ 决策记录（为什么这么做）
  ✅ Web Dashboard（可视化管理）

何时用 ie 而不是 TodoWrite？
  • 会丢了可惜 → 用 ie
  • 用完即弃 → 用 TodoWrite

AI 工作流：
  ie status   ← Session 开始时运行，恢复上下文
  ie plan     ← 声明式任务管理（创建/更新/完成）
  ie log      ← 记录决策、阻塞、里程碑
  ie search   ← 搜索任务和历史事件
"#;

#[derive(Parser, Clone)]
#[command(name = "intent-engine")]
#[command(about = "AI 长期任务记忆系统 - 跨 session 持久化、层级任务、决策记录")]
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
        #[arg(long, default_value = "json")]
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
