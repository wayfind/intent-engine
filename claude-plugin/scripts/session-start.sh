#!/usr/bin/env bash
# Intent-Engine Session Start Hook
# Compatible with: Linux, macOS, WSL, Git Bash on Windows
set -euo pipefail

# === Helper Functions ===

# Safe JSON parsing without jq dependency
parse_session_id() {
    local input="$1"
    # Try jq first (fastest and most reliable)
    if command -v jq &>/dev/null; then
        echo "$input" | jq -r '.session_id // empty' 2>/dev/null && return
    fi
    # Fallback: Python (usually available)
    if command -v python3 &>/dev/null; then
        echo "$input" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('session_id',''))" 2>/dev/null && return
    fi
    if command -v python &>/dev/null; then
        echo "$input" | python -c "import sys,json; d=json.load(sys.stdin); print(d.get('session_id',''))" 2>/dev/null && return
    fi
    # Fallback: grep/sed (basic, may fail on edge cases)
    echo "$input" | grep -o '"session_id"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*:.*"\([^"]*\)".*/\1/' 2>/dev/null || echo ""
}

log_debug() {
    # Uncomment for debugging: echo "[ie-hook] $*" >&2
    :
}

# === Main Logic ===

# Read stdin with timeout (avoid blocking)
input=""
if [ -t 0 ]; then
    log_debug "No stdin (terminal mode)"
else
    # Read stdin, timeout after 1 second
    if command -v timeout &>/dev/null; then
        input=$(timeout 1 cat 2>/dev/null) || true
    elif command -v gtimeout &>/dev/null; then
        # macOS with coreutils
        input=$(gtimeout 1 cat 2>/dev/null) || true
    else
        # Fallback: just read (may block if no input)
        read -t 1 -r input 2>/dev/null || true
    fi
fi

# Parse session_id
session_id=""
if [ -n "$input" ]; then
    session_id=$(parse_session_id "$input")
    log_debug "Parsed session_id: $session_id"
fi

# Set session environment variable
if [ -n "${CLAUDE_ENV_FILE:-}" ] && [ -n "$session_id" ]; then
    # Validate session_id (alphanumeric, dash, underscore only)
    if [[ "$session_id" =~ ^[a-zA-Z0-9_-]+$ ]]; then
        echo "export IE_SESSION_ID=\"$session_id\"" >> "$CLAUDE_ENV_FILE"
        log_debug "Wrote session_id to CLAUDE_ENV_FILE"
    else
        log_debug "Invalid session_id format, skipping"
    fi
fi

# === Auto-install ie if not found ===

install_ie() {
    log_debug "Attempting to install intent-engine..."

    # Try cargo (preferred for Rust users)
    if command -v cargo &>/dev/null; then
        log_debug "Installing via cargo..."
        if cargo install intent-engine 2>&1 | tail -1; then
            return 0
        fi
    fi

    # Try npm (cross-platform, no compiler needed)
    if command -v npm &>/dev/null; then
        log_debug "Installing via npm..."
        if npm install -g @m3task/intent-engine 2>&1 | tail -1; then
            return 0
        fi
    fi

    # Try brew (macOS/Linux)
    if command -v brew &>/dev/null; then
        log_debug "Installing via brew..."
        if brew install wayfind/tap/intent-engine 2>&1 | tail -1; then
            return 0
        fi
    fi

    return 1
}

if ! command -v ie &>/dev/null; then
    install_ie || true
fi

# Check if ie is available now
if ! command -v ie &>/dev/null; then
    cat << 'EOF'
<system-reminder>
intent-engine (ie) not installed. Install via one of:
  cargo install intent-engine
  npm install -g @m3task/intent-engine
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

# Export session_id for ie command
export IE_SESSION_ID="${session_id:-}"

# Run status, capture output
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
