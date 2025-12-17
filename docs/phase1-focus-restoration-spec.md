# Phase 1: Focus Restoration - Implementation Specification

**版本**: 1.0
**状态**: 实施规格 - 已定稿
**目标**: 解决跨会话的 Intent-Engine 焦点遗忘问题
**优先级**: P0 - 立即实施

---

## 1. 概述

### 1.1 目标

在每次新会话开始时，自动恢复 AI Agent 对 Intent-Engine 当前焦点的记忆，确保工作的连续性。

### 1.2 成功指标

- ✅ AI 在新会话的第一轮对话就知道当前焦点任务
- ✅ AI 能够引用上次会话的决策历史
- ✅ 用户无需手动提醒 AI "继续之前的工作"

---

## 2. 架构设计

### 2.1 组件关系

```
┌─────────────────────────────────────────────────┐
│  Claude Code Session Start                      │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│  .claude/hooks/session-start.sh                 │
│  - 检测工作目录                                  │
│  - 调用 Intent-Engine                           │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│  ie session-restore --json                      │
│  - 一次调用获取所有上下文                        │
│  - 包含焦点、父任务、子任务、事件                │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│  格式化为 <system-reminder>                     │
│  - 极简风格，纯数据                              │
│  - 包含必要的使用提示                            │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│  注入到 AI 初始上下文                            │
│  - 在会话正式开始前                              │
└─────────────────────────────────────────────────┘
```

---

## 3. 新命令规格：`ie session-restore`

### 3.1 命令签名

```bash
ie session-restore [OPTIONS]

Options:
  --json              Output in JSON format (default)
  --markdown          Output in Markdown format (for debugging)
  --include-events N  Include last N events (default: 3)
  --workspace PATH    Specify workspace path (default: current directory)
```

### 3.2 返回数据结构

```json
{
  "status": "success" | "no_focus" | "error",
  "workspace_path": "/path/to/project",
  "current_task": {
    "id": 42,
    "name": "Implement JWT authentication",
    "status": "doing",
    "spec": "Use jsonwebtoken crate, HS256 algorithm, 7-day expiry...",
    "spec_preview": "Use jsonwebtoken crate, HS256 algorithm...",  // 前100字
    "created_at": "2025-11-10T10:00:00Z",
    "first_doing_at": "2025-11-10T10:05:00Z"
  },
  "parent_task": {
    "id": 40,
    "name": "Implement user authentication system"
  },
  "siblings": {
    "total": 3,
    "done": 1,
    "doing": 1,
    "todo": 1,
    "done_list": [
      { "id": 41, "name": "Design auth schema" }
    ]
  },
  "children": {
    "total": 2,
    "todo": 2,
    "list": [
      { "id": 43, "name": "Token generation logic", "status": "todo" },
      { "id": 44, "name": "Token validation logic", "status": "todo" }
    ]
  },
  "recent_events": [
    {
      "type": "decision",
      "data": "Chose HS256 over RS256 because we don't need key rotation yet",
      "timestamp": "2025-11-10T14:30:00Z"
    },
    {
      "type": "note",
      "data": "Research shows jsonwebtoken crate is most mature option",
      "timestamp": "2025-11-10T14:15:00Z"
    },
    {
      "type": "blocker",
      "data": "Need to decide on token storage strategy",
      "timestamp": "2025-11-10T11:00:00Z"
    }
  ],
  "suggested_commands": [
    "ie event list --type blocker",
    "ie task done",
    "ie task spawn-subtask"
  ]
}
```

### 3.3 无焦点场景

```json
{
  "status": "no_focus",
  "workspace_path": "/path/to/project",
  "current_task": null,
  "stats": {
    "total_tasks": 15,
    "todo": 8,
    "doing": 0,
    "done": 7
  },
  "suggested_commands": [
    "ie pick-next",
    "ie task list --status todo"
  ]
}
```

### 3.4 错误场景

```json
{
  "status": "error",
  "error_type": "workspace_not_found" | "database_corrupted" | "permission_denied",
  "message": "No Intent-Engine workspace found in current directory",
  "recovery_suggestion": "Run 'ie workspace init' to create a new workspace",
  "suggested_commands": [
    "ie workspace init",
    "ie help"
  ]
}
```

---

## 4. SessionStart Hook 实现

### 4.1 脚本位置

```
.claude/hooks/session-start.sh
```

### 4.2 完整脚本

```bash
#!/bin/bash
# Intent-Engine Session Restoration Hook
# Version: 1.0
# Triggers: Before Claude Code session starts

set -euo pipefail

# 1. 检测是否安装了 Intent-Engine
if ! command -v ie &> /dev/null; then
    echo "<system-reminder>"
    echo "Intent-Engine not found. Install: cargo install intent-engine"
    echo "</system-reminder>"
    exit 0
fi

# 2. 使用当前工作目录
WORKSPACE_DIR="${CLAUDE_WORKSPACE_ROOT:-$(pwd)}"

# 3. 调用 session-restore
RESTORE_OUTPUT=$(ie session-restore --json --workspace "$WORKSPACE_DIR" 2>&1)
RESTORE_EXIT_CODE=$?

# 4. 解析 JSON 并生成提示
if [ $RESTORE_EXIT_CODE -eq 0 ]; then
    STATUS=$(echo "$RESTORE_OUTPUT" | jq -r '.status')

    if [ "$STATUS" = "success" ]; then
        # === 有焦点：丰富上下文 ===
        TASK_ID=$(echo "$RESTORE_OUTPUT" | jq -r '.current_task.id')
        TASK_NAME=$(echo "$RESTORE_OUTPUT" | jq -r '.current_task.name')
        TASK_SPEC=$(echo "$RESTORE_OUTPUT" | jq -r '.current_task.spec_preview')
        PARENT_NAME=$(echo "$RESTORE_OUTPUT" | jq -r '.parent_task.name // "None"')

        SIBLINGS_DONE=$(echo "$RESTORE_OUTPUT" | jq -r '.siblings.done')
        SIBLINGS_TOTAL=$(echo "$RESTORE_OUTPUT" | jq -r '.siblings.total')

        CHILDREN_TODO=$(echo "$RESTORE_OUTPUT" | jq -r '.children.todo')
        CHILDREN_TOTAL=$(echo "$RESTORE_OUTPUT" | jq -r '.children.total')

        # 最近的决策
        RECENT_DECISIONS=$(echo "$RESTORE_OUTPUT" | jq -r '.recent_events[] | select(.type == "decision") | "- " + .data' | head -3)

        # 当前的阻塞
        CURRENT_BLOCKERS=$(echo "$RESTORE_OUTPUT" | jq -r '.recent_events[] | select(.type == "blocker") | "- " + .data')

        # 已完成的兄弟任务（证明进度）
        DONE_SIBLINGS=$(echo "$RESTORE_OUTPUT" | jq -r '.siblings.done_list[]? | "- #" + (.id|tostring) + " " + .name')

        # === 极简风格输出 ===
        echo "<system-reminder priority=\"high\">"
        echo "Intent-Engine: Session Restored"
        echo ""
        echo "Focus: #${TASK_ID} '${TASK_NAME}'"
        echo "Parent: ${PARENT_NAME}"
        echo "Progress: ${SIBLINGS_DONE}/${SIBLINGS_TOTAL} siblings done, ${CHILDREN_TODO} subtasks remain"
        echo ""

        if [ -n "$TASK_SPEC" ]; then
            echo "Spec: ${TASK_SPEC}"
            echo ""
        fi

        if [ -n "$DONE_SIBLINGS" ]; then
            echo "Completed:"
            echo "$DONE_SIBLINGS"
            echo ""
        fi

        if [ -n "$RECENT_DECISIONS" ]; then
            echo "Recent decisions:"
            echo "$RECENT_DECISIONS"
            echo ""
        fi

        if [ -n "$CURRENT_BLOCKERS" ]; then
            echo "⚠️  Blockers:"
            echo "$CURRENT_BLOCKERS"
            echo ""
        fi

        # === 下一步建议 ===
        if [ "$CHILDREN_TODO" -gt 0 ]; then
            echo "Next: Work on subtasks or use 'ie task done' when complete"
        else
            echo "Next: Complete this task with 'ie task done'"
        fi

        # === 极其克制的工具提示 ===
        echo ""
        echo "Commands: ie event add --type decision|blocker|note, ie task spawn-subtask, ie task done"
        echo "</system-reminder>"

    elif [ "$STATUS" = "no_focus" ]; then
        # === 无焦点：简洁引导 ===
        TODO_COUNT=$(echo "$RESTORE_OUTPUT" | jq -r '.stats.todo')

        echo "<system-reminder>"
        echo "Intent-Engine: No active focus"
        echo ""
        echo "Tasks: ${TODO_COUNT} pending"
        echo ""
        echo "Next: Use 'ie pick-next' to get a recommended task, or 'ie task list --status todo'"
        echo "</system-reminder>"

    elif [ "$STATUS" = "error" ]; then
        # === 错误：尝试恢复 ===
        ERROR_TYPE=$(echo "$RESTORE_OUTPUT" | jq -r '.error_type')
        ERROR_MSG=$(echo "$RESTORE_OUTPUT" | jq -r '.message')
        RECOVERY=$(echo "$RESTORE_OUTPUT" | jq -r '.recovery_suggestion')

        echo "<system-reminder>"
        echo "Intent-Engine: Issue detected"
        echo ""
        echo "${ERROR_MSG}"
        echo ""
        echo "Recovery: ${RECOVERY}"
        echo "</system-reminder>"
    fi
else
    # === Intent-Engine 调用失败：友好提示 ===
    echo "<system-reminder>"
    echo "Intent-Engine: Unable to restore session"
    echo ""
    echo "The workspace may not be initialized or there may be a configuration issue."
    echo "Consider: ie workspace init"
    echo "</system-reminder>"
fi
```

### 4.3 关键设计决策

| 决策点 | 选择 | 理由 |
|-------|------|------|
| 触发时机 | 会话开始前 | AI第一轮就有上下文 |
| 上下文丰富度 | 丰富（选项B） | 包含spec预览、已完成任务、决策历史 |
| 下一步建议 | 包含 | 给AI明确行动指引 |
| 工具提示 | 极其克制 | 只列出最常用的3-4个命令 |
| 输出风格 | 极简、纯数据 | 信息密度高，token效率高 |
| 错误处理A | 友好提示 | 告知用户问题存在 |
| 错误处理B | 尝试恢复 | 提供具体的恢复建议 |

---

## 5. 自动化部署：`ie setup-claude-code`

### 5.1 命令规格

```bash
ie setup-claude-code [OPTIONS]

Options:
  --dry-run           Show what would be done without actually doing it
  --claude-dir PATH   Specify .claude directory location (default: ./.claude)
  --force             Overwrite existing hook
```

### 5.2 执行逻辑

```rust
// 伪代码

fn setup_claude_code(opts: SetupOptions) -> Result<()> {
    // 1. 检测 .claude 目录
    let claude_dir = opts.claude_dir.unwrap_or("./.claude");
    if !claude_dir.exists() {
        if opts.dry_run {
            println!("Would create: {}", claude_dir);
        } else {
            fs::create_dir_all(claude_dir)?;
            println!("✓ Created {}", claude_dir);
        }
    }

    // 2. 检测 hooks 目录
    let hooks_dir = claude_dir.join("hooks");
    if !hooks_dir.exists() {
        if opts.dry_run {
            println!("Would create: {}", hooks_dir);
        } else {
            fs::create_dir_all(&hooks_dir)?;
            println!("✓ Created {}", hooks_dir);
        }
    }

    // 3. 安装 hook 脚本
    let hook_path = hooks_dir.join("session-start.sh");
    if hook_path.exists() && !opts.force {
        return Err("session-start.sh already exists. Use --force to overwrite");
    }

    if opts.dry_run {
        println!("Would write: {}", hook_path);
    } else {
        let hook_content = include_str!("../templates/session-start.sh");
        fs::write(&hook_path, hook_content)?;
        println!("✓ Installed session-start.sh");
    }

    // 4. 设置执行权限 (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !opts.dry_run {
            let mut perms = fs::metadata(&hook_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms)?;
            println!("✓ Set executable permissions");
        }
    }

    // 5. 验证安装
    if !opts.dry_run {
        println!("\n✅ Setup complete!");
        println!("\nNext steps:");
        println!("1. Start a new Claude Code session");
        println!("2. The session-start hook will automatically restore your focus");
        println!("\nDocumentation: docs/integration/claude-code-setup.md");
    }

    Ok(())
}
```

### 5.3 输出示例

```bash
$ ie setup-claude-code

✓ Found .claude directory
✓ Created .claude/hooks
✓ Installed session-start.sh
✓ Set executable permissions

✅ Setup complete!

Next steps:
1. Start a new Claude Code session
2. The session-start hook will automatically restore your focus

Documentation: docs/integration/claude-code-setup.md
```

---

## 6. 用户文档

### 6.1 文档位置

```
docs/integration/claude-code-setup.md
```

### 6.2 文档大纲

```markdown
# Claude Code Integration: Focus Restoration

## Quick Setup

```bash
# Automatic setup (recommended)
ie setup-claude-code

# Manual setup
cp templates/session-start.sh .claude/hooks/
chmod +x .claude/hooks/session-start.sh
```

## How It Works

[解释工作原理，配图]

## Customization

[如何自定义 hook 行为]

## Troubleshooting

### Hook not triggering
[解决方案]

### Permission denied
[解决方案]

### Intent-Engine not found
[解决方案]

## Advanced: Hook Template

[提供可定制的 hook 模板]
```

---

## 7. 测试计划

### 7.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_restore_with_focus() {
        // 设置测试数据库
        let db = setup_test_db();
        let task_id = db.add_task("Test task");
        db.set_current_task(task_id);

        // 执行 session-restore
        let result = session_restore(&db).unwrap();

        assert_eq!(result.status, "success");
        assert_eq!(result.current_task.unwrap().id, task_id);
    }

    #[test]
    fn test_session_restore_no_focus() {
        let db = setup_test_db();

        let result = session_restore(&db).unwrap();

        assert_eq!(result.status, "no_focus");
        assert!(result.current_task.is_none());
    }

    #[test]
    fn test_session_restore_error_handling() {
        let db = setup_corrupted_db();

        let result = session_restore(&db).unwrap();

        assert_eq!(result.status, "error");
        assert!(result.recovery_suggestion.is_some());
    }
}
```

### 7.2 集成测试

```bash
#!/bin/bash
# tests/integration/test-session-restore.sh

# 测试1: 有焦点场景
ie workspace init
ie task add "Test task" --spec "Test spec"
ie task start 1

OUTPUT=$(ie session-restore --json)
STATUS=$(echo "$OUTPUT" | jq -r '.status')
assert_eq "$STATUS" "success"

# 测试2: 无焦点场景
ie task done
OUTPUT=$(ie session-restore --json)
STATUS=$(echo "$OUTPUT" | jq -r '.status')
assert_eq "$STATUS" "no_focus"

# 测试3: 错误场景
rm -rf .intent-engine
OUTPUT=$(ie session-restore --json)
STATUS=$(echo "$OUTPUT" | jq -r '.status')
assert_eq "$STATUS" "error"
```

### 7.3 用户验收测试

**测试场景A: 跨会话恢复**
1. 创建任务并开始工作
2. 记录一些决策
3. 关闭会话
4. 开启新会话
5. 验证：AI 的第一条回复中提到了之前的任务

**测试场景B: 无焦点引导**
1. 完成所有正在做的任务
2. 关闭会话
3. 开启新会话
4. 验证：AI 提示使用 `ie pick-next`

**测试场景C: 错误恢复**
1. 损坏 Intent-Engine 数据库
2. 开启新会话
3. 验证：AI 提示恢复步骤

---

## 8. 实施路线图

### Week 1: 核心实现
- [ ] 实现 `ie session-restore` 命令
- [ ] 编写单元测试
- [ ] 实现 JSON 输出格式

### Week 2: Hook 和自动化
- [ ] 创建 session-start.sh 模板
- [ ] 实现 `ie setup-claude-code` 命令
- [ ] 编写集成测试

### Week 3: 文档和打磨
- [ ] 编写用户文档
- [ ] 创建示例和截图
- [ ] 错误处理优化

### Week 4: 验证和发布
- [ ] 用户验收测试
- [ ] 性能优化
- [ ] 发布到 crates.io

---

## 9. 成功指标

### 技术指标
- `ie session-restore` 执行时间 < 100ms
- Hook 脚本执行时间 < 200ms
- 单元测试覆盖率 > 80%

### 用户体验指标
- 用户设置成功率 > 95%
- AI 在新会话中正确引用焦点的比例 > 90%
- 用户反馈：跨会话工作连续性改善

---

## 10. 风险和缓解

| 风险 | 影响 | 概率 | 缓解措施 |
|-----|------|------|---------|
| Claude Code 不支持 SessionStart hook | 高 | 低 | 提供手动触发方式 |
| Hook 脚本执行失败导致会话无法启动 | 高 | 中 | 严格错误处理，失败时静默 |
| JSON 解析失败 | 中 | 低 | 使用 jq 安全解析，提供降级输出 |
| 性能问题（大型workspace） | 中 | 中 | 优化查询，限制 events 数量 |

---

## 11. 未来改进方向

### Phase 1.5: 增强（可选）
- 支持多 workspace 自动检测
- 缓存机制减少重复查询
- Hook 配置文件（用户可自定义输出格式）

### Phase 2: 持续强化
- 在工具输出中注入提醒
- 参见 `speckit-guardian.md` Phase 2

---

**文档版本**: 1.0
**最后更新**: 2025-11-13
**负责人**: Intent-Engine Core Team
**审核状态**: ✅ 已定稿，可以开始实施
