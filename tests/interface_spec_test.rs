/// Test to ensure spec-03-interface-current.md stays in sync with actual implementation
use std::fs;
use std::process::Command;

#[test]
fn test_spec_version_matches_cargo() {
    // Read Cargo.toml version
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let cargo_version = cargo_toml
        .lines()
        .find(|line| line.starts_with("version = "))
        .and_then(|line| line.split('"').nth(1))
        .expect("Failed to extract version from Cargo.toml");

    // Read spec-03-interface-current.md
    let spec = fs::read_to_string("docs/spec-03-interface-current.md")
        .expect("Failed to read spec-03-interface-current.md");

    // Extract version from spec (first occurrence of "**Version**: X.Y")
    let spec_version = spec
        .lines()
        .find(|line| line.starts_with("**Version**:"))
        .and_then(|line| line.split(':').nth(1))
        .map(|v| v.trim())
        .expect("Failed to extract version from spec-03-interface-current.md");

    // Extract major.minor from Cargo.toml version (e.g., "0.1.12" -> "0.1")
    let cargo_minor = cargo_version
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");

    assert_eq!(
        cargo_minor, spec_version,
        "\nInterface version mismatch!\n  Cargo.toml (major.minor): {}\n  spec-03-interface-current.md: {}\n\nspec-03-interface-current.md should reflect interface contract version (major.minor), not patch version.\nOnly update when interface changes (breaking or feature changes).",
        cargo_minor, spec_version
    );
}

// Note: MCP tools test removed in v0.10.0 - MCP server has been replaced with system prompt approach

#[test]
fn test_spec_documents_cli_commands() {
    let spec = fs::read_to_string("docs/spec-03-interface-current.md")
        .expect("Failed to read spec-03-interface-current.md");

    // Core CLI commands for v0.10.0 (simplified structure)
    // Only check the 3 main commands that must be in spec
    let required_commands = vec![
        "plan",   // Declarative task creation/update
        "log",    // Quick event logging
        "search", // Unified search across tasks and events
    ];

    // Optional utility commands (may not be in spec yet)
    // "init", "dashboard", "doctor"

    for cmd in required_commands {
        assert!(
            spec.contains(&format!("`{}`", cmd))
                || spec.contains(&format!("#### `{}`", cmd))
                || spec.contains(&format!("`ie {}`", cmd)),
            "Command '{}' is not documented in spec-03-interface-current.md",
            cmd
        );
    }

    println!("✅ All core CLI commands are documented");
}

#[test]
fn test_cli_help_matches_spec() {
    // Test new CLI structure (v0.10.0): plan, log, search
    let bin_path = env!("CARGO_BIN_EXE_ie");

    // Test 'plan' command help
    let output = Command::new(bin_path)
        .args(["plan", "--help"])
        .output()
        .expect("Failed to run plan --help");

    let help_text = String::from_utf8_lossy(&output.stdout);

    // Verify plan command documents format parameter
    assert!(
        help_text.contains("--format") || help_text.contains("format"),
        "plan --help should document format parameter"
    );

    // Test 'log' command help
    let output = Command::new(bin_path)
        .args(["log", "--help"])
        .output()
        .expect("Failed to run log --help");

    let help_text = String::from_utf8_lossy(&output.stdout);

    assert!(
        help_text.contains("event_type")
            || help_text.contains("decision")
            || help_text.contains("blocker"),
        "log --help should document event types"
    );
    assert!(
        help_text.contains("message"),
        "log --help should document message parameter"
    );
    assert!(
        help_text.contains("--task") || help_text.contains("task"),
        "log --help should document task parameter"
    );

    // Test 'search' command help
    let output = Command::new(bin_path)
        .args(["search", "--help"])
        .output()
        .expect("Failed to run search --help");

    let help_text = String::from_utf8_lossy(&output.stdout);

    assert!(
        help_text.contains("query"),
        "search --help should document query parameter"
    );
    assert!(
        help_text.contains("--tasks") || help_text.contains("tasks"),
        "search --help should document tasks flag"
    );
    assert!(
        help_text.contains("--events") || help_text.contains("events"),
        "search --help should document events flag"
    );

    println!("✅ CLI help output contains documented parameters for new command structure");
}

#[test]
fn test_spec_data_model_matches_schema() {
    let spec = fs::read_to_string("docs/spec-03-interface-current.md")
        .expect("Failed to read spec-03-interface-current.md");

    // Check that data model section exists and contains key fields
    assert!(
        spec.contains("### 1.1 Data Model"),
        "Spec should have Data Model section"
    );

    // Core Task fields (based on actual database schema)
    let task_fields = vec![
        "id:",
        "name:",
        "spec:",
        "status:",
        "complexity:",
        "priority:",
        "parent_id:",
        "first_todo_at:",
        "first_doing_at:",
        "first_done_at:",
    ];

    for field in task_fields {
        assert!(
            spec.contains(field),
            "Data Model should document Task field: {}",
            field
        );
    }

    // Core Event fields (based on actual database schema)
    let event_fields = vec![
        "id:",
        "task_id:",
        "timestamp:",
        "log_type:",
        "discussion_data:",
    ];

    for field in event_fields {
        assert!(
            spec.contains(field),
            "Data Model should document Event field: {}",
            field
        );
    }

    println!("✅ Data model documentation is complete");
}

#[test]
fn test_spec_has_version_guarantees() {
    let spec = fs::read_to_string("docs/spec-03-interface-current.md")
        .expect("Failed to read spec-03-interface-current.md");

    // Check that spec includes stability guarantees
    assert!(
        spec.contains("## 6. Interface Guarantees")
            || spec.contains("Semantic Versioning")
            || spec.contains("SemVer"),
        "Spec should document interface stability guarantees"
    );

    assert!(
        spec.contains("Stability Guarantees") || spec.contains("Experimental"),
        "Spec should document current stability status"
    );

    println!("✅ Interface guarantees are documented");
}
