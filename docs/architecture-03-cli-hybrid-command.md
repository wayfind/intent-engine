
### 规格 1: `ie` 命令行接口 (CLI) 混合模型

**功能名称**: Hybrid Command Model for the `ie` CLI
**状态**: **已提议 (Proposed)**
**版本**: 1.0

#### 1. 摘要 (Abstract)

本规格旨在为 `ie` 命令行工具定义一个**混合命令模型 (Hybrid Command Model)**。该模型结合了**扁平化的、以动词为中心 (Verb-Centric)** 的命令用于高频操作，以实现极致的效率和流畅性；同时保留了**层级化的、以名词为中心 (Noun-Centric)** 的完整命令结构，以保证清晰性、可发现性和脚本友好性。

#### 2. 动机 (Motivation)

用户的核心工作流是“想法 -> 执行 -> 记录”。为了让 `ie` 成为一个无缝的“外接大脑”，必须最大限度地减少从想到做到之间的认知摩擦。

*   **当前模型 (`ie task add ...`)**: 虽然清晰，但在高频使用中显得冗长，打断了思维流。用户需要先思考操作的对象（`task`, `event`），再思考具体的操作 (`add`, `list`)。
*   **期望模型 (`ie add ...`)**: 直接将用户的意图（“我要添加”）映射到命令，符合直觉，输入成本更低，极大地提升了用户体验。

同时，保留完整的层级命令（如 `ie task ...`）对于工具的长期健康至关重要，它使得帮助系统 (`--help`) 更具结构，也让编写自动化脚本时意图更明确。

#### 3. 设计提案 (Proposed Design)

##### 3.1. 核心原则

1.  **别名优先 (Alias-First for High Frequency)**: 为最常见的 8-10 个操作提供顶层别名。
2.  **结构化为基础 (Structured as Foundation)**: 所有功能必须有其对应的、完整的层级化命令。别名仅仅是完整命令的快捷方式。
3.  **一致性 (Consistency)**: 参数的顺序和命名在别名和完整命令之间应尽可能保持一致。

##### 3.2. 命令映射表

| 别名 (Verb-Centric Alias) | 完整命令 (Noun-Centric Full Command) | 参数/说明 |
| :--- | :--- | :--- |
| `ie add <name>` | `ie task add <name>` | 快速添加一个任务。 |
| `ie start <id>` | `ie task start <id>` | 开始并聚焦一个任务。 |
| `ie done` | `ie task done` | 完成当前聚焦的任务。 |
| `ie switch <id>` | `ie task switch <id>` | 切换焦点到另一个任务。 |
| `ie log <type> <data>` | `ie event add --type <type> --data <data>` | 为当前任务快速记录事件。示例: `ie log blocker "API offline"` |
| `ie next` | `ie task pick-next` | 推荐下一个任务。 |
| `ie list` 或 `ie ls` | `ie task list` | 列出任务。可以接受 `--status` 等参数。 |
| `ie context` 或 `ie ctx` | `ie task context` | 显示当前任务的上下文。 |
| `ie search <query>` | `ie search <query>` | （见规格2）执行统一搜索。此命令天然是动词，保留在顶层。 |
| `ie get <id>` | `ie task get <id>` | 获取单个任务的详细信息。 |

##### 3.3. 实现策略

推荐使用现代 CLI 框架（如 Python 的 `Click`, `Typer` 或 Go 的 `Cobra`）来实现。这些框架通常原生支持命令别名或通过简单的函数调用转发来实现。

**示例 (Python Click):**
将 `ie add` 定义为一个顶层命令，其内部实现直接调用 `task_add` 命令的处理函数。

#### 4. 用户体验 (User Experience)

**日常工作流示例:**

```bash
# 1. 捕捉一个新想法，毫不费力
$ ie add "重构认证模块以支持OAuth2"
> Task 55 added.

# 2. 查看待办列表
$ ie ls todo
> ID   Priority   Name
> 55   medium     重构认证模块以支持OAuth2

# 3. 开始工作
$ ie start 55
> Started task 55: 重构认证模块以支持OAuth2. Focus is set.

# 4. 发现一个前置任务，立即派生并切换过去
$ ie task spawn-subtask "研究OAuth2的最佳安全实践"
> Subtask 56 created and started. Focus switched to 56.

# 5. 记录一个重要的决策
$ ie log decision "决定使用Authorization Code Flow with PKCE"
> Event added to task 56.

# 6. 完成子任务
$ ie done
> Task 56 done. Focus cleared.

# 7. 决定下一步做什么
$ ie next
> Recommended next task: 55 (Parent task)
```
这个流程展示了高频别名如何让日常交互变得极其流畅和高效。

---

### 规格2: 统一搜索功能

**功能名称**: Unified Search for Tasks and Events
**状态**: **已提议 (Proposed)**
**版本**: 1.0

#### 1. 摘要 (Abstract)

本规格定义了一个**统一搜索 (`ie search`)** 功能。该功能将对一个包含**所有任务 (`tasks`) 和所有事件 (`events`)** 内容的统一索引进行全文检索。这使用户能够通过单一入口点，检索到项目历史中的任何相关信息，无论是计划中的工作，还是过去的决策、笔记或障碍，从而将 `ie` 从任务管理器转变为一个真正的个人知识库。

#### 2. 动机 (Motivation)

*   **模拟人类记忆**: 人类在回忆信息时，不会区分这个信息是“待办事项”还是“笔记”。统一搜索模仿了这种模糊检索模式，极大地降低了用户查找信息的认知负荷。
*   **提升信息价值**: 当所有记录的事件都可被即时搜索到时，记录事件的行为就从“被动的文档工作”转变为“主动的知识投资”。这会激励用户更频繁、更详细地记录，形成良性循环。
*   **赋能决策**: 在开始新任务或解决新问题时，能够快速检索到过去所有相关的决策和上下文，可以避免重复劳动和重复犯错。

#### 3. 设计提案 (Proposed Design)

##### 3.1. 后端 / 数据模型

1.  **统一索引 (Unified Index)**:
    *   建议使用数据库的 Full-Text Search 功能（如 SQLite FTS5）创建一个专门的搜索索引表 `search_index`。
    *   **索引表结构**:
        *   `source_type`: (TEXT) 来源类型，值为 'task' 或 'event'。
        *   `source_id`: (INTEGER) 来源表的主键 ID (即 `task_id` 或 `event_id`)。
        *   `parent_task_id`: (INTEGER) 所属任务的 ID。对于 `source_type` 为 'task' 的记录，此值等于 `source_id`；对于 'event'，此值为该事件所属的任务 ID。**此字段对于在结果中提供上下文至关重要**。
        *   `content`: (TEXT) 要被索引的文本内容。对于任务，是 `name` + `spec` 的组合；对于事件，是 `data`。
        *   `created_at`: (DATETIME) 来源条目的创建时间，用于排序。

2.  **索引逻辑 (Indexing Logic)**:
    *   每当创建或更新一个 `task` 或 `event` 时，必须同步更新 `search_index` 表。使用数据库触发器是实现这一点的健壮方式。

3.  **查询逻辑 (Query Logic)**:
    *   `ie search <query>` 将查询 `search_index` 表的 `content` 字段。
    *   为了提供丰富的上下文，查询结果需要 JOIN 回原始的 `tasks` 和 `events` 表来获取元数据（如任务名称、事件类型、时间戳等）。
    *   **排序/排名 (Ranking)**: 默认排序应综合考虑相关性（由 FTS 引擎提供）和时间（`created_at` DESC）。可以引入简单的权重，例如：`task.name` 的匹配 > `task.spec` 的匹配 > `event.data` 的匹配。

##### 3.2. 命令行接口 (CLI) 输出

CLI 的输出必须清晰、信息丰富且易于扫描。

**命令**: `ie search <query>`

**输出解剖 (Anatomy of a Search Result):**

```
[TYPE] Context Header
(match in <source_field> @ <timestamp>)
└── Match Snippet with **highlighting**...```

*   **`[TYPE]`**: 明确的类型标签，`[TASK]` 或 `[EVENT]`。
*   **`Context Header`**:
    *   对于 `[TASK]`，显示 `ID <id>: <Task Name>`。
    *   对于 `[EVENT]`，显示 `in Task <id>: <Task Name>`。**这至关重要**，它告诉用户这个事件发生在哪个任务的背景下。
*   **`Match Metadata`**: (可选但推荐) 提供匹配来源的元信息，如 `(match in spec)` 或 `(match in 'decision' event @ 2025-11-14 10:30)`。
*   **`Match Snippet`**: FTS 引擎生成的、带有高亮关键词的上下文片段。

**示例输出:**

```bash
$ ie search "security"

Searching for "security"... Found 4 results.

[TASK] ID 56: 研究OAuth2的最佳安全实践
(match in name)
└── Spec: 深入研究OAuth2协议的各种流程，特别是与**security**相关的方面，例如...

[EVENT] in Task 55: 重构认证模块以支持OAuth2
(match in 'decision' event @ 2025-11-15 11:00)
└── ...经过讨论，为了提升**security**，我们决定必须实施PKCE扩展...

[TASK] ID 23: 升级服务器依赖
(match in spec @ 2025-10-20 09:15)
└── ...需要将log4j库升级到最新版本以修复已知的**security**漏洞...

[EVENT] in Task 56: 研究OAuth2的最佳安全实践
(match in 'note' event @ 2025-11-15 14:30)
└── ...查阅了IETF的**security**最佳实践文档，链接：https://...
```

#### 4. 未来考虑 (Future Considerations)

*   **高级过滤**: 增加参数以缩小搜索范围，如 `ie search "jwt" --type=event --since=30d`。
*   **交互式预览**: 集成 `fzf` 等工具，在搜索结果上提供交互式的预览和选择。
