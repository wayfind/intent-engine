# Plan Interface Guide

**Version**: 0.6.0
**Status**: Stable
**MCP Tool**: `plan`

---

## Overview

The **Plan interface** is a declarative, batch-oriented way to create and update task structures in Intent-Engine. It complements the traditional imperative commands by providing:

- **Declarative**: Describe "what you want" instead of "how to do it"
- **Batch operations**: Create entire task trees in one call
- **Idempotency**: Repeated execution produces the same result
- **Transactional**: All-or-nothing atomicity
- **Dependency resolution**: Automatic handling of task dependencies
- **Cycle detection**: Prevents invalid circular dependencies

---

## When to Use Plan vs Traditional Commands

### Use Plan Interface When

✅ **Batch creating task structures** (e.g., project initialization)
```javascript
// Create 10 tasks with hierarchy in one call
await plan({
  tasks: [
    { name: "Project Setup", children: [...] }
  ]
})
```

✅ **Complex dependency networks**
```javascript
await plan({
  tasks: [
    { name: "Foundation" },
    { name: "Layer 1", depends_on: ["Foundation"] },
    { name: "Layer 2", depends_on: ["Layer 1"] }
  ]
})
```

✅ **Idempotent operations** (e.g., automation scripts)
```javascript
// Run multiple times - same result
await plan({ tasks: [...] })
await plan({ tasks: [...] }) // Updates, not duplicates
```

✅ **Importing from external systems** (YAML/JSON)
```javascript
const taskTree = await readYAML("project.yaml")
await plan({ tasks: taskTree.tasks })
```

### Use Traditional Commands When

✅ **Single task operations**
```javascript
// Simpler than plan for one task
await task_add({ name: "Quick task", spec: "..." })
```

✅ **Interactive CLI workflows**
```bash
ie task add "Task name"
ie task start 42
ie task done
```

✅ **Dynamic workflows**
```javascript
await task_start({ task_id: 42 })
// ... do work ...
await task_done()
```

✅ **Fine-grained control**
```javascript
// Precise control over each step
await task_add(...)
await task_add_dependency(...)
await task_update(...)
```

---

## Plan Interface Reference

### Data Structure

```typescript
interface PlanRequest {
  tasks: TaskTree[]
}

interface TaskTree {
  name: string                // Required: Task name
  spec?: string               // Optional: Task specification
  priority?: "critical" | "high" | "medium" | "low"
  children?: TaskTree[]       // Optional: Nested child tasks
  depends_on?: string[]       // Optional: Task names this depends on
  task_id?: number            // Optional: Explicit task ID (for updates)
}

interface PlanResult {
  success: boolean
  task_id_map: { [name: string]: number }
  created_count: number
  updated_count: number
  dependency_count: number
  error?: string
}
```

### Operation Modes

#### 1. Create Mode (Default)

When a task with the given `name` doesn't exist, it will be created:

```javascript
const result = await plan({
  tasks: [{
    name: "New Task",
    spec: "This will be created",
    priority: "high"
  }]
})

// result.created_count === 1
// result.updated_count === 0
```

#### 2. Update Mode (Idempotent)

When a task with the given `name` already exists, it will be updated:

```javascript
// First call - creates
await plan({
  tasks: [{ name: "Task A", spec: "Initial" }]
})

// Second call - updates (idempotent)
await plan({
  tasks: [{ name: "Task A", spec: "Updated" }]
})
```

**Fields updated**:
- ✅ `spec` (if provided)
- ✅ `priority` (if provided)
- ❌ `name` (used for identity, cannot be changed)
- ❌ `status` (managed through workflow commands)
- ❌ Timestamps (preserved from original creation)

#### 3. Mixed Mode

You can create and update in the same call:

```javascript
await plan({
  tasks: [
    { name: "Existing Task", spec: "Updated" },   // Updates
    { name: "New Task", spec: "Fresh" }            // Creates
  ]
})
```

---

## Common Patterns

### Pattern 1: Project Initialization

```javascript
await plan({
  tasks: [{
    name: "User Authentication Project",
    spec: "Implement full authentication system",
    priority: "critical",
    children: [
      {
        name: "JWT Implementation",
        spec: "Token-based auth",
        priority: "high"
      },
      {
        name: "OAuth2 Integration",
        spec: "Social login support",
        priority: "medium",
        depends_on: ["JWT Implementation"]
      },
      {
        name: "Session Management",
        spec: "Redis-based sessions"
      }
    ]
  }]
})
```

### Pattern 2: Dependency Chain

```javascript
await plan({
  tasks: [
    { name: "Database Setup" },
    { name: "API Development", depends_on: ["Database Setup"] },
    { name: "Frontend Integration", depends_on: ["API Development"] },
    { name: "Testing", depends_on: ["Frontend Integration"] }
  ]
})
```

### Pattern 3: Diamond Dependencies

```javascript
await plan({
  tasks: [
    { name: "Foundation" },
    { name: "Module A", depends_on: ["Foundation"] },
    { name: "Module B", depends_on: ["Foundation"] },
    { name: "Integration", depends_on: ["Module A", "Module B"] }
  ]
})
```

### Pattern 4: Incremental Updates

```javascript
// Initial plan
await plan({
  tasks: [{ name: "Feature X", spec: "Draft" }]
})

// Later: update specification
await plan({
  tasks: [{ name: "Feature X", spec: "Finalized spec" }]
})

// Later: add priority
await plan({
  tasks: [{ name: "Feature X", priority: "critical" }]
})
```

---

## Error Handling

### Duplicate Names

```javascript
const result = await plan({
  tasks: [
    { name: "Task A" },
    { name: "Task A" }  // ❌ Duplicate
  ]
})

// result.success === false
// result.error === "Duplicate task names in request: [\"Task A\"]"
```

### Missing Dependencies

```javascript
const result = await plan({
  tasks: [{
    name: "Task A",
    depends_on: ["NonExistent"]  // ❌ Not in plan
  }]
})

// result.success === false
// result.error contains "NonExistent is not in the plan"
```

### Circular Dependencies

```javascript
const result = await plan({
  tasks: [
    { name: "A", depends_on: ["B"] },
    { name: "B", depends_on: ["A"] }  // ❌ Cycle
  ]
})

// result.success === false
// result.error === "Circular dependency detected: A → B"
```

---

## Migration Examples

### Example 1: task_add → plan

**Before** (Traditional):
```javascript
await task_add({ name: "Task 1", spec: "First" })
await task_add({ name: "Task 2", spec: "Second" })
await task_add({ name: "Task 3", spec: "Third" })
```

**After** (Plan):
```javascript
await plan({
  tasks: [
    { name: "Task 1", spec: "First" },
    { name: "Task 2", spec: "Second" },
    { name: "Task 3", spec: "Third" }
  ]
})
```

**Benefits**: Atomic, idempotent, single transaction

### Example 2: task_add_dependency → plan.depends_on

**Before** (Traditional):
```javascript
const task1 = await task_add({ name: "Foundation" })
const task2 = await task_add({ name: "Layer 1" })
await task_add_dependency({
  blocked_task_id: task2.id,
  blocking_task_id: task1.id
})
```

**After** (Plan):
```javascript
await plan({
  tasks: [
    { name: "Foundation" },
    { name: "Layer 1", depends_on: ["Foundation"] }
  ]
})
```

**Benefits**: Name-based references, automatic validation, cycle detection

### Example 3: Nested Subtasks → plan.children

**Before** (Traditional):
```javascript
const parent = await task_add({ name: "Parent" })
await task_spawn_subtask({ name: "Child 1" })
await task_done()
await task_spawn_subtask({ name: "Child 2" })
await task_done()
```

**After** (Plan):
```javascript
await plan({
  tasks: [{
    name: "Parent",
    children: [
      { name: "Child 1" },
      { name: "Child 2" }
    ]
  }]
})
```

**Benefits**: Declarative hierarchy, easier to visualize, less code

---

## Best Practices

### 1. Use Meaningful Names

Task names are used for identity and dependency resolution:

```javascript
// ✅ Good
await plan({
  tasks: [{
    name: "Implement User Authentication",
    depends_on: ["Database Setup", "API Framework"]
  }]
})

// ❌ Avoid
await plan({
  tasks: [{
    name: "Task 1",
    depends_on: ["Task 2", "Task 3"]
  }]
})
```

### 2. Leverage Idempotency

Run the same plan multiple times safely:

```javascript
// automation.js - safe to run repeatedly
const projectPlan = {
  tasks: [...]
}

setInterval(async () => {
  await plan(projectPlan)  // Updates, doesn't duplicate
}, 3600000)
```

### 3. Structure for Readability

```javascript
// ✅ Clear hierarchy
await plan({
  tasks: [{
    name: "Project Alpha",
    priority: "critical",
    children: [
      { name: "Phase 1: Research", children: [...] },
      { name: "Phase 2: Development", children: [...] },
      { name: "Phase 3: Deployment", children: [...] }
    ]
  }]
})
```

### 4. Partial Updates

Only specify fields you want to update:

```javascript
// Update only spec, keep priority unchanged
await plan({
  tasks: [{
    name: "Existing Task",
    spec: "New specification"
    // priority not specified - remains unchanged
  }]
})
```

---

## Comparison Matrix

| Feature | Plan Interface | Traditional Commands |
|---------|---------------|---------------------|
| Batch operations | ✅ Excellent | ❌ One at a time |
| Idempotency | ✅ Built-in | ❌ Manual tracking |
| Declarative | ✅ Yes | ❌ Imperative |
| Transaction safety | ✅ All-or-nothing | ⚠️ Partial on error |
| Dependency resolution | ✅ Automatic | ❌ Manual |
| Cycle detection | ✅ Automatic | ❌ Manual |
| Learning curve | ⚠️ Moderate | ✅ Low |
| Fine-grained control | ⚠️ Limited | ✅ Precise |
| Interactive use | ❌ Verbose | ✅ Concise |
| Automation friendly | ✅ Perfect | ⚠️ Requires scripting |

---

## Limitations

### Current Limitations (v0.6.0)

1. **No task deletion**: Plan cannot delete tasks
   - Use `task_delete` for removal

2. **No status changes**: Plan doesn't change task status
   - Use `task_start`, `task_done` for workflow

3. **Name-based identity**: Tasks identified by name
   - Renaming requires delete + recreate

4. **No partial tree updates**: Entire hierarchy must be specified
   - Cannot update just one child without listing siblings

### Future Enhancements (Planned)

- ⏳ YAML/JSON file import: `ie plan -f project.yaml`
- ⏳ Dry-run mode: Preview changes without applying
- ⏳ Selective field updates: Update only specified nested tasks
- ⏳ Task renaming: Change name while preserving ID

---

## Summary

The Plan interface is a powerful addition to Intent-Engine that excels at:
- 🎯 Batch task creation
- 🔄 Idempotent operations
- 🏗️ Complex hierarchies and dependencies
- 🤖 Automation and scripting

It **complements** rather than **replaces** traditional commands. Choose the right tool for your use case:

- **Plan**: For structure, automation, and batch operations
- **Traditional**: For interaction, workflows, and fine control

Both approaches are fully supported and will remain available.

---

**Questions?** See [AGENT.md](../AGENT.md) for technical details or [CLAUDE.md](../CLAUDE.md) for AI agent integration.
