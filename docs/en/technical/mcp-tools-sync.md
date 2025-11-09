# MCP Tools Synchronization System

## Background

The `mcp-server.json` file defines the tool list exposed by Intent-Engine MCP server to AI. This file needs to stay in sync with:

1. **Version Number** - Match package version in `Cargo.toml`
2. **Tool List** - Match handler functions in `src/bin/mcp-server.rs`
3. **Tool Parameters** - Match actual CLI command parameters

Manual maintenance is error-prone, leading to:
- Outdated version numbers
- Tool definitions not matching implementations
- Stale parameters after CLI changes

## Automated Synchronization Solution

We implement a **three-layer defense mechanism** to ensure synchronization:

### 1. Sync Script (Ready to Use)

**Script**: `scripts/sync-mcp-tools.sh`

**Features**:
- Automatically read version from `Cargo.toml`
- Update version field in `mcp-server.json`
- Detect version mismatches and alert

**Usage**:
```bash
# Check and sync version
./scripts/sync-mcp-tools.sh

# Run before release
make release-check  # Automatically calls sync script
```

**Integration Points**:
- ✅ Auto-run in release workflow
- ✅ Optional pre-commit hook integration
- ✅ CI check (fail if version mismatch)

### 2. Automated Tests (CI Validation)

**Test File**: `tests/mcp_tools_sync_test.rs`

**Test Coverage**:

#### Test 1: Version Sync
```rust
#[test]
fn test_mcp_version_matches_cargo_toml()
```
- Verify `mcp-server.json` version = `Cargo.toml` version
- Prompt to run sync script on failure

#### Test 2: Tool List Sync
```rust
#[test]
fn test_mcp_tools_match_handlers()
```
- Extract tool names from `mcp-server.json`
- Extract handler implementations from `mcp-server.rs`
- Detect bidirectional mismatches:
  - Defined in JSON but not implemented in code
  - Implemented in code but not defined in JSON

#### Test 3: Schema Completeness
```rust
#[test]
fn test_mcp_tools_have_required_fields()
```
- Verify each tool has `name`, `description`, `inputSchema`
- Validate `inputSchema` structure

**Run Methods**:
```bash
# Run all MCP sync tests
cargo test --test mcp_tools_sync_test

# Run individual tests
cargo test mcp_version_matches_cargo_toml
cargo test mcp_tools_match_handlers
cargo test mcp_tools_have_required_fields
```

**CI Integration**:
- ✅ Auto-run on every PR
- ✅ Test failures block merges
- ✅ Ensure main branch stays in sync

### 3. Development Workflow Integration

#### Pre-commit Hook (Optional)
```bash
# Install git hooks
./scripts/setup-git-hooks.sh

# Auto-check version sync before each commit
```

#### Release Checklist
Auto-run in `.github/workflows/release.yml`:
```yaml
- name: Sync MCP Tools
  run: ./scripts/sync-mcp-tools.sh

- name: Verify MCP Sync
  run: cargo test --test mcp_tools_sync_test
```

## Future Improvements

### Short-term (Implemented)
- ✅ Automatic version sync
- ✅ Tool list consistency tests
- ✅ CI auto-validation

### Mid-term (Planned)
- 🔜 **Parameter Validation**: Compare CLI `--help` output with JSON schema
- 🔜 **Description Sync**: Auto-generate tool descriptions from code comments
- 🔜 **Change Detection**: Alert to update JSON after CLI command changes

### Long-term (Exploring)
- 💡 **Code Generation**: Generate JSON + Handler from single definition
- 💡 **Macro-based Tools**: Use Rust macros to define tools, auto-generate schema
- 💡 **Full Automation**: Generate `mcp-server.json` at compile time via `build.rs`

## Long-term Solution: Code Generation

### Option A: Single YAML Definition
```yaml
# tools.yaml
version: "${CARGO_VERSION}"
tools:
  - name: task_add
    description: "Create a new task..."
    params:
      - name: name
        type: string
        required: true
      - name: spec
        type: string
        required: false
```

**Pros**:
- Single source of truth
- Easy to read and maintain
- Can generate JSON + type definitions

**Cons**:
- Requires additional build step
- Increases complexity

### Option B: Rust Macro Definition
```rust
define_mcp_tool! {
    name: "task_add",
    description: "Create a new task...",
    handler: handle_task_add,
    params: {
        name: String (required),
        spec: Option<String>,
    }
}
```

**Pros**:
- Stays in Rust code
- Compile-time type checking
- Auto-generate handler skeleton

**Cons**:
- High macro complexity
- Difficult to debug

### Option C: build.rs Dynamic Generation
```rust
// build.rs
fn main() {
    // Read Cargo.toml version
    // Scan tool definitions in mcp-server.rs
    // Generate mcp-server.json
}
```

**Pros**:
- Auto-generate at compile time
- Zero runtime overhead
- Guaranteed sync

**Cons**:
- Increases build complexity
- May affect incremental compilation

## Recommended Approach

### Current Stage (0.1.x)
Use **three-layer defense mechanism** (script + tests + CI):
- ✅ Simple and effective
- ✅ No extra complexity
- ✅ Good developer experience

### Post 1.0
If tool count grows significantly (>30), consider **Option B (Rust Macros)**:
- Maintain type safety
- Reduce manual maintenance
- Better developer experience

## Maintenance Guide

### When Adding New Tools

1. **Update `mcp-server.json`**:
   ```json
   {
     "name": "new_tool",
     "description": "...",
     "inputSchema": { ... }
   }
   ```

2. **Implement handler**:
   ```rust
   async fn handle_new_tool(args: Value) -> Result<Value, String> {
       // Implementation
   }
   ```

3. **Register in dispatcher**:
   ```rust
   match params.name.as_str() {
       "new_tool" => handle_new_tool(params.arguments).await,
       // ...
   }
   ```

4. **Run tests to verify**:
   ```bash
   cargo test --test mcp_tools_sync_test
   ```

### When Releasing Version

1. Update `Cargo.toml` version
2. Run sync script:
   ```bash
   ./scripts/sync-mcp-tools.sh
   ```
3. Commit changes
4. CI auto-validates

## Troubleshooting

### Test Failure: Version Mismatch
```bash
# Run sync script
./scripts/sync-mcp-tools.sh

# Manual check
grep version Cargo.toml
jq .version mcp-server.json
```

### Test Failure: Tool Inconsistency
```bash
# List tools in JSON
jq -r '.tools[].name' mcp-server.json | sort

# List handlers in code
grep -o '"[a-z_]*" => handle_' src/bin/mcp-server.rs | sed 's/" => handle_//' | sed 's/"//g' | sort

# Compare differences
diff <(jq -r '.tools[].name' mcp-server.json | sort) \
     <(grep -o '"[a-z_]*" => handle_' src/bin/mcp-server.rs | sed 's/" => handle_//' | sed 's/"//g' | grep -v "^tools/" | sort)
```

## Related Files

- `mcp-server.json` - MCP tool definitions
- `src/bin/mcp-server.rs` - MCP server implementation
- `scripts/sync-mcp-tools.sh` - Version sync script
- `tests/mcp_tools_sync_test.rs` - Sync validation tests
- `.github/workflows/ci.yml` - CI configuration

## References

- [MCP Protocol Specification](https://modelcontextprotocol.io/)
- [Rust MCP Server Implementation](../../../src/bin/mcp-server.rs)
- [Contributing Guide](../../contributing/contributing.md)
