# CJK Search Implementation

## Overview

Intent-Engine provides robust full-text search support for CJK (Chinese, Japanese, Korean) languages through an intelligent dual-path search architecture that combines FTS5 trigram tokenization with LIKE-based fallback.

## The Challenge

SQLite's FTS5 full-text search with the trigram tokenizer requires a minimum of 3 consecutive characters to create searchable tokens. This is problematic for CJK languages where:

- **Single-character searches are common**: Characters like "用" (use), "认" (recognize), "証" (proof)
- **Two-character words are prevalent**: "用户" (user), "认证" (authentication), "データ" (data)
- **Each character carries meaning**: Unlike English where individual letters have limited semantic value

## Solution Architecture

### Dual-Path Search Strategy

```
User Query
    │
    ▼
┌──────────────────────┐
│ Query Analysis       │
│ - Empty/whitespace?  │
│ - Only special chars?│
│ - CJK length check   │
└──────┬───────────────┘
       │
       ├─── Length < 3 AND all CJK ──→ LIKE Fallback
       │                                (src/tasks.rs:search_tasks_like)
       │
       └─── Length >= 3 OR mixed ───→ FTS5 Trigram
                                       (src/tasks.rs:search_tasks_fts5)
```

### Query Routing Logic

The routing decision is made in `src/search.rs::needs_like_fallback()`:

```rust
pub fn needs_like_fallback(query: &str) -> bool {
    let chars: Vec<char> = query.chars().collect();

    // Single-character CJK
    if chars.len() == 1 && is_cjk_char(chars[0]) {
        return true;
    }

    // Two-character all-CJK
    if chars.len() == 2 && chars.iter().all(|c| is_cjk_char(*c)) {
        return true;
    }

    false
}
```

### CJK Character Detection

CJK characters are identified using Unicode code point ranges:

| Range | Description |
|-------|-------------|
| `0x4E00..=0x9FFF` | CJK Unified Ideographs (most common Chinese) |
| `0x3400..=0x4DBF` | CJK Extension A |
| `0x20000..=0x2EBEF` | CJK Extensions B-F (rare characters) |
| `0x3040..=0x309F` | Hiragana (Japanese) |
| `0x30A0..=0x30FF` | Katakana (Japanese) |
| `0xAC00..=0xD7AF` | Hangul Syllables (Korean) |

## Search Paths

### Path 1: FTS5 Trigram (3+ characters)

**Used for:**
- Three or more CJK characters: "用户认证"
- English queries of any length: "JWT", "authentication"
- Mixed language: "API接口", "JWT认证"

**Implementation:**
```rust
async fn search_tasks_fts5(&self, query: &str) -> Result<Vec<TaskSearchResult>> {
    // Uses SQLite FTS5 with trigram tokenizer
    // CREATE VIRTUAL TABLE tasks_fts USING fts5(
    //     name, spec,
    //     content=tasks,
    //     tokenize='trigram'
    // )

    // Returns results with highlighted snippets
    // Example: "Fix **authentication** bug"
}
```

**Advantages:**
- Fast for large datasets
- Supports advanced FTS5 syntax (AND, OR, NOT, phrase search)
- Rank-based result ordering
- Built-in snippet highlighting

**Limitations:**
- Requires 3+ characters for matching
- Trigram tokenization may highlight partial words

### Path 2: LIKE Fallback (1-2 CJK characters)

**Used for:**
- Single CJK character: "用", "認", "가"
- Two CJK characters: "用户", "認証", "사용"

**Implementation:**
```sql
SELECT * FROM tasks
WHERE name LIKE '%query%' OR spec LIKE '%query%'
ORDER BY name
```

**Advantages:**
- Works with any query length
- Exact substring matching
- Reliable for short CJK queries

**Limitations:**
- Slower than FTS5 for large datasets (O(n) scan)
- No ranking or advanced search syntax
- Manual snippet creation

## Edge Cases

### Empty and Special Character Queries

The system handles edge cases gracefully:

```rust
// Empty or whitespace → return empty results
if query.trim().is_empty() {
    return Ok(Vec::new());
}

// Only special characters (@#$%) → return empty results
let has_searchable = query.chars().any(|c| {
    c.is_alphanumeric() || is_cjk_char(c)
});
if !has_searchable {
    return Ok(Vec::new());
}
```

### Mixed Language Queries

Queries containing both CJK and non-CJK characters use FTS5:
- "JWT认证" → FTS5 (length >= 3)
- "API接口" → FTS5 (length >= 3)

### Punctuation and Spacing

CJK text often uses different punctuation:
- "实现：用户认证" (colon) → Punctuation is ignored
- "实现 用户 认证" (spaces) → Spaces treated as word boundaries

## Performance Characteristics

### FTS5 Trigram Path

From `tests/cjk_search_tests.rs::test_search_performance`:
- **1000 tasks**: < 100ms
- **Database size**: O(1) lookup using index
- **Scalability**: Excellent for large datasets

### LIKE Fallback Path

From the same test:
- **1000 tasks**: < 500ms
- **Database size**: O(n) table scan
- **Scalability**: Acceptable for datasets under 10,000 tasks

## Testing Coverage

Comprehensive test suite in `tests/cjk_search_tests.rs`:

1. **Single-character search** (Chinese, Japanese, Korean)
2. **Two-character search** (common CJK words)
3. **Multi-character search** (3+ characters, FTS5)
4. **Mixed language** (English + CJK)
5. **Japanese-specific** (Hiragana, Katakana, Kanji)
6. **Korean-specific** (Hangul syllables)
7. **Edge cases** (punctuation, numbers, spaces)
8. **Performance benchmarks** (1000 tasks)
9. **Empty queries** (whitespace, special characters)
10. **Case sensitivity** (English upper/lower case)

## Migration Notes

### Database Schema Changes

The implementation requires a schema change:

**Before (v0.3.2 and earlier):**
```sql
CREATE VIRTUAL TABLE tasks_fts USING fts5(
    name, spec,
    content=tasks
    -- No tokenize parameter (default tokenizer)
)
```

**After (v0.3.3+):**
```sql
CREATE VIRTUAL TABLE tasks_fts USING fts5(
    name, spec,
    content=tasks,
    tokenize='trigram'  -- Added trigram tokenizer
)
```

### Automatic Migration

The migration is handled automatically in `src/db/mod.rs::run_migrations()`:

```rust
// Drop existing FTS table if it exists
let _ = sqlx::query("DROP TABLE IF EXISTS tasks_fts")
    .execute(pool)
    .await;

// Create new FTS table with trigram tokenizer
sqlx::query(/* CREATE VIRTUAL TABLE ... */)
    .execute(pool)
    .await?;

// Rebuild index with existing data
sqlx::query("INSERT INTO tasks_fts(rowid, name, spec) SELECT id, name, spec FROM tasks")
    .execute(pool)
    .await?;
```

Users do not need to take any action - the migration happens transparently on first run.

## Usage Examples

### Chinese Search

```rust
// Single character
task_mgr.search_tasks("用").await  // Uses LIKE
// → Finds: "用户认证", "使用JWT"

// Two characters
task_mgr.search_tasks("用户").await  // Uses LIKE
// → Finds: "用户认证功能"

// Three+ characters
task_mgr.search_tasks("用户认证").await  // Uses FTS5
// → Finds: "实现用户认证功能"
```

### Japanese Search

```rust
// Hiragana
task_mgr.search_tasks("を").await  // Uses LIKE
// → Finds: "認証を実装"

// Katakana
task_mgr.search_tasks("ユーザー").await  // Uses FTS5
// → Finds: "ユーザー認証を実装"
```

### Korean Search

```rust
// Single Hangul syllable
task_mgr.search_tasks("사").await  // Uses LIKE
// → Finds: "사용자 인증"

// Word
task_mgr.search_tasks("사용자").await  // Uses FTS5
// → Finds: "사용자 인증 구현"
```

### Mixed Language

```rust
task_mgr.search_tasks("JWT认证").await  // Uses FTS5
// → Finds: "实现JWT认证", "JWT認証を実装"

task_mgr.search_tasks("API接口").await  // Uses FTS5
// → Finds: "添加API接口", "設計API接口"
```

## References

- **Implementation**: `src/search.rs`, `src/tasks.rs::search_tasks()`
- **Tests**: `tests/cjk_search_tests.rs`
- **Database**: `src/db/mod.rs::run_migrations()`
- **SQLite FTS5**: https://www.sqlite.org/fts5.html
- **Trigram Tokenizer**: https://www.sqlite.org/fts5.html#the_trigram_tokenizer

## Future Enhancements

Potential improvements for future versions:

1. **Better-Trigram Extension**: Evaluate integration of C-based Better-Trigram for optimal CJK support (blocked by sqlx extension loading limitations)
2. **Fuzzy Matching**: Support for typos and similar characters
3. **Synonym Support**: Search expansion for common synonyms
4. **Language Detection**: Automatic detection to optimize search strategy
5. **User Preferences**: Allow users to configure search behavior

---

**Version**: 0.3.3
**Last Updated**: 2025-11-14
**Status**: Production
