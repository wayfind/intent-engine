# Phase 1: Focus Restoration - Testing Specification

**版本**: 1.0
**状态**: 测试设计 - 待实施
**关联**: phase1-focus-restoration-spec.md

---

## 1. 测试策略

### 1.1 测试金字塔

```
        ┌─────────────┐
        │   E2E (5%)  │  用户验收测试
        ├─────────────┤
        │ Integration │  集成测试 (25%)
        │   (25%)     │
        ├─────────────┤
        │    Unit     │  单元测试 (70%)
        │   (70%)     │
        └─────────────┘
```

### 1.2 测试覆盖目标

- **单元测试覆盖率**: ≥ 80%
- **集成测试场景**: 100% 关键路径
- **E2E 测试**: 3个核心用户场景

---

## 2. 单元测试规格

### 2.1 `ie session-restore` 命令测试

#### Test Suite: SessionRestoreCommand

```rust
// src/commands/session_restore.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    // === 成功场景 ===

    #[test]
    fn test_restore_with_focus_minimal() {
        // Given: 有一个正在做的任务
        let db = setup_test_db();
        let task_id = db.add_task("Test task", None);
        db.set_current_task(task_id);

        // When: 执行 session-restore
        let result = session_restore(&db, Default::default()).unwrap();

        // Then: 返回成功状态
        assert_eq!(result.status, SessionStatus::Success);
        assert!(result.current_task.is_some());
        assert_eq!(result.current_task.unwrap().id, task_id);
    }

    #[test]
    fn test_restore_with_focus_rich_context() {
        // Given: 复杂的任务树
        let db = setup_test_db();

        // 创建父任务
        let parent_id = db.add_task("Parent task", None);

        // 创建3个子任务（1完成，1进行中，1待做）
        let sibling1 = db.add_task("Sibling 1", Some(parent_id));
        db.set_task_status(sibling1, TaskStatus::Done);

        let current = db.add_task("Current task", Some(parent_id));
        db.set_current_task(current);
        db.set_task_status(current, TaskStatus::Doing);

        let sibling3 = db.add_task("Sibling 3", Some(parent_id));

        // 当前任务有2个子任务
        let child1 = db.add_task("Child 1", Some(current));
        let child2 = db.add_task("Child 2", Some(current));

        // 添加一些事件
        db.add_event(current, EventType::Decision, "Decision 1");
        db.add_event(current, EventType::Blocker, "Blocker 1");
        db.add_event(current, EventType::Note, "Note 1");

        // When: 执行 session-restore
        let result = session_restore(&db, Default::default()).unwrap();

        // Then: 验证完整上下文
        assert_eq!(result.status, SessionStatus::Success);

        let task = result.current_task.unwrap();
        assert_eq!(task.id, current);

        // 验证父任务
        assert!(result.parent_task.is_some());
        assert_eq!(result.parent_task.unwrap().id, parent_id);

        // 验证兄弟任务统计
        assert_eq!(result.siblings.total, 3);
        assert_eq!(result.siblings.done, 1);
        assert_eq!(result.siblings.doing, 1);
        assert_eq!(result.siblings.todo, 1);
        assert_eq!(result.siblings.done_list.len(), 1);

        // 验证子任务
        assert_eq!(result.children.total, 2);
        assert_eq!(result.children.todo, 2);

        // 验证事件
        assert_eq!(result.recent_events.len(), 3);
        assert!(result.recent_events.iter().any(|e| e.event_type == EventType::Decision));
        assert!(result.recent_events.iter().any(|e| e.event_type == EventType::Blocker));
    }

    #[test]
    fn test_restore_with_spec_preview() {
        // Given: 任务有很长的 spec
        let db = setup_test_db();
        let long_spec = "a".repeat(200);
        let task_id = db.add_task_with_spec("Test task", &long_spec);
        db.set_current_task(task_id);

        // When: 执行 session-restore
        let result = session_restore(&db, Default::default()).unwrap();

        // Then: spec_preview 应该被截断到100字
        let task = result.current_task.unwrap();
        assert_eq!(task.spec.len(), 200);
        assert_eq!(task.spec_preview.len(), 100);
        assert!(task.spec_preview.ends_with("..."));
    }

    // === 无焦点场景 ===

    #[test]
    fn test_restore_no_focus() {
        // Given: 没有当前任务
        let db = setup_test_db();
        db.add_task("Task 1", None);
        db.add_task("Task 2", None);

        // When: 执行 session-restore
        let result = session_restore(&db, Default::default()).unwrap();

        // Then: 返回 no_focus 状态
        assert_eq!(result.status, SessionStatus::NoFocus);
        assert!(result.current_task.is_none());
        assert_eq!(result.stats.unwrap().total_tasks, 2);
    }

    // === 错误场景 ===

    #[test]
    fn test_restore_workspace_not_found() {
        // Given: workspace 不存在
        let non_existent_path = "/tmp/non-existent-workspace";

        // When: 执行 session-restore
        let result = session_restore_from_path(non_existent_path);

        // Then: 返回错误状态
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.status, SessionStatus::Error);
        assert_eq!(result.error_type.unwrap(), ErrorType::WorkspaceNotFound);
        assert!(result.recovery_suggestion.is_some());
    }

    #[test]
    fn test_restore_database_corrupted() {
        // Given: 数据库文件损坏
        let db_path = setup_corrupted_db();

        // When: 执行 session-restore
        let result = session_restore_from_path(&db_path);

        // Then: 返回错误状态
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.status, SessionStatus::Error);
        assert_eq!(result.error_type.unwrap(), ErrorType::DatabaseCorrupted);
        assert!(result.recovery_suggestion.unwrap().contains("init"));
    }

    // === 事件过滤 ===

    #[test]
    fn test_restore_recent_events_limit() {
        // Given: 任务有10个事件
        let db = setup_test_db();
        let task_id = db.add_task("Test task", None);
        db.set_current_task(task_id);

        for i in 0..10 {
            db.add_event(task_id, EventType::Note, &format!("Event {}", i));
        }

        // When: 执行 session-restore (默认返回3个)
        let result = session_restore(&db, Default::default()).unwrap();

        // Then: 只返回最近3个事件
        assert_eq!(result.recent_events.len(), 3);
        assert_eq!(result.recent_events[0].data, "Event 9"); // 最新的
    }

    #[test]
    fn test_restore_custom_events_limit() {
        // Given: 任务有10个事件
        let db = setup_test_db();
        let task_id = db.add_task("Test task", None);
        db.set_current_task(task_id);

        for i in 0..10 {
            db.add_event(task_id, EventType::Note, &format!("Event {}", i));
        }

        // When: 指定返回5个事件
        let opts = SessionRestoreOptions {
            include_events: 5,
            ..Default::default()
        };
        let result = session_restore(&db, opts).unwrap();

        // Then: 返回5个事件
        assert_eq!(result.recent_events.len(), 5);
    }

    // === 性能测试 ===

    #[test]
    fn test_restore_performance_small_workspace() {
        // Given: 10个任务
        let db = setup_test_db();
        for i in 0..10 {
            db.add_task(&format!("Task {}", i), None);
        }
        db.set_current_task(1);

        // When: 执行 session-restore
        let start = std::time::Instant::now();
        let result = session_restore(&db, Default::default()).unwrap();
        let duration = start.elapsed();

        // Then: 应该在 50ms 内完成
        assert!(duration.as_millis() < 50, "Took {}ms", duration.as_millis());
        assert_eq!(result.status, SessionStatus::Success);
    }

    #[test]
    fn test_restore_performance_large_workspace() {
        // Given: 1000个任务
        let db = setup_test_db();
        for i in 0..1000 {
            db.add_task(&format!("Task {}", i), None);
        }
        db.set_current_task(500);

        // When: 执行 session-restore
        let start = std::time::Instant::now();
        let result = session_restore(&db, Default::default()).unwrap();
        let duration = start.elapsed();

        // Then: 应该在 100ms 内完成
        assert!(duration.as_millis() < 100, "Took {}ms", duration.as_millis());
        assert_eq!(result.status, SessionStatus::Success);
    }
}
```

### 2.2 `ie setup-claude-code` 命令测试

#### Test Suite: SetupClaudeCodeCommand

```rust
// src/commands/setup_claude_code.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_setup_fresh_directory() {
        // Given: 一个空目录
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // When: 执行 setup
        let result = setup_claude_code(workspace, Default::default());

        // Then: 创建完整结构
        assert!(result.is_ok());
        assert!(workspace.join(".claude").exists());
        assert!(workspace.join(".claude/hooks").exists());
        assert!(workspace.join(".claude/hooks/session-start.sh").exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(
                workspace.join(".claude/hooks/session-start.sh")
            ).unwrap();
            let perms = metadata.permissions();
            assert_eq!(perms.mode() & 0o111, 0o111); // 可执行
        }
    }

    #[test]
    fn test_setup_existing_claude_dir() {
        // Given: .claude 目录已存在
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        std::fs::create_dir(workspace.join(".claude")).unwrap();

        // When: 执行 setup
        let result = setup_claude_code(workspace, Default::default());

        // Then: 成功安装 hook
        assert!(result.is_ok());
        assert!(workspace.join(".claude/hooks/session-start.sh").exists());
    }

    #[test]
    fn test_setup_existing_hook_without_force() {
        // Given: hook 已存在
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        setup_claude_code(workspace, Default::default()).unwrap();

        // When: 再次执行 setup（不带 --force）
        let result = setup_claude_code(workspace, Default::default());

        // Then: 应该失败
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_setup_existing_hook_with_force() {
        // Given: hook 已存在，且有旧内容
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        setup_claude_code(workspace, Default::default()).unwrap();

        let hook_path = workspace.join(".claude/hooks/session-start.sh");
        std::fs::write(&hook_path, "old content").unwrap();

        // When: 执行 setup --force
        let opts = SetupOptions { force: true, ..Default::default() };
        let result = setup_claude_code(workspace, opts);

        // Then: 成功覆盖
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("Intent-Engine Session Restoration Hook"));
    }

    #[test]
    fn test_setup_dry_run() {
        // Given: 一个空目录
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // When: 执行 setup --dry-run
        let opts = SetupOptions { dry_run: true, ..Default::default() };
        let result = setup_claude_code(workspace, opts);

        // Then: 不应该创建任何文件
        assert!(result.is_ok());
        assert!(!workspace.join(".claude").exists());
        assert!(!workspace.join(".claude/hooks").exists());
    }
}
```

---

## 3. 集成测试规格

### 3.1 完整工作流测试

```bash
#!/bin/bash
# tests/integration/test-session-restore-workflow.sh

set -euo pipefail

# 设置测试环境
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"

echo "=== Test 1: Complete workflow with focus ==="

# 1. 初始化 workspace
ie workspace init
assert_success "workspace init"

# 2. 创建任务树
ie task add "Implement authentication" --spec "Complete auth system with JWT and sessions"
assert_success "task add parent"

ie task start 1
ie task spawn-subtask "JWT implementation" --spec "Use jsonwebtoken crate, HS256"
ie task spawn-subtask "Session management"
ie task spawn-subtask "Password hashing"

# 3. 开始工作并记录决策
ie task start 2  # JWT implementation
ie event add --type decision --data "Chose HS256 algorithm for simplicity"
ie event add --type blocker --data "Need to decide on token storage location"
ie event add --type note --data "jsonwebtoken crate looks most mature"

# 4. 完成一个兄弟任务
ie task switch 4  # Password hashing
ie task done

# 5. 切回 JWT 任务
ie task switch 2

# 6. 执行 session-restore
RESTORE_OUTPUT=$(ie session-restore --json)

# 7. 验证输出
echo "$RESTORE_OUTPUT" | jq -e '.status == "success"'
assert_success "status is success"

echo "$RESTORE_OUTPUT" | jq -e '.current_task.id == 2'
assert_success "current task is JWT"

echo "$RESTORE_OUTPUT" | jq -e '.parent_task.id == 1'
assert_success "parent task is auth"

echo "$RESTORE_OUTPUT" | jq -e '.siblings.total == 3'
assert_success "3 siblings total"

echo "$RESTORE_OUTPUT" | jq -e '.siblings.done == 1'
assert_success "1 sibling done"

echo "$RESTORE_OUTPUT" | jq -e '.recent_events | length == 3'
assert_success "3 recent events"

# 验证事件类型
echo "$RESTORE_OUTPUT" | jq -e '.recent_events[] | select(.type == "decision")'
assert_success "has decision event"

echo "$RESTORE_OUTPUT" | jq -e '.recent_events[] | select(.type == "blocker")'
assert_success "has blocker event"

echo "✓ Test 1 passed"

# === Test 2: No focus scenario ===

echo "=== Test 2: No focus scenario ==="

ie task done  # 完成当前任务

RESTORE_OUTPUT=$(ie session-restore --json)

echo "$RESTORE_OUTPUT" | jq -e '.status == "no_focus"'
assert_success "status is no_focus"

echo "$RESTORE_OUTPUT" | jq -e '.current_task == null'
assert_success "no current task"

echo "$RESTORE_OUTPUT" | jq -e '.stats.total_tasks > 0'
assert_success "has total tasks stat"

echo "✓ Test 2 passed"

# === Test 3: Error scenario ===

echo "=== Test 3: Error scenario ==="

cd /tmp
RESTORE_OUTPUT=$(ie session-restore --json 2>&1 || true)

echo "$RESTORE_OUTPUT" | jq -e '.status == "error"'
assert_success "status is error"

echo "$RESTORE_OUTPUT" | jq -e '.error_type == "workspace_not_found"'
assert_success "error type correct"

echo "$RESTORE_OUTPUT" | jq -e '.recovery_suggestion != null'
assert_success "has recovery suggestion"

echo "✓ Test 3 passed"

# 清理
rm -rf "$TEST_DIR"

echo "✅ All integration tests passed!"
```

### 3.2 Hook 集成测试

```bash
#!/bin/bash
# tests/integration/test-session-start-hook.sh

set -euo pipefail

TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"

echo "=== Test: SessionStart Hook Integration ==="

# 1. 设置环境
ie workspace init
ie setup-claude-code

# 2. 创建任务并设置焦点
ie task add "Test task" --spec "This is a test specification with enough text to test preview truncation functionality"
ie task start 1
ie event add --type decision --data "Made a decision"

# 3. 模拟 SessionStart hook 触发
export CLAUDE_WORKSPACE_ROOT="$TEST_DIR"
HOOK_OUTPUT=$(.claude/hooks/session-start.sh)

# 4. 验证输出格式
echo "$HOOK_OUTPUT" | grep -q "Intent-Engine: Session Restored"
assert_success "has header"

echo "$HOOK_OUTPUT" | grep -q "Focus: #1 'Test task'"
assert_success "has focus line"

echo "$HOOK_OUTPUT" | grep -q "Spec:"
assert_success "has spec"

echo "$HOOK_OUTPUT" | grep -q "Recent decisions:"
assert_success "has decisions section"

echo "$HOOK_OUTPUT" | grep -q "Commands:"
assert_success "has commands hint"

# 5. 验证是否是 <system-reminder> 格式
echo "$HOOK_OUTPUT" | grep -q "<system-reminder"
assert_success "has system-reminder tag"

echo "$HOOK_OUTPUT" | grep -q "</system-reminder>"
assert_success "has closing tag"

echo "✓ Hook integration test passed"

rm -rf "$TEST_DIR"
```

---

## 4. 端到端（E2E）测试

### 4.1 E2E Test 1: 跨会话工作恢复

**测试场景**：用户在一个会话中开始任务，关闭，然后在新会话中继续

**步骤**：

```gherkin
Feature: Cross-session work restoration

Scenario: User resumes work in a new session
  Given I have initialized a workspace
  And I created a task "Implement authentication"
  And I started working on it
  And I recorded a decision "Using JWT with HS256"

  When I close the session
  And I start a new session

  Then the session-start hook should trigger
  And I should see "Focus: #1 'Implement authentication'"
  And I should see "Recent decisions: Using JWT with HS256"
  And I should see suggested commands
```

**验证标准**：
- [ ] Hook 在会话开始时自动触发
- [ ] 输出包含任务名称和ID
- [ ] 输出包含最近的决策
- [ ] 输出格式符合 <system-reminder> 规范
- [ ] AI 在第一轮对话中引用了焦点任务

### 4.2 E2E Test 2: 无焦点引导

**测试场景**：用户完成所有任务后开启新会话

```gherkin
Feature: No focus guidance

Scenario: User has no active task
  Given I have a workspace with tasks
  And all tasks are either "done" or "todo"
  And no task is currently focused

  When I start a new session

  Then the session-start hook should trigger
  And I should see "No active focus"
  And I should see "Use 'ie pick-next' to get a recommended task"
  And I should NOT see task-specific details
```

**验证标准**：
- [ ] Hook 正确识别无焦点状态
- [ ] 输出简洁，只包含引导信息
- [ ] 建议使用 `ie pick-next`

### 4.3 E2E Test 3: 错误恢复

**测试场景**：Intent-Engine 状态异常时的友好提示

```gherkin
Feature: Error recovery guidance

Scenario: Database is corrupted
  Given I have a workspace
  And the Intent-Engine database is corrupted

  When I start a new session

  Then the session-start hook should not crash
  And I should see a friendly error message
  And I should see recovery suggestions like "ie workspace init"
```

**验证标准**：
- [ ] Hook 不会因错误导致会话无法启动
- [ ] 错误消息清晰且有帮助
- [ ] 提供具体的恢复步骤

---

## 5. 性能基准测试

### 5.1 性能目标

| 场景 | 目标 | 测量方法 |
|-----|------|---------|
| `ie session-restore` (10个任务) | < 50ms | 单元测试 |
| `ie session-restore` (100个任务) | < 80ms | 单元测试 |
| `ie session-restore` (1000个任务) | < 100ms | 性能测试 |
| SessionStart hook 完整执行 | < 200ms | 集成测试 |

### 5.2 性能测试脚本

```bash
#!/bin/bash
# tests/performance/benchmark-session-restore.sh

set -euo pipefail

echo "=== Performance Benchmark: session-restore ==="

# Test 1: Small workspace (10 tasks)
echo "Test 1: 10 tasks"
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
ie workspace init

for i in {1..10}; do
    ie task add "Task $i" > /dev/null
done
ie task start 1

time_output=$( { time ie session-restore --json > /dev/null; } 2>&1 )
duration=$(echo "$time_output" | grep real | awk '{print $2}')
echo "Duration: $duration"

# Test 2: Medium workspace (100 tasks)
echo "Test 2: 100 tasks"
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
ie workspace init

for i in {1..100}; do
    ie task add "Task $i" > /dev/null
done
ie task start 50

time_output=$( { time ie session-restore --json > /dev/null; } 2>&1 )
duration=$(echo "$time_output" | grep real | awk '{print $2}')
echo "Duration: $duration"

# Test 3: Large workspace (1000 tasks)
echo "Test 3: 1000 tasks"
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
ie workspace init

for i in {1..1000}; do
    ie task add "Task $i" > /dev/null
done
ie task start 500

time_output=$( { time ie session-restore --json > /dev/null; } 2>&1 )
duration=$(echo "$time_output" | grep real | awk '{print $2}')
echo "Duration: $duration"
```

---

## 6. 测试数据工厂

### 6.1 测试工具函数

```rust
// src/test_utils.rs

use crate::database::Database;
use tempfile::TempDir;

pub struct TestDb {
    _temp_dir: TempDir,
    pub db: Database,
}

impl TestDb {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();
        TestDb {
            _temp_dir: temp_dir,
            db,
        }
    }

    pub fn add_task(&self, name: &str, parent_id: Option<u32>) -> u32 {
        self.db.add_task(name, None, parent_id).unwrap()
    }

    pub fn add_task_with_spec(&self, name: &str, spec: &str) -> u32 {
        self.db.add_task(name, Some(spec), None).unwrap()
    }

    pub fn set_current_task(&self, task_id: u32) {
        self.db.set_current_task(task_id).unwrap();
    }

    pub fn set_task_status(&self, task_id: u32, status: TaskStatus) {
        self.db.update_task_status(task_id, status).unwrap();
    }

    pub fn add_event(&self, task_id: u32, event_type: EventType, data: &str) {
        self.db.add_event(task_id, event_type, data).unwrap();
    }
}

pub fn setup_test_db() -> TestDb {
    TestDb::new()
}

pub fn setup_complex_task_tree() -> TestDb {
    let db = TestDb::new();

    // 创建一个复杂的任务树用于测试
    let root = db.add_task("Root project", None);

    let feature1 = db.add_task("Feature 1", Some(root));
    let feature2 = db.add_task("Feature 2", Some(root));

    let subtask1 = db.add_task("Subtask 1.1", Some(feature1));
    let subtask2 = db.add_task("Subtask 1.2", Some(feature1));

    db.set_task_status(subtask1, TaskStatus::Done);
    db.set_current_task(subtask2);

    db.add_event(subtask2, EventType::Decision, "Test decision");

    db
}

pub fn setup_corrupted_db() -> PathBuf {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("corrupted.db");
    std::fs::write(&db_path, b"invalid sqlite data").unwrap();
    db_path
}
```

---

## 7. 持续集成配置

### 7.1 GitHub Actions Workflow

```yaml
# .github/workflows/phase1-tests.yml

name: Phase 1 Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run unit tests
        run: cargo test --lib
      - name: Generate coverage
        run: cargo tarpaulin --out Xml
      - name: Upload coverage
        uses: codecov/codecov-action@v3

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build
        run: cargo build --release
      - name: Install
        run: cargo install --path .
      - name: Run integration tests
        run: ./tests/integration/run-all.sh

  performance-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build
        run: cargo build --release
      - name: Install
        run: cargo install --path .
      - name: Run benchmarks
        run: ./tests/performance/benchmark-session-restore.sh
```

---

## 8. 测试检查清单

### 8.1 开发时检查

在提交代码前，确保：

- [ ] 所有单元测试通过 (`cargo test`)
- [ ] 代码覆盖率 ≥ 80% (`cargo tarpaulin`)
- [ ] 集成测试通过 (`./tests/integration/run-all.sh`)
- [ ] Clippy 无警告 (`cargo clippy`)
- [ ] 格式化正确 (`cargo fmt --check`)

### 8.2 发布前检查

在发布到 crates.io 前，确保：

- [ ] 所有测试通过（unit + integration + e2e）
- [ ] 性能基准达标
- [ ] 文档完整且准确
- [ ] CHANGELOG 更新
- [ ] 版本号正确（遵循 SemVer）

---

## 9. 测试覆盖矩阵

| 功能 | 单元测试 | 集成测试 | E2E测试 |
|-----|---------|---------|---------|
| session-restore 基本功能 | ✅ | ✅ | ✅ |
| session-restore 丰富上下文 | ✅ | ✅ | ✅ |
| session-restore 无焦点 | ✅ | ✅ | ✅ |
| session-restore 错误处理 | ✅ | ✅ | ✅ |
| spec 预览截断 | ✅ | ❌ | ❌ |
| 事件数量限制 | ✅ | ✅ | ❌ |
| setup-claude-code 安装 | ✅ | ✅ | ✅ |
| setup-claude-code 覆盖保护 | ✅ | ❌ | ❌ |
| SessionStart hook 格式 | ❌ | ✅ | ✅ |
| SessionStart hook 性能 | ❌ | ✅ | ❌ |
| 跨会话工作恢复 | ❌ | ❌ | ✅ |

---

## 10. 测试实施时间表

| Week | 测试任务 | 负责人 | 状态 |
|------|---------|--------|------|
| 1 | 实现测试工具函数 (test_utils.rs) | - | 待开始 |
| 1 | session-restore 单元测试 | - | 待开始 |
| 2 | setup-claude-code 单元测试 | - | 待开始 |
| 2 | 集成测试脚本 (workflow, hook) | - | 待开始 |
| 3 | E2E 测试场景 | - | 待开始 |
| 3 | 性能基准测试 | - | 待开始 |
| 4 | CI/CD 配置 | - | 待开始 |
| 4 | 测试文档完善 | - | 待开始 |

---

**文档版本**: 1.0
**最后更新**: 2025-11-13
**状态**: ✅ 测试设计完成，可以开始实施
