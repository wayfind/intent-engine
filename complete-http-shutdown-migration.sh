#!/bin/bash
# Complete HTTP Shutdown Migration Script
# This script completes the remaining phases of the HTTP shutdown implementation

set -e

echo "🚀 Starting HTTP Shutdown Migration - Phases 2-6"
echo

# Phase 2 & 3: Modify dashboard.rs - Remove PID, update stop/start commands
echo "📝 Phase 2 & 3: Updating CLI commands..."

# Create backup
cp src/cli_handlers/dashboard.rs src/cli_handlers/dashboard.rs.backup

# Remove PID import
sed -i '/^use crate::dashboard::pid;$/d' src/cli_handlers/dashboard.rs

# Add helper function for shutdown request (insert after imports)
cat > /tmp/shutdown_helper.rs << 'EOF'

/// Send HTTP shutdown request to Dashboard
async fn send_shutdown_request(port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{}/api/internal/shutdown", port);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| IntentError::OtherError(anyhow::anyhow!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|e| IntentError::OtherError(anyhow::anyhow!("Failed to send shutdown request: {}", e)))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(IntentError::OtherError(anyhow::anyhow!(
            "Shutdown request failed with status: {}",
            response.status()
        )))
    }
}
EOF

# Find the line number where to insert the helper (after imports, before functions)
INSERT_LINE=$(grep -n "^pub async fn handle_dashboard_command" src/cli_handlers/dashboard.rs | cut -d: -f1)
INSERT_LINE=$((INSERT_LINE - 1))

# Insert helper function
head -n "$INSERT_LINE" src/cli_handlers/dashboard.rs > /tmp/dashboard_new.rs
cat /tmp/shutdown_helper.rs >> /tmp/dashboard_new.rs
tail -n +"$((INSERT_LINE + 1))" src/cli_handlers/dashboard.rs >> /tmp/dashboard_new.rs
mv /tmp/dashboard_new.rs src/cli_handlers/dashboard.rs

echo "✅ Added shutdown helper function"

# Phase 4: Delete PID file and update mod.rs
echo "🗑️  Phase 4: Removing PID code..."

# Delete PID file
if [ -f "src/dashboard/pid.rs" ]; then
    rm src/dashboard/pid.rs
    echo "✅ Deleted src/dashboard/pid.rs"
fi

# Update dashboard/mod.rs to remove pid module
sed -i '/^pub mod pid;$/d' src/dashboard/mod.rs
sed -i '/^mod pid;$/d' src/dashboard/mod.rs
echo "✅ Updated src/dashboard/mod.rs"

# Phase 5: Format code
echo "🎨 Phase 5: Formatting code..."
cargo fmt
echo "✅ Code formatted"

# Phase 6: Verify compilation
echo "🧪 Phase 6: Verifying compilation..."
if cargo check --lib --quiet; then
    echo "✅ Compilation successful"
else
    echo "❌ Compilation failed - manual fixes needed"
    exit 1
fi

echo
echo "✅ Migration phases 2-6 automated steps complete!"
echo
echo "⚠️  MANUAL STEPS REQUIRED:"
echo
echo "1. Update Stop command in src/cli_handlers/dashboard.rs (around line 395):"
echo "   Replace PID-based stop with HTTP request"
echo
echo "2. Update Start command in src/cli_handlers/dashboard.rs (around line 338):"
echo "   Remove lines 341-356 (PID file checks)"
echo "   Remove all calls to:"
echo "   - pid::cleanup_stale_pid()"
echo "   - pid::read_pid()"
echo "   - pid::write_pid()"
echo "   - pid::kill_process()"
echo "   - pid::delete_pid_file()"
echo
echo "3. Run tests:"
echo "   cargo test --lib -- --test-threads=1"
echo
echo "4. Review changes:"
echo "   git diff src/cli_handlers/dashboard.rs"
echo
echo "📄 Detailed instructions in: HTTP_SHUTDOWN_IMPLEMENTATION_PLAN.md"
echo "📄 Backup created at: src/cli_handlers/dashboard.rs.backup"
