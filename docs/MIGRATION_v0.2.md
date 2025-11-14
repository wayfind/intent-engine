# Migration Guide: v0.1 → v0.2

**Version**: 0.2.0
**Date**: 2025-11-11
**Theme**: "Intelligence & Interconnection"

---

## 📋 Overview

Version 0.2 introduces powerful new features for task dependencies and smart event querying while maintaining full backward compatibility. **No breaking changes** - all existing workflows continue to work.

---

## ✨ What's New

### 1. Task Dependency System

**New capability**: Define dependencies between tasks to model complex workflows.

**New CLI Command:**
```bash
# Make task 43 depend on task 42 (43 cannot start until 42 is done)
ie task depends-on 43 42
```

**New MCP Tool:**
```json
{
  "tool": "task_add_dependency",
  "arguments": {
    "blocked_task_id": 43,
    "blocking_task_id": 42
  }
}
```

**Automatic Behavior Changes:**
- `task start` now checks dependencies - fails if blockers are incomplete
- `task pick-next` automatically filters out blocked tasks
- `task context` now includes dependency information

**Example Workflow:**
```bash
# Create two tasks
ie task add --name "Implement authentication"  # Returns ID 1
ie task add --name "Implement API client"      # Returns ID 2

# API client depends on auth
ie task depends-on 2 1

# Try to start API client (will fail because auth not done)
ie task start 2
# Error: Task 2 is blocked by incomplete tasks: [1]

# Complete auth first
ie task start 1
# ... do the work ...
ie task done

# Now API client can start
ie task start 2  # ✅ Success
```

---

### 2. Smart Event Querying

**Enhancement**: Filter events by type and time to dramatically reduce token usage.

**CLI Enhancement:**
```bash
# Old way (still works)
ie event list 42

# New filters
ie event list 42 --type decision
ie event list 42 --since 7d
ie event list 42 --type blocker --since 24h
```

**MCP Enhancement:**
```json
{
  "tool": "event_list",
  "arguments": {
    "task_id": 42,
    "type": "decision",       // Filter by type (optional)
    "since": "7d",            // Time filter (optional)
    "limit": 10               // Existing parameter
  }
}
```

**Duration Formats:**
- `7d` - 7 days
- `24h` - 24 hours
- `30m` - 30 minutes
- `60s` - 60 seconds

**Performance Impact:**
- For a task with 100 events, filtering to recent decisions reduces output by 80-90%
- Dramatically reduces token usage for AI agents
- Faster context retrieval

---

### 3. Priority Enum Interface

**Change**: Priority now accepts human-friendly strings instead of raw integers.

**Old Way (still works internally):**
```bash
ie task update 1 --priority 1  # Low-level integers
```

**New Way (recommended):**
```bash
ie task update 1 --priority critical
ie task update 1 --priority high
ie task update 1 --priority medium
ie task update 1 --priority low
```

**Mapping:**
- `critical` → 1 (highest)
- `high` → 2
- `medium` → 3
- `low` → 4 (lowest)

**MCP Update:**
```json
{
  "tool": "task_update",
  "arguments": {
    "task_id": 1,
    "priority": "critical"  // String enum instead of integer
  }
}
```

**Benefits:**
- More intuitive for AI agents
- Better for human readability
- Case-insensitive (`Critical`, `CRITICAL`, `critical` all work)

---

### 4. Command Rename: find → list

**Change**: Renamed `task find` to `task list` for clarity.

**Old Command (deprecated but still works):**
```bash
ie task find --status todo
# ⚠️  Warning: 'task find' is deprecated. Please use 'task list' instead.
```

**New Command:**
```bash
ie task list --status todo
```

**MCP Rename:**
- Old: `task_find` (deprecated, shows warning in description)
- New: `task_list` (recommended)

**Both tools work identically** - the rename is purely for clarity. The old name will be supported indefinitely with a deprecation warning.

---

## 🔄 Migration Checklist

### For Human Users

- [ ] **Learn new commands**: Try `task depends-on` and filtered `event list`
- [ ] **Update scripts**: Replace `task find` with `task list` (optional, old still works)
- [ ] **Use priority strings**: Prefer `--priority high` over `--priority 2`
- [ ] **Read updated docs**: Review [INTERFACE_SPEC.md](INTERFACE_SPEC.md) and [CLAUDE.md](../CLAUDE.md)

### For AI Agent Integrations

- [ ] **Update MCP clients**: Use `task_list` instead of `task_find`
- [ ] **Leverage filtering**: Use `event_list` type/since filters to reduce tokens
- [ ] **Use priority enums**: Switch to `critical`/`high`/`medium`/`low` in `task_update`
- [ ] **Handle dependencies**: Check for dependency errors when calling `task_start`
- [ ] **Test workflows**: Verify existing automations still work

### For Existing Projects

**No action required!** Your existing `.intent-engine/project.db` will work seamlessly:

1. **Automatic migration**: Database schema updates automatically on first use
2. **Data preservation**: All existing tasks, events, and state maintained
3. **New tables**: `dependencies` table created transparently
4. **Backward compatible**: All old commands continue to work

---

## 🐛 Potential Issues

### Issue: Circular Dependencies

**Symptom**: `task depends-on` fails with "Circular dependency detected"

**Cause**: Attempting to create A→B→C→A dependency cycle

**Solution**: Review dependency graph, remove circular relationships
```bash
# This will fail if it creates a cycle
ie task depends-on 1 3  # Error if 3→2→1 exists
```

### Issue: Cannot Start Task

**Symptom**: `task start` fails with "Task is blocked"

**Cause**: Task has incomplete dependencies

**Solution**: Complete blocking tasks first, or remove dependency
```bash
# Check dependencies
ie task context 42  # View dependencies section

# Complete blockers first
ie task start <blocking_task_id>
ie task done
```

---

## 📊 Database Changes

### New Table: dependencies

```sql
CREATE TABLE dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    blocking_task_id INTEGER NOT NULL,
    blocked_task_id INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (blocking_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (blocked_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    UNIQUE(blocking_task_id, blocked_task_id)
);
```

**Automatic Migration:**
- Runs on first v0.2 command execution
- Non-destructive (only adds new table)
- No downtime required

---

## 🔗 Resources

- **Full Changelog**: See [INTERFACE_SPEC.md](INTERFACE_SPEC.md#changelog)
- **AI Integration Guide**: See [CLAUDE.md](../CLAUDE.md)
- **Interface Specification**: See [INTERFACE_SPEC.md](INTERFACE_SPEC.md)
- **Requirement Spec**: See [requirement_spec_v0.2.0.md](requirement_spec_v0.2.0.md)

---

## ❓ FAQ

**Q: Do I need to upgrade my database manually?**
A: No, migration happens automatically on first use of v0.2.

**Q: Can I downgrade back to v0.1?**
A: Yes, but you'll lose dependency data. Tasks and events are preserved.

**Q: Will my AI workflows break?**
A: No, all changes are backward-compatible. Existing MCP tools continue to work.

**Q: Should I update my AI agents to use new features?**
A: Recommended but not required. New features provide significant efficiency gains.

**Q: What if I don't need dependencies?**
A: No problem! Ignore the new commands - everything works as before.

---

## 📞 Support

Found an issue or have questions?

- **Bug Reports**: [GitHub Issues](https://github.com/wayfind/intent-engine/issues)
- **Documentation**: [docs/](.)
- **Discussions**: [GitHub Discussions](https://github.com/wayfind/intent-engine/discussions)

---

**Last Updated**: 2025-11-11
**Version**: 0.2.0
