# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.1] - TBD

### Added

- **Dashboard Notification Control**: Environment variable `IE_DISABLE_DASHBOARD_NOTIFICATIONS` to disable CLI→Dashboard notifications
  - Set to `1` or `true` (case-insensitive) to completely disable notifications
  - Useful for CI/CD pipelines, batch scripts, and users who don't use Dashboard UI
  - Zero performance overhead when disabled
- **New `log` Command**: Quick event logging for focused task or specific task
  - Usage: `ie log decision "message"`, `ie log blocker "message" --task 42`
  - Supports: decision, blocker, milestone, note
  - Replaces verbose `ie event add` syntax
- **Built-in Help System**: Detailed guides integrated into `--help`
  - `ie plan --help` shows 180-line comprehensive guide
  - `ie plan -h` shows quick reference (5 lines)
  - TodoWriter-style examples for easy migration
  - Common patterns, error handling, best practices
  - Follows Unix convention: `-h` (short) vs `--help` (detailed)

### Changed

- **BREAKING**: Disabled Dashboard auto-start by default
  - Dashboard will no longer automatically start in daemon mode on CLI commands
  - Users must manually start Dashboard with `ie dashboard start` or `ie dashboard start --daemon`
  - Improves CLI performance and reduces unnecessary background processes
- **BREAKING**: Simplified CLI to 6 essential commands for AI agents
  - **Core (3)**: `plan`, `log`, `search`
  - **System (3)**: `init`, `dashboard`, `doctor`
  - **Removed**: `task`, `event`, `report`, `current`, `setup`, `session-restore`, `guide`, and all hybrid commands
  - **Rationale**: CLI designed for AI agents (minimal, batch operations), humans use Dashboard UI (full CRUD)
  - **Help system**: Detailed guides moved to `--help` (Occam's razor: avoid unnecessary complexity)
- Reduced Dashboard notification timeout from 500ms to 100ms for faster CLI response

### Performance

- CLI commands skip notification logic entirely when `IE_DISABLE_DASHBOARD_NOTIFICATIONS` is enabled
- Faster timeout (500ms → 100ms) reduces overhead when Dashboard is offline
- No Dashboard auto-start overhead when Dashboard is not needed
- Batch script performance improved for users who don't need Dashboard UI updates
- Simplified CLI reduces binary size and maintenance overhead

### Tests

- **Test Suite Modernization**: Rewrote 4 test files from CLI-based to library-based testing (33 tests, ~550 lines reduced)
  - `pick_next_blocking_tests.rs` - 7 tests for dependency blocking (commit f2d5e0e)
  - `priority_and_list_tests.rs` - 8 tests for priority and filtering (commit 780147a)
  - `task_edge_cases_tests.rs` - 12 tests for error handling (commit d7f1240)
  - `task_start_blocking_tests.rs` - 6 tests for start validation (commit 7bb1c7b)
  - **Performance**: 10x faster execution (5s → 0.3s per file)
  - **Maintainability**: Tests no longer coupled to CLI interface changes
- **Feature Gate Fixes**: Added missing feature gate to `cli_special_chars_tests.rs` (commit 6972d61)
  - Prevents 10 test failures when removed CLI commands are not available
  - Consistent with other CLI test files using `#![cfg(feature = "test-removed-cli-commands")]`
- **Test Status**: 380 library tests + 33 rewritten tests = 413 total passing tests

### Documentation

- **Documentation Cleanup**: Removed 31 obsolete documentation files
  - Removed MCP Server documentation (deprecated in v0.10.0)
  - Removed WebSocket protocol documentation (superseded by HTTP notifications)
  - Removed legacy specification files (v0.2.x era)
  - Removed Phase 1 architecture documents (completed and superseded)
  - Removed temporary release planning documents (v0.10.0 completed)
  - Removed old release notes (v0.6.0 and earlier)
  - Current documentation focuses on v0.10.x+ features only

## [0.10.0] - 2025-12-16

### Added

- **Dashboard Auto-Start**: Dashboard automatically starts in daemon mode on any CLI command
  - Cross-platform support (Unix fork, Windows detached process)
  - PID file management with automatic stale cleanup
  - Health checks with 3-second timeout
- **Real-Time CLI → Dashboard Sync**: CLI operations trigger instant Dashboard UI updates
  - Fire-and-forget HTTP notifications (500ms timeout)
  - Non-blocking design for CLI operations
  - Dual notification pattern (CLI → HTTP, Dashboard → WebSocket)
- **Enhanced Help System**: Built-in AI guides via `ie guide` command
  - `ie guide ai` - AI integration patterns (345 lines)
  - `ie guide todo-writer` - TodoWriter migration guide
  - `ie guide workflow` - Core workflow patterns
  - `ie guide patterns` - Real-world usage examples
- **Embedded System Prompt**: 345-line AI guide compiled into binary
- **Migration Guide**: Comprehensive v0.9.x → v0.10.0 migration documentation

### Changed

- **BREAKING**: Replaced MCP server with embedded system prompt approach
  - Removed `ie mcp-server` command
  - Removed MCP configuration requirements
  - Zero configuration now required for Claude Code integration
- Updated architecture from MCP-based to system prompt-based
- Simplified installation: single binary, no external configuration
- Improved sync latency: < 100ms (was 1-2s with polling)

### Removed

- **BREAKING**: MCP (Model Context Protocol) server completely removed
  - `mcp-server.json` schema file
  - MCP setup and configuration commands
  - MCP-specific documentation
- Removed obsolete test: `test_spec_lists_all_mcp_tools`

### Documentation

- Created `MIGRATION_v0.10.0.md` - Migration guide from v0.9.x
- Created `RELEASE_NOTES_v0.10.0.md` - Detailed release notes
- Updated `README.md` - Replaced MCP section with Claude Code integration
- Updated `CLAUDE.md` - Version 0.10 with system prompt approach
- Updated `AGENT.md` - Removed MCP interface documentation
- Removed MCP server setup guides
- Removed MCP tools sync documentation

### Fixed

- Fixed interface spec tests to work without MCP schema file
- Resolved all compilation warnings in CLI notifier module

## [0.9.0] - 2025-12-01

### Documentation

- Update CHANGELOG for v0.8.6

### Features

- Simplify Human Task Protection to pure out-of-band confirmation

## [0.8.6] - 2025-12-01

### Documentation

- Update CHANGELOG for v0.8.5

### Features

- Display task ID in task tree

### Miscellaneous Tasks

- Bump version to 0.8.6

## [0.8.5] - 2025-12-01

### Bug Fixes

- Reset edit state when switching tasks

### Documentation

- Update CHANGELOG for v0.8.4

### Miscellaneous Tasks

- Bump version to 0.8.5

## [0.8.4] - 2025-11-30

### Bug Fixes

- Replace atty with std::io::IsTerminal

### Documentation

- Update CHANGELOG for v0.8.3

## [0.8.3] - 2025-11-30

### Bug Fixes

- Change CHANGELOG workflow to direct commit instead of PR

### Miscellaneous Tasks

- Bump version to 0.8.3

## [0.8.2] - 2025-11-30

### Bug Fixes

- Dashboard port binding address mismatch
- Rebuild frontend to fix dashboard search and task display
- Update tests for PaginatedTasks API and refactored doctor command

### Miscellaneous Tasks

- Bump version to 0.8.2

## [0.8.1] - 2025-11-28

### Bug Fixes

- Task_add priority support and remove --foreground references
- Import std::io::Read and fix clippy bool assertions

### Documentation

- Update MCP schema with pagination parameters

### Features

- Replace frontend v1 with v2 (renamed to front-end) and update dashboard server

### Miscellaneous Tasks

- Remove unstable dashboard_cli_tests.rs
- Bump version to 0.8.1

### Performance

- Optimize session restore when no task focused

### Refactor

- Remove dashboard daemon mode, always run in foreground

### Testing

- Improve test coverage for search, claude_code, and ws_client

## [0.8.0] - 2025-11-28

### Bug Fixes

- Resolve zombie green light and temp directory pollution
- Remove unused import in dashboard handlers

### Documentation

- Update spec to version 0.8

### Features

- Frontend V2 overhaul and UI refinements
- Refine UI (Operation Area & Logo)

### Miscellaneous Tasks

- Complete projects.json cleanup - remove all legacy registry references
- Add test files and update .gitignore
- Bump version to 0.8.0

### Refactor

- Streamline doctor command and remove obsolete registry.rs
- Extract get_status_badge() to eliminate 6 duplications
- Extract DASHBOARD_PORT as file-level constant
- Fix unsafe unwrap in check_mcp_connections
- Delete dead code (~138 lines removed)
- Unify notification logic with centralized NotificationSender
- Eliminate JSON config duplication via helper function
- Unify parse_duration into time_utils module
- Unify FTS5 escape logic into search module
- Centralize SQL query constants to reduce duplication
- Extract main.rs handlers to cli_handlers module + add unit tests
- Delete unused API functions (YAGNI cleanup)

### Testing

- Add 10 comprehensive tests (100% increase)
- Add 8 comprehensive tests (+160% coverage)

## [0.7.1] - 2025-11-27

### Bug Fixes

- Correct doing status semantics and pick_next priority logic

### Miscellaneous Tasks

- Bump version to 0.7.1
- Add .gitattributes for consistent line endings

## [0.6.10] - 2025-11-26

### Features

- Add ie init command with comprehensive tests

### Miscellaneous Tasks

- Bump actions/checkout from 5 to 6 (#102)
- Bump tokio-tungstenite from 0.21.0 to 0.24.0 (#104)
- Bump the production-dependencies group with 2 updates (#103)
- Remove Claude Code workflows
- Bump version to 0.6.10

## [0.6.9] - 2025-11-25

### Documentation

- Add security warnings for Dashboard 0.0.0.0 binding

### Features

- Bind Dashboard to 0.0.0.0 to allow external access

### Miscellaneous Tasks

- Bump version to 0.6.9

## [0.6.8] - 2025-11-25

### Bug Fixes

- Clippy error and mark cascade tests as planned feature
- Update dashboard test to use HTTP API instead of Registry file
- Parse API response data field in dashboard test

### Features

- UI optimization with WebSocket integration and enhanced logging system (#101)

### Miscellaneous Tasks

- Bump version to 0.6.8

## [0.6.7] - 2025-11-23

### Bug Fixes

- Support rotated log files in query operations

### Documentation

- Add missing documentation and test files

### Miscellaneous Tasks

- Bump version to 0.6.7

## [0.6.6] - 2025-11-23

### Bug Fixes

- Update integration tests for MCP server file logging and remove obsolete switch command test

### Documentation

- Add Intent-Engine protocol specification and migration plan

### Features

- Implement Intent-Engine Protocol v1.0 compliance (90%)

### Miscellaneous Tasks

- Bump version to 0.6.6

### Refactor

- Implement single source of truth for project status

## [0.6.5] - 2025-11-23

### Bug Fixes

- Canonicalize temp_dir for path comparison in Dashboard tests
- Resolve Dashboard integration test failures on macOS/Windows
- Set INTENT_ENGINE_PROJECT_DIR in Dashboard tests for macOS
- Use foreground mode for Dashboard in tests to fix macOS failures
- Add defensive programming to registry.rs save() method
- Add active_form column to schema and fix report queries

### Documentation

- Add comprehensive logging system documentation

### Features

- Add file logging infrastructure for Dashboard daemon mode (Phase 1)
- Implement Phase 1 - Dashboard file logging
- Implement Phase 2 - Log rotation and cleanup
- Enable file logging for MCP Server mode
- Add TodoWriter replacement with status management and active_form

### Miscellaneous Tasks

- Bump version to 0.6.5

### Refactor

- Implement multi-doing + single-focus design and remove task_switch

### Testing

- Add comprehensive integration tests for file logging
- Add comprehensive integration tests for Phase 2 and Phase 4

### Debug

- Add comprehensive Dashboard diagnostics for CI failures

## [0.6.4] - 2025-11-21

### Bug Fixes

- Cross-platform compatibility fixes for tests and CI
- Remove redundant /tmp path checks for cross-platform compatibility

### Miscellaneous Tasks

- Bump version to 0.6.4 - Cross-platform compatibility fixes

## [0.6.3] - 2025-11-21

### Bug Fixes

- Prevent MCP tests from registering temporary projects to Dashboard
- Downgrade temp path log messages to debug level to prevent MCP test failures

### Miscellaneous Tasks

- Bump version to 0.6.3 - MCP test fixes and temporary path protection

### Testing

- Add Dashboard WebSocket integration tests and fix temporary path pollution

## [0.6.2] - 2025-11-21

### Bug Fixes

- Add dashboard registry cleanup for v0.6.0 upgrade
- Add Cache-Control headers to prevent browser caching of Dashboard UI
- Exclude unimplemented tests from release script

### Miscellaneous Tasks

- Bump version to 0.6.1
- Update Cargo.lock for version 0.6.1
- Bump version to 0.6.2 - Dashboard upgrade fixes

## [0.6.0] - 2025-11-21

### Bug Fixes

- Add cwd field to MCP server configuration for proper project detection
- Integrate MCP → Dashboard WebSocket connection
- Check .intent-engine directory existence before loading database
- MCP test failures - downgrade WebSocket logs to debug level
- Exclude unimplemented tests from code coverage workflow
- Replace OpenSSL with rustls for ARM64 cross-compilation
- Revert workflow modifications that caused validation errors

### Documentation

- Clarify project boundary logic supports non-project startup
- Add plan tool to MCP tools table in spec

### Features

- Complete Dashboard UI redesign with sci-fi theme

### Miscellaneous Tasks

- Release v0.6.0 - Plan Interface and Dashboard Enhancements
- Add release notes for v0.6.0
- Add .claude to gitignore

## [0.5.5] - 2025-11-19

### Bug Fixes

- Dashboard daemon mode now properly detaches using setsid

### Documentation

- Update Dashboard port from dynamic allocation to fixed 11391

### Miscellaneous Tasks

- Bump version to 0.5.5

## [0.5.4] - 2025-11-19

### Bug Fixes

- 确保多项目注册表和界面数据一致性
- 修复MCP连接注册中的路径规范化问题
- MCP integration tests now use current project directory
- 修复CI/CD测试数据库初始化失败问题
- 修复 CI 并发测试中的目录切换竞争条件
- CI test failures - Windows port collision and doctor warnings
- Clean up test database before doctor command in CI
- Prevent Dashboard child process from blocking MCP server on Windows
- Disable Dashboard auto-start in test environments to prevent timeouts
- Handle port-in-use gracefully in test_allocate_port (Fix 6)
- Apply port-in-use graceful handling to test_fixed_port (Fix 6 complete)
- Initialize project before running coverage tests (Fix 7)
- Remove all eprintln! calls from MCP server to prevent Windows blocking

### Documentation

- 修复 rustdoc 警告 - 在文档中转义 HTML 标签

### Miscellaneous Tasks

- Add code coverage report
- Bump version to 0.5.4

### Refactor

- Rename MCP tool 'unified_search' to 'search'
- 本地化所有静态资源并优化UI设计
- 更新Dashboard UI为浅色主题并刷新静态资源

## [0.5.3] - 2025-11-17

### Features

- Add MCP auto-registration and browser auto-open for Dashboard

### Miscellaneous Tasks

- Bump which from 6.0.3 to 8.0.0 (#99)
- Bump tower from 0.4.13 to 0.5.2 (#98)
- Bump dirs from 5.0.1 to 6.0.0 (#94)
- Bump peter-evans/create-pull-request from 6 to 7 (#93)
- Bump actions/checkout from 4 to 5 (#92)
- Bump codecov/codecov-action from 4 to 5 (#91)
- Bump actions/labeler from 5 to 6 (#90)
- Bump version to 0.5.3

## [0.5.2] - 2025-11-17

### Bug Fixes

- Add PostToolUse Hook formatting for task mutation tools
- Resolve test failures caused by home project fallback
- Dashboard integration tests - 6/9 now passing
- Temporarily ignore 3 failing Dashboard tests to unblock CI
- Resolve CI test failures in dependency_tests
- Apply common test utilities to doctor_command_tests
- Comprehensive test suite migration to shared test utilities
- Migrate dashboard_integration_tests to use common test utilities
- Complete migration of mcp_integration_test to common utilities
- Dashboard tests failing in CI coverage environment

### Documentation

- Add optional UI tests to CI/CD workflow
- Add Dashboard documentation and test suite

### Features

- Add Dashboard web UI module

### Miscellaneous Tasks

- Bump version to 0.5.2

## [0.5.1] - 2025-11-16

### Bug Fixes

- PostToolUse hook JSON parsing and output mechanism
- Update spec file paths after docs reorganization
- Prevent nested projects from sharing databases

### Documentation

- Reorganize documentation with English as default and categorized naming

### Features

- Implement hybrid command model with optimized parameter syntax

### Miscellaneous Tasks

- Bump version to 0.4.1
- Bump version to 0.5.0 and update interface spec
- Bump version to 0.5.1

### Testing

- Add comprehensive nested project test matrix (17 new tests)

## [0.4.1] - 2025-11-15

### Bug Fixes

- Align CLI and MCP interface output formats

### Documentation

- Add AI feedback collection directory

## [0.4.0] - 2025-11-14

### Miscellaneous Tasks

- Bump version to 0.3.6

## [0.3.5] - 2025-11-14

### Miscellaneous Tasks

- Bump version to 0.3.5

## [0.3.4] - 2025-11-14

### Fix

- Serialize MCP integration tests to avoid env var races

### Miscellaneous Tasks

- Bump version to 0.3.4

## [0.3.3] - 2025-11-13

### Bug Fixes

- Address clippy warnings for cleaner code
- Replace map_or with is_some_and for clippy compliance
- Remove orphaned test code causing syntax error
- Remove unused imports and dead code
- Use CARGO_BIN_EXE for doctor test to fix CI failures
- Use CARGO_BIN_EXE in smart_initialization_tests for CI compatibility

### Features

- Add database path resolution diagnostics to doctor command

### Miscellaneous Tasks

- Bump version to 0.3.3

### Performance

- Optimize test_cli_help_matches_spec from 100s to 0.06s

### Testing

- Add comprehensive tests for database path diagnostics
- Add 17 additional tests for near-100% coverage

## [0.3.2] - 2025-11-13

### Bug Fixes

- Update MCP config path to ~/.claude.json for Claude Code v2.0.37+

### Features

- Add 'setup-mcp' command for automatic MCP server configuration

### Miscellaneous Tasks

- Bump version to 0.3.2

## [0.3.1] - 2025-11-13

### Bug Fixes

- Correct Claude Code config path from ~/.config/claude-code to ~/.claude/

### Miscellaneous Tasks

- Bump version to 0.3.1

## [0.3.0] - 2025-11-13

### Bug Fixes

- Add 'ie' binary alias and fix session-start hook
- Update MCP server version to 0.3

### Documentation

- Add Speckit Guardian integration protocol specification
- Update Speckit Guardian to v2.0 with phased approach
- Split Sub-Agent architecture into separate specification
- Add Phase 1 Focus Restoration implementation specification
- Add comprehensive Phase 1 testing specification
- Add Phase 1 implementation summary
- Update INTERFACE_SPEC to v0.3 with Phase 1 commands
- Add Phase 1 completion report

### Features

- Implement Phase 1 Focus Restoration (session-restore & setup-claude-code)

### Testing

- Add comprehensive unit tests for session_restore module
- Add Phase 1 integration tests for session restoration

## [0.2.1] - 2025-11-11

### Bug Fixes

- Update tests and MCP version for v0.2.0
- Implement standard FromStr trait for PriorityLevel
- Replace deprecated Command::cargo_bin with cargo::cargo_bin! macro

### Documentation

- 文档体系重构 Phase 1-2
- Add v0.2.0 requirement specification
- Supplement v0.2.0 spec with detailed technical specifications
- Update documentation for v0.2.0 release

### Features

- 版本同步系统实现 (Phase 3)
- Implement database migration and circular dependency detection for v0.2.0
- Add CLI depends-on command with comprehensive tests
- Add task start blocking check with comprehensive tests
- Filter blocked tasks from pick-next recommendations with comprehensive tests
- Enhance task_context with dependency information
- Add MCP task_add_dependency tool
- Implement Smart Event Querying with type and since filters
- Phase 3 - Priority Enum & Command Rename (P1)

### Miscellaneous Tasks

- Bump version to 0.2.1

## [0.1.17] - 2025-11-11

### Bug Fixes

- Fix MCP integration tests and remove duplicate tool definitions
- Use cargo_bin! macro for reliable test binary path resolution
- Ensure CLI commands inherit project directory in MCP integration tests
- Update mcp-server.json version to match Cargo.toml major.minor

### Documentation

- Add task_context MCP tool to INTERFACE_SPEC.md

### Features

- Implement task_context MCP tool

### Miscellaneous Tasks

- Fix critical inconsistencies and enhance AI prompting
- Configure MCP integration tests to run sequentially
- Bump version to 0.1.17

### Testing

- Add comprehensive test suite for task_context functionality
- Derive expected tool count from mcp-server.json instead of hard-coding
- Add debug output for CI diagnosis of task_context test failures

## [0.1.16] - 2025-11-10

### Miscellaneous Tasks

- Bump version to 0.1.16

## [0.1.15] - 2025-11-10

### Bug Fixes

- Add console input encoding support for Windows Chinese characters
- Add automatic GBK to UTF-8 conversion for Windows piped input
- Update SetConsoleCP API calls for windows crate 0.58

### Documentation

- Add CHANGELOG entry for smart lazy initialization feature

### Miscellaneous Tasks

- Bump version to 0.1.15

## [0.1.14] - 2025-11-10

### Bug Fixes

- Enable Release workflow trigger from Version Bump
- Use marker file for idempotent git hooks installation
- Update Windows API call to match new Result-based interface
- Add wrapper script to MCP server installer for working directory resolution
- Add MCP initialize method for proper handshake
- Address clippy warnings in test code
- Resolve clippy warnings in project.rs
- Suppress deprecated rand API warnings in performance tests
- Update Windows console API to use Result-based interface

### Documentation

- Add code formatting reminder to AGENT.md
- Add comprehensive build.rs design documentation
- Add comprehensive implementation summary report
- Update MCP server documentation to reflect Rust native implementation
- Enhance MCP tool descriptions with stdin usage and documentation guidance
- Update all documentation for unified binary architecture

### Features

- Add Windows console UTF-8 encoding support for Chinese characters
- Add build.rs to auto-install git hooks on first build
- Implement smart lazy initialization with project root inference
- Improve MCP server robustness per official specification
- Add task_search and task_delete MCP tools with comprehensive tests

### Miscellaneous Tasks

- Bump windows from 0.58.0 to 0.62.2
- Bump actions/stale from 9 to 10
- Bump orhun/git-cliff-action from 3 to 4
- Bump actions/checkout from 3 to 5
- Bump actions/download-artifact from 4 to 6
- Bump actions/github-script from 6 to 8
- Bump rand from 0.8.5 to 0.9.2
- Bump version to 0.1.14

### Refactor

- Enhance build.rs robustness and clean up documentation
- Remove Python dependency from MCP server install script
- Unify MCP server into single binary with environment variable support

### Styling

- Apply cargo fmt formatting

### Testing

- Fix Windows encoding tests to match actual API behavior
- Verify git hooks work
- Add comprehensive edge case tests for smart initialization
- Exclude 'initialize' from MCP tools sync test

## [0.1.13] - 2025-11-10

### Bug Fixes

- Enable publish to crates.io for workflow_dispatch
- Add input validation for workflow_dispatch tag parameter
- Change INTERFACE_SPEC version to reflect interface contract only

### Features

- Implement interface-contract-based version management system

### Miscellaneous Tasks

- Sync version to 0.1.12 across all files
- Bump version to 0.1.13

## [0.1.12] - 2025-11-10

### Miscellaneous Tasks

- Bump version to 0.1.12

## [0.1.11] - 2025-11-10

### Bug Fixes

- Add Cargo.lock for reproducible binary builds
- Handle existing tags in version bump workflow
- Use force push for tags to handle remote conflicts

### Miscellaneous Tasks

- Bump version to 0.1.10
- Remove unused reusable workflow files
- Update INTERFACE_SPEC.md version to 0.1.10
- Sync mcp-server.json version to 0.1.10
- Bump version to 0.1.11

### Refactor

- Comprehensive CI/CD system overhaul

## [.0.1.11] - 2025-11-10

### Bug Fixes

- Resolve immutable release error in GitHub Actions
- Remove needless borrow in test args
- Use GitHub API directly to delete releases
- Improve release deletion with better error handling

## [0.1.10] - 2025-11-09

### Bug Fixes

- Apply cargo fmt and fix clippy warnings
- 更新接口规范测试以匹配实际数据模型
- 修改实现以匹配 INTERFACE_SPEC.md 规范

### Documentation

- Add "Replace Intermediate Files" pattern to AI Quick Guide
- 整理安装脚本和文档结构
- 更新命令参考和 CI 文档中的安装脚本路径
- 强调 INTERFACE_SPEC.md 作为权威规范的基石作用
- 添加深度测试覆盖分析报告

### Features

- 从根本上解决代码格式化问题
- 添加灵活的手动构建系统
- 添加专门的Codecov工作流 - 一键触发代码覆盖率
- 添加 MCP 工具自动同步系统
- 添加权威接口规范文档系统

### Fix

- Add JSON-RPC version validation to eliminate dead code warning
- Resolve codecov workflow exit code 1 error
- Remove duplicate if condition in manual-build workflow

### Miscellaneous Tasks

- Remove unnecessary shebang from mcp-server.rs
- Specify default binary to resolve cargo run ambiguity
- Add permissions for PR comment posting in codecov workflow
- 更新 mcp-server.json 版本号至 0.1.9

### Refactor

- 删除废弃的 Python MCP 服务器
- 根据实际实现完全修正接口规范文档

### Styling

- 运行 cargo fmt 修复代码格式

### Testing

- Add comprehensive unit and integration tests to improve coverage
- Add comprehensive performance tests for large datasets
- Add missing coverage for get_task_with_events and pick_next_tasks

## [0.1.9] - 2025-11-08

### Bug Fixes

- Remove redundant wildcard pattern in format match

### Documentation

- 更新所有文档以反映 task done 命令的正确语义
- 修复 command-reference-full.md 中遗漏的 task done 语义
- Update pick-next documentation to reflect new functionality
- Update event add command documentation

### Features

- Implement intelligent pick-next command
- Make --task-id optional in event add command

### Styling

- Apply rustfmt formatting
- Apply rustfmt formatting to main.rs

## [0.1.8] - 2025-11-08

### Features

- 添加 --version 支持

## [0.1.7] - 2025-11-08

### Bug Fixes

- 修复 report 命令中 tasks_by_status 统计不一致的问题
- 修复 clippy doc_lazy_continuation lint 错误

### Documentation

- 文档结构优化 - 支持中英文双语和清晰导航
- 添加英文翻译和语言切换功能
- 添加核心文档英文翻译
- 翻译核心集成和理念文档
- 翻译安装指南和贡献指南
- 翻译技术文档(性能和安全)
- 翻译贡献者发布指南
- 完成剩余超长文档翻译 - 100%完成
- 更新所有文档以反映 task done 命令的语义变化
- 添加 task search 命令的文档说明
- 增强 FTS5 搜索引擎特性描述
- 添加开发脚本使用说明
- 在主要文档中添加 git hooks 设置说明
- 添加 v0.1.7 版本的 PR 描述文档

### Features

- 增强 task done 命令的响应结构，添加智能的下一步建议
- 实现 task search 命令支持全文搜索

### Miscellaneous Tasks

- 添加自动格式化工具和 git hooks
- Bump version to 0.1.7

### Refactor

- 重构 done 命令，只对当前焦点任务生效

### Styling

- 修复 rustfmt 代码格式问题

## [.0.1.1] - 2025-11-08

### Bug Fixes

- Ensure cargo bin directory is in PATH for install-scripts job
- Install OpenSSL development libraries for install-scripts job

### Documentation

- Add comprehensive CI/CD system overview
- Fix repository URLs from yourusername to wayfind

### Features

- Enhance CI/CD with comprehensive quality checks
- Add local CI check scripts

### Miscellaneous Tasks

- Update criterion requirement from 0.5 to 0.7
- Update sqlx requirement from 0.7 to 0.8
- Update thiserror requirement from 1.0 to 2.0
- Bump softprops/action-gh-release from 1 to 2
- Bump actions/upload-artifact from 3 to 5
- Bump actions/cache from 3 to 4
- Bump actions/dependency-review-action from 3 to 4
- Bump actions/checkout from 4 to 5

<!-- generated by git-cliff -->
