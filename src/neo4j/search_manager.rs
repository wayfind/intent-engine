//! Full-text search backed by Neo4j Lucene indexes.
//!
//! Uses `db.index.fulltext.queryNodes()` for the task_fulltext and event_fulltext
//! indexes created in `schema.rs`. Falls back to `CONTAINS` for very short CJK
//! queries (1-2 chars) that Lucene's StandardAnalyzer may not tokenize well.

use crate::db::models::{Event, PaginatedSearchResults, SearchResult, Task};
use crate::error::Result;
use crate::search::{is_cjk_char, needs_like_fallback};
use neo4rs::{query, Graph};

use super::event_manager::node_to_event;
use super::task_manager::{neo4j_err, node_to_task};

/// Search manager backed by Neo4j fulltext indexes.
pub struct Neo4jSearchManager {
    graph: Graph,
    project_id: String,
}

impl Neo4jSearchManager {
    pub fn new(graph: Graph, project_id: String) -> Self {
        Self { graph, project_id }
    }

    /// Unified search across tasks and events.
    ///
    /// Uses Neo4j fulltext indexes (Lucene) for general queries and
    /// `CONTAINS` for short CJK queries as a safety net.
    pub async fn search(
        &self,
        query_str: &str,
        include_tasks: bool,
        include_events: bool,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PaginatedSearchResults> {
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        // Handle empty or whitespace-only queries
        if query_str.trim().is_empty() {
            return Ok(PaginatedSearchResults {
                results: Vec::new(),
                total_tasks: 0,
                total_events: 0,
                has_more: false,
                limit,
                offset,
            });
        }

        // Handle queries with no searchable content
        let has_searchable = query_str
            .chars()
            .any(|c| c.is_alphanumeric() || is_cjk_char(c));
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

        let use_contains = needs_like_fallback(query_str);

        let mut total_tasks: i64 = 0;
        let mut total_events: i64 = 0;
        let mut all_results: Vec<(SearchResult, f64)> = Vec::new();

        if include_tasks {
            let (tasks, count) = if use_contains {
                self.search_tasks_contains(query_str, limit, offset).await?
            } else {
                self.search_tasks_fulltext(query_str, limit, offset).await?
            };
            total_tasks = count;
            all_results.extend(tasks);
        }

        if include_events {
            let (events, count) = if use_contains {
                self.search_events_contains(query_str, limit, offset)
                    .await?
            } else {
                self.search_events_fulltext(query_str, limit, offset)
                    .await?
            };
            total_events = count;
            all_results.extend(events);
        }

        // Sort by relevance score descending (higher = better for both Lucene and CONTAINS)
        all_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<SearchResult> =
            all_results.into_iter().map(|(result, _)| result).collect();

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

    // ── Fulltext search (Lucene) ──────────────────────────────────

    async fn search_tasks_fulltext(
        &self,
        query_str: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<(SearchResult, f64)>, i64)> {
        // Escape for Lucene: wrap in quotes for literal phrase search
        let lucene_query = format!("\"{}\"", query_str.replace('"', "\\\""));

        // Count
        let count = self.count_task_fulltext_matches(&lucene_query).await?;

        // Fetch
        let mut result = self
            .graph
            .execute(
                query(
                    "CALL db.index.fulltext.queryNodes('task_fulltext', $query) \
                     YIELD node, score \
                     WHERE node.project_id = $pid \
                     RETURN node, score \
                     ORDER BY score DESC \
                     SKIP $offset LIMIT $limit",
                )
                .param("query", lucene_query)
                .param("pid", self.project_id.clone())
                .param("offset", offset)
                .param("limit", limit),
            )
            .await
            .map_err(|e| neo4j_err("search_tasks_fulltext", e))?;

        let mut results = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("search_tasks_fulltext iterate", e))?
        {
            let node: neo4rs::Node = row
                .get("node")
                .map_err(|e| neo4j_err("search_tasks_fulltext node", e))?;
            let score: f64 = row.get("score").unwrap_or(0.0);
            let task = node_to_task(&node)?;
            let (match_field, match_snippet) = build_task_snippet(&task, query_str);

            results.push((
                SearchResult::Task {
                    task,
                    match_snippet,
                    match_field,
                },
                score,
            ));
        }

        Ok((results, count))
    }

    async fn search_events_fulltext(
        &self,
        query_str: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<(SearchResult, f64)>, i64)> {
        let lucene_query = format!("\"{}\"", query_str.replace('"', "\\\""));

        let count = self.count_event_fulltext_matches(&lucene_query).await?;

        let mut result = self
            .graph
            .execute(
                query(
                    "CALL db.index.fulltext.queryNodes('event_fulltext', $query) \
                     YIELD node, score \
                     WHERE node.project_id = $pid \
                     RETURN node, score \
                     ORDER BY score DESC \
                     SKIP $offset LIMIT $limit",
                )
                .param("query", lucene_query)
                .param("pid", self.project_id.clone())
                .param("offset", offset)
                .param("limit", limit),
            )
            .await
            .map_err(|e| neo4j_err("search_events_fulltext", e))?;

        let task_mgr = super::Neo4jTaskManager::new(self.graph.clone(), self.project_id.clone());

        let mut results = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("search_events_fulltext iterate", e))?
        {
            let node: neo4rs::Node = row
                .get("node")
                .map_err(|e| neo4j_err("search_events_fulltext node", e))?;
            let score: f64 = row.get("score").unwrap_or(0.0);
            let event = node_to_event(&node)?;
            let match_snippet = build_event_snippet(&event, query_str);
            let task_chain = task_mgr.get_task_ancestry(event.task_id).await?;

            results.push((
                SearchResult::Event {
                    event,
                    task_chain,
                    match_snippet,
                },
                score,
            ));
        }

        Ok((results, count))
    }

    // ── CONTAINS fallback (short CJK) ────────────────────────────

    async fn search_tasks_contains(
        &self,
        query_str: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<(SearchResult, f64)>, i64)> {
        // Count
        let mut count_result = self
            .graph
            .execute(
                query(
                    "MATCH (t:Task {project_id: $pid}) \
                     WHERE t.name CONTAINS $q OR t.spec CONTAINS $q \
                     RETURN count(t) AS cnt",
                )
                .param("pid", self.project_id.clone())
                .param("q", query_str.to_string()),
            )
            .await
            .map_err(|e| neo4j_err("search_tasks_contains count", e))?;

        let count: i64 = count_result
            .next()
            .await
            .map_err(|e| neo4j_err("search_tasks_contains count fetch", e))?
            .and_then(|row| row.get::<i64>("cnt").ok())
            .unwrap_or(0);

        // Fetch
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (t:Task {project_id: $pid}) \
                     WHERE t.name CONTAINS $q OR t.spec CONTAINS $q \
                     RETURN t \
                     ORDER BY t.id ASC \
                     SKIP $offset LIMIT $limit",
                )
                .param("pid", self.project_id.clone())
                .param("q", query_str.to_string())
                .param("offset", offset)
                .param("limit", limit),
            )
            .await
            .map_err(|e| neo4j_err("search_tasks_contains query", e))?;

        let mut results = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("search_tasks_contains iterate", e))?
        {
            let task = super::task_manager::row_to_task(&row, "t")?;
            let (match_field, match_snippet) = build_task_snippet(&task, query_str);

            results.push((
                SearchResult::Task {
                    task,
                    match_snippet,
                    match_field,
                },
                1.0,
            ));
        }

        Ok((results, count))
    }

    async fn search_events_contains(
        &self,
        query_str: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<(SearchResult, f64)>, i64)> {
        let mut count_result = self
            .graph
            .execute(
                query(
                    "MATCH (e:Event {project_id: $pid}) \
                     WHERE e.discussion_data CONTAINS $q \
                     RETURN count(e) AS cnt",
                )
                .param("pid", self.project_id.clone())
                .param("q", query_str.to_string()),
            )
            .await
            .map_err(|e| neo4j_err("search_events_contains count", e))?;

        let count: i64 = count_result
            .next()
            .await
            .map_err(|e| neo4j_err("search_events_contains count fetch", e))?
            .and_then(|row| row.get::<i64>("cnt").ok())
            .unwrap_or(0);

        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (e:Event {project_id: $pid}) \
                     WHERE e.discussion_data CONTAINS $q \
                     RETURN e \
                     ORDER BY e.id ASC \
                     SKIP $offset LIMIT $limit",
                )
                .param("pid", self.project_id.clone())
                .param("q", query_str.to_string())
                .param("offset", offset)
                .param("limit", limit),
            )
            .await
            .map_err(|e| neo4j_err("search_events_contains query", e))?;

        let task_mgr = super::Neo4jTaskManager::new(self.graph.clone(), self.project_id.clone());

        let mut results = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("search_events_contains iterate", e))?
        {
            let node: neo4rs::Node = row
                .get("e")
                .map_err(|e| neo4j_err("search_events_contains node", e))?;
            let event = node_to_event(&node)?;
            let match_snippet = build_event_snippet(&event, query_str);
            let task_chain = task_mgr.get_task_ancestry(event.task_id).await?;

            results.push((
                SearchResult::Event {
                    event,
                    task_chain,
                    match_snippet,
                },
                1.0,
            ));
        }

        Ok((results, count))
    }

    // ── Count helpers ─────────────────────────────────────────────

    async fn count_task_fulltext_matches(&self, lucene_query: &str) -> Result<i64> {
        let mut result = self
            .graph
            .execute(
                query(
                    "CALL db.index.fulltext.queryNodes('task_fulltext', $query) \
                     YIELD node \
                     WHERE node.project_id = $pid \
                     RETURN count(node) AS cnt",
                )
                .param("query", lucene_query.to_string())
                .param("pid", self.project_id.clone()),
            )
            .await
            .map_err(|e| neo4j_err("count_task_fulltext", e))?;

        Ok(result
            .next()
            .await
            .map_err(|e| neo4j_err("count_task_fulltext fetch", e))?
            .and_then(|row| row.get::<i64>("cnt").ok())
            .unwrap_or(0))
    }

    async fn count_event_fulltext_matches(&self, lucene_query: &str) -> Result<i64> {
        let mut result = self
            .graph
            .execute(
                query(
                    "CALL db.index.fulltext.queryNodes('event_fulltext', $query) \
                     YIELD node \
                     WHERE node.project_id = $pid \
                     RETURN count(node) AS cnt",
                )
                .param("query", lucene_query.to_string())
                .param("pid", self.project_id.clone()),
            )
            .await
            .map_err(|e| neo4j_err("count_event_fulltext", e))?;

        Ok(result
            .next()
            .await
            .map_err(|e| neo4j_err("count_event_fulltext fetch", e))?
            .and_then(|row| row.get::<i64>("cnt").ok())
            .unwrap_or(0))
    }
}

// ── Snippet helpers ─────────────────────────────────────────────

/// Build a match snippet for a task result.
///
/// Since Neo4j fulltext doesn't provide snippet(), we manually find the
/// query string in the task's name/spec and return context around it.
fn build_task_snippet(task: &Task, query_str: &str) -> (String, String) {
    let query_lower = query_str.to_lowercase();

    // Check name first
    if task.name.to_lowercase().contains(&query_lower) {
        return ("name".to_string(), task.name.clone());
    }

    // Check spec
    if let Some(ref spec) = task.spec {
        if spec.to_lowercase().contains(&query_lower) {
            return ("spec".to_string(), build_context_snippet(spec, query_str));
        }
    }

    // Fallback: return name
    ("name".to_string(), task.name.clone())
}

/// Build a match snippet for an event result.
fn build_event_snippet(event: &Event, query_str: &str) -> String {
    build_context_snippet(&event.discussion_data, query_str)
}

/// Extract a context window around the first occurrence of `needle` in `haystack`.
/// Returns up to ~120 chars of context centered on the match.
fn build_context_snippet(haystack: &str, needle: &str) -> String {
    let needle_lower = needle.to_lowercase();
    let haystack_lower = haystack.to_lowercase();

    if let Some(pos) = haystack_lower.find(&needle_lower) {
        let chars: Vec<char> = haystack.chars().collect();
        let char_pos = haystack[..pos].chars().count();
        let context_chars = 60;

        let start = char_pos.saturating_sub(context_chars);
        let end = (char_pos + needle.chars().count() + context_chars).min(chars.len());

        let snippet: String = chars[start..end].iter().collect();

        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < chars.len() { "..." } else { "" };

        format!("{}{}{}", prefix, snippet, suffix)
    } else {
        // No match found — return truncated haystack
        let chars: Vec<char> = haystack.chars().collect();
        if chars.len() > 120 {
            let truncated: String = chars[..117].iter().collect();
            format!("{}...", truncated)
        } else {
            haystack.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_context_snippet_found() {
        let text = "This is a long text with the keyword somewhere in the middle of it and some more text after";
        let snippet = build_context_snippet(text, "keyword");
        assert!(snippet.contains("keyword"));
    }

    #[test]
    fn test_build_context_snippet_not_found() {
        let text = "Short text without the search term";
        let snippet = build_context_snippet(text, "nonexistent");
        assert_eq!(snippet, text);
    }

    #[test]
    fn test_build_context_snippet_short() {
        let text = "hello";
        let snippet = build_context_snippet(text, "hello");
        assert_eq!(snippet, "hello");
    }

    #[test]
    fn test_build_context_snippet_truncation() {
        let long_text: String = "x".repeat(200);
        let snippet = build_context_snippet(&long_text, "nonexistent");
        assert!(snippet.len() <= 130); // 117 chars + "..."
    }

    #[test]
    fn test_build_task_snippet_name_match() {
        let task = Task {
            id: 1,
            parent_id: None,
            name: "Fix authentication bug".to_string(),
            spec: Some("Detailed spec here".to_string()),
            status: "todo".to_string(),
            complexity: None,
            priority: None,
            first_todo_at: None,
            first_doing_at: None,
            first_done_at: None,
            active_form: None,
            owner: "human".to_string(),
            metadata: None,
        };
        let (field, snippet) = build_task_snippet(&task, "authentication");
        assert_eq!(field, "name");
        assert!(snippet.contains("authentication"));
    }

    #[test]
    fn test_build_task_snippet_spec_match() {
        let task = Task {
            id: 1,
            parent_id: None,
            name: "Task A".to_string(),
            spec: Some("Use JWT tokens for user authentication".to_string()),
            status: "todo".to_string(),
            complexity: None,
            priority: None,
            first_todo_at: None,
            first_doing_at: None,
            first_done_at: None,
            active_form: None,
            owner: "human".to_string(),
            metadata: None,
        };
        let (field, _snippet) = build_task_snippet(&task, "JWT");
        assert_eq!(field, "spec");
    }
}
