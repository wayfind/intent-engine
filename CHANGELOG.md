# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
