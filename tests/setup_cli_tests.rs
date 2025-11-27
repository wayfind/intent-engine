/// Tests for `ie setup` CLI commands to improve main.rs coverage
/// Focuses on the handle_setup_diagnose() function
mod common;

use predicates::prelude::*;

// ============================================================================
// Setup Diagnose Tests
// ============================================================================

#[test]
fn test_setup_diagnose_claude_code() {
    let mut cmd = common::ie_command();
    cmd.arg("setup")
        .arg("--target")
        .arg("claude-code")
        .arg("--diagnose");

    cmd.assert().success().stdout(predicate::str::contains(
        "Setup Diagnosis for 'claude-code'",
    ));
}

#[test]
fn test_setup_diagnose_invalid_target() {
    let mut cmd = common::ie_command();
    cmd.arg("setup")
        .arg("--target")
        .arg("invalid-target")
        .arg("--diagnose");

    cmd.assert().failure().stderr(predicate::str::contains(
        "Diagnosis not supported for target",
    ));
}
