# Codebase Structure

## Top-Level Directories

```
intent-engine/
├── src/              - Source code
├── tests/            - Integration and unit tests
├── docs/             - Documentation
├── scripts/          - Development and CI scripts
├── benches/          - Performance benchmarks
├── templates/        - Template files
├── examples/         - Example code
├── .github/          - GitHub Actions CI/CD
├── .claude/          - Claude Code integration
└── .intent-engine/   - Project's own task database
```

## Source Code (src/)

### Main Files
- **main.rs** - Entry point, main() function, command routing
- **lib.rs** - Library entry point, exports public API
- **cli.rs** - CLI command definitions (clap derive)
- **error.rs** - Error types and Result alias
- **tasks.rs** - Task management (TaskManager)
- **events.rs** - Event management (EventManager)
- **workspace.rs** - Workspace/focus management (WorkspaceManager)
- **report.rs** - Report generation (ReportManager)
- **search.rs** - Full-text search functionality
- **project.rs** - Project context and initialization
- **dependencies.rs** - Task dependency system
- **priority.rs** - Task priority handling
- **session_restore.rs** - Session restoration for AI agents
- **windows_console.rs** - Windows UTF-8 console support
- **test_utils.rs** - Test utilities

### Subdirectories
- **src/db/** - Database layer
  - models.rs - Data models (Task, Event, etc.)
  - schema.rs - Database schema and migrations
  
- **src/mcp/** - MCP (Model Context Protocol) server
  - MCP tool definitions
  - JSON-RPC protocol handling
  
- **src/setup/** - Setup commands
  - Claude Code integration setup
  - Hook installation

## Tests (tests/)

- **cli_tests.rs** - CLI command integration tests
- **integration_tests.rs** - Full integration tests
- **mcp_integration_test.rs** - MCP server tests
- **dependency_tests.rs** - Task dependency tests
- **priority_and_list_tests.rs** - Priority and filtering tests
- **special_chars_tests.rs** - Special character handling
- **performance_tests.rs** - Performance benchmarks
- **interface_spec_test.rs** - Interface specification validation
- **integration/** - Additional integration test modules

## Documentation (docs/)

- **INTERFACE_SPEC.md** - Authoritative interface specification (CLI/MCP/API)
- **zh-CN/** - Chinese documentation
  - guide/ - User guides
  - integration/ - Integration guides
  - technical/ - Technical documentation
  - contributing/ - Contributor guides
- **en/** - English documentation (if exists)

## Scripts (scripts/)

- **setup-git-hooks.sh** - Install pre-commit hooks
- **sync-mcp-tools.sh** - Sync MCP tool definitions
- **ci-*.sh** - CI-related scripts
- **install/** - Installation scripts
- **test/** - Test utilities
- **git-hooks/** - Git hook implementations

## Key Files

- **Cargo.toml** - Rust package manifest and dependencies
- **Cargo.lock** - Locked dependency versions
- **rustfmt.toml** - Code formatting configuration
- **Makefile** - Development convenience commands
- **CLAUDE.md** - AI assistant integration guide
- **README.md** - Project README (Chinese)
- **README.en.md** - Project README (English)
