# MCP 工具同步系统

## 问题背景

`mcp-server.json` 文件定义了 Intent-Engine MCP 服务器暴露给 AI 的工具列表。该文件需要与以下内容保持同步：

1. **版本号** - 与 `Cargo.toml` 中的包版本一致
2. **工具列表** - 与 `src/bin/mcp-server.rs` 中实现的处理函数一致
3. **工具参数** - 与 CLI 命令的实际参数匹配

手动维护容易出现不一致，导致：
- 版本号过时
- 工具定义与实现不匹配
- 参数变更后 JSON 未更新

## 自动化同步方案

我们实现了**三层防护机制**来确保同步：

### 1. 同步脚本 (立即可用)

**脚本**: `scripts/sync-mcp-tools.sh`

**功能**:
- 自动从 `Cargo.toml` 读取版本号
- 更新 `mcp-server.json` 的版本字段
- 检测版本不一致并提示

**使用方法**:
```bash
# 检查并同步版本
./scripts/sync-mcp-tools.sh

# 在发版前运行
make release-check  # 自动调用同步脚本
```

**集成点**:
- ✅ 发版 workflow 自动运行
- ✅ Pre-commit hook 可选集成
- ✅ CI 检查（如果版本不一致则失败）

### 2. 自动化测试 (CI 验证)

**测试文件**: `tests/mcp_tools_sync_test.rs`

**测试内容**:

#### 测试 1: 版本号同步
```rust
#[test]
fn test_mcp_version_matches_cargo_toml()
```
- 验证 `mcp-server.json` 版本 = `Cargo.toml` 版本
- 失败时提示运行同步脚本

#### 测试 2: 工具列表同步
```rust
#[test]
fn test_mcp_tools_match_handlers()
```
- 从 `mcp-server.json` 提取工具名称
- 从 `mcp-server.rs` 提取 handler 实现
- 检测双向不匹配：
  - JSON 中定义但代码未实现
  - 代码实现但 JSON 未定义

#### 测试 3: Schema 完整性
```rust
#[test]
fn test_mcp_tools_have_required_fields()
```
- 验证每个工具有 `name`, `description`, `inputSchema`
- 验证 `inputSchema` 结构正确

**运行方法**:
```bash
# 运行所有 MCP 同步测试
cargo test --test mcp_tools_sync_test

# 单独测试
cargo test mcp_version_matches_cargo_toml
cargo test mcp_tools_match_handlers
cargo test mcp_tools_have_required_fields
```

**CI 集成**:
- ✅ 每次 PR 自动运行
- ✅ 测试失败阻止合并
- ✅ 确保 main 分支始终同步

### 3. 开发工作流集成

#### Pre-commit Hook (可选)
```bash
# 安装 git hooks
./scripts/setup-git-hooks.sh

# 每次 commit 前自动检查版本同步
```

#### Release Checklist
在 `.github/workflows/release.yml` 中自动运行：
```yaml
- name: Sync MCP Tools
  run: ./scripts/sync-mcp-tools.sh

- name: Verify MCP Sync
  run: cargo test --test mcp_tools_sync_test
```

## 未来改进方向

### 短期 (已实现)
- ✅ 版本号自动同步
- ✅ 工具列表一致性测试
- ✅ CI 自动验证

### 中期 (计划中)
- 🔜 **参数验证**: 比对 CLI `--help` 输出与 JSON schema
- 🔜 **描述同步**: 从代码注释自动生成工具描述
- 🔜 **变更检测**: 检测 CLI 命令变更后提示更新 JSON

### 长期 (探索中)
- 💡 **代码生成方案**: 从单一定义生成 JSON + Handler
- 💡 **宏定义工具**: 使用 Rust 宏定义工具，自动生成 schema
- 💡 **完全自动化**: `build.rs` 编译时生成 `mcp-server.json`

## 长期方案: 代码生成

### 方案 A: 单一 YAML 定义
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

**优点**:
- 单一真实来源
- 易于阅读和维护
- 可生成 JSON + 类型定义

**缺点**:
- 需要额外的构建步骤
- 增加复杂度

### 方案 B: Rust 宏定义
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

**优点**:
- 保持在 Rust 代码中
- 编译时类型检查
- 自动生成 handler 骨架

**缺点**:
- 宏复杂度高
- 调试困难

### 方案 C: build.rs 动态生成
```rust
// build.rs
fn main() {
    // 读取 Cargo.toml 版本
    // 扫描 mcp-server.rs 中的工具定义
    // 生成 mcp-server.json
}
```

**优点**:
- 编译时自动生成
- 零运行时开销
- 保证同步

**缺点**:
- 增加编译复杂度
- 可能影响增量编译

## 推荐方案

### 当前阶段 (0.1.x)
使用**三层防护机制**（脚本 + 测试 + CI）:
- ✅ 简单有效
- ✅ 无额外复杂度
- ✅ 开发体验好

### 1.0 后考虑
如果工具数量大幅增加（>30），考虑**方案 B (Rust 宏)**:
- 保持类型安全
- 减少手动维护
- 更好的开发体验

## 维护指南

### 添加新工具时

1. **更新 `mcp-server.json`**:
   ```json
   {
     "name": "new_tool",
     "description": "...",
     "inputSchema": { ... }
   }
   ```

2. **实现 handler**:
   ```rust
   async fn handle_new_tool(args: Value) -> Result<Value, String> {
       // Implementation
   }
   ```

3. **注册到 dispatcher**:
   ```rust
   match params.name.as_str() {
       "new_tool" => handle_new_tool(params.arguments).await,
       // ...
   }
   ```

4. **运行测试验证**:
   ```bash
   cargo test --test mcp_tools_sync_test
   ```

### 版本发布时

1. 更新 `Cargo.toml` 版本
2. 运行同步脚本:
   ```bash
   ./scripts/sync-mcp-tools.sh
   ```
3. 提交更改
4. CI 自动验证

## 故障排查

### 测试失败: 版本不匹配
```bash
# 运行同步脚本
./scripts/sync-mcp-tools.sh

# 手动检查
grep version Cargo.toml
jq .version mcp-server.json
```

### 测试失败: 工具不一致
```bash
# 列出 JSON 中的工具
jq -r '.tools[].name' mcp-server.json | sort

# 列出代码中的 handler
grep -o '"[a-z_]*" => handle_' src/bin/mcp-server.rs | sed 's/" => handle_//' | sed 's/"//g' | sort

# 对比差异
diff <(jq -r '.tools[].name' mcp-server.json | sort) \
     <(grep -o '"[a-z_]*" => handle_' src/bin/mcp-server.rs | sed 's/" => handle_//' | sed 's/"//g' | grep -v "^tools/" | sort)
```

## 相关文件

- `mcp-server.json` - MCP 工具定义
- `src/bin/mcp-server.rs` - MCP 服务器实现
- `scripts/sync-mcp-tools.sh` - 版本同步脚本
- `tests/mcp_tools_sync_test.rs` - 同步验证测试
- `.github/workflows/ci.yml` - CI 配置

## 参考资料

- [MCP Protocol Specification](https://modelcontextprotocol.io/)
- [Rust MCP Server Implementation](../../../src/bin/mcp-server.rs)
- [Contributing Guide](../../contributing/contributing.md)
