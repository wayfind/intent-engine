# Multi-Session Focus 设计文档

> **Version**: 0.1
> **Status**: Draft
> **Related Task**: #63 支持多 session 并行 focus

## 1. 背景与动机

### 1.1 当前设计的局限

当前 ie 使用全局单一 focus：

```sql
-- workspace_state 表（KV 存储）
key = 'current_task_id', value = '42'  -- 全局唯一
```

**问题**：当多个 Claude Code session 或 subagent 并行工作时，它们会互相覆盖 focus 状态。

### 1.2 目标场景

```
┌─────────────────────────────────────────────────────────────┐
│  Main Agent (session: aaa-111)                              │
│  ie status → focus: Task #1 "实现认证"                      │
│                                                             │
│  ├── Subagent A (session: bbb-222)                          │
│  │   ie status → focus: Task #2 "JWT 模块"                  │
│  │                                                          │
│  └── Subagent B (session: ccc-333)                          │
│      ie status → focus: Task #3 "OAuth 模块"                │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Dashboard (session: "-1" 默认)                             │
│  ie status → 全局视图，不关联特定 session                   │
└─────────────────────────────────────────────────────────────┘
```

## 2. 设计原则

1. **Session 隔离**：每个 Claude session 有独立的 focus 状态
2. **任务全局**：Task 本身不属于任何 session，只有 focus 是 per-session
3. **向后兼容**：无 session 参数时使用默认 session "-1"
4. **自动清理**：过期 session 自动清理，避免数据膨胀
5. **对 AI 无感**：通过环境变量自动传递 session_id

## 3. 数据模型

### 3.1 新增 sessions 表

```sql
CREATE TABLE sessions (
    session_id TEXT PRIMARY KEY,           -- Claude session UUID 或 "-1"
    current_task_id INTEGER,               -- 当前 focus 的任务
    created_at TEXT NOT NULL,              -- 创建时间
    last_active_at TEXT NOT NULL,          -- 最后活跃时间
    FOREIGN KEY (current_task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

-- 索引：按活跃时间查询（用于清理）
CREATE INDEX idx_sessions_last_active ON sessions(last_active_at);
```

### 3.2 数据迁移

从 `workspace_state` 迁移现有 focus 到默认 session：

```sql
INSERT INTO sessions (session_id, current_task_id, created_at, last_active_at)
SELECT '-1', CAST(value AS INTEGER), datetime('now'), datetime('now')
FROM workspace_state
WHERE key = 'current_task_id' AND value IS NOT NULL;
```

### 3.3 关系图

```
┌─────────────┐       ┌─────────────┐
│   tasks     │       │  sessions   │
├─────────────┤       ├─────────────┤
│ id (PK)     │◄──────│ current_task_id (FK)
│ name        │       │ session_id (PK)
│ status      │       │ created_at
│ ...         │       │ last_active_at
└─────────────┘       └─────────────┘

关系：多个 session 可以 focus 同一个 task
```

## 4. Session ID 获取机制

### 4.1 优先级

```rust
fn resolve_session_id(args: &Args) -> String {
    // 1. 显式参数 --session
    if let Some(s) = &args.session {
        return s.clone();
    }

    // 2. 环境变量 IE_SESSION_ID
    if let Ok(s) = std::env::var("IE_SESSION_ID") {
        if !s.is_empty() {
            return s;
        }
    }

    // 3. 默认 session
    "-1".to_string()
}
```

### 4.2 环境变量设置（通过 Claude Code Hook）

**SessionStart Hook** (`~/.claude/hooks/session-start.sh`):

```bash
#!/bin/bash
input=$(cat)
session_id=$(echo "$input" | jq -r '.session_id // ""' 2>/dev/null)

# 持久化到 CLAUDE_ENV_FILE，供后续 Bash 命令使用
if [ -n "$CLAUDE_ENV_FILE" ] && [ -n "$session_id" ]; then
    echo "export IE_SESSION_ID=\"$session_id\"" >> "$CLAUDE_ENV_FILE"
fi

# 显示任务状态
if command -v ie &> /dev/null; then
    IE_SESSION_ID="$session_id" ie status 2>/dev/null
fi

exit 0
```

### 4.3 流程图

```
Claude Code Session 启动
        │
        ▼
SessionStart Hook 触发
        │
        ├── 从 stdin 读取 JSON，获取 session_id
        │
        ├── 写入 CLAUDE_ENV_FILE: export IE_SESSION_ID="xxx"
        │
        └── 调用 ie status（带 session_id）

        │
        ▼
Claude Code 加载 CLAUDE_ENV_FILE 中的环境变量
        │
        ▼
后续 Bash Tool 调用
        │
        └── ie plan / ie status / ie log
            └── 自动读取 IE_SESSION_ID 环境变量
```

## 5. API 变更

### 5.1 CLI 参数

所有命令增加可选 `--session` 参数：

```bash
ie status [--session <session_id>]
ie plan [--session <session_id>]
ie log <type> <message> [--session <session_id>]
ie search <query> [--session <session_id>]
```

### 5.2 新增命令

```bash
# 查看所有活跃 session
ie session list

# 手动清理过期 session
ie session clean [--before <datetime>]

# 查看特定 session 信息
ie session show <session_id>
```

## 6. 清理策略

### 6.1 自动清理

在以下时机触发自动清理：

1. **新 session 注册时**：清理超过 24 小时未活跃的 session
2. **ie 命令执行时**：懒清理，概率触发（1/100）

```rust
fn auto_cleanup(pool: &SqlitePool) {
    sqlx::query("DELETE FROM sessions WHERE last_active_at < datetime('now', '-24 hours')")
        .execute(pool)
        .await;
}
```

### 6.2 手动清理

```bash
# 清理所有过期 session
ie session clean

# 清理指定时间之前的 session
ie session clean --before "2025-01-01"

# 清理特定 session
ie session clean --session <session_id>
```

### 6.3 数量限制

保留最多 1000 个 session，超过时清理最旧的：

```sql
DELETE FROM sessions
WHERE session_id IN (
    SELECT session_id FROM sessions
    ORDER BY last_active_at DESC
    LIMIT -1 OFFSET 1000
);
```

## 7. 各调用场景

| 调用者 | session_id 来源 | 说明 |
|--------|----------------|------|
| Claude Code (主 Agent) | 环境变量 IE_SESSION_ID | Hook 自动设置 |
| Dashboard | 默认 "-1" | 全局视图 |
| 用户终端 | 默认 "-1" 或 --session | 手动指定 |
| CI/CD | 环境变量或 --session | 可手动设置 |

### 7.1 Subagent 限制（当前版本不支持）

**已确认限制**：Subagent **不会触发 SessionStart hook**，因此：
- Subagent 无法获取自己的 session_id
- Subagent 中 IE_SESSION_ID 环境变量为空
- Subagent 会 fallback 到默认 session "-1"

**当前版本策略**：
- **不支持 subagent 独立 focus**
- Subagent 与 Dashboard 共享默认 session "-1"
- 多个并行 subagent 会共享同一个 focus 状态

**未来增强**（待 Claude Code 支持）：
- 等待 Claude Code 为 subagent 提供 SessionStart hook 或其他机制
- 或通过进程组 (PGID) 查找关联的 session

## 8. 兼容性

### 8.1 向后兼容

- 无 `--session` 参数时使用默认 "-1"
- 无 `IE_SESSION_ID` 环境变量时使用默认 "-1"
- Dashboard 和终端用户使用默认 session

### 8.2 迁移路径

1. 新增 sessions 表
2. 迁移 workspace_state 中的 current_task_id 到默认 session
3. 保留 workspace_state 表用于其他 KV 存储

## 9. 实现任务分解

| 任务 | 描述 | 依赖 |
|------|------|------|
| #64 设计 sessions 表 schema | 本文档 | - |
| #65 实现数据库迁移 | 创建表 + 迁移数据 | #64 |
| #66 修改 WorkspaceManager | 支持 per-session focus | #65 |
| #67 CLI 添加 --session 参数 | 所有命令支持 | #66 |
| #68 更新 ie status/plan | 使用 session-aware focus | #67 |
| #69 测试多 session 并行 | 集成测试 | #68 |

## 10. 测试计划

### 10.1 单元测试

- [ ] sessions 表 CRUD 操作
- [ ] session_id 解析优先级
- [ ] 自动清理逻辑
- [ ] 数量限制逻辑

### 10.2 集成测试

- [ ] 多 session 并行 focus 隔离
- [ ] 环境变量传递
- [ ] Dashboard 使用默认 session
- [ ] 迁移后数据一致性

### 10.3 手动测试

```bash
# 测试 1: 不同 session 有独立 focus
IE_SESSION_ID="test-1" ie plan < task1.json
IE_SESSION_ID="test-2" ie plan < task2.json
IE_SESSION_ID="test-1" ie status  # 应该看到 task1
IE_SESSION_ID="test-2" ie status  # 应该看到 task2

# 测试 2: 默认 session
ie status  # 使用 "-1" session

# 测试 3: 显式参数优先
IE_SESSION_ID="env-session" ie status --session "arg-session"
# 应该使用 "arg-session"
```

## 11. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Session 数据膨胀 | 存储空间 | 自动清理 + 数量限制 |
| 环境变量丢失 | Focus 错误 | 默认 session fallback |
| Subagent 隔离不完整 | 并行冲突 | 文档说明 + 未来增强 |
| 迁移失败 | 数据丢失 | 备份 + 回滚方案 |

---

*文档更新: 2025-12-25*
