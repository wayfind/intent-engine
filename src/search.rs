//! Search utilities for intent-engine
//!
//! This module provides:
//! 1. CJK (Chinese, Japanese, Korean) search utilities for detecting when to use
//!    LIKE fallback vs FTS5 trigram search
//! 2. Unified search across tasks and events
//!
//! **Background**: SQLite FTS5 with trigram tokenizer requires at least 3 consecutive
//! characters to match. This is problematic for CJK languages where single-character
//! or two-character searches are common (e.g., "用户", "认证").
//!
//! **Solution**: For short CJK queries, we fallback to LIKE search which supports
//! any length substring matching, albeit slower.

/// Check if a character is a CJK character
pub fn is_cjk_char(c: char) -> bool {
    let code = c as u32;
    matches!(code,
        // CJK Unified Ideographs (most common Chinese characters)
        0x4E00..=0x9FFF |
        // CJK Extension A
        0x3400..=0x4DBF |
        // CJK Extension B-F (less common, but included for completeness)
        0x20000..=0x2A6DF |
        0x2A700..=0x2B73F |
        0x2B740..=0x2B81F |
        0x2B820..=0x2CEAF |
        0x2CEB0..=0x2EBEF |
        // Hiragana (Japanese)
        0x3040..=0x309F |
        // Katakana (Japanese)
        0x30A0..=0x30FF |
        // Hangul Syllables (Korean)
        0xAC00..=0xD7AF
    )
}

/// Determine if a query should use LIKE fallback instead of FTS5 trigram
///
/// Returns `true` if:
/// - Query is a single CJK character, OR
/// - Query is two CJK characters
///
/// Trigram tokenizer requires 3+ characters for matching, so we use LIKE
/// for shorter CJK queries to ensure they work.
pub fn needs_like_fallback(query: &str) -> bool {
    let chars: Vec<char> = query.chars().collect();

    // Single-character CJK
    if chars.len() == 1 && is_cjk_char(chars[0]) {
        return true;
    }

    // Two-character all-CJK
    // This is optional - could also let trigram handle it, but trigram
    // needs minimum 3 chars so two-char CJK won't work well
    if chars.len() == 2 && chars.iter().all(|c| is_cjk_char(*c)) {
        return true;
    }

    false
}

/// Escape FTS5 special characters in a query string
///
/// FTS5 queries support advanced syntax (AND, OR, NOT, *, #, "phrase search", etc.).
/// To safely search user input as literal text, we:
/// 1. Escape any double quotes by doubling them (`"` -> `""`)
/// 2. Wrap the entire query in double quotes for literal phrase search
///
/// This prevents special characters like `#`, `*`, `+`, `-`, etc. from being
/// interpreted as FTS5 syntax operators.
///
/// # Arguments
/// * `query` - The query string to escape
///
/// # Returns
/// The query string wrapped in double quotes with internal quotes escaped
///
/// # Example
/// ```ignore
/// use crate::search::escape_fts5;
///
/// let escaped = escape_fts5("user \"admin\" role");
/// assert_eq!(escaped, "\"user \"\"admin\"\" role\"");
///
/// let escaped = escape_fts5("#123");
/// assert_eq!(escaped, "\"#123\"");
/// ```
pub fn escape_fts5(query: &str) -> String {
    // Escape internal double quotes and wrap in quotes for literal search
    format!("\"{}\"", query.replace('"', "\"\""))
}

// ============================================================================
// Unified Search
// ============================================================================

use crate::db::models::{Event, PaginatedSearchResults, SearchResult, Task};
use crate::error::Result;
use crate::tasks::TaskManager;
use sqlx::{Row, SqlitePool};

pub struct SearchManager<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SearchManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Unified search across tasks and events with pagination support
    ///
    /// This is the new unified search method that replaces unified_search().
    /// Key improvements:
    /// - Pagination support (limit, offset)
    /// - Flexible result inclusion (tasks, events)
    /// - Optional priority-based secondary sorting
    /// - Returns PaginatedSearchResults with metadata
    ///
    /// # Parameters
    /// - `query`: FTS5 search query string
    /// - `include_tasks`: Whether to search in tasks (default: true)
    /// - `include_events`: Whether to search in events (default: true)
    /// - `limit`: Maximum number of results per source (default: 20)
    /// - `offset`: Number of results to skip (default: 0)
    /// - `sort_by_priority`: Enable priority-based secondary sorting (default: false)
    ///
    /// # Returns
    /// PaginatedSearchResults with mixed task and event results, ordered by relevance (FTS5 rank)
    pub async fn search(
        &self,
        query: &str,
        include_tasks: bool,
        include_events: bool,
        limit: Option<i64>,
        offset: Option<i64>,
        sort_by_priority: bool,
    ) -> Result<PaginatedSearchResults> {
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        // Handle empty or whitespace-only queries
        if query.trim().is_empty() {
            return Ok(PaginatedSearchResults {
                results: Vec::new(),
                total_tasks: 0,
                total_events: 0,
                has_more: false,
                limit,
                offset,
            });
        }

        // Handle queries with no searchable content (only special characters)
        let has_searchable = query.chars().any(|c| c.is_alphanumeric() || is_cjk_char(c));
        if !has_searchable {
            return Ok(PaginatedSearchResults {
                results: Vec::new(),
                total_tasks: 0,
                total_events: 0,
                has_more: false,
                limit,
                offset,
            });
        }

        // Escape FTS5 special characters
        let escaped_query = escape_fts5(query);

        let mut total_tasks: i64 = 0;
        let mut total_events: i64 = 0;
        let mut all_results: Vec<(SearchResult, f64)> = Vec::new();

        // Check if we need LIKE fallback for short CJK queries
        let use_like_fallback = needs_like_fallback(query);

        if use_like_fallback {
            // LIKE fallback path for short CJK queries (1-2 chars)
            let like_pattern = format!("%{}%", query);

            // Search tasks if enabled
            if include_tasks {
                // Get total count
                let count_result = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM tasks WHERE name LIKE ? OR spec LIKE ?",
                )
                .bind(&like_pattern)
                .bind(&like_pattern)
                .fetch_one(self.pool)
                .await?;
                total_tasks = count_result;

                // Build ORDER BY clause
                let order_by = if sort_by_priority {
                    "ORDER BY COALESCE(priority, 0) ASC, id ASC"
                } else {
                    "ORDER BY id ASC"
                };

                // Query tasks with pagination
                let task_query = format!(
                    r#"
                    SELECT
                        id,
                        parent_id,
                        name,
                        spec,
                        status,
                        complexity,
                        priority,
                        first_todo_at,
                        first_doing_at,
                        first_done_at,
                        active_form,
                        owner
                    FROM tasks
                    WHERE name LIKE ? OR spec LIKE ?
                    {}
                    LIMIT ? OFFSET ?
                    "#,
                    order_by
                );

                let rows = sqlx::query(&task_query)
                    .bind(&like_pattern)
                    .bind(&like_pattern)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(self.pool)
                    .await?;

                for row in rows {
                    let task = Task {
                        id: row.get("id"),
                        parent_id: row.get("parent_id"),
                        name: row.get("name"),
                        spec: row.get("spec"),
                        status: row.get("status"),
                        complexity: row.get("complexity"),
                        priority: row.get("priority"),
                        first_todo_at: row.get("first_todo_at"),
                        first_doing_at: row.get("first_doing_at"),
                        first_done_at: row.get("first_done_at"),
                        active_form: row.get("active_form"),
                        owner: row.get("owner"),
                    };

                    // Determine match field and create snippet
                    let (match_field, match_snippet) = if task.name.contains(query) {
                        ("name".to_string(), task.name.clone())
                    } else if let Some(ref spec) = task.spec {
                        if spec.contains(query) {
                            ("spec".to_string(), spec.clone())
                        } else {
                            ("name".to_string(), task.name.clone())
                        }
                    } else {
                        ("name".to_string(), task.name.clone())
                    };

                    all_results.push((
                        SearchResult::Task {
                            task,
                            match_snippet,
                            match_field,
                        },
                        1.0, // Constant rank for LIKE results
                    ));
                }
            }

            // Search events if enabled
            if include_events {
                // Get total count
                let count_result = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM events WHERE discussion_data LIKE ?",
                )
                .bind(&like_pattern)
                .fetch_one(self.pool)
                .await?;
                total_events = count_result;

                // Query events with pagination
                let rows = sqlx::query(
                    r#"
                    SELECT
                        id,
                        task_id,
                        timestamp,
                        log_type,
                        discussion_data
                    FROM events
                    WHERE discussion_data LIKE ?
                    ORDER BY id ASC
                    LIMIT ? OFFSET ?
                    "#,
                )
                .bind(&like_pattern)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await?;

                let task_mgr = TaskManager::new(self.pool);
                for row in rows {
                    let event = Event {
                        id: row.get("id"),
                        task_id: row.get("task_id"),
                        timestamp: row.get("timestamp"),
                        log_type: row.get("log_type"),
                        discussion_data: row.get("discussion_data"),
                    };

                    // Create match snippet
                    let match_snippet = event.discussion_data.clone();

                    // Get task ancestry chain for this event
                    let task_chain = task_mgr.get_task_ancestry(event.task_id).await?;

                    all_results.push((
                        SearchResult::Event {
                            event,
                            task_chain,
                            match_snippet,
                        },
                        1.0, // Constant rank for LIKE results
                    ));
                }
            }
        } else {
            // FTS5 path for longer queries (3+ chars)
            // Search tasks if enabled
            if include_tasks {
                // Get total count
                let count_result = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM tasks_fts WHERE tasks_fts MATCH ?",
                )
                .bind(&escaped_query)
                .fetch_one(self.pool)
                .await?;
                total_tasks = count_result;

                // Build ORDER BY clause
                let order_by = if sort_by_priority {
                    "ORDER BY rank ASC, COALESCE(t.priority, 0) ASC, t.id ASC"
                } else {
                    "ORDER BY rank ASC, t.id ASC"
                };

                // Query tasks with pagination
                let task_query = format!(
                    r#"
                SELECT
                    t.id,
                    t.parent_id,
                    t.name,
                    t.spec,
                    t.status,
                    t.complexity,
                    t.priority,
                    t.first_todo_at,
                    t.first_doing_at,
                    t.first_done_at,
                    t.active_form,
                    t.owner,
                    COALESCE(
                        snippet(tasks_fts, 1, '**', '**', '...', 15),
                        snippet(tasks_fts, 0, '**', '**', '...', 15)
                    ) as match_snippet,
                    rank
                FROM tasks_fts
                INNER JOIN tasks t ON tasks_fts.rowid = t.id
                WHERE tasks_fts MATCH ?
                {}
                LIMIT ? OFFSET ?
                "#,
                    order_by
                );

                let rows = sqlx::query(&task_query)
                    .bind(&escaped_query)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(self.pool)
                    .await?;

                for row in rows {
                    let task = Task {
                        id: row.get("id"),
                        parent_id: row.get("parent_id"),
                        name: row.get("name"),
                        spec: row.get("spec"),
                        status: row.get("status"),
                        complexity: row.get("complexity"),
                        priority: row.get("priority"),
                        first_todo_at: row.get("first_todo_at"),
                        first_doing_at: row.get("first_doing_at"),
                        first_done_at: row.get("first_done_at"),
                        active_form: row.get("active_form"),
                        owner: row.get("owner"),
                    };
                    let match_snippet: String = row.get("match_snippet");
                    let rank: f64 = row.get("rank");

                    // Determine match field based on snippet content
                    let match_field = if task
                        .spec
                        .as_ref()
                        .map(|s| match_snippet.to_lowercase().contains(&s.to_lowercase()))
                        .unwrap_or(false)
                    {
                        "spec".to_string()
                    } else {
                        "name".to_string()
                    };

                    all_results.push((
                        SearchResult::Task {
                            task,
                            match_snippet,
                            match_field,
                        },
                        rank,
                    ));
                }
            }

            // Search events if enabled
            if include_events {
                // Get total count
                let count_result = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM events_fts WHERE events_fts MATCH ?",
                )
                .bind(&escaped_query)
                .fetch_one(self.pool)
                .await?;
                total_events = count_result;

                // Query events with pagination
                let rows = sqlx::query(
                    r#"
                SELECT
                    e.id,
                    e.task_id,
                    e.timestamp,
                    e.log_type,
                    e.discussion_data,
                    snippet(events_fts, 0, '**', '**', '...', 15) as match_snippet,
                    rank
                FROM events_fts
                INNER JOIN events e ON events_fts.rowid = e.id
                WHERE events_fts MATCH ?
                ORDER BY rank ASC, e.id ASC
                LIMIT ? OFFSET ?
                "#,
                )
                .bind(&escaped_query)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await?;

                let task_mgr = TaskManager::new(self.pool);
                for row in rows {
                    let event = Event {
                        id: row.get("id"),
                        task_id: row.get("task_id"),
                        timestamp: row.get("timestamp"),
                        log_type: row.get("log_type"),
                        discussion_data: row.get("discussion_data"),
                    };
                    let match_snippet: String = row.get("match_snippet");
                    let rank: f64 = row.get("rank");

                    // Get task ancestry chain for this event
                    let task_chain = task_mgr.get_task_ancestry(event.task_id).await?;

                    all_results.push((
                        SearchResult::Event {
                            event,
                            task_chain,
                            match_snippet,
                        },
                        rank,
                    ));
                }
            }
        } // End of else block (FTS5 path)

        // Sort all results by rank (relevance)
        all_results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Extract results without rank
        let results: Vec<SearchResult> =
            all_results.into_iter().map(|(result, _)| result).collect();

        // Calculate has_more
        let total_count = total_tasks + total_events;
        let has_more = offset + (results.len() as i64) < total_count;

        Ok(PaginatedSearchResults {
            results,
            total_tasks,
            total_events,
            has_more,
            limit,
            offset,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cjk_char() {
        // Chinese characters
        assert!(is_cjk_char('中'));
        assert!(is_cjk_char('文'));
        assert!(is_cjk_char('认'));
        assert!(is_cjk_char('证'));

        // Japanese Hiragana
        assert!(is_cjk_char('あ'));
        assert!(is_cjk_char('い'));

        // Japanese Katakana
        assert!(is_cjk_char('ア'));
        assert!(is_cjk_char('イ'));

        // Korean Hangul
        assert!(is_cjk_char('가'));
        assert!(is_cjk_char('나'));

        // Non-CJK
        assert!(!is_cjk_char('a'));
        assert!(!is_cjk_char('A'));
        assert!(!is_cjk_char('1'));
        assert!(!is_cjk_char(' '));
        assert!(!is_cjk_char('.'));
    }

    #[test]
    fn test_needs_like_fallback() {
        // Single CJK character - needs fallback
        assert!(needs_like_fallback("中"));
        assert!(needs_like_fallback("认"));
        assert!(needs_like_fallback("あ"));
        assert!(needs_like_fallback("가"));

        // Two CJK characters - needs fallback
        assert!(needs_like_fallback("中文"));
        assert!(needs_like_fallback("认证"));
        assert!(needs_like_fallback("用户"));

        // Three+ CJK characters - can use FTS5
        assert!(!needs_like_fallback("用户认"));
        assert!(!needs_like_fallback("用户认证"));

        // English - can use FTS5
        assert!(!needs_like_fallback("JWT"));
        assert!(!needs_like_fallback("auth"));
        assert!(!needs_like_fallback("a")); // Single ASCII char, not CJK

        // Mixed - can use FTS5
        assert!(!needs_like_fallback("JWT认证"));
        assert!(!needs_like_fallback("API接口"));
    }

    #[test]
    fn test_needs_like_fallback_mixed_cjk_ascii() {
        // Two characters: one CJK + one ASCII - should NOT need fallback
        // because not all chars are CJK
        assert!(!needs_like_fallback("中a"));
        assert!(!needs_like_fallback("a中"));
        assert!(!needs_like_fallback("認1"));

        // Three+ characters with mixed CJK/ASCII - can use FTS5
        assert!(!needs_like_fallback("中文API"));
        assert!(!needs_like_fallback("JWT认证系统"));
        assert!(!needs_like_fallback("API中文文档"));
    }

    #[test]
    fn test_needs_like_fallback_edge_cases() {
        // Empty string - no fallback needed
        assert!(!needs_like_fallback(""));

        // Whitespace only - no fallback
        assert!(!needs_like_fallback(" "));
        assert!(!needs_like_fallback("  "));

        // Single non-CJK - no fallback
        assert!(!needs_like_fallback("1"));
        assert!(!needs_like_fallback("@"));
        assert!(!needs_like_fallback(" "));

        // Two non-CJK - no fallback
        assert!(!needs_like_fallback("ab"));
        assert!(!needs_like_fallback("12"));
    }

    #[test]
    fn test_is_cjk_char_extension_ranges() {
        // CJK Extension A (U+3400..U+4DBF)
        assert!(is_cjk_char('\u{3400}')); // First char of Extension A
        assert!(is_cjk_char('\u{4DBF}')); // Last char of Extension A

        // CJK Unified Ideographs (U+4E00..U+9FFF) - common range
        assert!(is_cjk_char('\u{4E00}')); // First common CJK
        assert!(is_cjk_char('\u{9FFF}')); // Last common CJK

        // Characters just outside ranges - should NOT be CJK
        assert!(!is_cjk_char('\u{33FF}')); // Just before Extension A
        assert!(!is_cjk_char('\u{4DC0}')); // Just after Extension A
        assert!(!is_cjk_char('\u{4DFF}')); // Just before Unified Ideographs
        assert!(!is_cjk_char('\u{A000}')); // Just after Unified Ideographs
    }

    #[test]
    fn test_is_cjk_char_japanese() {
        // Hiragana range (U+3040..U+309F)
        assert!(is_cjk_char('\u{3040}')); // First Hiragana
        assert!(is_cjk_char('ひ')); // Middle Hiragana
        assert!(is_cjk_char('\u{309F}')); // Last Hiragana

        // Katakana range (U+30A0..U+30FF)
        assert!(is_cjk_char('\u{30A0}')); // First Katakana
        assert!(is_cjk_char('カ')); // Middle Katakana
        assert!(is_cjk_char('\u{30FF}')); // Last Katakana

        // Just outside Japanese ranges
        assert!(!is_cjk_char('\u{303F}')); // Before Hiragana
        assert!(!is_cjk_char('\u{3100}')); // After Katakana (Bopomofo, not CJK by our definition)
    }

    #[test]
    fn test_is_cjk_char_korean() {
        // Hangul Syllables (U+AC00..U+D7AF)
        assert!(is_cjk_char('\u{AC00}')); // First Hangul syllable (가)
        assert!(is_cjk_char('한')); // Middle Hangul
        assert!(is_cjk_char('\u{D7AF}')); // Last Hangul syllable

        // Just outside Korean range
        assert!(!is_cjk_char('\u{ABFF}')); // Before Hangul
        assert!(!is_cjk_char('\u{D7B0}')); // After Hangul
    }

    #[test]
    fn test_escape_fts5_basic() {
        // No quotes - wrapped in quotes for literal search
        assert_eq!(escape_fts5("hello world"), "\"hello world\"");
        assert_eq!(escape_fts5("JWT authentication"), "\"JWT authentication\"");

        // Single quote (not escaped by this function, only double quotes)
        assert_eq!(escape_fts5("user's task"), "\"user's task\"");
    }

    #[test]
    fn test_escape_fts5_double_quotes() {
        // Single double quote - escaped and wrapped
        assert_eq!(escape_fts5("\"admin\""), "\"\"\"admin\"\"\"");

        // Multiple double quotes
        assert_eq!(
            escape_fts5("\"user\" and \"admin\""),
            "\"\"\"user\"\" and \"\"admin\"\"\""
        );

        // Double quotes at different positions
        assert_eq!(
            escape_fts5("start \"middle\" end"),
            "\"start \"\"middle\"\" end\""
        );
        assert_eq!(escape_fts5("\"start"), "\"\"\"start\"");
        assert_eq!(escape_fts5("end\""), "\"end\"\"\"");
    }

    #[test]
    fn test_escape_fts5_complex_queries() {
        // Mixed quotes and special characters
        assert_eq!(
            escape_fts5("search for \"exact phrase\" here"),
            "\"search for \"\"exact phrase\"\" here\""
        );

        // Empty string - still wrapped
        assert_eq!(escape_fts5(""), "\"\"");

        // Only quotes
        assert_eq!(escape_fts5("\""), "\"\"\"\"");
        assert_eq!(escape_fts5("\"\""), "\"\"\"\"\"\"");
        assert_eq!(escape_fts5("\"\"\""), "\"\"\"\"\"\"\"\"");
    }

    #[test]
    fn test_escape_fts5_special_chars() {
        // FTS5 special characters should be wrapped and treated as literals
        assert_eq!(escape_fts5("#123"), "\"#123\"");
        assert_eq!(escape_fts5("task*"), "\"task*\"");
        assert_eq!(escape_fts5("+keyword"), "\"+keyword\"");
        assert_eq!(escape_fts5("-exclude"), "\"-exclude\"");
        assert_eq!(escape_fts5("a AND b"), "\"a AND b\"");
        assert_eq!(escape_fts5("a OR b"), "\"a OR b\"");
    }

    #[test]
    fn test_escape_fts5_cjk_with_quotes() {
        // CJK text with quotes - escaped and wrapped
        assert_eq!(
            escape_fts5("用户\"管理员\"权限"),
            "\"用户\"\"管理员\"\"权限\""
        );
        assert_eq!(escape_fts5("\"認証\"システム"), "\"\"\"認証\"\"システム\"");

        // Mixed CJK and English with quotes
        assert_eq!(
            escape_fts5("API\"接口\"documentation"),
            "\"API\"\"接口\"\"documentation\""
        );
    }

    #[test]
    fn test_needs_like_fallback_unicode_normalization() {
        // Test with different Unicode representations
        // Most CJK characters don't have composition, but test general behavior

        // Standard CJK characters
        assert!(needs_like_fallback("中"));
        assert!(needs_like_fallback("日"));

        // Two CJK characters
        assert!(needs_like_fallback("中日"));
        assert!(needs_like_fallback("認證"));
    }
}
