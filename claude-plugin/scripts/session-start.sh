#!/bin/bash
# Intent-Engine Session Start Hook

input=$(cat)
session_id=$(echo "$input" | jq -r '.session_id // ""' 2>/dev/null)

if [ -n "$CLAUDE_ENV_FILE" ] && [ -n "$session_id" ]; then
    echo "export IE_SESSION_ID=\"$session_id\"" >> "$CLAUDE_ENV_FILE"
fi

if ! command -v ie &> /dev/null; then
    cat << 'EOF'
<system-reminder>
intent-engine not found. Install: cargo install intent-engine
</system-reminder>
EOF
    exit 0
fi

IE_SESSION_ID="$session_id" ie status 2>/dev/null

cat << 'EOF'

<system-reminder>
# Use `ie plan` instead of TodoWrite (cross-session, hierarchical, decision-tracking)

  • Session start: ie status
  • Create/update tasks: echo '{...}' | ie plan
  • Record decisions: ie log decision "..."
  • Details: ie --help
</system-reminder>
EOF

exit 0
