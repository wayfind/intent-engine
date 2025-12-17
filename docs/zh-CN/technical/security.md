# 特殊字符和边界情况处理

本文档说明 Intent-Engine 对各种特殊字符、Unicode、极端输入的处理能力。

## 测试覆盖概览

Intent-Engine 经过全面测试，验证了对以下输入的正确处理：

- ✅ SQL 注入防护
- ✅ Unicode 字符（中文、日文、阿拉伯文等）
- ✅ Emoji 表情符号
- ✅ JSON 特殊字符
- ✅ 控制字符（换行、制表符等）
- ✅ 极长输入（10,000+ 字符）
- ✅ 边界情况（空字符串、纯空格等）
- ✅ Shell 元字符
- ✅ Markdown/HTML 标签
- ✅ URL 和路径

## 安全性保证

### SQL 注入防护 ✅

Intent-Engine 使用参数化查询（prepared statements），完全防止 SQL 注入攻击。

**测试案例**：
```rust
// 尝试 SQL 注入
let malicious = "Task'; DROP TABLE tasks; --";
task_mgr.add_task(malicious, None, None).await.unwrap();

// ✅ 结果：恶意代码被当作普通字符串处理，表未被删除
```

**验证**：
- ✅ 单引号注入
- ✅ UNION SELECT 注入
- ✅ 注释符 `--` 和 `/**/`
- ✅ 事件数据中的 SQL 命令

## Unicode 支持

### 多语言字符 ✅

完全支持 Unicode 字符，包括各种语言：

```rust
// 中文
"实现用户认证功能"

// 日文
"タスクを実装する"

// 阿拉伯文
"تنفيذ المهمة"

// 混合语言
"实现 authentication 認証 مصادقة"
```

**验证**：
- ✅ 中文字符存储和检索
- ✅ 日文字符存储和检索
- ✅ 阿拉伯文（RTL）字符
- ✅ 混合语言内容

### Emoji 支持 ✅

完全支持 Emoji 表情符号，包括复合 emoji：

```rust
// 简单 emoji
"🚀 Deploy to production 🎉"

// 复合 emoji
"👨‍👩‍👧‍👦 Family task 🏳️‍🌈 🇺🇸"
```

**验证**：
- ✅ 基本 emoji（🚀🎉💻）
- ✅ 复合 emoji 序列（👨‍👩‍👧‍👦）
- ✅ 国旗 emoji（🇺🇸）
- ✅ 变体选择器（🏳️‍🌈）

## JSON 特殊字符

### 引号和转义 ✅

正确处理 JSON 中需要转义的字符：

```rust
// 双引号
r#"Task with "quoted" text"#

// 反斜杠
r"C:\Users\test\path"

// 控制字符
"Task\nwith\nnewlines\tand\ttabs"
```

**JSON 输出**：
```json
{
  "name": "Task with \"quoted\" text"
}
```

**验证**：
- ✅ 双引号正确转义为 `\"`
- ✅ 反斜杠正确转义为 `\\`
- ✅ 换行符转义为 `\n`
- ✅ 制表符转义为 `\t`

### Null 字节处理 ⚠️

SQLite 不支持文本中的 null 字节（`\0`）。系统会：
- 选项 1: 拒绝包含 null 字节的输入
- 选项 2: 自动移除 null 字节

**建议**：避免在输入中使用 null 字节。

## 控制字符

### 多行内容 ✅

完全支持多行文本：

```rust
let multiline_spec = r#"# Task Specification

## Requirements
1. Feature A
2. Feature B

## Notes
- Important detail
"#;

task_mgr.add_task("Task", Some(multiline_spec), None).await
```

**验证**：
- ✅ 换行符（`\n`）
- ✅ 回车换行（`\r\n`）
- ✅ 制表符（`\t`）
- ✅ 多个连续空格

## 极端长度

### 超长输入 ✅

系统支持极长的输入：

| 字段 | 测试长度 | 状态 | 说明 |
|------|---------|------|------|
| 任务名称 | 10,000 字符 | ✅ | 无限制 |
| 规格说明 | 35,000 字符 | ✅ | 无限制 |
| 事件数据 | 120,000 字符 | ✅ | 无限制 |

**性能**：
- 10,000 字符任务名：正常存储和检索
- 超长文本不影响查询性能
- JSON 序列化正常工作

## 边界情况

### 空和极小输入 ✅

```rust
// 空字符串（允许）
task_mgr.add_task("", None, None).await.unwrap()

// 纯空格（允许）
task_mgr.add_task("     ", None, None).await.unwrap()

// 单字符
task_mgr.add_task("A", None, None).await.unwrap()
```

**验证**：
- ✅ 空任务名（允许但不推荐）
- ✅ 纯空格任务名
- ✅ 单字符任务名
- ✅ 空规格说明
- ✅ 空事件数据

## 特殊符号组合

### Shell 元字符 ✅

安全处理 Shell 命令中的特殊字符：

```rust
"Task && echo 'test' | grep -v 'bad' > /dev/null"
```

**验证**：
- ✅ 管道 `|`
- ✅ 重定向 `>` `<`
- ✅ 逻辑运算符 `&&` `||`
- ✅ 命令替换 `` `command` ``

### Markdown/HTML ✅

```rust
// Markdown
"# Task **bold** *italic* `code`"

// HTML
"<script>alert('xss')</script>"
```

**注意**：系统不过滤或转义这些字符，原样存储。客户端负责安全渲染。

### 正则表达式元字符 ✅

```rust
r"Task.*[0-9]+\d{3}(test|prod)$"
```

所有正则元字符都被正确存储和检索。

### URL 和路径 ✅

```rust
// URL with query parameters
"Deploy to https://example.com/api?key=value&test=1"

// Windows path
r"C:\Users\test\Documents\file.txt"

// Unix path
"/home/user/project/file.txt"
```

## FTS5 全文搜索限制

### 英文搜索 ✅

对英文内容的全文搜索工作完美：

```rust
task: "Implement authentication feature"
search: "authentication" // ✅ 找到
```

### CJK 语言限制 ⚠️

SQLite FTS5 的 unicode61 tokenizer 对中日韩（CJK）语言的分词支持有限：

```rust
task: "实现用户认证功能"
search: "认证" // ⚠️ 可能无法找到（需要完整匹配）
search: "实现用户认证功能" // ✅ 可以找到（完整匹配）
```

**建议**：
- 对 CJK 内容使用完整短语搜索
- 考虑使用任务名称前缀的英文关键词
- 对中文任务使用非 FTS 的标准过滤

**改进方向**：
未来可考虑集成专用的 CJK 分词器（如 jieba、mecab 等）。

## CLI 特殊字符处理

### Shell 引号 ✅

在命令行中使用引号保护特殊字符：

```bash
# 正确
ie task add --name "Task with spaces"
ie task add --name 'Task with "quotes"'

# Unicode
ie task add --name "实现功能"

# Emoji
ie task add --name "🚀 Deploy"
```

### stdin 输入 ✅

复杂内容通过 stdin 传递：

```bash
echo "Multi-line\nspecification\nwith special chars" | \
  ie task add --name "Task" --spec-stdin
```

## 测试覆盖统计

### 单元测试

- **特殊字符测试**: 37 个测试
  - SQL 注入: 4 个测试
  - Unicode/Emoji: 7 个测试
  - JSON 特殊字符: 4 个测试
  - 控制字符: 4 个测试
  - 极端长度: 3 个测试
  - 边界情况: 5 个测试
  - 特殊符号: 7 个测试
  - FTS5 搜索: 3 个测试

### CLI 集成测试

- **CLI 特殊字符测试**: 10 个测试
  - Unicode 和 Emoji 通过 CLI
  - 多行和引号处理
  - 极长输入
  - 特殊符号组合

## 最佳实践

### 对开发者

1. **永远使用参数化查询** - 已内置，无需额外操作
2. **不要过滤用户输入** - 保持原始输入完整性
3. **依赖 JSON 序列化** - serde_json 自动处理转义

### 对用户

1. **Shell 引号使用**
   ```bash
   # 单引号保护大部分特殊字符
   ie task add --name 'Task with $var'

   # 双引号允许变量展开
   ie task add --name "Task for $USER"
   ```

2. **复杂内容使用 stdin**
   ```bash
   cat spec.md | ie task add --name "Task" --spec-stdin
   ```

3. **CJK 搜索提示**
   - 使用完整短语而非单个词
   - 考虑添加英文关键词

## 安全声明

Intent-Engine 的安全特性：

✅ **SQL 注入**: 完全防护（参数化查询）
✅ **命令注入**: 不执行外部命令，无风险
✅ **XSS 防护**: 存储层不执行转义，由展示层负责
✅ **路径遍历**: 仅操作指定的数据库文件
✅ **DoS 防护**: SQLite 事务和超时机制

## 运行测试

```bash
# 运行所有特殊字符测试
cargo test --test special_chars_tests

# 运行 CLI 特殊字符测试
cargo test --test cli_special_chars_tests

# 运行特定测试
cargo test test_sql_injection
cargo test test_unicode
cargo test test_emoji
```

## 已知限制

1. **Null 字节**: SQLite 文本字段不支持 null 字节
2. **FTS5 CJK 分词**: 对中日韩语言分词支持有限
3. **超大文本**: 虽然支持，但 JSON 序列化超大文本可能影响性能

## 总结

Intent-Engine 对特殊字符和边界情况的处理：

- ✅ **安全性**: SQL 注入完全防护
- ✅ **国际化**: 完全支持 Unicode 和 Emoji
- ✅ **鲁棒性**: 正确处理各种边界情况
- ✅ **完整性**: 保持原始输入不变
- ⚠️ **搜索限制**: FTS5 对 CJK 语言分词有限

系统经过 47 个专门测试验证，确保在实际使用中的可靠性。
