// Intent-Engine Plugin for OpenCode
// Provides cross-session task persistence, decision logging, and plan management
// https://github.com/anthropics/intent-engine

import { Plugin, tool } from "@opencode-ai/plugin"

// =============================================================================
// System Prompt - 引导 AI 使用 Intent-Engine
// =============================================================================

const IE_SYSTEM_PROMPT = `
## Intent-Engine Protocol (MANDATORY)

Intent-Engine is your **external brain** - persistent memory across sessions.

### Session Start
IMMEDIATELY call: ie_status
- For sub-agent work on specific task: ie_status <task_id> (retrieves full ancestry)

### Planning with ie_plan

MUST use ie_plan as ONLY plan storage. Structure:
{
  "tasks": [{
    "name": "<task name>",
    "status": "doing",
    "spec": "<RICH MARKDOWN>",
    "children": [<subtasks>],
    "depends_on": ["<task_name>"]
  }]
}

**spec is a DOCUMENTATION STORE** - supports GB-scale markdown:
- Include: mermaid diagrams, code blocks, tables
- Document EVERYTHING about the task

### Workflow-Specific Patterns

**Bug Fix Workflow** (reproduce→diagnose→fix→verify):
- FLAT task structure (linear steps, not deeply nested)
- Heavy \`note\` events for investigation findings
- \`blocker\` when investigation is stuck (missing logs, can't reproduce)
- \`decision\` for fix approach (quick patch vs proper fix)
- \`milestone\` when root cause identified

**Refactoring/Migration Workflow** (analyze→design→migrate→verify):
- DEEP task hierarchy (phase→component→step)
- SEQUENTIAL \`depends_on\` chain (migrate A before B)
- \`decision\` for risk mitigation strategies
- \`milestone\` after each component migrated

**Feature Development Workflow** (design→implement→integrate→test):
- PARALLEL branches: Backend and Frontend can start simultaneously
- \`depends_on\` shows what MUST wait vs what can run in parallel
- Integration task depends on BOTH Backend AND Frontend
- Rich specs with API contracts, schemas, diagrams

### Events as CQRS Audit Trail

ie_log records immutable events. Match event type to situation:

| Type | When to Use | Workflow Hint |
|------|-------------|---------------|
| decision | Chose X over Y with trade-offs | All workflows |
| blocker | Cannot proceed, waiting for X | Bug fix (stuck), Migration (dependency) |
| milestone | Significant checkpoint | Migration (component done), Feature (phase done) |
| note | Observations, findings | Bug fix (investigation clues) |

**Events support rich markdown** - document thoroughly.

### Search Liberally
ie_search PROACTIVELY before decisions or when resuming work.

### FORBIDDEN
- Creating .opencode/plan/*.md files
- Using todowrite for persistent work
- Making decisions without ie_log
`

// =============================================================================
// Plugin Implementation
// =============================================================================

const plugin: Plugin = async (ctx) => {
  const ie = async (sessionID: string, cmd: string) => {
    const ieSessionId = process.env.IE_SESSION_ID || sessionID
    return ctx.$`bash -c ${"IE_SESSION_ID=" + ieSessionId + " " + cmd}`.quiet().nothrow()
  }

  // Helper: Check if IE is available
  const ieAvailable = async (): Promise<boolean> => {
    try {
      const result = await ctx.$`which ie`.quiet().nothrow()
      return result.exitCode === 0
    } catch {
      return false
    }
  }

  // Helper: Check if IE is initialized in current project
  const ieInitialized = async (sessionID: string): Promise<boolean> => {
    try {
      const result = await ie(sessionID, "ie status --format json")
      return result.exitCode === 0 && !result.text().includes('"error"')
    } catch {
      return false
    }
  }

  return {
    // =========================================================================
    // 1. System Prompt Injection - 引导 AI 使用 IE
    // =========================================================================
    async "experimental.chat.system.transform"(input, output) {
      if (await ieAvailable()) {
        output.system.push(IE_SYSTEM_PROMPT)
      }
    },

    // =========================================================================
    // 2. Session Start Context Restoration - 自动恢复上下文
    // =========================================================================
    async "chat.message"(input, output) {
      // 只在新会话第一条消息时注入
      if (input.messageID !== undefined) return
      if (!(await ieAvailable())) return

      try {
        const result = await ie(input.sessionID, "ie status -e --format json")
        const statusText = result.text()

        // 只在有实际任务时注入
        if (
          result.exitCode === 0 &&
          statusText &&
          !statusText.includes('"error"') &&
          (statusText.includes('"current_task"') || statusText.includes('"name"'))
        ) {
          output.parts.unshift({
            type: "text" as const,
            text: `<intent-engine-session-context>
Your previous work context has been restored from Intent-Engine:

${statusText}

Continue from where you left off. Run ie_status for full details if needed.
</intent-engine-session-context>`,
          })
        }
      } catch (e) {
        // IE not available or not initialized, skip silently
      }
    },

    // =========================================================================
    // 3. Tool Execute Hooks - 监听关键事件并同步
    // =========================================================================
    async "tool.execute.after"(input, output) {
      if (!(await ieAvailable())) return

      // 3a. Sync todowrite to IE (backup mechanism)
      if (input.tool === "todowrite") {
        try {
          const todos = (output.metadata as any)?.todos || []
          if (todos.length === 0) return

          const ieTasks = todos.map((t: any) => ({
            name: t.content,
            status:
              t.status === "in_progress"
                ? "doing"
                : t.status === "completed"
                  ? "done"
                  : t.status === "cancelled"
                    ? "done"
                    : "todo",
            priority: t.priority || "medium",
          }))

          const json = JSON.stringify({ tasks: ieTasks })
          const escaped = json.replace(/'/g, "'\"'\"'")
          await ie(input.sessionID, `echo '${escaped}' | ie plan --format json`)
        } catch (e) {
          // Sync failed, continue silently
        }
      }

      // 3b. Validate ie_plan output and provide feedback
      if (input.tool === "ie_plan") {
        try {
          const result = await ie(input.sessionID, "ie status --format json")
          if (result.exitCode !== 0) return

          const data = JSON.parse(result.text())
          const warnings: string[] = []

          // Check spec quality
          if (data.current_task?.spec) {
            const spec = data.current_task.spec
            if (!/##\s*Goal/i.test(spec)) {
              warnings.push("Tip: Add '## Goal' section to spec for clarity")
            }
            if (!/##\s*Approach/i.test(spec)) {
              warnings.push("Tip: Add '## Approach' section to spec")
            }
          } else if (data.current_task?.status === "doing") {
            warnings.push("Tip: Tasks with status 'doing' should have a spec")
          }

          // Check for decision logging reminder
          const events = data.events || []
          const hasRecentDecision = events.some(
            (e: any) =>
              e.event_type === "decision" &&
              Date.now() - new Date(e.created_at).getTime() < 3600000
          )
          if (!hasRecentDecision && data.current_task) {
            warnings.push("Tip: Remember to log key decisions with ie_log decision")
          }

          if (warnings.length > 0) {
            output.output += "\n\n---\n" + warnings.join("\n")
          }
        } catch (e) {
          // Validation failed, skip
        }
      }
    },

    // =========================================================================
    // 4. Intent-Engine Tools
    // =========================================================================
    tool: {
      // ---------------------------------------------------------------------
      // ie_status - Get current task context
      // ---------------------------------------------------------------------
      ie_status: tool({
        description: `Get current task context from Intent-Engine.

ALWAYS run this at session start to restore your working context.

Usage:
- ie_status: Get current focused task
- ie_status <task_id>: Get specific task with full ancestry (for sub-agent work)

Returns:
- Current focused task with spec
- Parent and children hierarchy  
- Recent events (decisions, blockers, notes)
- Task status and priority
- Full ancestor chain (when querying specific task)`,
        args: {
          task_id: tool.schema
            .string()
            .optional()
            .describe("Specific task ID to query (retrieves full ancestry for sub-agent context)"),
          with_events: tool.schema
            .boolean()
            .optional()
            .describe("Include event history (default: true)"),
        },
        async execute(args, toolCtx) {
          const flags = args.with_events === false ? "" : "-e"
          const taskArg = args.task_id ? args.task_id : ""
          const result = await ie(toolCtx.sessionID, `ie status ${taskArg} ${flags} --format json`)

          if (result.exitCode !== 0) {
            const stderr = result.stderr.toString()
            if (stderr.includes("not found") || stderr.includes("no such")) {
              return "Intent-Engine not initialized in this project. Run: ie init"
            }
            return `Error: ${stderr || result.text()}`
          }

          return result.text()
        },
      }),

      // ---------------------------------------------------------------------
      // ie_plan - Create/update task tree
      // ---------------------------------------------------------------------
      ie_plan: tool({
        description: `Create or update tasks in Intent-Engine. 

Use this as your ONLY plan storage - do NOT create .opencode/plan/*.md files.

Supports:
- Hierarchical tasks with children
- Status: todo, doing, done
- Spec: RICH MARKDOWN (mermaid diagrams, code blocks, @file:path imports) - this is your documentation store
- Priority: critical, high, medium, low
- depends_on: array of task IDs for parallel coordination

Example:
{"tasks":[{
  "name": "Implement auth",
  "status": "doing",
  "spec": "## Goal\\nUsers can login securely\\n\\n## Approach\\n\`\`\`mermaid\\nsequenceDiagram...\\n\`\`\`\\n\\n1. JWT tokens\\n2. bcrypt passwords",
  "children": [
    {"name": "Design schema", "status": "todo", "spec": "## Goal\\nDefine User table"},
    {"name": "Add endpoints", "status": "todo", "depends_on": ["schema_task_id"]},
    {"name": "Write tests", "status": "todo"}
  ]
}]}

Rules:
- Same task name = update existing (idempotent)
- New tasks auto-parent to current focus
- Use parent_id: null for top-level tasks
- spec supports GB-scale content - document EVERYTHING about the task`,
        args: {
          tasks_json: tool.schema.string().describe("JSON object with tasks array"),
        },
        async execute(args, toolCtx) {
          // Validate JSON
          try {
            const parsed = JSON.parse(args.tasks_json)
            if (!parsed.tasks || !Array.isArray(parsed.tasks)) {
              return "Error: JSON must have 'tasks' array"
            }
          } catch (e) {
            return `Invalid JSON: ${e}`
          }

          const tmpFile = `/tmp/ie-plan-${Date.now()}.json`
          const fs = await import("fs")
          fs.writeFileSync(tmpFile, args.tasks_json)
          
          const result = await ie(
            toolCtx.sessionID,
            `cat ${tmpFile} | ie plan --format json && rm -f ${tmpFile}`
          )

          if (result.exitCode !== 0) {
            return `Error: ${result.stderr.toString() || result.text()}`
          }

          return result.text()
        },
      }),

      // ---------------------------------------------------------------------
      // ie_log - Record events (decision, blocker, milestone, note)
      // ---------------------------------------------------------------------
      ie_log: tool({
        description: `Record an event for the current focused task.

Event types:
- decision: WHY you chose something (architecture, library, approach)
- blocker: What's preventing progress
- milestone: Significant achievement reached
- note: General observation or finding

Events persist across sessions - future AI instances can understand your reasoning.
Events are IMMUTABLE (CQRS audit trail) - will be auto-summarized into spec on task completion.

**Events support RICH MARKDOWN** - not just short strings. Document thoroughly:

Example detailed decision:
"## Token Algorithm Decision

### Context
Multi-service architecture with separate auth server

### Options Considered
1. HS256 - symmetric, simpler
2. RS256 - asymmetric, can verify without secret

### Decision
RS256 - services only need public key

### Trade-offs
- (+) No shared secret needed
- (-) Larger tokens, more CPU"

Quick examples:
- ie_log decision "Chose PostgreSQL over MongoDB: need ACID transactions"
- ie_log blocker "Waiting for API credentials from client"
- ie_log milestone "Core auth flow working end-to-end"
- ie_log note "Found existing auth helper in /src/utils/auth.ts"`,
        args: {
          type: tool.schema
            .enum(["decision", "blocker", "milestone", "note"])
            .describe("Event type"),
          message: tool.schema.string().describe("Event message (markdown supported)"),
        },
        async execute(args, toolCtx) {
          // Escape for shell
          const escaped = args.message
            .replace(/'/g, "'\"'\"'")
            .replace(/\n/g, "\\n")

          const result = await ie(
            toolCtx.sessionID,
            `ie log ${args.type} '${escaped}'`
          )

          if (result.exitCode !== 0) {
            return `Error: ${result.stderr.toString() || result.text()}`
          }

          const output = result.text()
          return output || `Logged ${args.type}: ${args.message.slice(0, 100)}${args.message.length > 100 ? "..." : ""}`
        },
      }),

      // ---------------------------------------------------------------------
      // ie_search - Search tasks and events
      // ---------------------------------------------------------------------
      ie_search: tool({
        description: `Search tasks and events in Intent-Engine history.

Uses FTS5 full-text search. Useful for:
- Finding past decisions: "decision JWT"
- Finding unfinished work: "todo doing"  
- Finding specific tasks: "authentication"
- Finding blockers: "blocker"

Returns matching tasks and events with context.`,
        args: {
          query: tool.schema.string().describe("Search query"),
        },
        async execute(args, toolCtx) {
          const escaped = args.query.replace(/'/g, "'\"'\"'")
          const result = await ie(
            toolCtx.sessionID,
            `ie search '${escaped}' --format json`
          )

          if (result.exitCode !== 0) {
            return `Error: ${result.stderr.toString() || result.text()}`
          }

          return result.text()
        },
      }),

      // ---------------------------------------------------------------------
      // ie_done - Mark current task as done
      // ---------------------------------------------------------------------
      ie_done: tool({
        description: `Mark the current focused task as done.

Prerequisites:
- Must have a focused task (run ie_status first)
- All subtasks (children) must be done first

After completion, focus is cleared. Run ie_status or ie_plan to set new focus.`,
        args: {},
        async execute(args, toolCtx) {
          const result = await ie(toolCtx.sessionID, "ie done --format json")

          if (result.exitCode !== 0) {
            const error = result.stderr.toString() || result.text()
            if (error.includes("children") || error.includes("subtask")) {
              return `Error: Cannot complete - subtasks still pending. Complete children first.`
            }
            if (error.includes("focus") || error.includes("current")) {
              return `Error: No focused task. Use ie_plan to create/focus a task first.`
            }
            return `Error: ${error}`
          }

          return result.text()
        },
      }),

      // ---------------------------------------------------------------------
      // ie_verify - Verify IE usage compliance (for testing/debugging)
      // ---------------------------------------------------------------------
      ie_verify: tool({
        description: `Verify Intent-Engine usage compliance and plan quality.

Checks:
- No .opencode/plan files created
- Current task has proper spec (Goal + Approach)
- Decisions are being logged
- Task hierarchy is reasonable

Returns a score and recommendations.`,
        args: {},
        async execute(args, toolCtx) {
          const report: string[] = ["# IE Compliance Report\n"]
          let score = 0

          // 1. Check for .opencode/plan files
          const planCheck = await ctx.$`find .opencode/plan -name "*.md" 2>/dev/null | wc -l`
            .quiet()
            .nothrow()
          const planFileCount = parseInt(planCheck.text().trim()) || 0

          if (planFileCount === 0) {
            report.push("✅ No .opencode/plan files (+20)")
            score += 20
          } else {
            report.push(`❌ Found ${planFileCount} plan files - should use ie_plan only`)
          }

          // 2. Get IE status
          const status = await ie(toolCtx.sessionID, "ie status -e --format json")
          if (status.exitCode !== 0) {
            report.push("❌ IE not initialized or error")
            return report.join("\n") + `\n\n**Score: ${score}/100**`
          }

          let data: any
          try {
            data = JSON.parse(status.text())
          } catch {
            report.push("❌ Failed to parse IE status")
            return report.join("\n") + `\n\n**Score: ${score}/100**`
          }

          // 3. Check current task
          if (data.current_task) {
            report.push("✅ Has focused task (+10)")
            score += 10

            const spec = data.current_task.spec || ""
            const hasGoal = /##\s*Goal/i.test(spec)
            const hasApproach = /##\s*Approach/i.test(spec)

            if (hasGoal) {
              report.push("✅ Spec has Goal (+10)")
              score += 10
            } else {
              report.push("❌ Spec missing Goal")
            }

            if (hasApproach) {
              report.push("✅ Spec has Approach (+10)")
              score += 10
            } else {
              report.push("❌ Spec missing Approach")
            }

            if (spec.length > 200) {
              report.push("✅ Detailed spec >200 chars (+10)")
              score += 10
            } else if (spec.length > 100) {
              report.push("⚠️ Brief spec (+5)")
              score += 5
            } else if (spec.length > 0) {
              report.push("⚠️ Short spec (+2)")
              score += 2
            } else {
              report.push("❌ No spec")
            }
          } else {
            report.push("⚠️ No focused task")
          }

          // 4. Check children
          const children = data.children || []
          if (children.length >= 3) {
            report.push(`✅ Good breakdown: ${children.length} subtasks (+15)`)
            score += 15
          } else if (children.length > 0) {
            report.push(`⚠️ Few subtasks: ${children.length} (+5)`)
            score += 5
          } else {
            report.push("⚠️ No subtasks (may be OK for simple tasks)")
          }

          // 5. Check events
          const events = data.events || []
          const decisions = events.filter((e: any) => e.event_type === "decision")
          const notes = events.filter((e: any) => e.event_type === "note")

          if (decisions.length >= 2) {
            report.push(`✅ Multiple decisions logged: ${decisions.length} (+15)`)
            score += 15
          } else if (decisions.length === 1) {
            report.push(`⚠️ Only 1 decision logged (+5)`)
            score += 5
          } else {
            report.push("❌ No decisions logged - use ie_log decision")
          }

          if (notes.length > 0) {
            report.push(`✅ Notes recorded: ${notes.length} (+5)`)
            score += 5
          }

          // Grade
          const grade =
            score >= 90
              ? "A"
              : score >= 75
                ? "B"
                : score >= 60
                  ? "C"
                  : score >= 40
                    ? "D"
                    : "F"

          report.push("")
          report.push("---")
          report.push(`**Score: ${score}/100 (Grade: ${grade})**`)

          if (score < 75) {
            report.push("")
            report.push("### Improvements:")
            if (planFileCount > 0) report.push("- Delete .opencode/plan files, use ie_plan")
            if (!data.current_task?.spec?.match(/Goal/i))
              report.push("- Add ## Goal to spec")
            if (!data.current_task?.spec?.match(/Approach/i))
              report.push("- Add ## Approach to spec")
            if (decisions.length < 2)
              report.push("- Log decisions with ie_log decision")
            if (children.length < 3) report.push("- Break down into more subtasks")
          }

          return report.join("\n")
        },
      }),
    },
  }
}

export default plugin
