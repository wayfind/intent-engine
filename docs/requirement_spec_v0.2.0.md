# Intent-Engine v0.2.0 Requirement Specification

**版本**: `0.2.0`
**主题**: "智慧与互联" (Intelligence & Interconnection)
**核心目标**: 基于首个 AI Agent 的深度使用反馈，为 `Intent-Engine` 增加核心的智能查询与任务互联能力，将 AI 的协作效率提升一个数量级。

---

## 1. 核心功能增强 (P0 - Top Priority)

### 1.1 [Feature] 任务依赖管理 (Task Dependency System)

**用户故事**:
- **As an AI Agent**, I want to explicitly define that Task A must be completed before Task B can start, so that I can generate a valid execution plan and avoid working on blocked tasks.
- **As a Human Developer**, I want to see the dependency graph of tasks, so that I can understand the true workflow of a project.

**需求规格**:

1. **数据库变更**:
   - 新增一个 `dependencies` 表。
   - Schema: `id (PK)`, `blocking_task_id (INTEGER, FK to tasks.id)`, `blocked_task_id (INTEGER, FK to tasks.id)`。
   - 必须在 `blocking_task_id` 和 `blocked_task_id` 上建立索引。

2. **新增 CLI 命令**:
   - `intent-engine task depends-on <BLOCKED_ID> <BLOCKING_ID>`: 创建一条依赖关系，意为"任务 BLOCKED_ID 依赖于 任务 BLOCKING_ID"。
   - 命令必须进行**循环依赖检测**。如果添加此依赖会造成循环（例如 A->B->C->A），命令必须失败并返回错误。

3. **核心逻辑变更**:
   - **`task start`**: 在执行前，必须检查该任务是否被任何**未完成 (`todo` or `doing`)** 的任务所阻塞。如果是，`start` 命令必须失败，并返回一个清晰的 `TASK_BLOCKED` 错误，其中包含阻塞它的任务 ID。
   - **`pick-next`**: 推荐算法**必须过滤掉**所有当前被阻塞的任务。

4. **`task context` 增强**: `context` 命令的输出中，应增加 `dependencies` 字段，包含 `blocking_tasks` 和 `blocked_by_tasks` 两个列表。

5. **MCP 接口**: 新增 `task_add_dependency` 工具。

---

### 1.2 [Feature] 智能事件查询 (Smart Event Querying)

**用户故事**:
- **As an AI Agent**, when a task has hundreds of events, I want to filter events by their type (e.g., 'decision') or recency, so that I can quickly find the specific information I need without wasting tokens and processing power.

**需求规格**:

1. **CLI 命令增强**: `event list` 命令的签名变更为：
   ```bash
   intent-engine event list <TASK_ID> [--type <TYPE>] [--since <DURATION>]
   ```

2. **参数行为**:
   - `--type <TYPE>`: 只返回指定 `log_type` 的事件。
   - `--since <DURATION>`: 只返回在指定时间段内创建的事件。

3. **MCP 接口**: 更新 `event_list` 工具的 `inputSchema`，加入 `type` 和 `since` 两个可选参数。并在 `description` 中明确指导 AI 使用这些过滤器来提高效率。

---

## 2. 重要改进 (P1 - High Priority)

### 2.1 [Enhancement] 优先级枚举化 (Priority Enum Mapping)

**用户故事**:
- **As an AI Agent**, I want to understand the semantic weight of a priority (e.g., 'critical' vs 'low'), not just an abstract number, so that I can make more intelligent decisions in `pick-next` or when communicating with humans.

**需求规格**:

1. **API 层面变更**:
   - CLI 和 MCP 接口层面，`priority` 参数将接受**字符串**输入：`critical`, `high`, `medium`, `low`。
   - `Intent-Engine` 内部将这些字符串映射为**整数**进行存储和排序（`critical: 1`, `high: 2`, `medium: 3`, `low: 4`）。
   - 所有 JSON 和文本输出中，`priority` 字段应同时显示字符串和数字，或只显示字符串。

2. **`task add` & `task update`**: `inputSchema` 中的 `priority` 字段类型从 `integer` 变更为 `string`，并提供 `enum` 列表。

3. **数据库**: `priority` 字段类型保持 `INTEGER` 不变。

---

### 2.2 [Enhancement] 命令更名：`find` -> `list`

**用户故事**:
- **As a Developer/AI**, I want the command names to be unambiguous, so that I can intuitively know `list` is for browsing structured metadata and `search` is for discovering unstructured content.

**需求规格**:

1. **全局重命名**:
   - CLI 命令 `intent-engine task find` 更名为 `intent-engine task list`。
   - MCP 工具 `task_find` 更名为 `task_list`。

2. **文档更新**: 所有文档、示例和 `README.md` 中的 `find` 都必须同步更新为 `list`。

3. **别名 (可选，推荐)**: 可以在一段时间内，将 `find` 作为 `list` 的一个隐藏别名，并打印一条"`find` is deprecated, please use `list`"的警告，以实现平滑过渡。

---

## 3. 待办事项与未来规划 (Backlog & Future Scope)

以下功能已根据反馈确认其价值，但将**不包含**在 v0.2.0 的范围中。它们将作为后续版本 (v0.3.0+) 的核心候选功能。

- **`P1-Backlog` 批量操作**: 增强 `update` 等命令以支持多 ID 操作。
- **`P2-Backlog` 时间追踪**: 增强 `report` 命令，利用已有的时间戳数据，计算任务的周期时间 (Cycle Time) 和前置时间 (Lead Time)。
- **`P2-Backlog` 任务归档**: 引入 `is_archived` 状态，用于在 `list` 和 `report` 中隐藏已完成且不相关的旧任务。
- **`P2-Backlog` 可视化导出**: 新增 `intent-engine export --format dot` 命令，将任务树和依赖关系导出为 Graphviz 的 DOT 文件格式，以便生成可视化图形。

---

## 4. 版本策略

### 4.1 语义化版本控制

根据 INTERFACE_SPEC.md 的版本策略：
- **v0.2.0** 属于 minor 版本升级
- 新增功能（任务依赖、事件查询）
- 向后兼容的 API 增强（优先级枚举）
- 命令重命名通过别名保持兼容性

### 4.2 发布计划

1. **Alpha 阶段** (v0.2.0-alpha.1):
   - 实现核心 P0 功能
   - 内部测试和迭代

2. **Beta 阶段** (v0.2.0-beta.1):
   - 完成 P1 功能
   - 文档更新
   - 集成测试

3. **正式发布** (v0.2.0):
   - 所有测试通过
   - 文档完善
   - 发布到 crates.io

---

## 5. 验收标准

### 5.1 功能验收

- [ ] 任务依赖系统完整实现
- [ ] 循环依赖检测有效
- [ ] `task start` 正确阻止依赖未满足的任务
- [ ] `pick-next` 过滤被阻塞的任务
- [ ] 事件查询支持类型和时间过滤
- [ ] 优先级枚举化实现
- [ ] `find` -> `list` 重命名完成

### 5.2 测试覆盖

- [ ] 单元测试覆盖率 ≥ 95%
- [ ] 集成测试覆盖所有新功能（预估 60-70 个新测试）
- [ ] MCP 工具测试通过
- [ ] 循环依赖检测测试
- [ ] 性能基准测试（循环检测 <10ms，大数据集 10,000+ 任务）

### 5.3 文档更新

- [ ] INTERFACE_SPEC.md 更新到 v0.2
- [ ] CLAUDE.md 反映新 MCP 工具
- [ ] README.md 更新特性列表
- [ ] 迁移指南（如有 breaking changes）

---

## 6. 实施优先级建议

### Phase 1: 核心依赖系统
1. 数据库 schema 设计与迁移
2. 循环依赖检测算法
3. CLI `depends-on` 命令
4. `task start` 阻塞检查
5. `pick-next` 过滤逻辑

### Phase 2: 事件查询增强
1. CLI `event list` 参数扩展
2. 过滤逻辑实现
3. MCP 接口更新

### Phase 3: 优先级与命令重命名
1. 优先级枚举映射
2. `find` -> `list` 重命名
3. 别名兼容性

### Phase 4: 测试与文档
1. 单元测试补全
2. 集成测试编写
3. 文档全面更新
4. 性能测试

---

## 7. 非功能需求

### 7.1 性能

- 依赖检查不应显著影响 `task start` 性能（目标 < 10ms 额外开销）
- 事件过滤应使用数据库索引优化
- 循环依赖检测应使用高效算法（DFS/拓扑排序）

### 7.2 兼容性

- 数据库迁移必须无损且可回滚
- CLI 别名保证旧脚本兼容性
- MCP 工具保持向后兼容（添加可选参数）

### 7.3 可维护性

- 依赖图逻辑应独立模块化
- 添加充分的代码注释
- 保持代码覆盖率 ≥ 95%

---

## 8. 详细技术规格

### 8.1 任务依赖系统详细设计

#### 8.1.1 完整数据库 Schema

```sql
-- Dependencies 表（完整版）
CREATE TABLE IF NOT EXISTS dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    blocking_task_id INTEGER NOT NULL,
    blocked_task_id INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    -- 外键约束
    FOREIGN KEY (blocking_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (blocked_task_id) REFERENCES tasks(id) ON DELETE CASCADE,

    -- 唯一约束：防止重复的依赖关系
    UNIQUE(blocking_task_id, blocked_task_id),

    -- 检查约束：防止自依赖
    CHECK(blocking_task_id != blocked_task_id)
);

-- 必需索引（性能优化）
CREATE INDEX idx_dependencies_blocking ON dependencies(blocking_task_id);
CREATE INDEX idx_dependencies_blocked ON dependencies(blocked_task_id);

-- 事件表索引增强（支持事件过滤）
CREATE INDEX IF NOT EXISTS idx_events_task_type_time
ON events(task_id, log_type, timestamp);
```

**Schema 版本跟踪**:
```sql
-- 在 workspace_state 表中记录 schema 版本
INSERT INTO workspace_state (key, value)
VALUES ('schema_version', '0.2.0')
ON CONFLICT(key) DO UPDATE SET value = '0.2.0';
```

**删除行为说明**:
- 当 `blocking_task_id` 或 `blocked_task_id` 指向的任务被删除时，相关依赖关系通过 `ON DELETE CASCADE` 自动清理
- 不会留下孤儿依赖记录
- 删除操作在事务中执行，保证原子性

#### 8.1.2 循环依赖检测算法

**算法选择**: SQLite 递归 CTE (Common Table Expression) + 深度优先遍历

**核心 SQL 实现**:
```sql
-- 检测从 new_blocked 到 new_blocking 是否存在路径
-- 如果存在，则添加 new_blocking -> new_blocked 会形成循环
WITH RECURSIVE dep_chain(task_id, depth) AS (
    -- 初始节点：从 blocked 任务开始
    SELECT ? as task_id, 0 as depth

    UNION ALL

    -- 递归步骤：沿着阻塞链向上遍历
    SELECT d.blocking_task_id, dc.depth + 1
    FROM dependencies d
    JOIN dep_chain dc ON d.blocked_task_id = dc.task_id
    WHERE dc.depth < 100  -- 深度限制，防止无限循环
)
SELECT COUNT(*) > 0 as has_cycle
FROM dep_chain
WHERE task_id = ?;  -- 检查是否回到 blocking 任务
```

**绑定参数**:
- 第一个 `?`: `new_blocked` (被依赖的任务 ID)
- 第二个 `?`: `new_blocking` (依赖于的任务 ID)

**算法复杂度**:
- **时间**: O(V + E)，其中 V 是任务数，E 是依赖边数
- **空间**: O(D)，其中 D 是最大深度（限制为 100）
- **预期性能**: 对于 <10,000 任务的图，<10ms

**传递闭包检查**:
- 算法自动检查传递依赖（A→B→C 时不能添加 C→A）
- 深度限制防止病态图（如 A→B→C→...→Z→A）

**边界情况处理**:
1. **自依赖**: 通过 `CHECK(blocking_task_id != blocked_task_id)` 在数据库层阻止
2. **直接循环**: A→B→A
3. **间接循环**: A→B→C→A
4. **长链循环**: 超过 100 层深度时算法终止并返回"可能存在循环"

#### 8.1.3 JSON 输出格式定义

**`task context` 输出格式**:
```json
{
  "task": {
    "id": 42,
    "name": "Implement authentication",
    "status": "doing"
  },
  "ancestors": [...],
  "siblings": [...],
  "children": [...],
  "dependencies": {
    "blocking_tasks": [
      {
        "id": 45,
        "name": "Configure JWT secret",
        "status": "doing"
      },
      {
        "id": 46,
        "name": "Setup database schema",
        "status": "todo"
      }
    ],
    "blocked_by_tasks": [
      {
        "id": 41,
        "name": "Design authentication architecture",
        "status": "done"
      }
    ]
  }
}
```

**MCP `task_add_dependency` 工具返回格式**:
```json
{
  "dependency_id": 15,
  "blocking_task_id": 41,
  "blocked_task_id": 42,
  "created_at": "2025-11-11T10:30:00Z",
  "message": "Dependency added successfully"
}
```

**错误返回格式（循环检测失败）**:
```json
{
  "error": "CIRCULAR_DEPENDENCY",
  "message": "Adding this dependency would create a circular dependency",
  "details": {
    "attempted": {
      "blocking_task_id": 42,
      "blocked_task_id": 41
    },
    "cycle_detected": "Task 42 is already transitively blocked by Task 41"
  }
}
```

### 8.2 优先级枚举化详细设计

#### 8.2.1 优先级映射表

**字符串 → 整数映射** (新任务):
```
"critical" → 1
"high"     → 2
"medium"   → 3
"low"      → 4
```

**整数 → 字符串映射** (现有任务迁移):
```
0-2   → "critical" (1)
3-5   → "high"     (2)
6-8   → "medium"   (3)
9-999 → "low"      (4)
```

**数据库存储**:
- `priority` 字段类型保持 `INTEGER NOT NULL DEFAULT 3`
- 不需要数据库迁移（向后兼容）

#### 8.2.2 向后兼容策略

**输入兼容性** (v0.2.x 系列支持双格式):
```rust
// 伪代码：输入解析
enum PriorityInput {
    String(String),  // "critical", "high", "medium", "low"
    Integer(i32),    // 1, 2, 3, 4, 或遗留值 5, 8, 10 等
}

fn parse_priority(input: PriorityInput) -> Result<i32> {
    match input {
        PriorityInput::String(s) => match s.as_str() {
            "critical" => Ok(1),
            "high"     => Ok(2),
            "medium"   => Ok(3),
            "low"      => Ok(4),
            _          => Err("Invalid priority. Use: critical, high, medium, low")
        },
        PriorityInput::Integer(i) => {
            eprintln!("⚠️  Warning: Integer priority is deprecated. Use: critical, high, medium, low");
            // 遗留值映射
            if i <= 2      { Ok(1) }  // critical
            else if i <= 5 { Ok(2) }  // high
            else if i <= 8 { Ok(3) }  // medium
            else           { Ok(4) }  // low
        }
    }
}
```

**弃用警告格式**:
```
⚠️  Warning: Integer priority is deprecated since v0.2.0
    Please use string values: critical, high, medium, low
    Integer support will be removed in v0.3.0
```

**MCP 客户端兼容性**:
- MCP `inputSchema` 中 `priority` 字段类型改为 `string`
- 添加 `enum: ["critical", "high", "medium", "low"]` 约束
- 旧版 MCP 客户端传入整数时，服务端自动转换并警告

#### 8.2.3 输出格式规范

**JSON 输出** (同时显示字符串和数字):
```json
{
  "id": 42,
  "name": "Implement authentication",
  "priority": "high",
  "priority_value": 2
}
```

**CLI 文本输出** (仅显示字符串):
```
Task #42: Implement authentication
  Priority: high
  Status: doing
```

**排序行为**:
- 使用 `priority_value` (整数) 进行排序
- ASC 排序：critical (1) < high (2) < medium (3) < low (4)

### 8.3 数据库迁移规格

#### 8.3.1 完整迁移脚本

```sql
-- migration_v0.2.0.sql
-- 幂等性保证：可以重复执行

BEGIN TRANSACTION;

-- Step 1: 检查是否已迁移
CREATE TABLE IF NOT EXISTS workspace_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Step 2: 创建 dependencies 表
CREATE TABLE IF NOT EXISTS dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    blocking_task_id INTEGER NOT NULL,
    blocked_task_id INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (blocking_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (blocked_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    UNIQUE(blocking_task_id, blocked_task_id),
    CHECK(blocking_task_id != blocked_task_id)
);

-- Step 3: 创建 dependencies 表索引
CREATE INDEX IF NOT EXISTS idx_dependencies_blocking
ON dependencies(blocking_task_id);

CREATE INDEX IF NOT EXISTS idx_dependencies_blocked
ON dependencies(blocked_task_id);

-- Step 4: 创建事件过滤索引
CREATE INDEX IF NOT EXISTS idx_events_task_type_time
ON events(task_id, log_type, timestamp);

-- Step 5: 更新 schema 版本
INSERT INTO workspace_state (key, value)
VALUES ('schema_version', '0.2.0')
ON CONFLICT(key) DO UPDATE SET value = '0.2.0';

COMMIT;
```

#### 8.3.2 回滚策略

```sql
-- rollback_v0.2.0.sql
-- 回滚到 v0.1.x

BEGIN TRANSACTION;

-- Step 1: 删除 dependencies 表（会级联删除索引）
DROP TABLE IF EXISTS dependencies;

-- Step 2: 删除事件过滤索引
DROP INDEX IF EXISTS idx_events_task_type_time;

-- Step 3: 恢复 schema 版本
UPDATE workspace_state
SET value = '0.1.17'
WHERE key = 'schema_version';

COMMIT;
```

**回滚影响评估**:
- ✅ **安全**: 不影响 `tasks` 和 `events` 表数据
- ⚠️ **数据丢失**: 所有 `dependencies` 表中的依赖关系将丢失
- ✅ **可恢复**: 依赖关系可以通过 MCP 工具重新创建

#### 8.3.3 迁移验证

**迁移后验证 SQL**:
```sql
-- 验证 dependencies 表存在
SELECT name FROM sqlite_master
WHERE type='table' AND name='dependencies';

-- 验证索引存在
SELECT name FROM sqlite_master
WHERE type='index' AND name IN (
    'idx_dependencies_blocking',
    'idx_dependencies_blocked',
    'idx_events_task_type_time'
);

-- 验证约束生效
INSERT INTO dependencies (blocking_task_id, blocked_task_id)
VALUES (1, 1);  -- 应该失败：CHECK 约束

-- 验证 schema 版本
SELECT value FROM workspace_state WHERE key = 'schema_version';
-- 期望: '0.2.0'
```

### 8.4 性能基准测试规格

#### 8.4.1 循环检测性能测试

**测试用例 1: 小图（10 任务，20 依赖）**
- 目标: <5ms
- 操作: 添加依赖并检测循环

**测试用例 2: 中图（100 任务，300 依赖）**
- 目标: <10ms
- 操作: 添加依赖并检测循环

**测试用例 3: 大图（1,000 任务，5,000 依赖）**
- 目标: <50ms
- 操作: 添加依赖并检测循环

**测试用例 4: 超大图（10,000 任务，50,000 依赖）**
- 目标: <200ms
- 操作: 添加依赖并检测循环

**基准测试代码框架**:
```rust
#[bench]
fn bench_circular_detection_medium_graph(b: &mut Bencher) {
    // Setup: 创建 100 个任务，300 个依赖
    let pool = setup_test_db();
    create_tasks(&pool, 100).await;
    create_random_dependencies(&pool, 300).await;

    // Benchmark
    b.iter(|| {
        // 尝试添加可能形成循环的依赖
        check_circular_dependency(&pool, 50, 25).await
    });

    // Assert: 确保性能 <10ms
}
```

#### 8.4.2 `pick-next` 过滤性能测试

**测试场景**:
- 1,000 个 todo 任务
- 500 个依赖关系
- 当前无聚焦任务

**性能目标**: <20ms（相比 v0.1.x 的 <10ms，允许额外 10ms 开销）

**SQL 查询性能分析**:
```sql
EXPLAIN QUERY PLAN
SELECT t.id, t.name, t.priority
FROM tasks t
WHERE t.status = 'todo'
  AND t.parent_id IS NULL
  AND t.id NOT IN (
      SELECT blocked_task_id
      FROM dependencies
      WHERE blocking_task_id IN (
          SELECT id FROM tasks WHERE status IN ('todo', 'doing')
      )
  )
ORDER BY COALESCE(t.priority, 999999) ASC, t.id ASC
LIMIT 1;
```

#### 8.4.3 事件过滤性能测试

**测试场景**:
- 单个任务有 1,000 个事件
- 按类型过滤（返回 200 个 decision 事件）
- 按时间过滤（返回最近 7 天的 100 个事件）

**性能目标**: <5ms

**验证索引效果**:
```sql
-- 查询应该使用 idx_events_task_type_time 索引
EXPLAIN QUERY PLAN
SELECT id, log_type, discussion_data, timestamp
FROM events
WHERE task_id = ?
  AND log_type = 'decision'
  AND timestamp > datetime('now', '-7 days')
ORDER BY timestamp DESC;
```

---

**最后更新**: 2025-11-11
**作者**: AI Agent (基于使用反馈)
**审核**: 待定
**规格版本**: 1.1 (详细技术规格已补充)
