# Phase 2: 搜索重构与统一 - 详细实施方案

**任务ID**: #43
**状态**: 等待 Task #39 完成
**预计时间**: 3.5小时
**创建时间**: 2025-11-28

---

## 执行前提条件

⚠️ **必须等待 Task #39 (Phase 1) 完成后才能开始实施**

**依赖检查**:
```bash
ie task get 39  # 确认 status = "done"
ie task get 40  # ✅ DONE
ie task get 41  # 🔄 DOING (当前正在进行)
ie task get 42  # ⏳ TODO
```

---

## 当前问题分析

### 调用链混乱

```
SearchManager::unified_search()     [search.rs:329]
  ├─→ TaskManager::search_tasks()   [tasks.rs:504]
  │   ├─→ search_tasks_fts5()      [3+ 字符，ORDER BY rank]
  │   └─→ search_tasks_like()      [短CJK，ORDER BY name] ← 排序不一致!
  └─→ EventManager::search_events_fts5()
```

### 关键问题

1. ❌ **排序不一致**: FTS5用rank，LIKE用name，跨边界时用户体验突变
2. ❌ **固定比例**: unified_search 固定任务:事件=1:1，无法调整
3. ❌ **无分页支持**: 返回所有结果，无offset
4. ❌ **职责分散**: 搜索逻辑分散在 tasks.rs 和 search.rs

### 调用点分析（3个模块）

| 文件 | 行号 | 函数 | 调用方式 |
|------|------|------|----------|
| `src/cli_handlers/other.rs` | 158 | CLI search | `unified_search()` |
| `src/mcp/server.rs` | 791 | MCP tool | `handle_unified_search()` |
| `src/dashboard/handlers.rs` | 620 | Dashboard API | `unified_search()` |

**测试文件调用**:
- `src/search.rs`: 4个单元测试
- 无专门的集成测试

---

## 新设计方案

### 废弃方法

- ❌ `TaskManager::search_tasks()` (tasks.rs:504)
- ❌ `SearchManager::unified_search()` (search.rs:329)

### 新统一方法

```rust
// src/search.rs
impl SearchManager<'_> {
    pub async fn search(
        &self,
        query: &str,
        include_tasks: bool,      // 默认 true
        include_events: bool,     // 默认 true
        limit: Option<i64>,       // 默认 20
        offset: Option<i64>,      // 默认 0
        sort_by_priority: bool,   // 默认 false（仅按相关度）
    ) -> Result<PaginatedSearchResults>
}
```

### 新数据结构

```rust
// src/db/models.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedSearchResults {
    pub results: Vec<UnifiedSearchResult>,  // 混合任务+事件
    pub total_tasks: i64,                   // 任务总数
    pub total_events: i64,                  // 事件总数
    pub has_more: bool,                     // 是否还有更多结果
    pub limit: i64,                         // 当前分页限制
    pub offset: i64,                        // 当前偏移量
}
```

### 统一排序策略

**主排序**: 相关度（FTS5 rank）
**次级排序**（可选）: priority（当 sort_by_priority=true）

```sql
-- 任务搜索
SELECT
    t.*,
    rank,
    snippet(tasks_fts, 1, '**', '**', '...', 15) as snippet_spec,
    snippet(tasks_fts, 0, '**', '**', '...', 15) as snippet_name
FROM tasks_fts
INNER JOIN tasks t ON tasks_fts.rowid = t.id
WHERE tasks_fts MATCH ?
ORDER BY
  rank ASC,                           -- 相关度优先
  COALESCE(priority, 999) ASC,        -- 可选次级排序
  id ASC
LIMIT ? OFFSET ?

-- 事件搜索（相同逻辑）
SELECT
    e.*,
    rank,
    snippet(events_fts, 0, '**', '**', '...', 15) as match_snippet
FROM events_fts
INNER JOIN events e ON events_fts.rowid = e.id
WHERE events_fts MATCH ?
ORDER BY
  rank ASC,
  id ASC
LIMIT ? OFFSET ?
```

---

## 实施计划

### 任务44: 重构 SearchManager::search() 统一搜索（2.5h）

**文件**: `src/search.rs`, `src/db/models.rs`

#### Step 1: 添加数据结构（0.5h）

**位置**: `src/db/models.rs` (在 UnifiedSearchResult 后)

```rust
/// Paginated search results containing both tasks and events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedSearchResults {
    /// Mixed results (tasks and events ordered by relevance)
    pub results: Vec<UnifiedSearchResult>,
    /// Total number of matching tasks
    pub total_tasks: i64,
    /// Total number of matching events
    pub total_events: i64,
    /// Whether there are more results beyond this page
    pub has_more: bool,
    /// Current page limit
    pub limit: i64,
    /// Current offset
    pub offset: i64,
}
```

**导出**: 在 `src/db/mod.rs` 添加 `pub use models::PaginatedSearchResults;`

#### Step 2: 实现新的 search() 方法（1.5h）

**位置**: `src/search.rs` (保留现有 unified_search，新增 search)

```rust
impl<'a> SearchManager<'a> {
    /// Unified search across tasks and events with pagination
    ///
    /// # Parameters
    /// - `query`: FTS5 search query string
    /// - `include_tasks`: Whether to search in tasks (default: true)
    /// - `include_events`: Whether to search in events (default: true)
    /// - `limit`: Maximum number of total results (default: 20)
    /// - `offset`: Pagination offset (default: 0)
    /// - `sort_by_priority`: Use priority as secondary sort (default: false)
    ///
    /// # Returns
    /// PaginatedSearchResults with mixed task/event results ordered by relevance
    pub async fn search(
        &self,
        query: &str,
        include_tasks: bool,
        include_events: bool,
        limit: Option<i64>,
        offset: Option<i64>,
        sort_by_priority: bool,
    ) -> Result<PaginatedSearchResults> {
        // Apply defaults
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        // Validate query (same as before)
        if query.trim().is_empty() {
            return Ok(PaginatedSearchResults {
                results: vec![],
                total_tasks: 0,
                total_events: 0,
                has_more: false,
                limit,
                offset,
            });
        }

        // Check for searchable content
        let has_searchable = query.chars()
            .any(|c| c.is_alphanumeric() || is_cjk_char(c));
        if !has_searchable {
            return Ok(PaginatedSearchResults {
                results: vec![],
                total_tasks: 0,
                total_events: 0,
                has_more: false,
                limit,
                offset,
            });
        }

        // Escape FTS5 query
        let escaped_query = escape_fts5(query);

        // Search tasks and events in parallel
        let mut all_results = Vec::new();
        let mut total_tasks = 0i64;
        let mut total_events = 0i64;

        if include_tasks {
            let (tasks, count) = self.search_tasks_fts5_paginated(
                &escaped_query,
                limit,
                offset,
                sort_by_priority
            ).await?;
            total_tasks = count;
            all_results.extend(tasks);
        }

        if include_events {
            let (events, count) = self.search_events_fts5_paginated(
                &escaped_query,
                limit,
                offset
            ).await?;
            total_events = count;
            all_results.extend(events);
        }

        // Sort by relevance (assume results already sorted by rank from DB)
        // Limit to requested page size
        all_results.truncate(limit as usize);

        let has_more = (total_tasks + total_events) > (offset + limit);

        Ok(PaginatedSearchResults {
            results: all_results,
            total_tasks,
            total_events,
            has_more,
            limit,
            offset,
        })
    }

    /// Search tasks using FTS5 with pagination
    async fn search_tasks_fts5_paginated(
        &self,
        escaped_query: &str,
        limit: i64,
        offset: i64,
        sort_by_priority: bool,
    ) -> Result<(Vec<UnifiedSearchResult>, i64)> {
        // Count total matches
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM tasks_fts WHERE tasks_fts MATCH ?"
        )
        .bind(escaped_query)
        .fetch_one(self.pool)
        .await?;
        let total_count: i64 = count_row.get("count");

        // Build ORDER BY clause
        let order_clause = if sort_by_priority {
            "ORDER BY rank ASC, COALESCE(t.priority, 999) ASC, t.id ASC"
        } else {
            "ORDER BY rank ASC, t.id ASC"
        };

        let query_str = format!(
            r#"
            SELECT
                t.id,
                t.parent_id,
                t.name,
                t.spec,
                t.status,
                t.complexity,
                t.priority,
                t.first_todo_at,
                t.first_doing_at,
                t.first_done_at,
                t.active_form,
                COALESCE(
                    snippet(tasks_fts, 1, '**', '**', '...', 15),
                    snippet(tasks_fts, 0, '**', '**', '...', 15)
                ) as match_snippet
            FROM tasks_fts
            INNER JOIN tasks t ON tasks_fts.rowid = t.id
            WHERE tasks_fts MATCH ?
            {}
            LIMIT ? OFFSET ?
            "#,
            order_clause
        );

        let results = sqlx::query(&query_str)
            .bind(escaped_query)
            .bind(limit)
            .bind(offset)
            .fetch_all(self.pool)
            .await?;

        let mut search_results = Vec::new();
        for row in results {
            let task = Task {
                id: row.get("id"),
                parent_id: row.get("parent_id"),
                name: row.get("name"),
                spec: row.get("spec"),
                status: row.get("status"),
                complexity: row.get("complexity"),
                priority: row.get("priority"),
                first_todo_at: row.get("first_todo_at"),
                first_doing_at: row.get("first_doing_at"),
                first_done_at: row.get("first_done_at"),
                active_form: row.get("active_form"),
            };
            let match_snippet: String = row.get("match_snippet");

            // Determine match field
            let match_field = if match_snippet.to_lowercase()
                .contains(&task.name.to_lowercase()) {
                "name".to_string()
            } else {
                "spec".to_string()
            };

            search_results.push(UnifiedSearchResult::Task {
                task,
                match_snippet,
                match_field,
            });
        }

        Ok((search_results, total_count))
    }

    /// Search events using FTS5 with pagination
    async fn search_events_fts5_paginated(
        &self,
        escaped_query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<UnifiedSearchResult>, i64)> {
        let event_mgr = EventManager::new(self.pool);

        // Count total matches
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM events_fts WHERE events_fts MATCH ?"
        )
        .bind(escaped_query)
        .fetch_one(self.pool)
        .await?;
        let total_count: i64 = count_row.get("count");

        // Get paginated results
        let events = event_mgr
            .search_events_fts5(escaped_query, Some(limit))
            .await?;

        let task_mgr = TaskManager::new(self.pool);
        let mut results = Vec::new();

        for event_result in events.into_iter().skip(offset as usize) {
            let task_chain = task_mgr
                .get_task_ancestry(event_result.event.task_id)
                .await?;

            results.push(UnifiedSearchResult::Event {
                event: event_result.event,
                task_chain,
                match_snippet: event_result.match_snippet,
            });
        }

        Ok((results, total_count))
    }
}
```

#### Step 3: 添加单元测试（0.5h）

**位置**: `src/search.rs` (在现有测试后)

```rust
#[cfg(test)]
mod new_search_tests {
    use super::*;
    use crate::test_utils::test_helpers::TestContext;

    #[tokio::test]
    async fn test_search_with_pagination() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let search_mgr = SearchManager::new(ctx.pool());

        // Create 5 test tasks
        for i in 0..5 {
            task_mgr
                .add_task(&format!("JWT Task {}", i), None, None)
                .await
                .unwrap();
        }

        // First page (limit=2, offset=0)
        let results = search_mgr
            .search("JWT", true, false, Some(2), Some(0), false)
            .await
            .unwrap();

        assert_eq!(results.results.len(), 2);
        assert_eq!(results.total_tasks, 5);
        assert!(results.has_more);
        assert_eq!(results.limit, 2);
        assert_eq!(results.offset, 0);

        // Second page (limit=2, offset=2)
        let results = search_mgr
            .search("JWT", true, false, Some(2), Some(2), false)
            .await
            .unwrap();

        assert_eq!(results.results.len(), 2);
        assert_eq!(results.total_tasks, 5);
        assert!(results.has_more);
    }

    #[tokio::test]
    async fn test_search_defaults() {
        let ctx = TestContext::new().await;
        let search_mgr = SearchManager::new(ctx.pool());
        let task_mgr = TaskManager::new(ctx.pool());

        task_mgr.add_task("Test", None, None).await.unwrap();

        // Test default parameters
        let results = search_mgr
            .search("Test", true, true, None, None, false)
            .await
            .unwrap();

        assert_eq!(results.limit, 20);  // default limit
        assert_eq!(results.offset, 0);  // default offset
    }
}
```

### 任务45: 更新所有调用点并删除旧方法（1h）

#### Step 1: 更新 CLI 命令（0.3h）

**文件**: `src/cli_handlers/other.rs`

```rust
// 在 SearchArgs 结构体添加 offset 参数
#[derive(Args, Debug)]
pub struct SearchArgs {
    query: String,

    #[arg(long, default_value = "true")]
    include_tasks: bool,

    #[arg(long, default_value = "true")]
    include_events: bool,

    #[arg(long)]
    limit: Option<i64>,

    #[arg(long)]  // 新增
    offset: Option<i64>,
}

// 更新 handle_search 函数
pub async fn handle_search(args: SearchArgs) -> Result<()> {
    let ctx = create_context().await?;
    let search_mgr = SearchManager::new(&ctx.pool);

    let results = search_mgr
        .search(
            &args.query,
            args.include_tasks,
            args.include_events,
            args.limit,
            args.offset,  // 新增
            false,        // sort_by_priority 默认 false
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}
```

#### Step 2: 更新 MCP 工具（0.3h）

**文件**: `src/mcp/server.rs`

```rust
// 更新 handle_unified_search
async fn handle_unified_search(args: Value) -> Result<Value, String> {
    use crate::search::SearchManager;

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: query".to_string())?;

    let include_tasks = args
        .get("include_tasks")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let include_events = args
        .get("include_events")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let limit = args
        .get("limit")
        .and_then(|v| v.as_i64());

    let offset = args  // 新增
        .get("offset")
        .and_then(|v| v.as_i64());

    let ctx = crate::create_context()
        .await
        .map_err(|e| format!("Failed to create context: {}", e))?;

    let search_mgr = SearchManager::new(&ctx.pool);
    let results = search_mgr
        .search(query, include_tasks, include_events, limit, offset, false)
        .await
        .map_err(|e| format!("Failed to perform search: {}", e))?;

    serde_json::to_value(&results).map_err(|e| format!("Serialization error: {}", e))
}
```

**文件**: `mcp-server.json` (更新 tool schema)

```json
{
  "name": "search",
  "description": "Unified search across tasks and events",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "FTS5 search query"
      },
      "include_tasks": {
        "type": "boolean",
        "description": "Search in tasks (default: true)",
        "default": true
      },
      "include_events": {
        "type": "boolean",
        "description": "Search in events (default: true)",
        "default": true
      },
      "limit": {
        "type": "integer",
        "description": "Maximum results (default: 20)",
        "default": 20
      },
      "offset": {
        "type": "integer",
        "description": "Pagination offset (default: 0)",
        "default": 0
      }
    },
    "required": ["query"]
  }
}
```

#### Step 3: 更新 Dashboard API（0.2h）

**文件**: `src/dashboard/handlers.rs`

```rust
// 更新 SearchQuery 结构体
#[derive(Deserialize)]
struct SearchQuery {
    query: String,
    #[serde(default = "default_true")]
    include_tasks: bool,
    #[serde(default = "default_true")]
    include_events: bool,
    limit: Option<u32>,
    offset: Option<u32>,  // 新增
}

// 更新 search_unified 函数
pub async fn search_unified(
    Query(query): Query<SearchQuery>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let search_mgr = SearchManager::new(&pool);

    match search_mgr
        .search(
            &query.query,
            query.include_tasks,
            query.include_events,
            query.limit.map(|l| l as i64),
            query.offset.map(|o| o as i64),  // 新增
            false,
        )
        .await
    {
        Ok(results) => Json(results).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Search failed: {}", e),
        )
            .into_response(),
    }
}
```

#### Step 4: 删除旧方法（0.2h）

**文件**: `src/tasks.rs`

```rust
// 删除以下方法:
// - search_tasks()
// - search_tasks_fts5()
// - search_tasks_like()
```

**文件**: `src/search.rs`

```rust
// 删除:
// - unified_search() 方法
// - needs_like_fallback() 相关逻辑（如果不再需要）
```

**重要**: 先确保所有调用点已更新，运行测试通过后再删除

---

## 测试计划

### 单元测试
- ✅ 分页正确性（has_more 计算）
- ✅ 默认参数行为
- ✅ 空查询处理
- ✅ FTS5 排序一致性

### 集成测试
- ✅ CLI 命令带 --offset 参数
- ✅ MCP 工具调用
- ✅ Dashboard API 调用

### 性能测试
- ✅ 1000 任务 + 1000 事件搜索 < 200ms

---

## Breaking Changes

⚠️ **API 返回类型变更**:
- 旧: `Vec<UnifiedSearchResult>`
- 新: `PaginatedSearchResults { results, total_tasks, total_events, has_more, limit, offset }`

**影响范围**:
- CLI: 输出格式变化（JSON 多了分页字段）
- MCP: 工具返回值结构变化
- Dashboard: 前端需要适配新字段

---

## 成功标准

### 任务44
✅ PaginatedSearchResults 数据结构定义完成
✅ SearchManager::search() 方法实现
✅ 支持 offset 分页
✅ 统一使用 FTS5（废弃 LIKE fallback）
✅ 统一排序逻辑（rank + optional priority）
✅ 单元测试通过

### 任务45
✅ CLI 添加 --offset 参数
✅ MCP schema 更新
✅ Dashboard API 更新
✅ 删除 search_tasks, unified_search
✅ 所有调用点编译通过
✅ 集成测试通过

---

## 回滚计划

如果遇到问题需要回滚:

1. 恢复旧方法（从 git）
2. 撤销调用点修改
3. 删除新的 PaginatedSearchResults
4. 提交回滚 commit

---

## 后续优化（Phase 3+）

- 🔮 支持高级查询语法（AND, OR, NOT）
- 🔮 搜索结果高亮优化
- 🔮 搜索历史记录
- 🔮 搜索性能监控

---

**准备完成，等待 Task #39 完成后执行！**
