/// Test to ensure mcp-server.json stays in sync with:
/// 1. Cargo.toml version
/// 2. MCP server implementation (tool handlers)
use serde_json::Value;
use std::collections::HashSet;
use std::fs;

#[test]
fn test_mcp_version_matches_cargo_toml() {
    // Read Cargo.toml version
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let cargo_version = cargo_toml
        .lines()
        .find(|line| line.starts_with("version = "))
        .and_then(|line| line.split('"').nth(1))
        .expect("Failed to extract version from Cargo.toml");

    // Extract major.minor from Cargo.toml (e.g., "0.1.12" -> "0.1")
    let cargo_minor = cargo_version
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");

    // Read mcp-server.json version
    let mcp_json = fs::read_to_string("mcp-server.json").expect("Failed to read mcp-server.json");
    let mcp_config: Value =
        serde_json::from_str(&mcp_json).expect("Failed to parse mcp-server.json");
    let mcp_version = mcp_config["version"]
        .as_str()
        .expect("Failed to extract version from mcp-server.json");

    assert_eq!(
        cargo_minor, mcp_version,
        "\nMCP version mismatch!\n  Cargo.toml (major.minor): {}\n  mcp-server.json: {}\n\nmcp-server.json should use interface version (major.minor), not full version.\nRun: ./scripts/sync-mcp-tools.sh",
        cargo_minor, mcp_version
    );
}

#[test]
fn test_mcp_tools_match_handlers() {
    // Read mcp-server.json
    let mcp_json = fs::read_to_string("mcp-server.json").expect("Failed to read mcp-server.json");
    let mcp_config: Value =
        serde_json::from_str(&mcp_json).expect("Failed to parse mcp-server.json");

    // Extract tool names from mcp-server.json
    let json_tools: HashSet<String> = mcp_config["tools"]
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

    // Read mcp server implementation to find handler implementations
    let mcp_server_rs =
        fs::read_to_string("src/mcp/server.rs").expect("Failed to read mcp/server.rs");

    // Extract tool names from handle_tool_call match statement
    // Exclude MCP protocol methods (tools/list, tools/call)
    let code_tools: HashSet<String> = mcp_server_rs
        .lines()
        .filter(|line| line.trim().starts_with('"') && line.contains("=> handle_"))
        .map(|line| {
            line.trim()
                .split('"')
                .nth(1)
                .expect("Failed to extract tool name")
                .to_string()
        })
        .filter(|name| !name.starts_with("tools/")) // Exclude protocol methods
        .collect();

    // Check for tools in JSON but not in code
    let json_only: Vec<_> = json_tools.difference(&code_tools).collect();
    if !json_only.is_empty() {
        panic!(
            "Tools defined in mcp-server.json but missing handlers in code:\n  {:?}",
            json_only
        );
    }

    // Check for tools in code but not in JSON
    let code_only: Vec<_> = code_tools.difference(&json_tools).collect();
    if !code_only.is_empty() {
        panic!(
            "Tools implemented in code but missing from mcp-server.json:\n  {:?}",
            code_only
        );
    }

    println!("✅ All {} tools are in sync!", json_tools.len());
}

#[test]
fn test_mcp_tools_have_required_fields() {
    let mcp_json = fs::read_to_string("mcp-server.json").expect("Failed to read mcp-server.json");
    let mcp_config: Value =
        serde_json::from_str(&mcp_json).expect("Failed to parse mcp-server.json");

    let tools = mcp_config["tools"]
        .as_array()
        .expect("tools is not an array");

    for tool in tools {
        let name = tool["name"].as_str().expect("Tool missing 'name' field");

        // Check required fields
        assert!(
            tool.get("description").is_some(),
            "Tool '{}' missing 'description' field",
            name
        );
        assert!(
            tool.get("inputSchema").is_some(),
            "Tool '{}' missing 'inputSchema' field",
            name
        );

        // Validate inputSchema structure
        let schema = &tool["inputSchema"];
        assert_eq!(
            schema["type"].as_str(),
            Some("object"),
            "Tool '{}' inputSchema type should be 'object'",
            name
        );
        assert!(
            schema.get("properties").is_some(),
            "Tool '{}' inputSchema missing 'properties'",
            name
        );
    }

    println!("✅ All tools have valid schema structure!");
}
