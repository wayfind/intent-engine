# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.7] - 2025-11-23

### Fixed
- **Critical**: Fixed log rotation file discovery in `ie logs` command
  - `list_log_files()` now correctly identifies rotated files (e.g., `dashboard.log.2025-11-23`)
  - `query_logs()` now includes all rotated log files when filtering by mode
  - Previous behavior: `ie logs --mode dashboard` would return empty results as it only read the main (often empty) log file
  - Impact: Log query functionality is now fully operational for historical logs
- **Code Quality**: Fixed Clippy warnings
  - Use `strip_suffix()` instead of manual string slicing in `parse_duration()`
  - Use struct initialization pattern in `handle_logs_command()`
  - Added `#[allow(dead_code)]` for unused helper functions in tests

### Added
- **Documentation**: Complete logging system guide (`docs/logging-guide.md`, 458 lines)
  - Comprehensive usage examples for `ie logs` command
  - Log rotation and cleanup instructions
  - Troubleshooting guide and FAQ
  - Common use cases with copy-paste examples
- **Deployment**: Production-ready logrotate configuration (`docs/deployment/logrotate.conf`)
  - Daily rotation with 7-day retention
  - Automatic compression of old logs
  - SIGHUP signal handling for graceful log file reopening

### Tests
- All 24 logging tests passing (100% coverage)
  - `logging_integration_test.rs`: 6 tests for basic file logging
  - `logging_rotation_test.rs`: 6 tests for rotation and cleanup
  - `logs_integration_test.rs`: 12 tests for query functionality

## [0.6.6] - 2025-11-23

### Added
- **Unified Logging System**: Comprehensive file-based logging for all modes
  - Dashboard daemon mode logs to `~/.intent-engine/logs/dashboard.log`
  - MCP Server logs to `~/.intent-engine/logs/mcp-server.log` (JSON format)
  - Automatic log directory creation
  - Environment variable `IE_DASHBOARD_LOG_FILE=1` to force file logging (for testing)
- **Log Rotation**: Built-in daily rotation with automatic cleanup
  - Uses `tracing-appender` for cross-platform daily rotation
  - Creates dated files: `dashboard.log.2025-11-23`
  - Automatic cleanup of logs older than 7 days (configurable via `IE_LOG_RETENTION_DAYS`)
  - Recommended: Use `logrotate` on Linux for production (config provided)
- **Log Query Command**: New `ie logs` CLI command for querying historical logs
  - Filter by mode: `--mode dashboard|mcp-server|cli`
  - Filter by level: `--level error|warn|info|debug|trace`
  - Filter by time: `--since 1h|24h|7d`
  - Limit results: `--limit N`
  - Real-time monitoring: `--follow` (like `tail -f`)
  - Export formats: `--export text|json`
- **MCP Server Logging**: Dual output mechanism
  - Logs written to file in JSON format
  - JSON-RPC communication remains clean on stdout
  - Prevents log noise in AI assistant interactions

### Dependencies
- Added `tracing-appender = "0.2"` for log rotation support

## [0.6.0] - 2025-11-21

### Added
- **Plan Interface**: Declarative task management API for batch operations
  - New `ie plan` CLI command and `plan` MCP tool
  - Create entire task trees in one atomic operation
  - Hierarchical nesting via `children` field (no manual parent_id needed)
  - Name-based dependency references via `depends_on` field
  - Automatic dependency resolution and cycle detection (Tarjan's SCC algorithm)
  - Idempotent updates: run same plan multiple times → same result
  - Transaction-based atomicity: all-or-nothing execution
  - Comprehensive guide: `docs/PLAN_INTERFACE_GUIDE.md`
- **MCP WebSocket Integration**: Real-time connection between Dashboard and MCP server
  - WebSocket-based communication for live updates
  - Automatic reconnection with exponential backoff
  - Enhanced dashboard registry for tracking active sessions
- **Dashboard UI Redesign**: Complete sci-fi themed interface overhaul
  - Modern, futuristic design aesthetic
  - Improved user experience and visual hierarchy
  - Enhanced task visualization

### Changed
- **Dependencies**: Added WebSocket and async communication support
  - `axum` now includes `ws` feature for WebSocket support
  - Added `tokio-tungstenite` 0.21 for WebSocket client
  - Added `futures-util` 0.3 for async stream handling
  - Added `reqwest` with JSON support for HTTP communication
- **Dashboard**: Fixed port allocation to 11391 (previously dynamic)
- **MCP Server Schema**: Added `plan` tool to mcp-server.json with comprehensive documentation

### Fixed
- **MCP → Dashboard WebSocket Connection**: Resolved cross-session connection failures
  - Improved process lifecycle management
  - Better PID file and registry synchronization
  - Enhanced health check mechanisms
- **Project Boundary Logic**: Clarified support for non-project startup scenarios
- **Dashboard Daemon Mode**: Proper process detachment using `setsid` on Unix systems

### Documentation
- **New Guide**: `docs/PLAN_INTERFACE_GUIDE.md` - Comprehensive plan interface documentation with examples
- **Updated AGENT.md**: Added Plan Interface section (v0.6) with technical details and usage patterns
- **Updated CLAUDE.md**: Enhanced with plan tool guidance and when to use batch vs imperative operations
- **Updated README.md**: Added declarative task planning section highlighting v0.6 features

### Migration Notes
- **Plan Interface** is backward compatible - existing commands work as before
- For batch task creation, consider migrating to `plan` interface for better ergonomics
- Phase 1 (v0.6.0) is create-only; idempotent updates coming in v0.6.1

## [0.4.0] - 2025-11-14

### Added
- **Unified Search**: New `ie search` command searches across both tasks and events
  - Full-text search using FTS5 in both task specs/names and event discussion_data
  - Mixed results with task and event variants in a tagged union structure
  - Event results include full task ancestry chain for hierarchical context
  - Configurable search scope: tasks only, events only, or both (default)
  - Snippet highlighting with `**` markers for matched keywords
  - New `UnifiedSearchResult` data model with discriminated union
- **Global Event Queries**: `event list` now supports omitting task_id
  - Query events across all tasks with `--type` and `--since` filters
  - Useful for finding all recent blockers, decisions, or notes globally
  - Default limit changed from 100 to 50 for better performance
- **Enhanced MCP Parameter Validation**: Improved error messages for `task_add`
  - Specific messages distinguish between missing, null, empty, and wrong-type parameters
  - Error message examples:
    - "Missing required parameter: name"
    - "Parameter 'name' cannot be null"
    - "Parameter 'name' cannot be empty"
    - "Parameter 'name' must be a string, got: 123"
  - Makes debugging MCP tool calls much easier

### Changed
- **CLI Commands**: Replaced `ie task search` with `ie search` (top-level command)
- **MCP Tools**: Replaced `task_search` with `unified_search` tool
- **Event List**: `task_id` parameter is now optional (breaking for strict type checkers)
- **Atomic Commands Hidden**: Low-level atomic commands are now hidden from `--help` to guide users toward safer composite commands
  - `ie current set` and `ie current clear` are hidden (prefer `ie task start` and `ie task done`)
  - These commands still work but show deprecation warnings
  - MCP tools never exposed these atomic operations (already aligned)
  - Design principle: Expose only commands that ensure business logic consistency

### Removed
- **CLI**: `ie task search` command (use `ie search` instead)
- **MCP**: `task_search` tool (use `unified_search` instead)

### Fixed
- Improved parameter validation error messages for better debugging
- **PostToolUse Hook Cross-Platform Compatibility**:
  - Fixed macOS Bash 3.2 compatibility: Replaced `${var:offset:length}` syntax with POSIX-compliant `head -c`
  - Enhanced jq path detection with multi-platform fallback (macOS Intel/M1, Linux, Git Bash, custom installations)
  - Hook now works on macOS default Bash (3.2.57) without requiring Bash 4+ upgrade
  - Improved jq discovery for Homebrew installations on both Intel (`/usr/local/bin`) and Apple Silicon (`/opt/homebrew/bin`)

### Documentation
- Updated CLAUDE.md to version 0.4 with unified_search examples
- Updated INTERFACE_SPEC.md with version 0.4 changelog and new search section
- Added comprehensive documentation for unified search in both files
- Updated all usage patterns and examples to use new search command

### Migration Guide
- Replace all `ie task search` calls with `ie search`
- Replace all `task_search` MCP tool calls with `unified_search`
- Event results now include `task_chain` array for ancestry context
- When using `event_list`, `task_id` is now optional (omit for global queries)

## [0.3.x] - 2025-11-13

### Added
- **Smart Lazy Initialization**: Automatic project root detection and initialization
  - Intelligently infers project root by detecting common markers (.git, Cargo.toml, package.json, etc.)
  - Supports 8 project types: Git, Mercurial, Node.js, Rust, Python, Go, Maven, Gradle
  - Transparent initialization on first write operation - no manual `init` command needed
  - Works seamlessly from any subdirectory within a project
  - Priority-based marker detection (.git has highest priority)
  - Fallback to current directory with warning if no markers found
  - Comprehensive test coverage with 22 integration tests
  - **Monorepo support**: Each sub-project gets isolated `.intent-engine` directory
  - **Edge cases handled**: Symlinks, Git submodules, concurrent initialization, partial states
- **Enhanced Documentation**:
  - Complete implementation design document (`docs/en/technical/smart-initialization.md`)
  - Edge cases analysis document with 50+ scenarios (`docs/en/technical/smart-initialization-edge-cases.md`)
  - Updated INTERFACE_SPEC.md with Section 1.3: Project Initialization and Smart Root Inference
  - Implementation summary report with detailed statistics and examples
  - Updated quickstart guides (English and Chinese) with smart initialization explanation

### Changed
- Project initialization now automatically detects root directory instead of using CWD
- `.intent-engine` directory is created at project root rather than current directory
- Initialization behavior is now consistent across different project structures

### Fixed
- Fixed clippy warnings in `src/project.rs` (unused imports, const checks)
- Fixed deprecated rand API warnings in performance tests
- Fixed Windows console API compatibility with updated `windows` crate
  - Updated `SetConsoleOutputCP` to use Result-based interface

### Technical
- Total test coverage: 240 tests (all passing)
- New integration tests: +22 smart initialization tests
- Platform support: Linux, macOS, Windows (with platform-specific symlink handling)
- Performance: ~1-3ms overhead for root detection (negligible)
- Zero breaking changes - fully backward compatible

## [0.1.7] - 2025-11-08

### Added
- **FTS5 Full-text Search**: New `task search <QUERY>` command with millisecond-level performance
  - Uses SQLite FTS5 for blazing-fast full-text search
  - Returns match snippets with `**` highlighting for matched keywords
  - Supports advanced query syntax (AND, OR, NOT, prefix matching, phrase search)
  - Extremely Agent-context-friendly with ~64 character context snippets
- **Smart Next-step Suggestions**: Enhanced `task done` command now provides intelligent suggestions
  - Suggests switching to parent task after completing subtask
  - Recommends picking next task when current task is done
  - Helps maintain workflow momentum
- **Development Automation Tools**:
  - Git pre-commit hooks for automatic code formatting
  - Makefile with convenient development commands (`make fmt`, `make check`, `make test`)
  - Setup script: `./scripts/setup-git-hooks.sh`
- **Enhanced Documentation**:
  - Development setup guide in README and QUICKSTART
  - Comprehensive `task search` command documentation
  - Git hooks installation instructions for contributors
  - Scripts usage documentation in `scripts/README.md`

### Changed
- **`task done` Command Refactored**: Now only operates on current focused task
  - Clearer semantics: complete the task you're working on
  - More intuitive workflow with automatic task switching suggestions
  - Updated all documentation to reflect new behavior

### Fixed
- Fixed `report` command `tasks_by_status` statistics inconsistency
- Fixed clippy `doc_lazy_continuation` lint error in documentation comments
- Fixed rustfmt formatting issues through automated git hooks

### Documentation
- Added prominent git hooks setup instructions to README.md
- Enhanced FTS5 search engine feature description highlighting performance and Agent-friendliness
- Complete Chinese and English documentation for `task search` command
- Added AI Quick Guide references for search functionality

## [0.1.6] - 2024-XX-XX

### Changed
- Optimized CI pipeline for faster execution
- Improved scheduled CI to run only when there are new commits

## [0.1.5] - 2024-XX-XX

### Changed
- Simplified CI configuration
- Improved release process

## [0.1.4] - 2024-XX-XX

### Added
- Multiple package manager support (cargo-binstall, Homebrew)
- Comprehensive installation documentation
- Release testing guide

## [0.1.3] - 2024-XX-XX

### Changed
- Updated GitHub Actions workflows
- Improved artifact handling

## [0.1.2] - 2024-XX-XX

### Added
- Initial stable release
- Core task management functionality
- Event tracking system
- SQLite database backend

[0.1.7]: https://github.com/wayfind/intent-engine/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/wayfind/intent-engine/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/wayfind/intent-engine/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/wayfind/intent-engine/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/wayfind/intent-engine/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/wayfind/intent-engine/releases/tag/v0.1.2
