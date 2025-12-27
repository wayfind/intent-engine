#!/usr/bin/env bash
# Intent-Engine Session Start Hook
# Simplified cross-platform script (works on Linux, macOS, WSL, Git Bash)
set -euo pipefail

# === Helper Functions ===

log_debug() {
    # Uncomment for debugging: echo "[ie-hook] $*" >&2
    :
}

# === Parse stdin (session_id) ===

input=""
session_id=""

# Read stdin with timeout (avoid blocking)
if [ ! -t 0 ]; then
    if command -v timeout &>/dev/null; then
        input=$(timeout 2 cat 2>/dev/null) || true
    elif command -v gtimeout &>/dev/null; then
        input=$(gtimeout 2 cat 2>/dev/null) || true
    else
        read -t 2 -r input 2>/dev/null || true
    fi
fi

# Parse session_id from JSON (jq preferred, fallback to python/grep)
if [ -n "$input" ]; then
    if command -v jq &>/dev/null; then
        session_id=$(echo "$input" | jq -r '.session_id // empty' 2>/dev/null) || true
    elif command -v python3 &>/dev/null; then
        session_id=$(echo "$input" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('session_id',''))" 2>/dev/null) || true
    elif command -v python &>/dev/null; then
        session_id=$(echo "$input" | python -c "import sys,json; d=json.load(sys.stdin); print(d.get('session_id',''))" 2>/dev/null) || true
    else
        session_id=$(echo "$input" | grep -o '"session_id"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*:.*"\([^"]*\)".*/\1/' 2>/dev/null) || true
    fi
    log_debug "Parsed session_id: $session_id"
fi

# === Set environment variable ===

if [ -n "${CLAUDE_ENV_FILE:-}" ] && [ -n "$session_id" ]; then
    # Validate session_id (alphanumeric, dash, underscore only)
    if [[ "$session_id" =~ ^[a-zA-Z0-9_-]+$ ]]; then
        echo "export IE_SESSION_ID=\"$session_id\"" >> "$CLAUDE_ENV_FILE"
        log_debug "Wrote session_id to CLAUDE_ENV_FILE"
    fi
fi

export IE_SESSION_ID="${session_id:-}"

# === Check if ie is installed ===

if ! command -v ie &>/dev/null; then
    cat << 'EOF'
<system-reminder>
intent-engine (ie) not installed. Install via one of:
  npm install -g @m3task/intent-engine
  cargo install intent-engine
  brew install wayfind/tap/intent-engine
</system-reminder>
EOF
    exit 0
fi

# === Initialize project if needed ===

project_dir="${CLAUDE_PROJECT_DIR:-$(pwd)}"

if [ -d "$project_dir" ] && [ ! -d "$project_dir/.intent-engine" ]; then
    log_debug "Initializing ie in $project_dir"
    (cd "$project_dir" && ie init 2>/dev/null) || true
fi

# === Run ie status ===

if [ -d "$project_dir" ]; then
    cd "$project_dir" || true
fi

status_output=$(ie status 2>&1) || true

if [ -n "$status_output" ]; then
    echo "$status_output"
fi

# === Output system reminder ===

cat << 'EOF'

<system-reminder>
# Use `ie plan` instead of TodoWrite (cross-session, hierarchical, decision-tracking)

  - Session start: ie status
  - Create/update tasks: echo '{...}' | ie plan
  - Record decisions: ie log decision "..."
  - Details: ie --help
</system-reminder>
EOF

exit 0
