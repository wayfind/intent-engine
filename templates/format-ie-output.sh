#!/bin/bash
# Intent-Engine PostToolUse Output Formatter
# Version: 2.0
# Triggers: After MCP tool calls
# Purpose: Format JSON output from intent-engine MCP tools into human-friendly text

set -euo pipefail

# 0. Ensure jq is available - try multiple common locations
JQ_CMD=""
for jq_candidate in \
    "$(command -v jq 2>/dev/null)" \
    "/usr/bin/jq" \
    "/usr/local/bin/jq" \
    "/opt/homebrew/bin/jq" \
    "/mingw64/bin/jq" \
    "/usr/local/opt/jq/bin/jq" \
    "$HOME/.local/bin/jq"; do

    if [ -n "$jq_candidate" ] && [ -x "$jq_candidate" 2>/dev/null ]; then
        JQ_CMD="$jq_candidate"
        break
    fi
done

# Last resort: try jq without full path (rely on PATH)
if [ -z "$JQ_CMD" ]; then
    if command -v jq >/dev/null 2>&1; then
        JQ_CMD="jq"
    else
        exit 0  # Silent fail if jq not available
    fi
fi

# 1. Read JSON input from stdin
# Actual format: {"tool_name": "...", "tool_response": [{"type": "text", "text": "..."}]}
INPUT_JSON=$(cat)

# 2. Parse tool_name and tool_response
TOOL_NAME=$(echo "$INPUT_JSON" | $JQ_CMD -r '.tool_name // ""')
TOOL_OUTPUT=$(echo "$INPUT_JSON" | $JQ_CMD -r '.tool_response[0].text // ""')

# 3. Only process intent-engine MCP tools
if [[ ! "$TOOL_NAME" =~ ^mcp__intent-engine__ ]]; then
    exit 0
fi

# 4. Skip if output is empty
if [ -z "$TOOL_OUTPUT" ]; then
    exit 0
fi

# 5. Format based on tool type
# All output goes to stderr (exit code 2) so Claude Code passes it to Claude
{
case "$TOOL_NAME" in
    "mcp__intent-engine__task_context")
        # Format task context with tree view
        echo "Intent-Engine: Task Context"
        echo ""

        # Main task
        TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.id')
        TASK_NAME=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.name')
        TASK_STATUS=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.status')

        STATUS_BADGE="?"
        case "$TASK_STATUS" in
            "done") STATUS_BADGE="✓" ;;
            "doing") STATUS_BADGE="→" ;;
            "todo") STATUS_BADGE="○" ;;
        esac

        echo "Task #${TASK_ID}: ${TASK_NAME} [${STATUS_BADGE}]"
        echo ""

        # Ancestors (parent chain)
        ANCESTORS_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.ancestors[]?] | length')
        if [ "$ANCESTORS_COUNT" -gt 0 ]; then
            echo "Ancestors:"
            echo "$TOOL_OUTPUT" | $JQ_CMD -r '.ancestors[]? | "  #" + (.id|tostring) + " " + .name + " [" + .status + "]"'
            echo ""
        fi

        # Children (subtasks)
        CHILDREN_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.children[]?] | length')
        if [ "$CHILDREN_COUNT" -gt 0 ]; then
            echo "Children (${CHILDREN_COUNT} subtasks):"
            echo "$TOOL_OUTPUT" | $JQ_CMD -r '.children[]? | "  " + (if .status == "done" then "✓" elif .status == "doing" then "→" else "○" end) + " #" + (.id|tostring) + " " + .name'
            echo ""
        fi

        # Siblings
        SIBLINGS_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.siblings[]?] | length')
        if [ "$SIBLINGS_COUNT" -gt 0 ]; then
            echo "Siblings (${SIBLINGS_COUNT} tasks at same level):"
            echo "$TOOL_OUTPUT" | $JQ_CMD -r '.siblings[]? | "  " + (if .status == "done" then "✓" elif .status == "doing" then "→" else "○" end) + " #" + (.id|tostring) + " " + .name'
            echo ""
        fi

        # Dependencies
        BLOCKING_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.blocking_tasks[]?] | length')
        BLOCKED_BY_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.blocked_by_tasks[]?] | length')

        if [ "$BLOCKING_COUNT" -gt 0 ]; then
            echo "Blocking these tasks:"
            echo "$TOOL_OUTPUT" | $JQ_CMD -r '.blocking_tasks[]? | "  #" + (.id|tostring) + " " + .name'
            echo ""
        fi

        if [ "$BLOCKED_BY_COUNT" -gt 0 ]; then
            echo "⚠️  Blocked by:"
            echo "$TOOL_OUTPUT" | $JQ_CMD -r '.blocked_by_tasks[]? | "  #" + (.id|tostring) + " " + .name + " [" + .status + "]"'
            echo ""
        fi

        ;;

    "mcp__intent-engine__task_get")
        # Format task details
        echo "Intent-Engine: Task Details"
        echo ""

        TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.id')
        TASK_NAME=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.name')
        TASK_STATUS=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.status')
        TASK_SPEC=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.spec // ""')
        PRIORITY=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.priority // "medium"')

        STATUS_BADGE="?"
        case "$TASK_STATUS" in
            "done") STATUS_BADGE="✓" ;;
            "doing") STATUS_BADGE="→" ;;
            "todo") STATUS_BADGE="○" ;;
        esac

        echo "Task #${TASK_ID}: ${TASK_NAME} [${STATUS_BADGE}]"
        echo "Priority: ${PRIORITY}"
        echo ""

        # Timestamps
        CREATED=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.first_todo_at // ""')
        STARTED=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.first_doing_at // ""')
        COMPLETED=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.first_done_at // ""')

        if [ -n "$CREATED" ]; then
            echo "Created: $CREATED"
            [ -n "$STARTED" ] && echo "Started: $STARTED"
            [ -n "$COMPLETED" ] && echo "Completed: $COMPLETED"
            echo ""
        fi

        # Spec (truncated if too long)
        if [ -n "$TASK_SPEC" ]; then
            SPEC_LENGTH=${#TASK_SPEC}
            if [ "$SPEC_LENGTH" -gt 200 ]; then
                echo "Spec (truncated):"
                SPEC_PREVIEW=$(echo "$TASK_SPEC" | head -c 200)
                echo "$SPEC_PREVIEW..."
            else
                echo "Spec:"
                echo "$TASK_SPEC"
            fi
            echo ""
        fi

        # Events summary if present
        HAS_EVENTS=$(echo "$TOOL_OUTPUT" | $JQ_CMD 'has("events_summary")')
        if [ "$HAS_EVENTS" = "true" ]; then
            EVENTS_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.events_summary.events[]?] | length')
            if [ "$EVENTS_COUNT" -gt 0 ]; then
                echo "Recent events (${EVENTS_COUNT}):"
                echo "$TOOL_OUTPUT" | $JQ_CMD -r '.events_summary.events[]? | "  [" + .type + "] " + .data'
                echo ""
            fi
        fi

        ;;

    "mcp__intent-engine__current_task_get")
        # Format current task

        HAS_TASK=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.current_task_id != null')

        if [ "$HAS_TASK" = "true" ]; then
            TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.id')
            TASK_NAME=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.name')
            TASK_STATUS=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.status')

            STATUS_BADGE="?"
            case "$TASK_STATUS" in
                "done") STATUS_BADGE="✓" ;;
                "doing") STATUS_BADGE="→" ;;
                "todo") STATUS_BADGE="○" ;;
            esac

            echo "Current task: #${TASK_ID} ${TASK_NAME} [${STATUS_BADGE}]"

            # Spec preview if available
            SPEC_PREVIEW=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.spec // "" | split("\n")[0]')
            if [ -n "$SPEC_PREVIEW" ]; then
                echo ""
                echo "${SPEC_PREVIEW}"
            fi
        else
            echo "No task currently focused"
            echo ""
            echo "Tip: Use 'ie task pick-next' to get a recommendation"
        fi

        ;;

    "mcp__intent-engine__task_list")
        # Format task list
        echo "Intent-Engine: Task List"
        echo ""

        TASK_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD 'length')

        if [ "$TASK_COUNT" -eq 0 ]; then
            echo "No tasks found"
        else
            echo "Found ${TASK_COUNT} tasks:"
            echo ""
            echo "$TOOL_OUTPUT" | $JQ_CMD -r '.[] | "  " + (if .status == "done" then "✓" elif .status == "doing" then "→" else "○" end) + " #" + (.id|tostring) + " " + .name + " [p" + (.priority|tostring) + "]"'
        fi

        ;;

    "mcp__intent-engine__task_pick_next")
        # Format pick-next recommendation
        echo "Intent-Engine: Next Task Recommendation"
        echo ""

        HAS_TASK=$(echo "$TOOL_OUTPUT" | $JQ_CMD 'has("task")')

        if [ "$HAS_TASK" = "true" ]; then
            TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.id')
            TASK_NAME=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.name')
            SUGGESTION_TYPE=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.suggestion_type // ""')

            echo "Recommended: #${TASK_ID} ${TASK_NAME}"
            echo ""

            if [ -n "$SUGGESTION_TYPE" ]; then
                case "$SUGGESTION_TYPE" in
                    "SUBTASK_OF_CURRENT")
                        echo "Why: Subtask of current focused task"
                        ;;
                    "TOP_LEVEL_TASK")
                        echo "Why: Top-level task (no current focus)"
                        ;;
                esac
                echo ""
            fi

            echo "Next: Use 'ie task start ${TASK_ID}' to begin work"
        else
            echo "No tasks to recommend"
            echo ""
            echo "All tasks may be completed or blocked by dependencies"
        fi

        ;;

    "mcp__intent-engine__task_search"|"mcp__intent-engine__unified_search")
        # Format search results
        echo "Intent-Engine: Search Results"
        echo ""

        TASK_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.results[]? | select(.type == "task")] | length')
        EVENT_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.results[]? | select(.type == "event")] | length')

        if [ "$TASK_COUNT" -gt 0 ]; then
            echo "Tasks (${TASK_COUNT}):"
            echo "$TOOL_OUTPUT" | $JQ_CMD -r '[.results[]? | select(.type == "task")] | .[] | "  #" + (.id|tostring) + " " + .name + " [" + .status + "]"'
            echo ""
        fi

        if [ "$EVENT_COUNT" -gt 0 ]; then
            echo "Events (${EVENT_COUNT}):"
            echo "$TOOL_OUTPUT" | $JQ_CMD -r '[.results[]? | select(.type == "event")] | .[] | "  [" + .event_type + "] Task #" + (.task_id|tostring) + ": " + (.data | split("\n")[0])'
            echo ""
        fi

        if [ "$TASK_COUNT" -eq 0 ] && [ "$EVENT_COUNT" -eq 0 ]; then
            echo "No results found"
        fi

        ;;

    "mcp__intent-engine__event_list")
        # Format event list
        echo "Intent-Engine: Events"
        echo ""

        EVENT_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.events[]?] | length')

        if [ "$EVENT_COUNT" -eq 0 ]; then
            echo "No events found"
        else
            echo "Found ${EVENT_COUNT} events:"
            echo ""
            echo "$TOOL_OUTPUT" | $JQ_CMD -r '.events[]? | "  [" + .type + "] " + .created_at + "\n    " + (.data | split("\n")[0])'
        fi

        ;;

    "mcp__intent-engine__task_add")
        # Format task_add response
        TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.id')
        TASK_NAME=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.name')
        PARENT_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.parent_id // ""')

        echo "✓ Created task #${TASK_ID}: ${TASK_NAME}"

        if [ -n "$PARENT_ID" ]; then
            echo "  └─ Parent: #${PARENT_ID}"
        fi

        echo ""
        echo "Next: Use 'ie task start ${TASK_ID}' to begin work"
        ;;

    "mcp__intent-engine__task_start")
        # Format task_start response
        TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.id // .id')
        TASK_NAME=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task.name // .name')

        echo "→ Started task #${TASK_ID}: ${TASK_NAME}"

        # Check if events were included
        HAS_EVENTS=$(echo "$TOOL_OUTPUT" | $JQ_CMD 'has("events_summary")')
        if [ "$HAS_EVENTS" = "true" ]; then
            EVENTS_COUNT=$(echo "$TOOL_OUTPUT" | $JQ_CMD '[.events_summary.recent_events[]?] | length')
            if [ "$EVENTS_COUNT" -gt 0 ]; then
                echo ""
                echo "Recent events (${EVENTS_COUNT}):"
                echo "$TOOL_OUTPUT" | $JQ_CMD -r '.events_summary.recent_events[]? | "  [" + .type + "] " + (.data | split("\n")[0])'
            fi
        fi
        ;;

    "mcp__intent-engine__task_done")
        # Format task_done response
        TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.id')
        TASK_NAME=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.name')

        echo "✓ Completed task #${TASK_ID}: ${TASK_NAME}"
        ;;

    "mcp__intent-engine__task_switch")
        # Format task_switch response
        TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.id')
        TASK_NAME=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.name')

        echo "→ Switched to task #${TASK_ID}: ${TASK_NAME}"
        ;;

    "mcp__intent-engine__task_spawn_subtask")
        # Format task_spawn_subtask response
        TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.id')
        TASK_NAME=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.name')
        PARENT_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.parent_id // ""')

        echo "✓ Created and started subtask #${TASK_ID}: ${TASK_NAME}"

        if [ -n "$PARENT_ID" ]; then
            echo "  └─ Parent: #${PARENT_ID}"
        fi
        ;;

    "mcp__intent-engine__event_add")
        # Format event_add response
        EVENT_TYPE=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.type')
        TASK_ID=$(echo "$TOOL_OUTPUT" | $JQ_CMD -r '.task_id // ""')

        echo "✓ Recorded event: [${EVENT_TYPE}]"

        if [ -n "$TASK_ID" ]; then
            echo "  └─ Task: #${TASK_ID}"
        fi
        ;;

    *)
        # For any other intent-engine tools not explicitly handled
        # Just pass through without formatting
        exit 0
        ;;
esac
} >&2

# Exit with code 2 to pass output to Claude
exit 2
