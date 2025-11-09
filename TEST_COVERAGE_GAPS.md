# 测试覆盖缺口分析报告

## 执行摘要

当前代码覆盖率：58% → 目标提升中

已发现的测试缺口：
- **高优先级**: 4 个函数/分支
- **中优先级**: 8 个边界情况
- **低优先级**: 3 个辅助函数

---

## 一、未测试的函数

### 1.1 高优先级缺失

#### ❌ `TaskManager::get_task_with_events`
**位置**: `src/tasks.rs:67-75`
**影响**: 核心功能，CLI 命令 `task get --with-events` 依赖此功能
**风险**: 高 - 事件汇总逻辑未验证

**建议测试**:
```rust
#[tokio::test]
async fn test_get_task_with_events() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());
    let event_mgr = EventManager::new(ctx.pool());

    let task = task_mgr.add_task("Test", None, None).await.unwrap();

    // Add some events
    event_mgr.add_event(task.id, "progress", "Event 1").await.unwrap();
    event_mgr.add_event(task.id, "decision", "Event 2").await.unwrap();

    let result = task_mgr.get_task_with_events(task.id).await.unwrap();

    assert!(result.events_summary.is_some());
    let summary = result.events_summary.unwrap();
    assert_eq!(summary.total_count, 2);
    assert_eq!(summary.recent_events.len(), 2);
}

#[tokio::test]
async fn test_get_task_with_events_nonexistent() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    let result = task_mgr.get_task_with_events(999).await;
    assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
}

#[tokio::test]
async fn test_get_task_with_many_events() {
    // Test that only 10 recent events are returned
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());
    let event_mgr = EventManager::new(ctx.pool());

    let task = task_mgr.add_task("Test", None, None).await.unwrap();

    // Add 20 events
    for i in 0..20 {
        event_mgr.add_event(task.id, "test", &format!("Event {}", i)).await.unwrap();
    }

    let result = task_mgr.get_task_with_events(task.id).await.unwrap();
    let summary = result.events_summary.unwrap();

    assert_eq!(summary.total_count, 20);
    assert_eq!(summary.recent_events.len(), 10); // Limited to 10
}
```

---

## 二、未充分测试的边界情况

### 2.1 `update_task` 边界情况

#### ⚠️ 空更新测试
**位置**: `src/tasks.rs:180-182`
**代码**:
```rust
if updates.is_empty() {
    return Ok(task);
}
```

**缺失测试**: 调用 update_task 但不传任何参数

**建议测试**:
```rust
#[tokio::test]
async fn test_update_task_no_changes() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    let task = task_mgr.add_task("Original", None, None).await.unwrap();

    // Call update with all None
    let result = task_mgr.update_task(task.id, None, None, None, None, None, None).await.unwrap();

    assert_eq!(result.name, "Original");
    assert_eq!(result.id, task.id);
}
```

#### ⚠️ 特殊字符处理
**位置**: `src/tasks.rs:139,143`
**代码**:
```rust
updates.push(format!("name = '{}'", n.replace('\'', "''")));
```

**缺失测试**: 名称/spec 包含单引号、双引号、SQL 注入尝试

**建议测试**:
```rust
#[tokio::test]
async fn test_update_task_with_special_characters() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    let task = task_mgr.add_task("Original", None, None).await.unwrap();

    // Test single quotes
    let result = task_mgr.update_task(
        task.id,
        Some("Task with 'quotes'"),
        Some("Spec with 'quotes'"),
        None, None, None, None
    ).await.unwrap();

    assert_eq!(result.name, "Task with 'quotes'");
    assert_eq!(result.spec.unwrap(), "Spec with 'quotes'");
}

#[tokio::test]
async fn test_update_task_sql_injection_attempt() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    let task = task_mgr.add_task("Original", None, None).await.unwrap();

    // Attempt SQL injection
    let malicious = "'; DROP TABLE tasks; --";
    let result = task_mgr.update_task(
        task.id,
        Some(malicious),
        None, None, None, None, None
    ).await.unwrap();

    // Should be safely escaped
    assert_eq!(result.name, malicious);

    // Verify table still exists
    let tasks = task_mgr.find_tasks(None, None).await.unwrap();
    assert!(!tasks.is_empty());
}
```

#### ⚠️ 移除父任务 (parent_id = NULL)
**位置**: `src/tasks.rs:146-150`
**代码**:
```rust
if let Some(pid) = parent_id {
    match pid {
        Some(p) => updates.push(format!("parent_id = {}", p)),
        None => updates.push("parent_id = NULL".to_string()),
    }
}
```

**缺失测试**: 将有父任务的任务改为顶层任务

**建议测试**:
```rust
#[tokio::test]
async fn test_update_task_remove_parent() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    let parent = task_mgr.add_task("Parent", None, None).await.unwrap();
    let child = task_mgr.add_task("Child", None, Some(parent.id)).await.unwrap();

    assert_eq!(child.parent_id, Some(parent.id));

    // Remove parent
    let result = task_mgr.update_task(
        child.id,
        None, None,
        Some(None), // parent_id = NULL
        None, None, None
    ).await.unwrap();

    assert_eq!(result.parent_id, None);
}
```

#### ⚠️ 状态转换时间戳
**位置**: `src/tasks.rs:164-178`
**代码**:
```rust
match s {
    "todo" if task.first_todo_at.is_none() => {...}
    "doing" if task.first_doing_at.is_none() => {...}
    "done" if task.first_done_at.is_none() => {...}
    _ => {}
}
```

**缺失测试**:
- 重复设置相同状态（不应更新时间戳）
- 状态倒退（todo -> doing -> todo）

**建议测试**:
```rust
#[tokio::test]
async fn test_update_task_status_timestamp_idempotent() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    let task = task_mgr.add_task("Test", None, None).await.unwrap();

    // Set to doing
    let result1 = task_mgr.update_task(
        task.id, None, None, None,
        Some("doing"), None, None
    ).await.unwrap();

    let first_doing_at = result1.first_doing_at.unwrap();

    // Set to doing again
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let result2 = task_mgr.update_task(
        task.id, None, None, None,
        Some("doing"), None, None
    ).await.unwrap();

    // Timestamp should not change
    assert_eq!(result2.first_doing_at.unwrap(), first_doing_at);
}

#[tokio::test]
async fn test_update_task_status_regression() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    let task = task_mgr.add_task("Test", None, None).await.unwrap();

    // todo -> doing -> todo
    let _ = task_mgr.update_task(
        task.id, None, None, None,
        Some("doing"), None, None
    ).await.unwrap();

    let result = task_mgr.update_task(
        task.id, None, None, None,
        Some("todo"), None, None
    ).await.unwrap();

    assert_eq!(result.status, "todo");
    assert!(result.first_todo_at.is_some());
    assert!(result.first_doing_at.is_some()); // Should preserve
}
```

### 2.2 `search_tasks` 边界情况

#### ⚠️ 空查询字符串
**位置**: `src/tasks.rs:242`

**建议测试**:
```rust
#[tokio::test]
async fn test_search_tasks_empty_query() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    task_mgr.add_task("Test 1", None, None).await.unwrap();
    task_mgr.add_task("Test 2", None, None).await.unwrap();

    let results = task_mgr.search_tasks("").await.unwrap();
    // Should return all tasks or no tasks?
    // Document expected behavior
}

#[tokio::test]
async fn test_search_tasks_whitespace_only() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    task_mgr.add_task("Test", None, None).await.unwrap();

    let results = task_mgr.search_tasks("   ").await.unwrap();
    // Document expected behavior
}
```

### 2.3 `done_task` 边界情况

#### ⚠️ 无当前任务时调用
**代码**: `src/tasks.rs:355`

**现有测试**: ✓ 有 (test_done_task_no_current_task)

#### ⚠️ 当前任务不在 doing 状态
**建议测试**:
```rust
#[tokio::test]
async fn test_done_task_not_in_doing_status() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());
    let workspace_mgr = WorkspaceManager::new(ctx.pool());

    let task = task_mgr.add_task("Test", None, None).await.unwrap();

    // Set as current but don't start it (still in todo)
    workspace_mgr.set_current_task(task.id).await.unwrap();

    let result = task_mgr.done_task().await;
    assert!(result.is_err());
    // Should return ActionNotAllowed
}
```

### 2.4 `pick_next_tasks` 边界情况

#### ⚠️ Capacity 为 0
**建议测试**:
```rust
#[tokio::test]
async fn test_pick_next_tasks_zero_capacity() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    task_mgr.add_task("Task 1", None, None).await.unwrap();

    let results = task_mgr.pick_next_tasks(0, None).await.unwrap();
    assert_eq!(results.len(), 0);
}
```

#### ⚠️ Capacity 大于可用任务数
**建议测试**:
```rust
#[tokio::test]
async fn test_pick_next_tasks_capacity_exceeds_available() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    task_mgr.add_task("Task 1", None, None).await.unwrap();
    task_mgr.add_task("Task 2", None, None).await.unwrap();

    let results = task_mgr.pick_next_tasks(10, None).await.unwrap();
    assert_eq!(results.len(), 2); // Only returns available tasks
}
```

---

## 三、集成测试缺口

### 3.1 CLI 命令测试

#### ⚠️ `task get --with-events`
**文件**: `tests/cli_tests.rs` 或 `tests/integration_tests.rs`

**缺失**: 没有测试 `--with-events` 标志

**建议**:
```rust
#[test]
fn test_task_get_with_events_flag() {
    let temp_dir = setup_test_env();

    // Add task with events
    let mut add = Command::cargo_bin("intent-engine").unwrap();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Get with events
    let mut get = Command::cargo_bin("intent-engine").unwrap();
    get.current_dir(temp_dir.path())
        .arg("task")
        .arg("get")
        .arg("1")
        .arg("--with-events");

    get.assert()
        .success()
        .stdout(predicate::str::contains("events_summary"));
}
```

### 3.2 错误处理测试

#### ⚠️ 数据库连接失败
**建议**:
```rust
#[tokio::test]
async fn test_database_connection_error() {
    let invalid_path = Path::new("/invalid/path/db.sqlite");

    let result = create_pool(invalid_path).await;
    // Should handle permission errors gracefully
}
```

#### ⚠️ 数据库文件损坏
**建议**:
```rust
#[tokio::test]
async fn test_corrupted_database() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("corrupt.db");

    // Create corrupted file
    std::fs::write(&db_path, b"not a database").unwrap();

    let result = create_pool(&db_path).await;
    assert!(result.is_err());
}
```

---

## 四、性能测试建议

### 4.1 并发写入
```rust
#[tokio::test]
async fn test_concurrent_task_updates() {
    let ctx = TestContext::new().await;
    let task_mgr = TaskManager::new(ctx.pool());

    let task = task_mgr.add_task("Test", None, None).await.unwrap();

    // Spawn multiple concurrent updates
    let handles: Vec<_> = (0..10).map(|i| {
        let task_mgr = TaskManager::new(ctx.pool());
        let task_id = task.id;
        tokio::spawn(async move {
            task_mgr.update_task(
                task_id,
                Some(&format!("Update {}", i)),
                None, None, None, None, None
            ).await
        })
    }).collect();

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Final state should be valid
    let final_task = task_mgr.get_task(task.id).await.unwrap();
    assert!(final_task.name.starts_with("Update"));
}
```

---

## 五、优先级总结

### 立即修复 (P0)
1. ✗ `get_task_with_events` - 核心功能未测试
2. ⚠️ `update_task` SQL 注入防护测试

### 近期修复 (P1)
3. ⚠️ `update_task` 特殊字符处理
4. ⚠️ `update_task` 空更新边界
5. ⚠️ 状态转换时间戳幂等性
6. ⚠️ `done_task` 状态验证

### 计划修复 (P2)
7. ⚠️ `search_tasks` 空查询
8. ⚠️ `pick_next_tasks` 边界值
9. ⚠️ CLI --with-events 标志
10. ⚠️ 数据库错误处理

---

## 六、行动建议

1. **立即**: 添加 `get_task_with_events` 测试（3个测试用例）
2. **本周**: 补充 `update_task` 边界测试（5个测试用例）
3. **本月**: 完成所有 P0 和 P1 测试
4. **Q1**: 达到 80% 代码覆盖率目标

## 七、测试覆盖率目标

当前: 58%
目标: 80%+
差距: 需要约 50-60 个额外测试用例
