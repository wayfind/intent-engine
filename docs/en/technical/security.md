# Special Character and Edge Case Handling

This document explains Intent-Engine's ability to handle various special characters, Unicode, and extreme inputs.

## Test Coverage Overview

Intent-Engine is thoroughly tested and verified for correct handling of:

- ✅ SQL injection protection
- ✅ Unicode characters (Chinese, Japanese, Arabic, etc.)
- ✅ Emoji symbols
- ✅ JSON special characters
- ✅ Control characters (newlines, tabs, etc.)
- ✅ Extremely long inputs (10,000+ characters)
- ✅ Edge cases (empty strings, pure whitespace, etc.)
- ✅ Shell metacharacters
- ✅ Markdown/HTML tags
- ✅ URLs and paths

## Security Guarantees

### SQL Injection Protection ✅

Intent-Engine uses parameterized queries (prepared statements), completely preventing SQL injection attacks.

**Test Case**:
```rust
// Attempt SQL injection
let malicious = "Task'; DROP TABLE tasks; --";
task_mgr.add_task(malicious, None, None).await.unwrap();

// ✅ Result: Malicious code treated as regular string, table not dropped
```

**Verification**:
- ✅ Single quote injection
- ✅ UNION SELECT injection
- ✅ Comment markers `--` and `/**/`
- ✅ SQL commands in event data

## Unicode Support

### Multi-language Characters ✅

Full support for Unicode characters, including various languages:

```rust
// Chinese
"实现用户认证功能"

// Japanese
"タスクを実装する"

// Arabic
"تنفيذ المهمة"

// Mixed languages
"实现 authentication 認証 مصادقة"
```

**Verification**:
- ✅ Chinese character storage and retrieval
- ✅ Japanese character storage and retrieval
- ✅ Arabic (RTL) characters
- ✅ Mixed language content

### Emoji Support ✅

Full support for emoji symbols, including compound emojis:

```rust
// Simple emoji
"🚀 Deploy to production 🎉"

// Compound emoji
"👨‍👩‍👧‍👦 Family task 🏳️‍🌈 🇺🇸"
```

**Verification**:
- ✅ Basic emoji (🚀🎉💻)
- ✅ Compound emoji sequences (👨‍👩‍👧‍👦)
- ✅ Flag emoji (🇺🇸)
- ✅ Variant selectors (🏳️‍🌈)

## JSON Special Characters

### Quotes and Escaping ✅

Correctly handles characters that need escaping in JSON:

```rust
// Double quotes
r#"Task with "quoted" text"#

// Backslash
r"C:\Users\test\path"

// Control characters
"Task\nwith\nnewlines\tand\ttabs"
```

**JSON Output**:
```json
{
  "name": "Task with \"quoted\" text"
}
```

**Verification**:
- ✅ Double quotes correctly escaped as `\"`
- ✅ Backslash correctly escaped as `\\`
- ✅ Newlines escaped as `\n`
- ✅ Tabs escaped as `\t`

### Null Byte Handling ⚠️

SQLite doesn't support null bytes (`\0`) in text. The system will:
- Option 1: Reject input containing null bytes
- Option 2: Automatically remove null bytes

**Recommendation**: Avoid using null bytes in input.

## Control Characters

### Multi-line Content ✅

Full support for multi-line text:

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

**Verification**:
- ✅ Newlines (`\n`)
- ✅ Carriage return + newline (`\r\n`)
- ✅ Tabs (`\t`)
- ✅ Multiple consecutive spaces

## Extreme Lengths

### Extra-Long Inputs ✅

System supports extremely long inputs:

| Field | Test Length | Status | Notes |
|-------|-----------|--------|-------|
| Task Name | 10,000 characters | ✅ | No limit |
| Specification | 35,000 characters | ✅ | No limit |
| Event Data | 120,000 characters | ✅ | No limit |

**Performance**:
- 10,000 character task name: Normal storage and retrieval
- Extra-long text doesn't affect query performance
- JSON serialization works normally

## Edge Cases

### Empty and Minimal Inputs ✅

```rust
// Empty string (allowed)
task_mgr.add_task("", None, None).await.unwrap()

// Pure whitespace (allowed)
task_mgr.add_task("     ", None, None).await.unwrap()

// Single character
task_mgr.add_task("A", None, None).await.unwrap()
```

**Verification**:
- ✅ Empty task name (allowed but not recommended)
- ✅ Pure whitespace task name
- ✅ Single character task name
- ✅ Empty specification
- ✅ Empty event data

## Special Symbol Combinations

### Shell Metacharacters ✅

Safely handles special characters in shell commands:

```rust
"Task && echo 'test' | grep -v 'bad' > /dev/null"
```

**Verification**:
- ✅ Pipe `|`
- ✅ Redirection `>` `<`
- ✅ Logical operators `&&` `||`
- ✅ Command substitution `` `command` ``

### Markdown/HTML ✅

```rust
// Markdown
"# Task **bold** *italic* `code`"

// HTML
"<script>alert('xss')</script>"
```

**Note**: System doesn't filter or escape these characters, stores as-is. Client is responsible for safe rendering.

### Regex Metacharacters ✅

```rust
r"Task.*[0-9]+\d{3}(test|prod)$"
```

All regex metacharacters are correctly stored and retrieved.

### URLs and Paths ✅

```rust
// URL with query parameters
"Deploy to https://example.com/api?key=value&test=1"

// Windows path
r"C:\Users\test\Documents\file.txt"

// Unix path
"/home/user/project/file.txt"
```

## FTS5 Full-Text Search Limitations

### English Search ✅

Full-text search works perfectly for English content:

```rust
task: "Implement authentication feature"
search: "authentication" // ✅ Found
```

### CJK Language Limitations ⚠️

SQLite FTS5's unicode61 tokenizer has limited word segmentation support for Chinese-Japanese-Korean (CJK) languages:

```rust
task: "实现用户认证功能"
search: "认证" // ⚠️ May not find (requires exact match)
search: "实现用户认证功能" // ✅ Can find (exact match)
```

**Recommendations**:
- Use complete phrase search for CJK content
- Consider using English keyword prefixes for task names
- Use non-FTS standard filtering for Chinese tasks

**Improvement Direction**:
Future consideration for integrating specialized CJK tokenizers (e.g., jieba, mecab).

## CLI Special Character Handling

### Shell Quoting ✅

Use quotes in command line to protect special characters:

```bash
# Correct
intent-engine task add --name "Task with spaces"
intent-engine task add --name 'Task with "quotes"'

# Unicode
intent-engine task add --name "实现功能"

# Emoji
intent-engine task add --name "🚀 Deploy"
```

### stdin Input ✅

Pass complex content via stdin:

```bash
echo "Multi-line\nspecification\nwith special chars" | \
  intent-engine task add --name "Task" --spec-stdin
```

## Test Coverage Statistics

### Unit Tests

- **Special Character Tests**: 37 tests
  - SQL injection: 4 tests
  - Unicode/Emoji: 7 tests
  - JSON special characters: 4 tests
  - Control characters: 4 tests
  - Extreme lengths: 3 tests
  - Edge cases: 5 tests
  - Special symbols: 7 tests
  - FTS5 search: 3 tests

### CLI Integration Tests

- **CLI Special Character Tests**: 10 tests
  - Unicode and Emoji via CLI
  - Multi-line and quote handling
  - Extra-long inputs
  - Special symbol combinations

## Best Practices

### For Developers

1. **Always use parameterized queries** - Built-in, no extra action needed
2. **Don't filter user input** - Preserve original input integrity
3. **Rely on JSON serialization** - serde_json automatically handles escaping

### For Users

1. **Shell Quote Usage**
   ```bash
   # Single quotes protect most special characters
   intent-engine task add --name 'Task with $var'

   # Double quotes allow variable expansion
   intent-engine task add --name "Task for $USER"
   ```

2. **Use stdin for Complex Content**
   ```bash
   cat spec.md | intent-engine task add --name "Task" --spec-stdin
   ```

3. **CJK Search Tips**
   - Use complete phrases rather than single words
   - Consider adding English keywords

## Security Statement

Intent-Engine's security features:

✅ **SQL Injection**: Complete protection (parameterized queries)
✅ **Command Injection**: Doesn't execute external commands, no risk
✅ **XSS Protection**: Storage layer doesn't escape, presentation layer responsible
✅ **Path Traversal**: Only operates on specified database file
✅ **DoS Protection**: SQLite transactions and timeout mechanisms

## Running Tests

```bash
# Run all special character tests
cargo test --test special_chars_tests

# Run CLI special character tests
cargo test --test cli_special_chars_tests

# Run specific tests
cargo test test_sql_injection
cargo test test_unicode
cargo test test_emoji
```

## Known Limitations

1. **Null Bytes**: SQLite text fields don't support null bytes
2. **FTS5 CJK Tokenization**: Limited word segmentation for Chinese-Japanese-Korean languages
3. **Extra-Large Text**: Although supported, JSON serialization of very large text may affect performance

## Summary

Intent-Engine's handling of special characters and edge cases:

- ✅ **Security**: SQL injection fully protected
- ✅ **Internationalization**: Full Unicode and Emoji support
- ✅ **Robustness**: Correct handling of various edge cases
- ✅ **Integrity**: Preserves original input unchanged
- ⚠️ **Search Limitation**: FTS5 has limited CJK tokenization

System verified through 47 dedicated tests, ensuring reliability in actual use.
