#!/bin/bash
# Intent-Engine Session Restoration Hook
# Version: 1.0
# Triggers: Before Claude Code session starts

set -euo pipefail

# 0. Ensure jq is available
JQ_CMD=$(command -v jq || echo "/usr/bin/jq")
if [ ! -x "$JQ_CMD" ]; then
    echo "<system-reminder>"
    echo "Intent-Engine: jq not found. Install: apt-get install jq"
    echo "</system-reminder>"
    exit 0
fi

# 1. Check if Intent-Engine is installed
if ! command -v ie &> /dev/null; then
    echo "<system-reminder>"
    echo "Intent-Engine not found. Install: cargo install intent-engine"
    echo "</system-reminder>"
    exit 0
fi

# 2. Use current working directory
WORKSPACE_DIR="${CLAUDE_WORKSPACE_ROOT:-$(pwd)}"

# 3. Call session-restore
RESTORE_OUTPUT=$(ie session-restore --workspace "$WORKSPACE_DIR" 2>&1)
RESTORE_EXIT_CODE=$?

# 4. Parse JSON and generate reminder
if [ $RESTORE_EXIT_CODE -eq 0 ]; then
    STATUS=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.status')

    if [ "$STATUS" = "success" ]; then
        # === Has focus: Rich context ===
        TASK_ID=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.current_task.id')
        TASK_NAME=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.current_task.name')
        TASK_SPEC=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.current_task.spec_preview // empty')
        PARENT_NAME=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.parent_task.name // "None"')

        SIBLINGS_DONE=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.siblings.done // 0')
        SIBLINGS_TOTAL=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.siblings.total // 0')

        CHILDREN_TODO=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.children.todo // 0')
        CHILDREN_TOTAL=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.children.total // 0')

        # Recent decisions
        RECENT_DECISIONS=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '[.recent_events[]? | select(.type == "decision")] | .[:3][] | "- " + .data')

        # Current blockers
        CURRENT_BLOCKERS=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.recent_events[]? | select(.type == "blocker") | "- " + .data')

        # Done siblings (proof of progress)
        DONE_SIBLINGS=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.siblings.done_list[]? | "- #" + (.id|tostring) + " " + .name')

        # === Minimal style output ===
        echo "<system-reminder priority=\"high\">"
        echo "Intent-Engine: Session Restored"
        echo ""
        echo "Focus: #${TASK_ID} '${TASK_NAME}'"
        echo "Parent: ${PARENT_NAME}"
        echo "Progress: ${SIBLINGS_DONE}/${SIBLINGS_TOTAL} siblings done, ${CHILDREN_TODO} subtasks remain"
        echo ""

        if [ -n "$TASK_SPEC" ]; then
            echo "Spec: ${TASK_SPEC}"
            echo ""
        fi

        if [ -n "$DONE_SIBLINGS" ]; then
            echo "Completed:"
            echo "$DONE_SIBLINGS"
            echo ""
        fi

        if [ -n "$RECENT_DECISIONS" ]; then
            echo "Recent decisions:"
            echo "$RECENT_DECISIONS"
            echo ""
        fi

        if [ -n "$CURRENT_BLOCKERS" ]; then
            echo "⚠️  Blockers:"
            echo "$CURRENT_BLOCKERS"
            echo ""
        fi

        # === Next step suggestion ===
        if [ "$CHILDREN_TODO" -gt 0 ]; then
            echo "Next: Work on subtasks or use 'ie task done' when complete"
        else
            echo "Next: Complete this task with 'ie task done'"
        fi

        # === Ultra-restrained tool hints ===
        echo ""
        echo "Commands: ie event add --type decision|blocker|note, ie task spawn-subtask, ie task done"
        echo "</system-reminder>"

    elif [ "$STATUS" = "no_focus" ]; then
        # === No focus: Simple guidance ===
        TODO_COUNT=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.stats.todo // 0')

        echo "<system-reminder>"
        echo "Intent-Engine: No active focus"
        echo ""
        echo "Tasks: ${TODO_COUNT} pending"
        echo ""
        echo "Next: Use 'ie pick-next' to get a recommended task, or 'ie task list --status todo'"
        echo "</system-reminder>"

    elif [ "$STATUS" = "error" ]; then
        # === Error: Recovery guidance ===
        ERROR_MSG=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.message // "Unknown error"')
        RECOVERY=$(echo "$RESTORE_OUTPUT" | $JQ_CMD -r '.recovery_suggestion // "Check workspace state"')

        echo "<system-reminder>"
        echo "Intent-Engine: Issue detected"
        echo ""
        echo "${ERROR_MSG}"
        echo ""
        echo "Recovery: ${RECOVERY}"
        echo "</system-reminder>"
    fi
else
    # === Intent-Engine call failed: Friendly message ===
    echo "<system-reminder>"
    echo "Intent-Engine: Unable to restore session"
    echo ""
    echo "The workspace may not be initialized or there may be a configuration issue."
    echo "Consider: ie workspace init"
    echo "</system-reminder>"
fi
