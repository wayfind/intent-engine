# Smart Lazy Initialization Design

**Feature**: Intelligent Project Root Inference
**Version**: 0.1
**Status**: Implemented
**Last Updated**: 2025-11-10

---

## Overview

Intent-Engine implements a smart lazy initialization mechanism that automatically detects the project root directory and initializes the system transparently on the first write operation. This design eliminates the need for manual initialization commands and provides a seamless user experience.

## Design Philosophy

### Core Principles

1. **Transparency**: Users should never need to run an `init` command manually
2. **Intelligence**: System should understand project structure, not just use CWD
3. **Predictability**: Initialization location should be deterministic and logical
4. **Fail-Safe**: System should have a sensible fallback when project markers are absent

### User Experience Goals

- **Zero Configuration**: Works out of the box with no setup required
- **Context-Aware**: Understands different project types (Git, Node.js, Rust, Python, etc.)
- **Consistent Location**: `.intent-engine` always appears at the project root, regardless of where commands are executed
- **Helpful Feedback**: Clear warnings when falling back to non-standard locations

## Algorithm Design

### High-Level Flow

```
Write Command Executed
        ↓
    Check for existing .intent-engine
        ↓
    ┌───Exists?───┐
    ↓             ↓
   Yes           No
    ↓             ↓
Use existing   Infer root
               └→ Initialize
```

### Root Inference Algorithm

#### Phase 1: Marker Definition

The system maintains a prioritized list of project root markers:

| Priority | Marker          | Project Type      | Reason                                    |
|----------|-----------------|-------------------|-------------------------------------------|
| 1        | `.git`          | Git VCS           | Most universal, highest confidence        |
| 2        | `.hg`           | Mercurial VCS     | Version control system                    |
| 3        | `package.json`  | Node.js           | Standard Node.js project file             |
| 4        | `Cargo.toml`    | Rust              | Rust package manifest                     |
| 5        | `pyproject.toml`| Python            | Modern Python project standard (PEP 518)  |
| 6        | `go.mod`        | Go                | Go modules file                           |
| 7        | `pom.xml`       | Java (Maven)      | Maven project file                        |
| 8        | `build.gradle`  | Java/Kotlin       | Gradle build file                         |

**Rationale for Priority Order**:
- Version control markers (`.git`, `.hg`) have highest priority because they typically mark the true project root
- Language-specific markers follow, ordered by prevalence in modern development

#### Phase 2: Upward Traversal

```rust
fn infer_project_root() -> Option<PathBuf> {
    let cwd = current_dir()?;
    let mut current = cwd.clone();

    loop {
        // Check each marker in priority order
        for marker in PROJECT_ROOT_MARKERS {
            if current.join(marker).exists() {
                return Some(current);  // First match wins
            }
        }

        // Move up one level
        if !current.pop() {
            break;  // Reached filesystem root
        }
    }

    None  // No marker found
}
```

**Key Behaviors**:
1. **Depth-First Search**: Check all markers at current level before moving up
2. **First-Match Wins**: Stop immediately when any marker is found
3. **Priority Within Level**: Higher-priority markers checked first at each level
4. **Filesystem Root Detection**: `Path::pop()` returns false at filesystem root

#### Phase 3: Initialization

```rust
pub async fn initialize_project() -> Result<Self> {
    let cwd = current_dir()?;

    let root = match infer_project_root() {
        Some(inferred_root) => {
            // Success path: use inferred root
            inferred_root
        }
        None => {
            // Fallback path: use CWD and warn
            eprintln!(
                "Warning: Could not determine a project root...\n\
                 Initialized Intent-Engine in the current directory '{}'.",
                cwd.display()
            );
            cwd
        }
    };

    // Create .intent-engine directory and database
    let intent_dir = root.join(INTENT_DIR);
    fs::create_dir_all(&intent_dir)?;

    let pool = create_pool(&intent_dir.join(DB_FILE)).await?;
    run_migrations(&pool).await?;

    Ok(ProjectContext { root, db_path, pool })
}
```

## Implementation Details

### File Structure

```
project_root/
├── .git/                    # Project marker (highest priority)
├── .intent-engine/          # Created by initialization
│   └── project.db          # SQLite database
├── src/
│   └── components/
│       └── [user runs command here]
```

### Command Classification

**Write Commands** (trigger initialization):
- `task add`
- `task update`
- `task del`
- `task start`
- `task done`
- `task spawn-subtask`
- `task switch`
- `event add`
- `current --set`

**Read Commands** (require existing project):
- `task get`
- `task find`
- `task search`
- `event list`
- `current` (without --set)
- `report`

### Error Handling

| Scenario                        | Behavior                                      | Exit Code |
|---------------------------------|-----------------------------------------------|-----------|
| No markers found                | Initialize in CWD with warning                | 0         |
| Permission denied               | Fail with filesystem error                    | 1         |
| Read command, no project        | Return NOT_A_PROJECT error                    | 1         |
| Write command, project exists   | Use existing project                          | 0         |

## Testing Strategy

### Unit Tests

Located in `src/project.rs`:

1. **Marker List Validation**
   - Verify all expected markers are present
   - Verify priority order (`.git` first)

2. **Constants Testing**
   - Verify `INTENT_DIR` and `DB_FILE` constants

### Integration Tests

Located in `tests/smart_initialization_tests.rs`:

1. **Single Marker Tests**
   - Test with `.git` marker
   - Test with `Cargo.toml` marker
   - Test with `package.json`, `pyproject.toml`, `go.mod`, etc.

2. **Priority Tests**
   - Multiple markers: verify highest priority wins
   - Nested markers: verify closest ancestor with marker wins

3. **Edge Cases**
   - No markers: verify fallback to CWD with warning
   - Deep nesting: verify traversal to root
   - Existing `.intent-engine`: verify no re-initialization

4. **Cross-Platform Tests**
   - Unix paths: `/home/user/project`
   - Windows paths: `C:\Users\user\project` (via CI)

## Examples

### Example 1: Standard Git Project

```bash
$ tree -a -L 2
.
├── .git/
├── src/
│   ├── main.rs
│   └── lib.rs
└── Cargo.toml

$ cd src
$ ie task add --name "Implement feature X"
# ✓ .intent-engine created at project root (parent of src)
```

### Example 2: Monorepo Structure

```bash
$ tree -a -L 3
.
├── .git/                    # Root marker
├── backend/
│   ├── Cargo.toml          # Rust service
│   └── src/
└── frontend/
    ├── package.json        # Node.js app
    └── src/

$ cd backend/src
$ ie task add --name "Add API endpoint"
# ✓ .intent-engine created at repository root (not backend/)
```

### Example 3: Scripts Directory (No Markers)

```bash
$ tree -a
.
├── cleanup.sh
└── deploy.sh

$ ie task add --name "Refactor cleanup script"
# ⚠ Warning printed to stderr
# ✓ .intent-engine created in current directory
```

## Future Enhancements

### Potential Improvements

1. **Configurable Markers**
   - Allow `.intent-engine.toml` to define custom markers
   - Support regex patterns for markers

2. **Multi-Project Workspaces**
   - Detect monorepo structures
   - Support multiple `.intent-engine` instances

3. **Explicit Initialization**
   - Add optional `init --here` command for explicit control
   - Useful for edge cases where inference fails

4. **Marker Detection Improvements**
   - Check file contents (e.g., verify `package.json` is valid JSON)
   - Support platform-specific markers (e.g., `.xcodeproj` for iOS)

## Related Documentation

- **Specification**: `docs/INTERFACE_SPEC.md` (Section 1.3)
- **User Guide**: `docs/en/guide/quickstart.md`
- **AI Integration**: `CLAUDE.md`
- **Implementation**: `src/project.rs`

## Change History

- **2025-11-10**: Initial implementation and documentation
- **2024-11-09**: Design specification created
