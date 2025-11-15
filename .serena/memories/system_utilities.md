# System Utilities and Tools

## Operating System
**Linux** (WSL2 on Windows)
- Kernel: 6.6.87.2-microsoft-standard-WSL2

## Standard Unix/Linux Commands Available

### File Operations
- `ls` - List directory contents
- `cd` - Change directory
- `cat` - Display file contents
- `find` - Search for files
- `grep` - Search text patterns
- `cp` - Copy files
- `mv` - Move/rename files
- `rm` - Remove files
- `mkdir` - Create directories

### Version Control
```bash
# Git commands
git status
git add
git commit
git push
git pull
git log
git diff
git branch
```

### Rust Toolchain
- `cargo` - Rust package manager and build tool
- `rustc` - Rust compiler
- `rustfmt` - Code formatter
- `clippy` - Linter
- `rust-analyzer` - LSP language server (installed at `/home/david/.cargo/bin/rust-analyzer`)

### Process Management
- `ps` - Process status
- `kill` - Terminate processes
- `top` / `htop` - Process monitoring

### Text Processing
- `sed` - Stream editor
- `awk` - Text processing
- `cut` - Cut sections from lines
- `sort` - Sort lines
- `uniq` - Report or filter repeated lines

### System Information
- `uname` - System information
- `df` - Disk space usage
- `du` - Directory space usage
- `free` - Memory usage

## Project-Specific Tools

### Intent-Engine CLI
After building, the `ie` binary is available:
```bash
# Via cargo
cargo run --bin ie -- <command>

# Direct binary
./target/debug/ie <command>
./target/release/ie <command>

# If installed globally
ie <command>
```

### Make Commands
```bash
make help          # Show available commands
make fmt           # Format code
make check         # Run format + clippy + tests
make test          # Run tests only
make setup-hooks   # Install git hooks
```

### Script Utilities
Located in `scripts/` directory:
- `setup-git-hooks.sh` - Install pre-commit hooks
- `ci-quick.sh` - Quick CI checks
- `ci-local.sh` - Full local CI simulation
- `sync-mcp-tools.sh` - Sync MCP tool definitions
