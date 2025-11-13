/// Test to ensure INTERFACE_SPEC.md stays in sync with actual implementation
use serde_json::Value;
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

    // Read INTERFACE_SPEC.md
    let spec =
        fs::read_to_string("docs/INTERFACE_SPEC.md").expect("Failed to read INTERFACE_SPEC.md");

    // Extract version from spec (first occurrence of "**Version**: X.Y")
    let spec_version = spec
        .lines()
        .find(|line| line.starts_with("**Version**:"))
        .and_then(|line| line.split(':').nth(1))
        .map(|v| v.trim())
        .expect("Failed to extract version from INTERFACE_SPEC.md");

    // Extract major.minor from Cargo.toml version (e.g., "0.1.12" -> "0.1")
    let cargo_minor = cargo_version
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");

    assert_eq!(
        cargo_minor, spec_version,
        "\nInterface version mismatch!\n  Cargo.toml (major.minor): {}\n  INTERFACE_SPEC.md: {}\n\nINTERFACE_SPEC.md should reflect interface contract version (major.minor), not patch version.\nOnly update when interface changes (breaking or feature changes).",
        cargo_minor, spec_version
    );
}

#[test]
fn test_spec_lists_all_mcp_tools() {
    // Read mcp-server.json
    let mcp_json = fs::read_to_string("mcp-server.json").expect("Failed to read mcp-server.json");
    let mcp_config: Value =
        serde_json::from_str(&mcp_json).expect("Failed to parse mcp-server.json");

    // Extract tool names
    let tools: Vec<String> = mcp_config["tools"]
        .as_array()
        .expect("tools is not an array")
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .expect("tool name is not a string")
                .to_string()
        })
        .collect();

    // Read INTERFACE_SPEC.md
    let spec =
        fs::read_to_string("docs/INTERFACE_SPEC.md").expect("Failed to read INTERFACE_SPEC.md");

    // Check that each tool is documented in the spec
    for tool in &tools {
        assert!(
            spec.contains(&format!("`{}`", tool)),
            "Tool '{}' is missing from INTERFACE_SPEC.md MCP tools table",
            tool
        );
    }

    println!("✅ All {} MCP tools are documented in spec", tools.len());
}

#[test]
fn test_spec_documents_cli_commands() {
    let spec =
        fs::read_to_string("docs/INTERFACE_SPEC.md").expect("Failed to read INTERFACE_SPEC.md");

    // Core CLI commands that must be documented
    let required_commands = vec![
        "task add",
        "task start",
        "task done",
        "task spawn-subtask",
        "task pick-next",
        "task find",
        "task search",
        "event add",
        "event list",
        "report",
        "current",
    ];

    for cmd in required_commands {
        assert!(
            spec.contains(&format!("`{}`", cmd)) || spec.contains(&format!("#### `{}`", cmd)),
            "Command '{}' is not documented in INTERFACE_SPEC.md",
            cmd
        );
    }

    println!("✅ All core CLI commands are documented");
}

#[test]
fn test_cli_help_matches_spec() {
    // Test that `task add --help` contains key parameters
    // Use pre-compiled binary instead of cargo run for speed
    let bin_path = env!("CARGO_BIN_EXE_intent-engine");
    let output = Command::new(bin_path)
        .args(["task", "add", "--help"])
        .output()
        .expect("Failed to run task add --help");

    let help_text = String::from_utf8_lossy(&output.stdout);

    // Check for essential parameters (flexible on exact names)
    assert!(
        help_text.contains("--name"),
        "task add --help should document name parameter"
    );
    assert!(
        help_text.contains("--spec") || help_text.contains("spec"),
        "task add --help should document spec parameter"
    );
    assert!(
        help_text.contains("--parent") || help_text.contains("parent"),
        "task add --help should document parent parameter"
    );

    println!("✅ CLI help output contains documented parameters");
}

#[test]
fn test_spec_data_model_matches_schema() {
    let spec =
        fs::read_to_string("docs/INTERFACE_SPEC.md").expect("Failed to read INTERFACE_SPEC.md");

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
    let spec =
        fs::read_to_string("docs/INTERFACE_SPEC.md").expect("Failed to read INTERFACE_SPEC.md");

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
