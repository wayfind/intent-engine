# Plan Command - Complete Guide

Reads a JSON plan from stdin and creates/updates tasks atomically.
Supports hierarchical nesting, explicit parent assignment, dependencies, and status management.

## Quick Start

```bash
# Simplest example - create a task
echo '{"tasks":[{"name":"Implement user login"}]}' | ie plan

# TodoWriter style - status management
echo '{
  "tasks": [
    {"name": "Design database schema", "status": "done"},
    {"name": "Implement API", "status": "doing", "active_form": "Implementing API"},
    {"name": "Write tests", "status": "todo"}
  ]
}' | ie plan
```

## Core Features

### 1. Hierarchical Tasks (children)
```bash
echo '{
  "tasks": [{
    "name": "User Authentication",
    "status": "doing",
    "children": [
      {"name": "JWT generation", "status": "done"},
      {"name": "Login API", "status": "doing"},
      {"name": "Token validation", "status": "todo"}
    ]
  }]
}' | ie plan
```

### 2. Explicit Parent Assignment (parent_id)

Control task hierarchy explicitly using `parent_id`:

```bash
# Create a root task (ignores focused task auto-parenting)
echo '{"tasks":[{"name":"Independent Task","parent_id":null}]}' | ie plan

# Assign to specific parent by ID
echo '{"tasks":[{"name":"Child Task","parent_id":42}]}' | ie plan

# Move existing task to new parent
echo '{"tasks":[{"name":"Existing Task","parent_id":99}]}' | ie plan
```

**Three-state logic:**
- `parent_id` absent → Default behavior (auto-parent to focused task)
- `parent_id: null` → Explicitly create as root task
- `parent_id: 42` → Explicitly set parent to task #42

**Priority:** `children` nesting > `parent_id` > auto-parenting

### 3. Dependencies (depends_on)
```bash
echo '{
  "tasks": [
    {"name": "Design API"},
    {"name": "Backend", "depends_on": ["Design API"]},
    {"name": "Frontend", "depends_on": ["Design API"]},
    {"name": "Integration", "depends_on": ["Backend", "Frontend"]}
  ]
}' | ie plan
```

### 4. Idempotent Updates (by name)
```bash
# Run 1 - create
echo '{"tasks":[{"name":"Login","status":"todo"}]}' | ie plan

# Run 2 - update status
echo '{"tasks":[{"name":"Login","status":"doing"}]}' | ie plan

# Run 3 - mark done
echo '{"tasks":[{"name":"Login","status":"done"}]}' | ie plan
```

### 5. Delete Tasks (delete)

Delete tasks by ID - no name required:

```bash
# Delete a task by ID only
echo '{"tasks":[{"id":42,"delete":true}]}' | ie plan

# Mixed operations in one request: create, update, delete
echo '{
  "tasks": [
    {"name": "New Task", "status": "todo"},
    {"name": "Existing Task", "spec": "Updated spec"},
    {"id": 99, "delete": true}
  ]
}' | ie plan
```

**Key points:**
- Delete requires `id` field (name is optional)
- Delete operations are processed first
- Can mix create/update/delete in one request
- **Cascade delete**: Deleting a parent task also deletes all its descendants
- **Non-existent ID**: Returns success with a warning (not an error)
- **Focus protection**: Cannot delete ANY session's focus task or its ancestors
  - Direct delete of focused task → Error
  - Delete parent when child is focused → Error (would cascade-delete focus)
  - Focus protection is **global** - protects tasks focused by ANY session
  - Error message includes which session holds the focus
  - The session owning the focus must switch focus first, then delete

## JSON Schema

```typescript
{
  "tasks": [
    {
      "name": string?,          // Task name (required for create/update, optional for delete)
      "spec": string?,          // Optional: description/specification
      "status": "todo"|"doing"|"done"?,  // Optional: task status
      "priority": "critical"|"high"|"medium"|"low"?,
      "active_form": string?,   // Optional: display text when doing
      "parent_id": number|null?, // Optional: explicit parent (null = root)
      "children": TaskTree[]?,  // Optional: nested child tasks
      "depends_on": string[]?,  // Optional: dependency names
      "id": number?,            // Optional: task ID (required for delete, alias: task_id)
      "delete": boolean?        // Optional: set true to delete the task
    }
  ]
}
```

## Common Patterns

### Sprint Planning
```bash
echo '{
  "tasks": [{
    "name": "Sprint 10: User System",
    "priority": "high",
    "children": [
      {"name": "Registration flow", "priority": "high"},
      {"name": "Profile page", "priority": "medium"},
      {"name": "Settings", "priority": "low"}
    ]
  }]
}' | ie plan
```

### Bug Tracking
```bash
echo '{
  "tasks": [{
    "name": "Fix login timeout",
    "priority": "critical",
    "status": "doing",
    "children": [
      {"name": "Reproduce issue", "status": "done"},
      {"name": "Find root cause", "status": "doing"},
      {"name": "Write fix", "status": "todo"},
      {"name": "Test", "status": "todo"}
    ]
  }]
}' | ie plan
```

### Creating Independent Tasks
```bash
# When you need a task NOT under the current focus
echo '{
  "tasks": [{
    "name": "Unrelated Bug Fix",
    "parent_id": null,
    "status": "todo"
  }]
}' | ie plan
```

## Key Concepts

- **Idempotent**: Safe to run multiple times (updates by name)
- **Batch**: Create, update, and delete multiple tasks in one operation
- **Hierarchical**: Nest tasks with children or parent_id
- **Dependencies**: Automatic cycle detection
- **Status**: todo/doing/done (only one doing allowed per batch)
- **Focus**: Doing task auto-focuses
- **Delete**: Remove tasks by ID (processed first)

## Common Errors

**Multiple doing tasks**:
```bash
# Error - only one doing allowed per batch
echo '{"tasks":[
  {"name":"A","status":"doing"},
  {"name":"B","status":"doing"}
]}' | ie plan
```

**Circular dependencies**:
```bash
# Error - A depends on B, B depends on A
echo '{"tasks":[
  {"name":"A","depends_on":["B"]},
  {"name":"B","depends_on":["A"]}
]}' | ie plan
```

## TodoWriter Migration

| TodoWriter | Intent-Engine |
|-----------|---------------|
| `status: "completed"` | `status: "done"` |
| `status: "in_progress"` | `status: "doing"` + `active_form` |
| `status: "pending"` | `status: "todo"` |

```typescript
// TodoWriter
TodoWrite({
  todos: [
    {content: "Task 1", status: "in_progress", activeForm: "Working on Task 1"},
    {content: "Task 2", status: "pending"}
  ]
});
```

```bash
# Intent-Engine equivalent
echo '{
  "tasks": [
    {"name": "Task 1", "status": "doing", "active_form": "Working on Task 1"},
    {"name": "Task 2", "status": "todo"}
  ]
}' | ie plan
```

## Best Practices

1. **Start simple**: Flat list first, add hierarchy when needed
2. **Batch operations**: Create related tasks together
3. **Clear names**: "Implement JWT auth" not "Do auth"
4. **2-3 levels max**: Avoid deep nesting
5. **Sync progress**: Use plan to update status
6. **Use parent_id: null**: When creating tasks independent of current focus

## Related Commands

- `ie status` - View current task context
- `ie log decision "message"` - Record decisions
- `ie search "query"` - Search tasks
- `ie plan --format json` - JSON output

## Output Formats

**Text (default)**:
```
Plan executed successfully

Created: 3 tasks
Updated: 1 tasks
Deleted: 1 tasks
Cascade deleted: 2 tasks
Dependencies: 2

Task ID mapping:
  Login -> #42
  Database -> #43
  Tests -> #44

Warnings:
  - Task #42 had 2 descendant(s) that were also deleted (cascade)
```

**JSON** (`--format json`):
```json
{
  "success": true,
  "created_count": 3,
  "updated_count": 1,
  "deleted_count": 1,
  "cascade_deleted_count": 2,
  "task_id_map": {"Login": 42},
  "warnings": ["Task #42 had 2 descendant(s) that were also deleted (cascade)"]
}
```

---

**Principle**: Plan is declarative - tell the system "what you want", not "how to do it"
