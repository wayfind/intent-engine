# 需求规格说明书 (Requirement Speckit): Intent-Engine

**版本**: 0.2

## 1. 核心哲学：意图的引擎

### 1.1 核心理念
`Intent-Engine` 是一个专为人机协作打造的**命令行意图引擎**。它将您和 AI 伙伴短暂、易失的协作瞬间，沉淀为项目可追溯、可恢复的永恒智慧。

### 1.2 “工具箱”协作模型
`Intent-Engine` 是 AI 协作者（如 `cc`）工具箱中一个**高层、专用**的工具，与 `git`, `ls` 等底层工具协同工作。它的职责，就是把“意图管理”这一件事做到极致。

## 2. CLI 接口总则

### 2.1 输出模式 (Output Modes)

`Intent-Engine` 支持两种输出模式，以同时满足人类和机器的需求。

#### **默认模式: 人类可读 (Human-Readable)**
*   **触发**: 默认行为，不需任何标志。
*   **特点**: 输出是经过**格式化、着色**的文本，旨在提供最佳的可读性和上下文引导。**此格式不稳定，不应用于脚本解析**。

#### **机器模式: JSON**
*   **触发**: 在任何命令后附加全局标志 `--json`。
*   **特点**: 输出是**严格的、结构化的 JSON**，保证向后兼容性（遵循 SemVer）。所有 AI Agent 和自动化脚本**必须**使用此模式。

### 2.2 错误处理
*   **人类可读模式**: 错误信息将以清晰、着色的文本形式输出到 `stderr`。
*   **JSON 模式**: 错误将以结构化的 JSON 对象形式输出到 `stdout`。
*   所有失败的命令都将以**非零退出码**退出。

## 3. 智能惰性初始化

`Intent-Engine` **没有** `init` 命令。初始化是在首次写入操作时，由系统**自动、智能**完成的后台过程。

### 3.1 触发条件
1.  一个**写入型**命令被执行（如 `task add`）。
2.  在当前目录及其所有上级目录中，都**未找到** `.intent-engine` 文件夹。

### 3.2 根目录推断算法
1.  从当前工作目录 (CWD) 开始，向上递归查找。
2.  在每一级目录，按顺序查找是否存在**“项目根标记”**（`.git`, `package.json`, `Cargo.toml` 等）。
3.  **首次匹配即确定根目录**，并在此创建 `.intent-engine` 文件夹。
4.  若查找到系统根目录仍未找到标记，则**回退**到在 CWD 创建，并向 `stderr` 打印一条警告。

## 4. 数据库 Schema

数据库文件位于**`<项目根目录>/.intent-engine/project.db`**。

### 4.1 `tasks` 表
*   `id` (INTEGER, PK), `parent_id` (INTEGER, FK), `name` (TEXT), `spec` (TEXT), `status` (TEXT), `priority` (INTEGER, 1=highest, nullable), `first_todo_at`, `first_doing_at`, `first_done_at` (DATETIME)
*   **全文搜索**: `name` 和 `spec` 字段**必须**通过 **SQLite FTS5** 进行索引。

### 4.2 `events` 表
*   `id` (INTEGER, PK), `task_id` (INTEGER, FK, Indexed), `timestamp` (DATETIME), `log_type` (TEXT), `discussion_data` (TEXT)

### 4.3 `workspace_state` 表
*   `key` (TEXT, PK), `value` (TEXT) - 主要用于存储 `current_task_id`。

## 5. 命令行 API 详解

### 5.1 `task` - 任务管理

#### `task add`
*   **用途**: 捕获一个新的战略意图。
*   **签名**: `ie task add --name <NAME> [--parent <ID>] [--priority <1-5>] [--spec-stdin]`

#### `task start`
*   **用途**: 激活一个意图并设置为当前焦点。
*   **签名**: `ie task start <TASK_ID>`
*   **原子行为**: 1. 任务状态 -> `doing`；2. 设置为 `current_task_id`。

#### `task done`
*   **用途**: 完成当前焦点任务。
*   **签名**: `ie task done`  **(无参数)**
*   **原子行为**: 1. 校验子任务；2. 任务状态 -> `done`；3. 清空 `current_task_id`。
*   **JSON 输出**: 包含 `completed_task`, `workspace_status`, `next_step_suggestion`。

#### `task find`
*   **用途**: 根据结构化元数据**过滤**任务。
*   **签名**: `ie task find [--status <STATUS>] [--parent <ID|"null">]`
*   **输出**: 任务摘要对象数组（不含 `spec`）。

#### `task search`
*   **用途**: 在 `name` 和 `spec` 中进行全文**搜索**。
*   **签名**: `ie task search <QUERY> [--limit <N>]`
*   **实现**: **必须**使用 FTS5 索引。
*   **JSON 输出**: 任务摘要对象数组，并额外包含 `spec_snippet` (高亮匹配) 和 `rank`。

#### `task context`
*   **用途**: 获取一个任务完整的“家族树”（祖先、兄弟、子任务）。
*   **签名**: `ie task context [<TASK_ID>]` (ID 可选，默认使用当前焦点)
*   **输出**: 包含 `focus_task`, `ancestors`, `siblings`, `children` 的结构化 JSON 对象。

#### `task pick-next`
*   **用途**: 智能推荐下一个最应该处理的任务。
*   **签名**: `ie task pick-next`
*   **逻辑**: 1. 优先推荐当前焦点任务的 `todo` 子任务；2. 其次推荐顶级的 `todo` 任务。均按 `priority` 排序。
*   **JSON 输出**: 包含 `recommended_task` 和 `reason` 的对象。空状态下 `recommended_task` 为 `null`。

#### `task spawn-subtask`
*   **用途**: 在当前焦点下创建子任务，并立即切换到该子任务。
*   **签名**: `ie task spawn-subtask --name <NAME> [...]`
*   **原子行为**: 1. `task add`；2. `task start`。

#### `task switch`
*   **用途**: 暂停当前任务，并切换到新任务。
*   **签名**: `ie task switch <TASK_ID>`
*   **原子行为**: 1. 原 `doing` 任务 -> `todo`；2. `task start` 新任务。

#### `task get`, `task update`, `task del`
*   提供标准的获取、更新、删除单个任务的功能。

### 5.2 `event` - 记忆与历史

#### `event add`
*   **用途**: 为任务记录一个关键事件（决策、障碍、里程碑等）。
*   **签名**: `ie event add --type <TYPE> [--task-id <ID>] [--data-stdin]`
*   **逻辑**: `--task-id` 可选。若省略，则自动作用于当前焦点任务。

#### `event list`
*   **用途**: 查看一个任务的完整历史事件流。
*   **签名**: `ie event list <TASK_ID>`

### 5.3 `current` - 工作区焦点

#### `current`
*   **用途**: 查看当前焦点任务。
*   **签名**: `ie current`

#### `current --set`
*   **用途**: 设置当前焦点任务。
*   **签名**: `ie current --set <TASK_ID>`

### 5.4 `report` - 分析与洞察

#### `report`
*   **用途**: 生成项目活动报告。
*   **签名**: `ie report [--since <DURATION>] [--summary-only]`
*   **核心功能**: `--summary-only` 标志将在服务端完成聚合计算，极大节省 Token。

## 6. MCP 接口与 Rust API

*   **MCP 接口**: `tools` 列表应与 CLI 命令一一对应，并通过上一轮我们优化的“战术菜谱”式 `description` 和 `usage_examples` 来指导 AI。
*   **Rust API**: 提供与 CLI 功能对等的、符合 Rust 编程习惯的模块化接口。

## 7. 并发与数据完整性

*   数据库连接**必须**使用 `WAL` (Write-Ahead Logging) 模式。
*   数据库连接**必须**设置一个合理的 `busy_timeout` (例如 5000 毫秒)。
*   所有多步写入操作（`start`, `done`, `spawn-subtask`, `switch`）**必须**在单个数据库事务中执行。