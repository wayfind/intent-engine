#!/bin/bash
# Debug Claude Code hook setup
# Checks configuration and provides fix suggestions

set -euo pipefail

echo "🐛 Claude Code Hook Setup Debugger"
echo "===================================="
echo ""

PROJECT_DIR="$(pwd)"
HOOK_SCRIPT=".claude/hooks/session-start.sh"
PROJECT_SETTINGS=".claude/settings.json"
PROJECT_LOCAL_SETTINGS=".claude/settings.local.json"
USER_SETTINGS="$HOME/.claude/settings.json"

echo "📍 Project: $PROJECT_DIR"
echo ""

# Check 1: Hook script exists
echo "━━━ Check 1: Hook Script ━━━"
if [ -f "$HOOK_SCRIPT" ]; then
    echo "✅ Hook script exists: $HOOK_SCRIPT"

    # Check executable permission
    if [ -x "$HOOK_SCRIPT" ]; then
        echo "✅ Hook script is executable"
    else
        echo "❌ Hook script is NOT executable"
        echo "   Fix: chmod +x $HOOK_SCRIPT"
    fi

    # Check shebang
    FIRST_LINE=$(head -1 "$HOOK_SCRIPT")
    if [[ "$FIRST_LINE" == "#!/bin/bash"* ]]; then
        echo "✅ Valid shebang: $FIRST_LINE"
    else
        echo "⚠️  Unexpected shebang: $FIRST_LINE"
    fi
else
    echo "❌ Hook script NOT found: $HOOK_SCRIPT"
    echo "   Run: ie setup --target claude-code"
fi
echo ""

# Check 2: Settings files
echo "━━━ Check 2: Settings Files ━━━"

check_hook_config() {
    local file=$1
    local label=$2

    if [ -f "$file" ]; then
        echo "✅ $label exists: $file"

        if grep -q "SessionStart" "$file" 2>/dev/null; then
            echo "   ✅ Contains SessionStart hook configuration"
            echo "   📄 Configuration:"
            jq '.hooks.SessionStart' "$file" 2>/dev/null | sed 's/^/      /' || echo "      (Failed to parse JSON)"
        else
            echo "   ❌ Missing SessionStart hook configuration"
        fi
    else
        echo "⚠️  $label NOT found: $file"
    fi
    echo ""
}

check_hook_config "$PROJECT_SETTINGS" "Project settings"
check_hook_config "$PROJECT_LOCAL_SETTINGS" "Project local settings"
check_hook_config "$USER_SETTINGS" "User settings"

# Check 3: Expected configuration
echo "━━━ Check 3: Required Configuration ━━━"
echo "Claude Code looks for hooks in settings.json files."
echo "The SessionStart hook should be configured like this:"
echo ""
cat << 'EOF'
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/session-start.sh"
          }
        ]
      }
    ]
  }
}
EOF
echo ""

# Check 4: Provide fix command
echo "━━━ Recommended Fix ━━━"

NEEDS_FIX=false

if [ ! -f "$HOOK_SCRIPT" ]; then
    echo "1. Create hook script:"
    echo "   ie setup --target claude-code"
    NEEDS_FIX=true
fi

if [ ! -f "$PROJECT_SETTINGS" ] || ! grep -q "SessionStart" "$PROJECT_SETTINGS" 2>/dev/null; then
    if [ ! -f "$PROJECT_LOCAL_SETTINGS" ] || ! grep -q "SessionStart" "$PROJECT_LOCAL_SETTINGS" 2>/dev/null; then
        echo "2. Add SessionStart configuration to .claude/settings.json:"
        echo ""
        cat << 'EOF'
cat > .claude/settings.json << 'JSON'
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/session-start.sh"
          }
        ]
      }
    ]
  }
}
JSON
EOF
        echo ""
        echo "   Or use the auto-fix script below ⬇️"
        NEEDS_FIX=true
    fi
fi

if [ "$NEEDS_FIX" = false ]; then
    echo "✅ Configuration looks good!"
    echo ""
    echo "If hook still doesn't work:"
    echo "1. Restart Claude Code completely"
    echo "2. Check Claude Code version (requires v2.0+)"
    echo "3. Use /hooks command in Claude Code to verify"
else
    echo ""
    echo "━━━ Auto-Fix Script ━━━"
    cat << 'EOF'
#!/bin/bash
# Quick fix for hook configuration

# 1. Create hook script
ie setup --target claude-code

# 2. Create settings.json with SessionStart hook
mkdir -p .claude
cat > .claude/settings.json << 'JSON'
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/session-start.sh"
          }
        ]
      }
    ]
  }
}
JSON

echo "✅ Hook configuration fixed!"
echo "   Restart Claude Code to apply changes"
EOF
fi

echo ""
echo "━━━ Additional Debugging ━━━"
echo "To manually test the hook:"
echo "  cd $PROJECT_DIR"
echo "  .claude/hooks/session-start.sh"
echo ""
echo "To monitor Claude Code file access:"
echo "  ./scripts/watch-claude-access.sh"
