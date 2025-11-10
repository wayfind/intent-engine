# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
