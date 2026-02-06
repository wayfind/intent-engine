# Design: `ie task` CRUD Commands

**Version**: 0.11 (Proposed)
**Date**: 2026-02-06
**Status**: Draft
**Parent Task**: #175

---

## 1. Motivation

### Problem: ie plan is the only task mutation interface

Currently, **all** task creation/update/deletion in intent-engine goes through `ie plan`, which requires constructing JSON and piping it through stdin:

```bash
echo '{"tasks":[{"name":"Fix bug","status":"doing","spec":"..."}]}' | ie plan
```

This is:
- **Verbose** for single-task operations (most common case)
- **Error-prone** (JSON escaping, quoting)
- **Unfriendly** for both humans and AI tools

### Reference: Claude Code Task Management

Claude Code provides 4 simple tools for task management:

| Tool | Purpose | Interface |
|------|---------|-----------|
| `TaskCreate` | Create one task | `(subject, description, activeForm)` |
| `TaskGet` | Get one task | `(taskId)` → full details + deps |
| `TaskUpdate` | Update one task | `(taskId, status?, subject?, description?, owner?, metadata?, addBlocks?, addBlockedBy?)` |
| `TaskList` | List all tasks | `()` → summary array |

Key characteristics:
- **Individual operations** - one task per call
- **Simple parameters** - no JSON construction needed
- **Metadata support** - arbitrary key-value storage
- **Dependency management** - `addBlocks`/`addBlockedBy` by task ID
- **Flexible owner** - any agent name string

---

## 2. Gap Analysis

### What Claude Code has that ie lacks

| Feature | Claude Code | ie Current |
|---------|------------|------------|
| Single-task CRUD | Direct tool params | JSON stdin batch only |
| Metadata (KV store) | `metadata: {key: val}` | Not supported |
| Flexible owner | Any string (`"agent-1"`) | `human` or `ai` only |
| Deps by ID on update | `addBlocks: [id]` | `depends_on: ["name"]` at plan-time only |
| Delete by update | `status: "deleted"` | `delete: true` flag in plan |
| Simple list-all | `TaskList()` no params | `ie search "todo doing"` workaround |

### What ie has that Claude Code lacks

| Feature | ie | Claude Code |
|---------|------|------------|
| Hierarchical tasks | Parent/children nesting | Flat only |
| Spec (execution contract) | Goal + approach markdown | Just description |
| Event/decision logging | `ie log` 4 event types | None |
| Full-text search | `ie search` with FTS5 | None |
| Cross-session persistence | SQLite database | In-memory only |
| Priority levels | critical/high/medium/low | None |
| Web Dashboard | Real-time UI | None |
| Focus system | Current task per session | No concept |
| Rich context | Ancestors/siblings/descendants | blocks/blockedBy only |
| Lifecycle timestamps | first_todo_at/doing_at/done_at | None |
| Batch operations | `ie plan` atomic batch | Individual only |

---

## 3. Interface Design

### 3.1 New `ie task` Subcommands

All commands output JSON by default. Add `--format text` for human-readable output.

#### `ie task create <name>`

Create a single task.

```
ie task create <NAME> [OPTIONS]

Arguments:
  <NAME>                Task name (required)

Options:
  -d, --description <TEXT>    Task description/spec (markdown)
      --parent <ID>           Parent task ID (default: current focused task, use 0 for root)
      --priority <LEVEL>      Priority: critical|high|medium|low
      --active-form <TEXT>    Present continuous display text (e.g. "Fixing bug")
      --blocked-by <IDS>      Comma-separated IDs of blocking tasks
      --blocks <IDS>          Comma-separated IDs of tasks this blocks
      --metadata <KV>         Metadata as key=value pairs (comma-separated)
      --owner <NAME>          Task owner (default: "ai")
      --status <STATUS>       Initial status: todo|doing (default: todo)
      --format <FMT>          Output format: json|text (default: json)
```

**Output** (JSON):
```json
{
  "id": 42,
  "name": "Fix login bug",
  "status": "todo",
  "description": null,
  "parent_id": null,
  "priority": null,
  "active_form": null,
  "owner": "ai",
  "metadata": {},
  "blocked_by": [],
  "blocks": []
}
```

**Mapping to Claude Code**:
| Claude Code `TaskCreate` | `ie task create` |
|--------------------------|------------------|
| `subject` | `<NAME>` (positional) |
| `description` | `--description` / `-d` |
| `activeForm` | `--active-form` |
| (N/A - always pending) | `--status` (default: todo) |
| (N/A) | `--parent`, `--priority`, `--metadata`, `--owner`, `--blocked-by`, `--blocks` |

---

#### `ie task get <ID>`

Get full task details.

```
ie task get <ID> [OPTIONS]

Arguments:
  <ID>                  Task ID (required)

Options:
  -e, --with-events     Include event history
  -c, --with-context    Include family tree (ancestors/siblings/descendants)
      --format <FMT>    Output format: json|text (default: json)
```

**Output** (JSON):
```json
{
  "id": 42,
  "name": "Fix login bug",
  "status": "doing",
  "description": "Login fails with 500 error on expired sessions",
  "parent_id": 40,
  "priority": "high",
  "active_form": "Fixing login bug",
  "owner": "agent-1",
  "metadata": {"component": "auth", "severity": "P1"},
  "blocked_by": [{"id": 41, "name": "DB migration", "status": "done"}],
  "blocks": [{"id": 43, "name": "Deploy release", "status": "todo"}],
  "first_todo_at": "2026-02-06T10:00:00Z",
  "first_doing_at": "2026-02-06T10:05:00Z",
  "first_done_at": null,
  "events": [],
  "context": null
}
```

With `--with-events`:
```json
{
  ...
  "events": [
    {"id": 1, "log_type": "decision", "discussion_data": "Root cause: session TTL mismatch", "timestamp": "..."}
  ]
}
```

With `--with-context`:
```json
{
  ...
  "context": {
    "ancestors": [{"id": 40, "name": "Auth System", "status": "doing"}],
    "siblings": [{"id": 41, "name": "DB migration", "status": "done"}],
    "descendants": [{"id": 44, "name": "Add retry logic", "status": "todo"}]
  }
}
```

**Mapping to Claude Code**:
| Claude Code `TaskGet` | `ie task get` |
|----------------------|---------------|
| `taskId` | `<ID>` (positional) |
| Returns: blocks/blockedBy | `blocked_by`, `blocks` |
| (N/A) | `--with-events`, `--with-context` (ie power features) |

---

#### `ie task update <ID>`

Update a single task.

```
ie task update <ID> [OPTIONS]

Arguments:
  <ID>                        Task ID (required)

Options:
      --name <NAME>           New name
  -d, --description <TEXT>    New description/spec
      --status <STATUS>       New status: todo|doing|done
      --priority <LEVEL>      Priority: critical|high|medium|low
      --active-form <TEXT>    Present continuous display text
      --add-blocks <IDS>      Add task IDs this task blocks
      --add-blocked-by <IDS>  Add task IDs that block this task
      --rm-blocks <IDS>       Remove blocking relationships
      --rm-blocked-by <IDS>   Remove blocked-by relationships
      --metadata <KV>         Merge metadata (key=value, use key= to delete)
      --owner <NAME>          Change owner
      --parent <ID>           Move to new parent (0 for root)
      --format <FMT>          Output format: json|text (default: json)
```

**Output**: Same as `ie task get` (returns updated task).

**Setting status to `doing`**: Automatically sets task as current focus (same as `ie task start`).

**Setting status to `done`**: Validates all children are complete first.

**Mapping to Claude Code**:
| Claude Code `TaskUpdate` | `ie task update` |
|--------------------------|------------------|
| `taskId` | `<ID>` (positional) |
| `status` (pending/in_progress/completed/deleted) | `--status` (todo/doing/done) + delete via `ie task delete` |
| `subject` | `--name` |
| `description` | `--description` / `-d` |
| `activeForm` | `--active-form` |
| `owner` | `--owner` |
| `metadata` | `--metadata` |
| `addBlocks` | `--add-blocks` |
| `addBlockedBy` | `--add-blocked-by` |
| (N/A) | `--priority`, `--parent`, `--rm-blocks`, `--rm-blocked-by` |

---

#### `ie task list`

List tasks with filtering.

```
ie task list [OPTIONS]

Options:
      --status <STATUS>   Filter by status: todo|doing|done
      --parent <ID>       Filter by parent (0 for roots)
      --owner <NAME>      Filter by owner
      --tree              Tree view (hierarchical)
      --format <FMT>      Output format: json|text (default: json)
```

**Output** (JSON):
```json
[
  {
    "id": 42,
    "name": "Fix login bug",
    "status": "doing",
    "parent_id": 40,
    "owner": "agent-1",
    "blocked_by": [41]
  },
  ...
]
```

**Mapping to Claude Code**:
| Claude Code `TaskList` | `ie task list` |
|------------------------|----------------|
| (no params) | Filters: `--status`, `--parent`, `--owner` |
| Returns: id, subject, status, owner, blockedBy | Same + parent_id |

---

#### `ie task delete <ID>`

Delete a task.

```
ie task delete <ID> [OPTIONS]

Arguments:
  <ID>                  Task ID (required)

Options:
      --cascade         Also delete all descendants
      --format <FMT>    Output format: json|text (default: json)
```

**Note**: Claude Code uses `TaskUpdate(status: "deleted")`. ie provides a dedicated delete command for clarity.

---

#### `ie task start <ID>` (convenience)

Start working on a task (sets doing + focus).

```
ie task start <ID> [OPTIONS]

Arguments:
  <ID>                        Task ID (required)

Options:
  -d, --description <TEXT>    Set description when starting
      --format <FMT>          Output format: json|text (default: json)
```

Equivalent to: `ie task update <ID> --status doing`

---

#### `ie task done [ID]` (convenience)

Complete a task.

```
ie task done [ID] [OPTIONS]

Arguments:
  [ID]                  Task ID (default: current focused task)

Options:
      --format <FMT>    Output format: json|text (default: json)
```

**Output includes next task suggestion:**
```json
{
  "completed_task": {"id": 42, "name": "Fix login bug", "status": "done"},
  "next_suggestion": {"id": 43, "name": "Deploy release", "reason": "sibling_task"}
}
```

---

#### `ie task next` (convenience)

Pick next available task.

```
ie task next [OPTIONS]

Options:
      --format <FMT>    Output format: json|text (default: json)
```

Same behavior as existing `task pick-next`.

---

### 3.2 Status Mapping

| Claude Code | ie | Notes |
|------------|-----|-------|
| `pending` | `todo` | Initial state |
| `in_progress` | `doing` | Active work, sets focus |
| `completed` | `done` | Children must be done first |
| `deleted` | `ie task delete` | Separate command |

---

### 3.3 Metadata System

New `metadata` field on tasks for arbitrary key-value storage.

**Storage**: TEXT column containing JSON object.

**CLI Interface**:
```bash
# Set metadata
ie task create "My task" --metadata component=auth,severity=P1

# Update metadata (merge)
ie task update 42 --metadata priority=urgent

# Delete metadata key
ie task update 42 --metadata severity=

# Read metadata
ie task get 42   # metadata field in output
```

**Use Cases**:
- AI agent tagging (component, module, type)
- Priority annotations beyond the 4-level system
- Custom workflow fields

---

### 3.4 Owner System

Expanded `owner` field allowing any string identifier.

**Current**: `"human"` | `"ai"` (enum)
**New**: Any string (e.g., `"agent-1"`, `"david"`, `"ci-bot"`)

**Backward Compatible**: `"human"` and `"ai"` still work as before.

---

## 4. Data Model Changes

### 4.1 New Column: `metadata`

```sql
ALTER TABLE tasks ADD COLUMN metadata TEXT DEFAULT '{}';
```

### 4.2 Owner Field

No schema change needed - `owner` is already TEXT. Only need to remove the `human`/`ai` enum validation in Rust code.

### 4.3 Updated Task Struct

```rust
pub struct Task {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub spec: Option<String>,          // description maps here
    pub status: String,
    pub complexity: Option<i32>,
    pub priority: Option<i32>,
    pub first_todo_at: Option<DateTime>,
    pub first_doing_at: Option<DateTime>,
    pub first_done_at: Option<DateTime>,
    pub active_form: Option<String>,
    pub owner: String,                 // Now accepts any string
    pub metadata: Option<String>,      // NEW: JSON string
}
```

---

## 5. Backward Compatibility

### `ie plan` continues to work

- `ie plan` is NOT removed, only deprecated as primary interface
- All existing `ie plan` JSON schemas remain valid
- `ie plan` can optionally gain `metadata` support in TaskTree

### Migration path

```
Before (ie plan):
  echo '{"tasks":[{"name":"Fix bug","status":"doing"}]}' | ie plan

After (ie task):
  ie task create "Fix bug" --status doing
```

### System prompt update

Recommend `ie task` commands in system prompts. Keep `ie plan` documented for batch operations.

---

## 6. Test Plan

### 6.1 Unit Tests

| Test Case | Command | Verify |
|-----------|---------|--------|
| Create minimal | `ie task create "Test"` | Returns id, status=todo |
| Create full | `ie task create "Test" -d "desc" --parent 1 --priority high` | All fields set |
| Create with metadata | `ie task create "Test" --metadata k=v` | metadata populated |
| Create with deps | `ie task create "Test" --blocked-by 1,2` | Dependencies created |
| Create with doing | `ie task create "Test" --status doing` | Sets as current focus |
| Get basic | `ie task get 1` | Full task returned |
| Get with events | `ie task get 1 -e` | Events included |
| Get with context | `ie task get 1 -c` | Family tree included |
| Update name | `ie task update 1 --name "New"` | Name changed |
| Update status | `ie task update 1 --status doing` | Status + focus set |
| Update metadata | `ie task update 1 --metadata k=v` | Metadata merged |
| Update deps | `ie task update 1 --add-blocked-by 2` | Dep added |
| List all | `ie task list` | All tasks returned |
| List filtered | `ie task list --status todo` | Only todo tasks |
| List tree | `ie task list --tree` | Hierarchical output |
| Delete basic | `ie task delete 1` | Task removed |
| Delete cascade | `ie task delete 1 --cascade` | Descendants removed |
| Start | `ie task start 1` | Status=doing, focus set |
| Done focused | `ie task done` | Current task completed |
| Done by id | `ie task done 1` | Specific task completed |
| Done blocked | `ie task done 1` (children undone) | Error returned |
| Next | `ie task next` | Suggestion returned |

### 6.2 Integration Tests

| Scenario | Steps | Verify |
|----------|-------|--------|
| Full workflow | create → start → done → next | Each step correct |
| Hierarchy | create parent, create child --parent, list --tree | Tree correct |
| Dependencies | create A, create B --blocked-by A, start B (blocked) | Block enforced |
| Metadata CRUD | create --metadata, update --metadata, get | KV correct |
| Owner flow | create --owner agent-1, list --owner agent-1 | Filter works |
| Mixed with plan | ie plan + ie task commands | Both work together |

### 6.3 Regression Tests

| Test | Verify |
|------|--------|
| ie plan still works | Existing JSON input processed correctly |
| ie status reflects task changes | Focus/context updated |
| ie search finds new tasks | FTS index updated |
| ie log works with task tasks | Events attached correctly |
| Dashboard notifications | Changes trigger dashboard updates |

---

## 7. Implementation Order

1. **Design doc** (this document) ← Current
2. **Data model**: Add `metadata` column, relax `owner` validation
3. **CLI definition**: Add `task` subcommand group to clap
4. **TaskManager enhancement**: New methods + metadata/owner support
5. **CLI handlers**: Connect CLI commands to TaskManager
6. **Tests**: Unit + integration + regression
7. **Documentation**: Spec update, system prompt, CLAUDE.md

---

## 8. Open Questions

1. **Should `--description` map to `spec` or a new field?**
   Decision: Map to `spec`. The field is the same concept - task description/specification.
   In output JSON, use `description` as alias for `spec` for Claude Code compatibility.

2. **Auto-parenting behavior for `ie task create`?**
   Decision: Default parent = current focused task (consistent with `ie plan`).
   Use `--parent 0` to explicitly create at root level.

3. **Should `ie plan` be formally deprecated?**
   Decision: Soft deprecation. Keep functional, add deprecation notice to help text,
   recommend `ie task` in documentation. No removal timeline.

---

*End of Design Document*
