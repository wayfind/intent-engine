#!/bin/bash
# Intent-Engine Session Start Hook

input=$(cat)
session_id=$(echo "$input" | jq -r '.session_id // ""' 2>/dev/null)

if [ -n "$CLAUDE_ENV_FILE" ] && [ -n "$session_id" ]; then
    echo "export IE_SESSION_ID=\"$session_id\"" >> "$CLAUDE_ENV_FILE"
fi

# Auto-install ie if not found
if ! command -v ie &> /dev/null; then
    if command -v cargo &> /dev/null; then
        cargo install intent-engine 2>/dev/null
    elif command -v npm &> /dev/null; then
        npm install -g @m3task/intent-engine 2>/dev/null
    elif command -v brew &> /dev/null; then
        brew install wayfind/tap/intent-engine 2>/dev/null
    fi
fi

# Check again after install attempt
if ! command -v ie &> /dev/null; then
    cat << 'EOF'
<system-reminder>
intent-engine not found. Install: cargo install intent-engine
</system-reminder>
EOF
    exit 0
fi

# Initialize project if not exists
if [ -n "$CLAUDE_PROJECT_DIR" ] && [ ! -d "$CLAUDE_PROJECT_DIR/.intent-engine" ]; then
    ie init --at "$CLAUDE_PROJECT_DIR" 2>/dev/null
fi

# Run status from project root
cd "$CLAUDE_PROJECT_DIR" 2>/dev/null
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
