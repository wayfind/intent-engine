# Owner Mechanism（任务所有权机制）

**实现日期**: 2025-12-16
**版本**: v0.10.0+
**原则**: 谁创建，谁负责 (Whoever creates it is responsible)

---

## 概述

Intent-Engine 实现了任务所有权机制，用于区分 AI 创建的任务和人类创建的任务，并控制任务的完成权限。

## 核心规则

### 1. 任务所有权 (Task Ownership)

每个任务都有一个 `owner` 字段，可能的值：
- `'ai'` - AI 通过 CLI 创建的任务
- `'human'` - 人类通过 Dashboard UI 创建的任务

### 2. 创建规则

| 创建方式 | Owner 值 | 说明 |
|---------|----------|------|
| **CLI - task add** | `'ai'` | AI 通过命令行创建任务 |
| **CLI - spawn-subtask** | `'ai'` | AI 创建子任务 |
| **CLI - plan** | `'ai'` | AI 通过 plan 接口批量创建 |
| **Dashboard API** | `'human'` | 人类通过 Web UI 创建 |

### 3. 完成规则

- ✅ **AI 可以完成 AI 创建的任务** (`owner='ai'`)
- ✅ **人类可以完成任何任务** (`owner='human'` 或 `owner='ai'`)
- ❌ **AI 不能完成人类创建的任务** (`owner='human'`)

## 实现细节

### 数据库 Schema

```sql
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    spec TEXT,
    status TEXT NOT NULL DEFAULT 'todo',
    owner TEXT NOT NULL DEFAULT 'human',
    -- 其他字段...
    CHECK (owner IN ('human', 'ai'))
);
```

### 代码层面

#### CLI 创建任务 (AI)

```rust
// src/cli_handlers/task.rs
let task = task_mgr
    .add_task(&name, spec.as_deref(), parent, Some("ai"))
    .await?; // 显式设置 owner='ai'
```

#### Dashboard 创建任务 (Human)

```rust
// src/dashboard/handlers.rs
let result = task_mgr
    .add_task(&req.name, req.spec.as_deref(), req.parent_id, None)
    .await; // None 默认为 'human'
```

#### Plan 创建任务 (AI)

```rust
// src/plan.rs - create_task_in_tx
INSERT INTO tasks (name, spec, priority, status, active_form, first_todo_at, owner)
VALUES (?, ?, ?, ?, ?, ?, ?)
// 最后一个参数绑定 "ai"
```

#### 完成任务权限检查

```rust
// src/tasks.rs - done_task
pub async fn done_task(&self, is_ai_caller: bool) -> Result<DoneTaskResponse> {
    // ...
    if owner == "human" && is_ai_caller {
        return Err(IntentError::HumanTaskCannotBeCompletedByAI {
            task_id: id,
            task_name: task_name.clone(),
        });
    }
    // ...
}
```

## 使用场景

### 场景 1: AI 工作流

```bash
# AI 通过 CLI 创建任务
echo '{"tasks":[{"name": "Implement feature X", "status": "doing", "spec": "..."}]}' | ie plan
# -> owner='ai'

# AI 可以完成自己创建的任务
echo '{"tasks":[{"name": "Implement feature X", "status": "done"}]}' | ie plan
# -> ✅ 成功
```

### 场景 2: 人类分配任务

```bash
# 人类通过 Dashboard 创建任务
POST /api/tasks { "name": "Review PR #123" }
# -> owner='human'

# AI 尝试完成（通过 CLI）
echo '{"tasks":[{"name": "Review PR #123", "status": "done"}]}' | ie plan
# -> ❌ 错误: "AI cannot complete human-created task"
```

### 场景 3: 混合层级

```rust
// 人类创建父任务
let parent = task_mgr.add_task("Project Alpha", None, None, None).await?;
// -> owner='human'

// AI 创建子任务
let child = task_mgr.add_task("Subtask", None, Some(parent.id), Some("ai")).await?;
// -> owner='ai', parent_id=parent.id

// AI 可以完成子任务，但不能完成父任务
```

## 测试覆盖

完整的测试套件位于 `tests/owner_mechanism_tests.rs`，包括：

1. ✅ **test_cli_task_add_creates_ai_owned_task** - CLI 创建 AI 任务
2. ✅ **test_dashboard_task_add_creates_human_owned_task** - Dashboard 创建 Human 任务
3. ✅ **test_plan_creates_ai_owned_tasks** - Plan 创建 AI 任务
4. ✅ **test_spawn_subtask_creates_ai_owned_task** - Spawn 创建 AI 子任务
5. ✅ **test_ai_cannot_complete_human_owned_task** - AI 无法完成 Human 任务
6. ✅ **test_human_can_complete_human_owned_task** - Human 可以完成 Human 任务
7. ✅ **test_ai_can_complete_ai_owned_task** - AI 可以完成 AI 任务
8. ✅ **test_mixed_ownership_in_hierarchy** - 混合所有权层级

所有测试通过率: **100% (8/8)**

## 架构优势

### 1. 清晰的责任边界
- AI 和人类的任务创建职责明确
- 防止 AI 意外修改人类的计划

### 2. 协作友好
- 支持 AI 和人类在同一项目中工作
- 人类可以通过 Dashboard 创建任务，AI 通过 CLI 执行
- 父子任务可以有不同的所有者

### 3. 安全保护
- 数据库 CHECK 约束确保 owner 值有效
- 完成任务时的运行时权限检查
- 清晰的错误消息

## 迁移指南

### 从旧版本升级

如果你从没有 owner 机制的版本升级：

1. **数据库迁移** - 自动应用（v0.9.0+ 已包含 owner 列）
2. **默认值** - 现有任务默认 `owner='human'`
3. **CLI 改动** - CLI 创建的任务现在自动设置 `owner='ai'`
4. **Dashboard 行为** - 保持不变，继续创建 `owner='human'` 任务

### 兼容性

- ✅ **向后兼容** - 现有 CLI 命令和 API 保持不变
- ✅ **数据兼容** - 旧任务自动标记为 `owner='human'`
- ✅ **功能完整** - 所有功能保持一致

## 常见问题

### Q: AI 创建的任务能否转移给 Human？
A: 目前不支持动态转移所有权。任务的 owner 在创建时确定。

### Q: 如果我想让 AI 完成 Human 创建的任务怎么办？
A: 人类需要通过 CLI 运行 `ie done` (is_ai_caller=false) 或通过 Dashboard UI 完成任务。

### Q: Plan 批量创建任务时，可以混合 owner 吗？
A: 目前不支持。Plan 创建的所有任务统一为 `owner='ai'`。如需创建 Human 任务，请使用 Dashboard API。

### Q: Subtask 的 owner 会继承父任务吗？
A: 不会。Subtask 的 owner 由创建方式决定：
  - CLI spawn-subtask → `owner='ai'`
  - Dashboard API → `owner='human'`

## 相关文件

### 核心实现
- `src/tasks.rs` - TaskManager::add_task, done_task
- `src/plan.rs` - PlanExecutor::create_task_in_tx
- `src/cli_handlers/task.rs` - CLI task add handler
- `src/dashboard/handlers.rs` - Dashboard task creation API

### 测试
- `tests/owner_mechanism_tests.rs` - 完整测试套件

### 数据库
- `src/db/mod.rs` - Schema definition with owner column
- `src/db/models.rs` - Task struct with owner field

---

**实现完成日期**: 2025-12-16
**测试状态**: ✅ 8/8 通过
**生产就绪**: ✅ 是
