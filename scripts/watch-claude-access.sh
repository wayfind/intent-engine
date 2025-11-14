#!/bin/bash
# Watch .claude directory access patterns
# Requires: inotify-tools (apt install inotify-tools)

set -euo pipefail

echo "👀 Claude Directory Access Monitor"
echo "==================================="
echo ""

# Check if inotify-tools is installed
if ! command -v inotifywait &> /dev/null; then
    echo "❌ inotifywait not found"
    echo ""
    echo "Install with:"
    echo "  sudo apt-get install inotify-tools"
    echo ""
    echo "Alternative: Use manual file monitoring:"
    echo "  watch -n 1 'ls -la .claude/'"
    exit 1
fi

# Create .claude directory if it doesn't exist
if [ ! -d ".claude" ]; then
    echo "⚠️  .claude directory not found in current directory"
    echo "Creating .claude for testing..."
    mkdir -p .claude
fi

echo "✓ Monitoring: $(pwd)/.claude/"
echo "✓ Also monitoring: ~/.claude/"
echo ""
echo "Waiting for file access events (Ctrl+C to stop)..."
echo "=================================================="
echo ""

# Monitor both project and user .claude directories
inotifywait -m -r -e access,open,modify,create,delete \
    --format '%T %e %w%f' \
    --timefmt '%H:%M:%S' \
    .claude/ ~/.claude/ 2>/dev/null | \
    while read -r timestamp event filepath; do
        # Color code different event types
        case "$event" in
            *ACCESS*)
                echo "🔍 [$timestamp] READ:   $filepath"
                ;;
            *OPEN*)
                echo "📖 [$timestamp] OPEN:   $filepath"
                ;;
            *MODIFY*)
                echo "✏️  [$timestamp] MODIFY: $filepath"
                ;;
            *CREATE*)
                echo "➕ [$timestamp] CREATE: $filepath"
                ;;
            *DELETE*)
                echo "🗑️  [$timestamp] DELETE: $filepath"
                ;;
            *)
                echo "   [$timestamp] $event: $filepath"
                ;;
        esac
    done
