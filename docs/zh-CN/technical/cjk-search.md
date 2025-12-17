# CJK 搜索实现

## 概述

Intent-Engine 通过智能双路径搜索架构，为 CJK（中文、日文、韩文）语言提供强大的全文搜索支持，该架构结合了 FTS5 trigram 分词和基于 LIKE 的回退机制。

## 面临的挑战

SQLite 的 FTS5 全文搜索使用 trigram 分词器时，需要至少 3 个连续字符才能创建可搜索的 token。这对 CJK 语言来说存在问题：

- **单字搜索很常见**：如"用"、"认"、"証"等字符
- **双字词汇非常普遍**："用户"、"认证"、"データ"等
- **每个字符都有含义**：不像英语中单个字母的语义价值有限

## 解决方案架构

### 双路径搜索策略

```
用户查询
    │
    ▼
┌──────────────────────┐
│ 查询分析              │
│ - 空查询/仅空格?      │
│ - 仅特殊字符?         │
│ - CJK 字符长度检查    │
└──────┬───────────────┘
       │
       ├─── 长度 < 3 且全为 CJK ──→ LIKE 回退
       │                           (src/tasks.rs:search_tasks_like)
       │
       └─── 长度 >= 3 或混合 ────→ FTS5 Trigram
                                    (src/tasks.rs:search_tasks_fts5)
```

### 查询路由逻辑

路由决策在 `src/search.rs::needs_like_fallback()` 中做出：

```rust
pub fn needs_like_fallback(query: &str) -> bool {
    let chars: Vec<char> = query.chars().collect();

    // 单个 CJK 字符
    if chars.len() == 1 && is_cjk_char(chars[0]) {
        return true;
    }

    // 两个全 CJK 字符
    if chars.len() == 2 && chars.iter().all(|c| is_cjk_char(*c)) {
        return true;
    }

    false
}
```

### CJK 字符检测

使用 Unicode 码点范围识别 CJK 字符：

| 范围 | 描述 |
|------|------|
| `0x4E00..=0x9FFF` | CJK 统一表意文字（最常见的中文） |
| `0x3400..=0x4DBF` | CJK 扩展 A |
| `0x20000..=0x2EBEF` | CJK 扩展 B-F（生僻字） |
| `0x3040..=0x309F` | 平假名（日文） |
| `0x30A0..=0x30FF` | 片假名（日文） |
| `0xAC00..=0xD7AF` | 谚文音节（韩文） |

## 搜索路径

### 路径 1：FTS5 Trigram（3+ 字符）

**适用于：**
- 三个或更多 CJK 字符："用户认证"
- 任意长度的英文查询："JWT"、"authentication"
- 混合语言："API接口"、"JWT认证"

**实现：**
```rust
async fn search_tasks_fts5(&self, query: &str) -> Result<Vec<TaskSearchResult>> {
    // 使用 SQLite FTS5 和 trigram 分词器
    // CREATE VIRTUAL TABLE tasks_fts USING fts5(
    //     name, spec,
    //     content=tasks,
    //     tokenize='trigram'
    // )

    // 返回带高亮片段的结果
    // 示例: "Fix **authentication** bug"
}
```

**优点：**
- 对大型数据集搜索速度快
- 支持高级 FTS5 语法（AND、OR、NOT、短语搜索）
- 基于相关性排序的结果
- 内置片段高亮功能

**限制：**
- 需要 3+ 字符才能匹配
- Trigram 分词可能会高亮部分单词

### 路径 2：LIKE 回退（1-2 个 CJK 字符）

**适用于：**
- 单个 CJK 字符："用"、"認"、"가"
- 两个 CJK 字符："用户"、"認証"、"사용"

**实现：**
```sql
SELECT * FROM tasks
WHERE name LIKE '%query%' OR spec LIKE '%query%'
ORDER BY name
```

**优点：**
- 适用于任何查询长度
- 精确的子字符串匹配
- 对短 CJK 查询可靠

**限制：**
- 对大型数据集较慢（O(n) 扫描）
- 无排序或高级搜索语法
- 手动创建代码片段

## 边界情况

### 空查询和特殊字符查询

系统优雅地处理边界情况：

```rust
// 空查询或仅空格 → 返回空结果
if query.trim().is_empty() {
    return Ok(Vec::new());
}

// 仅特殊字符 (@#$%) → 返回空结果
let has_searchable = query.chars().any(|c| {
    c.is_alphanumeric() || is_cjk_char(c)
});
if !has_searchable {
    return Ok(Vec::new());
}
```

### 混合语言查询

包含 CJK 和非 CJK 字符的查询使用 FTS5：
- "JWT认证" → FTS5（长度 >= 3）
- "API接口" → FTS5（长度 >= 3）

### 标点符号和空格

CJK 文本通常使用不同的标点符号：
- "实现：用户认证"（冒号） → 标点符号被忽略
- "实现 用户 认证"（空格） → 空格被视为词边界

## 性能特征

### FTS5 Trigram 路径

来自 `tests/cjk_search_tests.rs::test_search_performance`：
- **1000 个任务**：< 100ms
- **数据库大小**：使用索引的 O(1) 查找
- **可扩展性**：对大型数据集表现优秀

### LIKE 回退路径

来自同一测试：
- **1000 个任务**：< 500ms
- **数据库大小**：O(n) 表扫描
- **可扩展性**：对少于 10,000 个任务的数据集可接受

## 测试覆盖

`tests/cjk_search_tests.rs` 中的综合测试套件：

1. **单字搜索**（中文、日文、韩文）
2. **双字搜索**（常见 CJK 词汇）
3. **多字符搜索**（3+ 字符，FTS5）
4. **混合语言**（英文 + CJK）
5. **日文专项**（平假名、片假名、汉字）
6. **韩文专项**（谚文音节）
7. **边界情况**（标点符号、数字、空格）
8. **性能基准**（1000 个任务）
9. **空查询**（空格、特殊字符）
10. **大小写敏感性**（英文大小写）

## 迁移说明

### 数据库 Schema 变更

该实现需要 schema 变更：

**之前（v0.3.2 及更早版本）：**
```sql
CREATE VIRTUAL TABLE tasks_fts USING fts5(
    name, spec,
    content=tasks
    -- 无 tokenize 参数（默认分词器）
)
```

**之后（v0.3.3+）：**
```sql
CREATE VIRTUAL TABLE tasks_fts USING fts5(
    name, spec,
    content=tasks,
    tokenize='trigram'  -- 添加 trigram 分词器
)
```

### 自动迁移

迁移在 `src/db/mod.rs::run_migrations()` 中自动处理：

```rust
// 如果存在则删除现有的 FTS 表
let _ = sqlx::query("DROP TABLE IF EXISTS tasks_fts")
    .execute(pool)
    .await;

// 使用 trigram 分词器创建新的 FTS 表
sqlx::query(/* CREATE VIRTUAL TABLE ... */)
    .execute(pool)
    .await?;

// 使用现有数据重建索引
sqlx::query("INSERT INTO tasks_fts(rowid, name, spec) SELECT id, name, spec FROM tasks")
    .execute(pool)
    .await?;
```

用户无需采取任何操作 - 迁移在首次运行时透明进行。

## 使用示例

### 中文搜索

```rust
// 单字
task_mgr.search_tasks("用").await  // 使用 LIKE
// → 找到："用户认证"、"使用JWT"

// 双字
task_mgr.search_tasks("用户").await  // 使用 LIKE
// → 找到："用户认证功能"

// 三字及以上
task_mgr.search_tasks("用户认证").await  // 使用 FTS5
// → 找到："实现用户认证功能"
```

### 日文搜索

```rust
// 平假名
task_mgr.search_tasks("を").await  // 使用 LIKE
// → 找到："認証を実装"

// 片假名
task_mgr.search_tasks("ユーザー").await  // 使用 FTS5
// → 找到："ユーザー認証を実装"
```

### 韩文搜索

```rust
// 单个谚文音节
task_mgr.search_tasks("사").await  // 使用 LIKE
// → 找到："사용자 인증"

// 词汇
task_mgr.search_tasks("사용자").await  // 使用 FTS5
// → 找到："사용자 인증 구현"
```

### 混合语言

```rust
task_mgr.search_tasks("JWT认证").await  // 使用 FTS5
// → 找到："实现JWT认证"、"JWT認証を実装"

task_mgr.search_tasks("API接口").await  // 使用 FTS5
// → 找到："添加API接口"、"設計API接口"
```

## 参考资料

- **实现**：`src/search.rs`、`src/tasks.rs::search_tasks()`
- **测试**：`tests/cjk_search_tests.rs`
- **数据库**：`src/db/mod.rs::run_migrations()`
- **SQLite FTS5**：https://www.sqlite.org/fts5.html
- **Trigram 分词器**：https://www.sqlite.org/fts5.html#the_trigram_tokenizer

## 未来增强

未来版本的潜在改进：

1. **Better-Trigram 扩展**：评估集成基于 C 的 Better-Trigram 以获得最佳 CJK 支持（受 sqlx 扩展加载限制的阻碍）
2. **模糊匹配**：支持拼写错误和相似字符
3. **同义词支持**：常见同义词的搜索扩展
4. **语言检测**：自动检测以优化搜索策略
5. **用户偏好**：允许用户配置搜索行为

---

**版本**：0.3.3
**最后更新**：2025-11-14
**状态**：生产环境
