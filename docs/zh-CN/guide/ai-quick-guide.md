# Intent-Engine: AI Quick Reference

**Purpose**: Strategic intent tracking for human-AI collaboration. Not a todo list—a shared memory layer for long-term, complex work.

## When to Use

Create a task when work requires:
- Multiple steps
- Extensive context/spec
- Session interruptions
- Human-AI collaboration

## Core Commands (Atomic = Single Call)

### Start Work
```bash
ie task start <ID> --with-events  # ATOMIC: status→doing + set current + get context
```

### Create & Switch to Subtask
```bash
ie task spawn-subtask --name "X"  # ATOMIC: create + status→doing + switch
```

### Switch Tasks
```bash
ie task switch <ID>  # ATOMIC: status→doing + set current + get events
```

### Smart Batch Selection
```bash
ie task pick-next --max-count 3  # ATOMIC: query + sort + batch transition
```

### Record Critical Moments
```bash
# To current task (concise)
echo "Decision/blocker/milestone..." | \
  ie event add --type decision --data-stdin

# To specific task (flexible)
echo "Decision/blocker/milestone..." | \
  ie event add --task-id <ID> --type decision --data-stdin
```

### Complete (Enforces Hierarchy)
```bash
ie task done  # Completes current focused task, fails if subtasks incomplete
```

### Get Summary (Token-Efficient)
```bash
ie report --since 7d --summary-only  # Returns stats only, not full tasks
```

## Workflow Pattern

```bash
# 1. Create task with rich spec
echo "Multi-line markdown spec..." | \
  ie task add --name "Implement OAuth2" --spec-stdin

# 2. Start & load context (returns spec + event history)
ie task start 1 --with-events

# 3. Execute + record key decisions (to current task)
echo "Chose Passport.js for OAuth strategies" | \
  ie event add --type decision --data-stdin

# 4. Hit sub-problem? Create & auto-switch
ie task spawn-subtask --name "Configure Google OAuth app"

# 5. Complete child (child is now focused after spawn-subtask), switch back to parent
ie task done
ie task switch 1

# 6. Complete parent
ie task done
```

## Batch Problem Handling

```bash
# Discovered 5 bugs? Create all, then smartly select:
for bug in A B C D E; do
  ie task add --name "Fix bug $bug"
done

# Evaluate each
ie task update 1 --complexity 3 --priority 10  # Simple+urgent
ie task update 2 --complexity 8 --priority 10  # Complex+urgent
ie task update 3 --complexity 2 --priority 5   # Simple+normal

# Auto-select by: priority DESC, complexity ASC
ie task pick-next --max-count 3
# → Selects: #1 (P10/C3), #3 (P5/C2), #2 (P10/C8)
```

## Event Types

- `decision` - Chose X over Y because...
- `blocker` - Stuck, need human help
- `milestone` - Completed phase X
- `discussion` - Captured conversation
- `note` - General observation

## 高级模式：替代中间文件

AI 工作时常创建临时文件（scratchpad.md, plan.md 等）。Intent-Engine 提供了更优雅的替代方案。

### 核心映射

| 传统文件 | Intent-Engine 方式 | 优势 |
|---------|-------------------|------|
| `requirements.md` | Task Spec (`--spec-stdin`) | 与任务强绑定，start 时自动加载 |
| `scratchpad.md` | Event (`type: note`) | 带时间戳，永远关联到具体任务 |
| `plan.md` | Subtasks | 可追踪状态的动态计划 |
| `error_log.txt` | Event (`type: blocker`) | 明确标记为障碍，方便复盘 |
| `design_v2.md` | Task Spec (新任务) | 方案与执行合二为一 |

### 实例：调试分析

```bash
# ❌ 旧方式：创建 debug_analysis.md
cat > debug_analysis_task5.md <<EOF
通过浏览器控制台发现：
1. 按钮点击没有触发网络请求
2. Console 报错: TypeError...
EOF

# ✅ 新方式：直接存入事件流
echo "通过浏览器控制台发现：
1. 按钮点击没有触发网络请求
2. Console 报错: TypeError...
3. 初步判断是事件绑定失效" | \
  ie event add --type note --data-stdin  # 使用当前任务
```

### 实例：技术方案

```bash
# ❌ 旧方式：创建 design_v2.md 等待确认
cat > design_v2.md <<EOF
V2 重构方案：使用 React Hooks 替代 Class Components
EOF

# ✅ 新方式：创建子任务，方案即规格
cat design_v2.md | \
  ie task spawn-subtask --name "执行 V2 重构" --spec-stdin
# 自动切换到新任务，方案已加载
```

### 质变收益

1. **Token 效率**:
   - 旧：读取整个 scratchpad.md（包含无关信息）
   - 新：精准读取 `event list --task-id 5 --limit 10`

2. **查询能力**:
   - 旧："我上次怎么解决的？" → 手动翻文件
   - 新：`task search "TypeError" --status done` → 瞬间定位

3. **工作区整洁**:
   - 旧：项目根目录被 `temp_xxx.md` 淹没
   - 新：所有思维碎片收纳在 `.intent-engine/project.db`

4. **自动关联**:
   - 旧：文件名标记任务 ID（`bug_5_analysis.md`）
   - 新：数据库外键自动关联，无需人工维护

## Token Optimization

| Old Way | Calls | Atomic | Calls | Saving |
|---------|-------|--------|-------|--------|
| query+update+set current | 3 | pick-next | 1 | 67% |
| create+start+set current | 3 | spawn-subtask | 1 | 67% |
| update+set+get events | 3 | switch | 1 | 67% |
| query all+filter+format | many | report --summary-only | 1 | 90%+ |

## Key Rules

1. **Always use --with-events** when starting/switching tasks (loads context)
2. **Always use --summary-only** for reports (unless debugging)
3. **Record all key decisions** via `event add` (your external memory)
4. **Use atomic commands** (start, switch, spawn-subtask, pick-next)
5. **Respect hierarchy** (complete children before parents)

## Status Flow

```
todo → (start/pick-next/spawn-subtask) → doing → (done) → done
       ↑                                    ↓
       └────────────── (switch) ────────────┘
```

## Quick Checks

```bash
ie current                          # What am I working on?
ie task find --status doing         # All active tasks
ie task search "keyword"            # Search tasks by content (FTS5)
ie event list --task-id <ID> --limit 5  # Recent context
ie report --since 1d --summary-only     # Today's summary
```

## Anti-Patterns

❌ Don't manually chain: `task update <ID> --status doing && current --set <ID>`
✅ Do use atomic: `task start <ID> --with-events`

❌ Don't forget to record decisions
✅ Do log every key choice via `event add`

❌ Don't use `report` without `--summary-only` for routine checks
✅ Do use `--summary-only` (saves 90% tokens)

## Philosophy

Intent-Engine is AI's **strategic memory**. Context window = short-term. Events table = long-term. Tasks = goals. Commands = how we achieve them together.

---

**Full docs**: [the-intent-engine-way.md](the-intent-engine-way.md), [README.md](../../../README.md)
