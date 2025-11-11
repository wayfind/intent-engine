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

- [ ] 单元测试覆盖率 ≥ 85%
- [ ] 集成测试覆盖所有新功能
- [ ] MCP 工具测试通过
- [ ] 循环依赖检测测试

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
- 保持代码覆盖率 ≥ 85%

---

**最后更新**: 2025-11-11
**作者**: AI Agent (基于使用反馈)
**审核**: 待定
