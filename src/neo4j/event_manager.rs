use crate::db::models::Event;
use crate::error::{IntentError, Result};
use chrono::Utc;
use neo4rs::{query, Graph};

use super::task_manager::neo4j_err;

/// Event management backed by Neo4j.
///
/// Events are stored as (:Event) nodes with (:Event)-[:BELONGS_TO]->(:Task)
/// relationships. IDs are generated from the shared `next_id` counter.
pub struct Neo4jEventManager {
    graph: Graph,
    project_id: String,
}

impl Neo4jEventManager {
    pub fn new(graph: Graph, project_id: String) -> Self {
        Self { graph, project_id }
    }

    /// Add a new event linked to a task.
    ///
    /// Creates an Event node and a BELONGS_TO relationship to the target task.
    /// Returns TaskNotFound if the task doesn't exist.
    pub async fn add_event(
        &self,
        task_id: i64,
        log_type: &str,
        discussion_data: &str,
    ) -> Result<Event> {
        // Verify task exists
        let mut check = self
            .graph
            .execute(
                query("MATCH (t:Task {project_id: $pid, id: $tid}) RETURN t.id AS id")
                    .param("pid", self.project_id.clone())
                    .param("tid", task_id),
            )
            .await
            .map_err(|e| neo4j_err("add_event check task", e))?;

        if check
            .next()
            .await
            .map_err(|e| neo4j_err("add_event check task fetch", e))?
            .is_none()
        {
            return Err(IntentError::TaskNotFound(task_id));
        }

        let id = super::next_id(&self.graph, &self.project_id, "event").await?;
        let now = Utc::now();
        let timestamp_str = now.to_rfc3339();

        // Create event node + BELONGS_TO relationship in one query
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (t:Task {project_id: $pid, id: $tid}) \
                     CREATE (e:Event {project_id: $pid, id: $eid, task_id: $tid, \
                             log_type: $log_type, discussion_data: $data, \
                             timestamp: $ts})-[:BELONGS_TO]->(t) \
                     RETURN e",
                )
                .param("pid", self.project_id.clone())
                .param("tid", task_id)
                .param("eid", id)
                .param("log_type", log_type.to_string())
                .param("data", discussion_data.to_string())
                .param("ts", timestamp_str),
            )
            .await
            .map_err(|e| neo4j_err("add_event create", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("add_event fetch", e))?
        {
            Some(_) => Ok(Event {
                id,
                task_id,
                timestamp: now,
                log_type: log_type.to_string(),
                discussion_data: discussion_data.to_string(),
            }),
            None => Err(IntentError::OtherError(anyhow::anyhow!(
                "add_event: CREATE did not return a node"
            ))),
        }
    }

    /// List events with optional filters.
    ///
    /// Supports filtering by task_id, log_type, and since (duration string).
    /// Results are ordered by timestamp DESC.
    pub async fn list_events(
        &self,
        task_id: Option<i64>,
        limit: Option<i64>,
        log_type: Option<String>,
        since: Option<String>,
    ) -> Result<Vec<Event>> {
        // Verify task exists if filtering by task_id
        if let Some(tid) = task_id {
            let mut check = self
                .graph
                .execute(
                    query("MATCH (t:Task {project_id: $pid, id: $tid}) RETURN t.id AS id")
                        .param("pid", self.project_id.clone())
                        .param("tid", tid),
                )
                .await
                .map_err(|e| neo4j_err("list_events check task", e))?;

            if check
                .next()
                .await
                .map_err(|e| neo4j_err("list_events check task fetch", e))?
                .is_none()
            {
                return Err(IntentError::TaskNotFound(tid));
            }
        }

        let limit = limit.unwrap_or(50);

        // Parse since duration if provided
        let since_timestamp = if let Some(duration_str) = &since {
            Some(crate::time_utils::parse_duration(duration_str)?)
        } else {
            None
        };

        // Build dynamic WHERE clause
        let mut where_parts = vec!["e.project_id = $pid".to_string()];
        let mut has_task_filter = false;
        let mut has_type_filter = false;
        let mut has_since_filter = false;

        if task_id.is_some() {
            where_parts.push("e.task_id = $filter_task_id".to_string());
            has_task_filter = true;
        }
        if log_type.is_some() {
            where_parts.push("e.log_type = $filter_log_type".to_string());
            has_type_filter = true;
        }
        if since_timestamp.is_some() {
            where_parts.push("e.timestamp >= $filter_since".to_string());
            has_since_filter = true;
        }

        let cypher = format!(
            "MATCH (e:Event) WHERE {} \
             RETURN e \
             ORDER BY e.timestamp DESC \
             LIMIT $limit",
            where_parts.join(" AND ")
        );

        let mut q = query(&cypher)
            .param("pid", self.project_id.clone())
            .param("limit", limit);

        if has_task_filter {
            q = q.param("filter_task_id", task_id.unwrap());
        }
        if has_type_filter {
            q = q.param("filter_log_type", log_type.unwrap());
        }
        if has_since_filter {
            q = q.param("filter_since", since_timestamp.unwrap().to_rfc3339());
        }

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| neo4j_err("list_events query", e))?;

        let mut events = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("list_events iterate", e))?
        {
            let node: neo4rs::Node = row
                .get("e")
                .map_err(|e| neo4j_err("list_events get node", e))?;
            events.push(node_to_event(&node)?);
        }

        Ok(events)
    }
    /// Get events summary for a task: total count + most recent 10 events.
    ///
    /// Uses a single Cypher query with a count subquery to avoid two round-trips.
    pub async fn get_events_summary(
        &self,
        task_id: i64,
    ) -> Result<crate::db::models::EventsSummary> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (e:Event {project_id: $pid, task_id: $tid}) \
                     WITH e ORDER BY e.timestamp DESC \
                     WITH collect(e) AS all_events \
                     RETURN size(all_events) AS cnt, all_events[0..10] AS recent",
                )
                .param("pid", self.project_id.clone())
                .param("tid", task_id),
            )
            .await
            .map_err(|e| neo4j_err("get_events_summary", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("get_events_summary fetch", e))?
        {
            Some(row) => {
                let total_count: i64 = row
                    .get("cnt")
                    .map_err(|e| neo4j_err("get_events_summary cnt", e))?;

                let recent_nodes: Vec<neo4rs::Node> = row
                    .get("recent")
                    .map_err(|e| neo4j_err("get_events_summary recent", e))?;

                let mut recent_events = Vec::with_capacity(recent_nodes.len());
                for node in &recent_nodes {
                    recent_events.push(node_to_event(node)?);
                }

                Ok(crate::db::models::EventsSummary {
                    total_count,
                    recent_events,
                })
            },
            None => Ok(crate::db::models::EventsSummary {
                total_count: 0,
                recent_events: Vec::new(),
            }),
        }
    }
}

/// Convert a Neo4j Event node to an Event struct.
pub(crate) fn node_to_event(node: &neo4rs::Node) -> Result<Event> {
    let id: i64 = node.get("id").map_err(|e| neo4j_err("event.id", e))?;
    let task_id: i64 = node
        .get("task_id")
        .map_err(|e| neo4j_err("event.task_id", e))?;
    let log_type: String = node
        .get("log_type")
        .map_err(|e| neo4j_err("event.log_type", e))?;
    let discussion_data: String = node
        .get("discussion_data")
        .map_err(|e| neo4j_err("event.discussion_data", e))?;

    let timestamp_str: String = node
        .get("timestamp")
        .map_err(|e| neo4j_err("event.timestamp", e))?;
    let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            IntentError::OtherError(anyhow::anyhow!(
                "Failed to parse event timestamp '{}': {}",
                timestamp_str,
                e
            ))
        })?;

    Ok(Event {
        id,
        task_id,
        timestamp,
        log_type,
        discussion_data,
    })
}
