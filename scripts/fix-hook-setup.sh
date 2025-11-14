#!/bin/bash
# Auto-fix Claude Code hook setup

set -euo pipefail

echo "🔧 Fixing Claude Code hook setup..."
echo ""

# 1. Create hook script
echo "Step 1: Creating hook script..."
ie setup --target claude-code --force
echo ""

# 2. Create settings.json with SessionStart hook
echo "Step 2: Creating settings.json..."
mkdir -p .claude

# Backup existing settings if present
if [ -f ".claude/settings.json" ]; then
    cp .claude/settings.json ".claude/settings.json.backup.$(date +%s)"
    echo "   ✓ Backed up existing settings.json"
fi

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

echo "   ✓ Created .claude/settings.json"
echo ""

# 3. Verify setup
echo "Step 3: Verifying..."
if [ -x ".claude/hooks/session-start.sh" ]; then
    echo "   ✅ Hook script is executable"
else
    echo "   ❌ Hook script missing or not executable"
    exit 1
fi

if grep -q "SessionStart" ".claude/settings.json"; then
    echo "   ✅ settings.json has SessionStart configuration"
else
    echo "   ❌ settings.json missing SessionStart"
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Hook setup complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Next steps:"
echo "1. Restart Claude Code completely"
echo "2. In Claude Code, run: /hooks"
echo "3. Verify SessionStart hook is listed"
echo ""
echo "Test manually:"
echo "  .claude/hooks/session-start.sh"
