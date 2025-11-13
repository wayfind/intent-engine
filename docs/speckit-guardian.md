# Speckit: Intent-Engine "Guardian" Integration Protocol

**版本**: 1.0
**目标**: 解决 AI Agent 在长期、跨会话任务中对 Intent-Engine 的"意图遗忘"和"状态分裂"问题。
**核心哲学**: 从一个被动的外部工具，进化为一个主动的、嵌入式的协作伙伴，通过在关键节点注入上下文,确保 Intent-Engine 始终是人机协作的"唯一事实来源"。

---

## 1. 总体方案描述 (Why, How, What)

### 1.1 Why (为什么需要守护方案)

Intent-Engine 存储了项目的战略意图,但 AI Agent 的工作记忆是会话级的、易失的。这种"记忆断层"导致了两个核心问题:

- **意图遗忘**: 跨会话后,AI Agent 会忘记 Intent-Engine 中的长期目标,倾向于使用更即时、但无记忆的工具(如 TodoWrite)。
- **状态分裂**: AI 在原生工具中取得的战术进展,无法自动同步回 Intent-Engine 的战略历史中,导致两个系统的状态不一致。

"守护"方案旨在通过主动的、上下文感知的提醒与引导,将 AI 的注意力持续地锚定在 Intent-Engine 的战略轨道上。

### 1.2 How (如何实现守护)

本方案通过在 AI Agent 的工作流中,利用宿主环境 (Host Environment, e.g., Claude Code) 可能提供的 Hook/Plugin 机制,在三个关键时机注入系统提示 (System Prompt Injection)。我们不直接控制 AI,而是通过塑造其接收到的信息环境来引导其行为。

这套方案是**非侵入式的**,它不改变 Intent-Engine 的核心命令,而是构建了一个外部的、智能的"引导层"。

### 1.3 What (守护方案的构成)

"守护"方案由三个独立的、协同工作的组件构成,我们称之为"三位一体"守护体系:

1. **"焦点恢复" (Focus Restoration)**: 在会话开始时,重建战略上下文。
2. **"智能审计" (Smart Audit)**: 在 AI 的每个战术动作后,鼓励其记录进展。
3. **"特洛伊木马" (Trojan Horse Output)**: 将核心命令的输出,转化为下一步行动的指令。

---

## 2. 守护组件 #1: "焦点恢复" (SessionStart Hook)

### 2.1 Spec

- **触发时机**: 在宿主环境的一个新会话 (Session) 正式开始之前。
- **前置条件**: 需要宿主环境提供一个 `OnSessionStart` 或类似的 Hook 机制。

**核心逻辑**:

1. Hook 被触发后,在其执行环境中调用 `ie current --json`。
2. 如果 `current_task_id` 不为 `null`,则继续调用 `ie task context --json`。
3. 根据上述命令的 JSON 输出,动态构建一份 Markdown 格式的"焦点恢复提示"。
4. 将这份提示,作为高优先级的系统提示,注入到该会话的初始上下文中。
5. 如果 `current_task_id` 为 `null`,则注入一个更通用的"欢迎与引导"提示。

### 2.2 Prompt 注入内容模板

**场景 A: 存在当前焦点**

```markdown
<system-reminder priority="high">
## Intent-Engine: Focus Restoration ##

Welcome back. Your strategic focus has been restored.

**CURRENT FOCUS**: #{focus_id} '{focus_name}'

**IMMEDIATE CONTEXT**:
- **Mission**: #{parent_name}
- **Siblings**: {todo_siblings_count} other tasks remain in this mission.
- **Sub-tasks**: {todo_children_count} sub-tasks need to be completed for this focus.

Your primary objective is to continue making progress on the **CURRENT FOCUS**.
</system-reminder>
```

**场景 B: 无当前焦点**

```markdown
<system-reminder>
## Intent-Engine: Welcome ##

Welcome. There is no active focus.

To see what needs to be done, you can:
- Use `ie pick-next` to get the highest-priority available task.
- Use `ie task list --status todo` to review all pending intents.
</system-reminder>
```

---

## 3. 守护组件 #2: "智能审计" (PostToolUse Hook)

### 3.1 Spec

- **触发时机**: 在宿主环境中,每一次工具调用 (Tool Use) 完成之后,但在将结果返回给 AI 之前。
- **前置条件**: 需要宿主环境提供一个 `PostToolUse` 或类似的 Hook,该 Hook 必须能访问到上一次调用的工具名称和参数 (`last_tool_name`, `last_tool_args`)。

**核心逻辑**:

1. Hook 被触发后,分析 `last_tool_name` 和 `last_tool_args`。
2. 根据预设规则进行判断(见 3.2),确定是否需要以及需要注入何种提示。
3. 如果需要注入,则构建相应的提示信息,并将其附加到工具调用的结果中,或作为下一次 Prompt 的一部分。

### 3.2 审计规则与提示模板

**规则 1: 检测到"战术进展"**

- **条件**: `last_tool_name` 是 `TodoWrite`,并且其参数显示有任务状态变为 `completed`。
- **注入内容**:

```markdown
<system-reminder>
[Intent-Engine Auditor]: Progress detected in `TodoWrite`. This is a valuable milestone.

Consider persisting this progress to our long-term memory:
  `ie event add --type milestone --data "Completed tactical steps: ..."`
</system-reminder>
```

**规则 2: 检测到"代码变更"**

- **条件**: `last_tool_name` 是 `writeFile`, `patchFile` 或类似的直接文件修改工具。
- **注入内容**:

```markdown
<system-reminder>
[Intent-Engine Auditor]: Code modification detected.

If this change represents a key decision or solves a sub-problem, record it:
  `ie event add --type decision --data "Refactored function X because..."`
</system-reminder>
```

**规则 3: 检测到"持续调研"**

- **条件**: 连续 3 次以上的工具调用都是只读类型(如 `readFile`, `ls`, `grep`)。(需要 Hook 具有状态记忆能力)。
- **注入内容**:

```markdown
<system-reminder>
[Intent-Engine Auditor]: It seems you are in an investigation phase.

Don't let your findings be forgotten. Use a note to capture your thoughts:
  `ie event add --type note --data "Initial findings on API structure: ..."`
</system-reminder>
```

**默认**: 如果不符合任何特定规则,则不注入任何内容,保持安静。

---

## 4. 守护组件 #3: "特洛伊木马"输出 (Trojan Horse Output)

### 4.1 Spec

- **触发时机**: 由 Intent-Engine 的核心命令(如 `start`, `done`, `switch`, `spawn`)在生成其 JSON 输出时触发。
- **前置条件**:
  - Intent-Engine 的内部实现需要进行修改。
  - 需要宿主环境提供一个极轻量级的插件或中间件,用于在"响应后处理"阶段解析这个特殊输出。

**核心逻辑**:

1. Intent-Engine 的核心命令,在完成其主要功能后,除了生成标准的 JSON 数据外,还会根据当前操作和状态,额外在 JSON 的顶层,生成一个名为 `__system_prompt_injection` 的特殊 key。
2. 这个 key 包含 `priority` 和 `content` 两个字段。
3. 宿主环境的插件,在接收到 Intent-Engine 的 JSON 输出后,会检查是否存在 `__system_prompt_injection`。
4. 如果存在,插件会提取 `content`,并根据 `priority`,将其强制注入到下一次 AI 思考的系统提示中。然后,从最终返回给 AI 的 JSON 中移除 `__system_prompt_injection` 键,以保持输出干净。

### 4.2 JSON 输出结构示例

`ie task done` 的 JSON 输出 (内部):

```json
{
  "completed_task": { "id": 43, "status": "done" },
  "workspace_status": { "current_task_id": null },
  "next_step_suggestion": {
    "type": "PARENT_IS_READY",
    "parent_task_id": 42
  },
  "__system_prompt_injection": {
    "priority": "HIGH",
    "content": "## Intent-Engine Status Update ##\n\nTask #43 is COMPLETE. Your focus is now CLEAR.\n\nRECOMMENDATION: Parent task #42 is ready. Use `ie task start 42` to begin, or `ie pick-next` to find a new task."
  }
}
```

---

## 5. 技术可行性验证

本方案的成败,**完全取决于宿主环境是否提供上述的 Hook 或插件注入能力**。

**首要行动**: 在投入开发资源之前,必须对目标宿主环境(Claude Code)的扩展性进行技术验证,确认是否存在至少一种可行的 Prompt 注入机制。如果存在,本方案可行;如果不存在,必须启动 B 计划(如 `ie shell` 侵入式环境方案)。

---

**文档版本**: 1.0
**最后更新**: 2025-11-13
**状态**: 设计提案 - 待技术验证
