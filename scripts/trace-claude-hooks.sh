#!/bin/bash
# Trace Claude Code hook file access
# Usage: ./trace-claude-hooks.sh

set -euo pipefail

echo "🔍 Claude Code Hook Tracer"
echo "=========================="
echo ""

# Find Claude Code process
CLAUDE_PID=$(pgrep -f "claude.*code" | head -1 || echo "")

if [ -z "$CLAUDE_PID" ]; then
    echo "⚠️  Claude Code is not running."
    echo ""
    echo "Options:"
    echo "1. Start Claude Code first, then run this script"
    echo "2. Or use: sudo strace -e trace=openat,open -f -p <PID> 2>&1 | grep -E 'claude|hooks|settings'"
    echo ""
    echo "Manual monitoring:"
    echo "  ps aux | grep claude"
    echo "  sudo strace -e trace=openat,open -f -p <PID> 2>&1 | grep settings"
    exit 1
fi

echo "✓ Found Claude Code process: PID $CLAUDE_PID"
echo ""

# Check if we have permission to trace
if [ "$(id -u)" -ne 0 ]; then
    echo "⚠️  This script needs sudo to trace system calls"
    echo ""
    echo "Restarting with sudo..."
    exec sudo "$0" "$@"
fi

echo "Starting trace (Ctrl+C to stop)..."
echo "Monitoring file access patterns for: .claude, hooks, settings.json"
echo ""
echo "=================================================="

# Trace file operations, filter for relevant paths
strace -e trace=openat,open,stat,lstat,access -f -p "$CLAUDE_PID" 2>&1 | \
    grep --line-buffered -E '\.claude|hooks|settings\.json|settings\.local\.json' | \
    while IFS= read -r line; do
        # Highlight key patterns
        if echo "$line" | grep -q "settings.json"; then
            echo "🔧 [SETTINGS] $line"
        elif echo "$line" | grep -q "hooks"; then
            echo "🪝 [HOOKS]    $line"
        elif echo "$line" | grep -q "\.claude"; then
            echo "📁 [CLAUDE]   $line"
        else
            echo "   $line"
        fi
    done
