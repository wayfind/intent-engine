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
/// FTS5 queries support advanced syntax (AND, OR, NOT, *, "phrase search", etc.).
/// This function only escapes double quotes, which is the most common case where
/// user input needs escaping.
///
/// # Arguments
/// * `query` - The query string to escape
///
/// # Returns
/// The escaped query string with double quotes escaped as `""`
///
/// # Example
/// ```ignore
/// use crate::search::escape_fts5;
///
/// let escaped = escape_fts5("user \"admin\" role");
/// assert_eq!(escaped, "user \"\"admin\"\" role");
/// ```
pub fn escape_fts5(query: &str) -> String {
    query.replace('"', "\"\"")
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
}

// ============================================================================
// Unified Search
// ============================================================================

use crate::db::models::UnifiedSearchResult;
use crate::error::Result;
use crate::events::EventManager;
use crate::tasks::TaskManager;
use sqlx::SqlitePool;

pub struct SearchManager<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SearchManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Unified search across tasks and events
    ///
    /// # Parameters
    /// - `query`: FTS5 search query string
    /// - `include_tasks`: Whether to search in tasks
    /// - `include_events`: Whether to search in events
    /// - `limit`: Maximum number of total results (default: 20)
    ///
    /// # Returns
    /// A mixed vector of task and event search results, ordered by relevance (FTS5 rank)
    pub async fn unified_search(
        &self,
        query: &str,
        include_tasks: bool,
        include_events: bool,
        limit: Option<i64>,
    ) -> Result<Vec<UnifiedSearchResult>> {
        let total_limit = limit.unwrap_or(20);
        let mut results = Vec::new();

        // Calculate limits for each source
        let (task_limit, event_limit) = match (include_tasks, include_events) {
            (true, true) => (total_limit / 2, total_limit / 2),
            (true, false) => (total_limit, 0),
            (false, true) => (0, total_limit),
            (false, false) => return Ok(results), // Early return if nothing to search
        };

        // Search tasks if enabled
        if include_tasks && task_limit > 0 {
            let task_mgr = TaskManager::new(self.pool);
            let mut task_results = task_mgr.search_tasks(query).await?;

            // Apply limit
            task_results.truncate(task_limit as usize);

            for task_result in task_results {
                // Determine which field matched based on snippet content
                let match_field = if task_result
                    .match_snippet
                    .to_lowercase()
                    .contains(&task_result.task.name.to_lowercase())
                {
                    "name".to_string()
                } else {
                    "spec".to_string()
                };

                results.push(UnifiedSearchResult::Task {
                    task: task_result.task,
                    match_snippet: task_result.match_snippet,
                    match_field,
                });
            }
        }

        // Search events if enabled
        if include_events && event_limit > 0 {
            let event_mgr = EventManager::new(self.pool);
            let event_results = event_mgr
                .search_events_fts5(query, Some(event_limit))
                .await?;

            let task_mgr = TaskManager::new(self.pool);
            for event_result in event_results {
                // Get task ancestry chain for this event
                let task_chain = task_mgr
                    .get_task_ancestry(event_result.event.task_id)
                    .await?;

                results.push(UnifiedSearchResult::Event {
                    event: event_result.event,
                    task_chain,
                    match_snippet: event_result.match_snippet,
                });
            }
        }

        // Limit to total_limit (in case we got more from both sources)
        results.truncate(total_limit as usize);

        Ok(results)
    }
}

#[cfg(test)]
mod unified_search_tests {
    use super::*;
    use crate::test_utils::test_helpers::TestContext;

    #[tokio::test]
    async fn test_unified_search_basic() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());
        let search_mgr = SearchManager::new(ctx.pool());

        // Create test task
        let task = task_mgr
            .add_task("JWT Authentication", Some("Implement JWT auth"), None)
            .await
            .unwrap();

        // Add test event
        event_mgr
            .add_event(task.id, "decision", "Chose JWT over OAuth")
            .await
            .unwrap();

        // Search for "JWT" - should find both task and event
        let results = search_mgr
            .unified_search("JWT", true, true, None)
            .await
            .unwrap();

        assert!(results.len() >= 2);

        // Verify we got both task and event results
        let has_task = results
            .iter()
            .any(|r| matches!(r, UnifiedSearchResult::Task { .. }));
        let has_event = results
            .iter()
            .any(|r| matches!(r, UnifiedSearchResult::Event { .. }));

        assert!(has_task);
        assert!(has_event);
    }

    #[tokio::test]
    async fn test_unified_search_tasks_only() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let search_mgr = SearchManager::new(ctx.pool());

        // Create test task
        task_mgr
            .add_task("OAuth Implementation", None, None)
            .await
            .unwrap();

        // Search tasks only
        let results = search_mgr
            .unified_search("OAuth", true, false, None)
            .await
            .unwrap();

        assert!(!results.is_empty());

        // All results should be tasks
        for result in results {
            assert!(matches!(result, UnifiedSearchResult::Task { .. }));
        }
    }

    #[tokio::test]
    async fn test_unified_search_events_only() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());
        let search_mgr = SearchManager::new(ctx.pool());

        // Create test task and event
        let task = task_mgr.add_task("Test task", None, None).await.unwrap();

        event_mgr
            .add_event(task.id, "blocker", "OAuth library missing")
            .await
            .unwrap();

        // Search events only
        let results = search_mgr
            .unified_search("OAuth", false, true, None)
            .await
            .unwrap();

        assert!(!results.is_empty());

        // All results should be events
        for result in results {
            assert!(matches!(result, UnifiedSearchResult::Event { .. }));
        }
    }

    #[tokio::test]
    async fn test_unified_search_with_limit() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let search_mgr = SearchManager::new(ctx.pool());

        // Create multiple test tasks
        for i in 0..10 {
            task_mgr
                .add_task(&format!("Test task {}", i), None, None)
                .await
                .unwrap();
        }

        // Search with limit of 3
        let results = search_mgr
            .unified_search("Test", true, true, Some(3))
            .await
            .unwrap();

        assert!(results.len() <= 3);
    }
}
