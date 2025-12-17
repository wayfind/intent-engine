# 业务逻辑测试重写计划

## 背景

v0.10.0 简化了 CLI 接口，但保留了所有业务逻辑功能。部分业务逻辑测试通过旧 CLI 接口编写，需要重写为直接调用 library 函数。

## 原则

1. **CLI 测试** → Feature-gate (保留参考，默认禁用)
2. **业务逻辑测试** → 重写为 library 函数调用 (必须执行)

## 需要重写的测试

### 优先级 1: 核心业务逻辑

#### 1. pick_next_blocking_tests.rs (8 tests)
**业务逻辑**: 依赖阻塞下的 pick_next 推荐

**当前实现**:
```rust
cmd.arg("task").arg("add")...
cmd.arg("task").arg("depends-on")...
cmd.arg("task").arg("pick-next")...
```

**重写目标**:
```rust
use intent_engine::{tasks, dependencies};

#[tokio::test]
async fn test_pick_next_skips_blocked_task() {
    let pool = setup_test_db().await;
    let task1 = tasks::add_task(&pool, "Task 1", None, None).await.unwrap();
    let task2 = tasks::add_task(&pool, "Task 2", None, None).await.unwrap();
    dependencies::add_dependency(&pool, task2.id, task1.id).await.unwrap();

    let next = tasks::pick_next(&pool).await.unwrap();
    assert_eq!(next.unwrap().id, task1.id);
}
```

**测试用例**:
- test_pick_next_skips_blocked_task
- test_pick_next_blocked_subtask
- test_pick_next_unblocked_task_normal_behavior
- test_pick_next_no_available_tasks_due_to_blocking
- test_pick_next_recommends_after_blocking_complete
- test_pick_next_multiple_dependencies
- test_pick_next_respects_priority_with_blocking
- test_task_start_validation_blocked_task (可能重复)

#### 2. priority_and_list_tests.rs (9 tests)
**业务逻辑**: 优先级排序和列表查询

**重写目标**:
```rust
use intent_engine::tasks;

#[tokio::test]
async fn test_priority_sorting() {
    let pool = setup_test_db().await;
    tasks::add_task(&pool, "High", None, Some(tasks::Priority::High)).await.unwrap();
    tasks::add_task(&pool, "Low", None, Some(tasks::Priority::Low)).await.unwrap();

    let list = tasks::list_tasks(&pool, None, None, None, None).await.unwrap();
    assert_eq!(list[0].priority, tasks::Priority::High as i32);
}
```

#### 3. task_edge_cases_tests.rs
**业务逻辑**: 任务边界情况处理

**测试场景**:
- 空字符串任务名
- 超长任务名
- 特殊字符处理
- NULL 值处理
- 并发更新

**重写目标**:
```rust
#[tokio::test]
async fn test_empty_task_name() {
    let pool = setup_test_db().await;
    let result = tasks::add_task(&pool, "", None, None).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, ErrorCode::InvalidInput);
}
```

#### 4. task_start_blocking_tests.rs
**业务逻辑**: 任务启动时的依赖阻塞验证

**重写目标**:
```rust
#[tokio::test]
async fn test_start_blocked_task_fails() {
    let pool = setup_test_db().await;
    let task1 = tasks::add_task(&pool, "Blocking", None, None).await.unwrap();
    let task2 = tasks::add_task(&pool, "Blocked", None, None).await.unwrap();
    dependencies::add_dependency(&pool, task2.id, task1.id).await.unwrap();

    let result = tasks::start_task(&pool, task2.id, false).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, ErrorCode::TaskBlocked);
}
```

### 优先级 2: 系统行为

#### 5. smart_initialization_tests.rs
**业务逻辑**: 项目智能初始化

**考虑**: 可能需要保留部分 CLI 测试，但核心初始化逻辑应该有 library 测试

#### 6. windows_encoding_tests.rs
**业务逻辑**: Windows 平台编码处理

**考虑**: 如果是测试 `encoding_rs` 库的使用，应该保留

### 优先级 3: 可选/重构

#### 7. protocol_compliance_tests.rs
**性质**: 可能是测试 MCP 协议合规性

**考虑**: 如果是 MCP 相关，可以删除（v0.10.0 已移除 MCP）

## 重写步骤模板

### 1. 创建测试工具函数
```rust
// tests/common/mod.rs
use sqlx::SqlitePool;
use tempfile::TempDir;

pub async fn setup_test_db() -> (TempDir, SqlitePool) {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite:{}", db_path.display());

    let pool = SqlitePool::connect(&db_url).await.unwrap();
    intent_engine::db::run_migrations(&pool).await.unwrap();

    (temp_dir, pool)
}
```

### 2. 转换测试用例
```rust
// 旧: CLI 测试
#[test]
fn test_something() {
    let cmd = Command::new("ie");
    cmd.arg("task").arg("add")...
}

// 新: Library 测试
#[tokio::test]
async fn test_something() {
    let (_dir, pool) = setup_test_db().await;
    let result = tasks::add_task(&pool, ...).await;
    assert!(result.is_ok());
}
```

### 3. 运行验证
```bash
# 测试单个文件
cargo test --test pick_next_blocking_tests

# 测试所有 library
cargo test --lib

# 确保覆盖率不降低
cargo tarpaulin --out Html
```

## 时间估算

| 文件 | 测试数 | 估算时间 | 优先级 |
|------|-------|---------|--------|
| pick_next_blocking_tests.rs | 8 | 2-3 小时 | P0 |
| priority_and_list_tests.rs | 9 | 2-3 小时 | P0 |
| task_edge_cases_tests.rs | ? | 2-3 小时 | P0 |
| task_start_blocking_tests.rs | ? | 1-2 小时 | P0 |
| smart_initialization_tests.rs | ? | 2-3 小时 | P1 |
| windows_encoding_tests.rs | ? | 1-2 小时 | P1 |
| protocol_compliance_tests.rs | ? | 评估后决定 | P2 |

**总计**: ~12-18 小时工作量

## 当前状态

- ✅ **短期解决**: Feature gate 已添加，CI 通过
- 🚀 **中期目标**: 重写业务逻辑测试 (进行中)
- ✅ **已完成**: pick_next_blocking_tests.rs (7 tests, 272 lines)
- 📊 **覆盖率**: 380 library 测试 + 7 重写测试

## 参考

- Library 测试示例: `src/lib.rs` (380 个测试)
- 异步测试框架: `#[tokio::test]`
- 测试工具: `tests/common/mod.rs`
- 数据库初始化: `intent_engine::db::run_migrations()`

---

*Created: 2025-12-17*
*Status: 待实施 (Pending)*
