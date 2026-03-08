//! Neo4j graph database backend for intent-engine.
//!
//! This module provides a complete Neo4j-backed implementation that reuses
//! the same `Task`, `Event`, and CLI types from the main crate, but stores
//! everything in Neo4j instead of SQLite.
//!
//! Activated by the `neo4j` feature flag; used exclusively by the `ie-neo4j` binary.

pub mod config;
pub mod event_manager;
pub mod plan_executor;
pub mod schema;
pub mod search_manager;
pub mod task_manager;
pub mod workspace_manager;

use crate::error::{IntentError, Result};
use neo4rs::{query, Graph};

pub use config::Neo4jConfig;
pub use event_manager::Neo4jEventManager;
pub use plan_executor::Neo4jPlanExecutor;
pub use search_manager::Neo4jSearchManager;
pub use task_manager::Neo4jTaskManager;
pub use workspace_manager::Neo4jWorkspaceManager;

/// Atomically allocate the next sequential ID for an entity type.
///
/// Shared by task_manager and event_manager. Uses Counter nodes
/// with atomic SET to guarantee uniqueness under concurrent access.
///
/// **Concurrency assumption**: This relies on Neo4j write transactions being
/// serialised at the server level (which holds for single-instance Community
/// and Enterprise editions). On a Causal Cluster with multiple write replicas
/// (rare in practice), the `SET c.next_id = c.next_id + 1` is NOT atomic
/// across replicas and could produce duplicate IDs. Re-evaluate if this
/// codebase is ever deployed against a multi-primary cluster.
///
/// If the Counter node is missing (e.g. accidentally deleted), automatically
/// rebuilds it from the current maximum ID in the data, then retries once.
pub(crate) async fn next_id(graph: &Graph, project_id: &str, entity: &str) -> Result<i64> {
    if let Some(id) = try_next_id(graph, project_id, entity).await? {
        return Ok(id);
    }
    // Counter node missing — rebuild from existing data and retry once
    rebuild_counter(graph, project_id, entity).await?;
    try_next_id(graph, project_id, entity)
        .await?
        .ok_or_else(|| {
            IntentError::OtherError(anyhow::anyhow!(
                "Counter rebuild failed for entity '{}'",
                entity
            ))
        })
}

/// Attempt to atomically fetch-and-increment a Counter node.
/// Returns `Ok(None)` if the Counter node does not exist.
async fn try_next_id(graph: &Graph, project_id: &str, entity: &str) -> Result<Option<i64>> {
    let mut result = graph
        .execute(
            query(
                "MATCH (c:Counter {project_id: $pid, entity: $entity}) \
                 SET c.next_id = c.next_id + 1 \
                 RETURN c.next_id - 1 AS id",
            )
            .param("pid", project_id.to_string())
            .param("entity", entity.to_string()),
        )
        .await
        .map_err(|e| {
            IntentError::OtherError(anyhow::anyhow!("Neo4j try_next_id({}): {}", entity, e))
        })?;

    match result.next().await.map_err(|e| {
        IntentError::OtherError(anyhow::anyhow!(
            "Neo4j try_next_id({}) fetch: {}",
            entity,
            e
        ))
    })? {
        Some(row) => {
            let id: i64 = row.get("id").map_err(|e| {
                IntentError::OtherError(anyhow::anyhow!(
                    "Neo4j try_next_id({}) value: {}",
                    entity,
                    e
                ))
            })?;
            Ok(Some(id))
        },
        None => Ok(None),
    }
}

/// Rebuild a missing Counter node by computing MAX(id)+1 from existing data.
///
/// Uses MERGE so it is safe to call even if the Counter was recreated
/// concurrently — in that case it only bumps the counter up, never down.
async fn rebuild_counter(graph: &Graph, project_id: &str, entity: &str) -> Result<()> {
    let node_label = match entity {
        "task" => "Task",
        "event" => "Event",
        other => {
            return Err(IntentError::OtherError(anyhow::anyhow!(
                "Unknown counter entity: {}",
                other
            )))
        },
    };

    graph
        .run(
            query(&format!(
                "OPTIONAL MATCH (n:{node_label} {{project_id: $pid}}) \
                 WITH COALESCE(MAX(n.id) + 1, 1) AS start_id \
                 MERGE (c:Counter {{project_id: $pid, entity: $entity}}) \
                 ON CREATE SET c.next_id = start_id \
                 ON MATCH SET c.next_id = CASE \
                     WHEN c.next_id <= start_id THEN start_id \
                     ELSE c.next_id END"
            ))
            .param("pid", project_id.to_string())
            .param("entity", entity.to_string()),
        )
        .await
        .map_err(|e| {
            IntentError::OtherError(anyhow::anyhow!("rebuild_counter({}): {}", entity, e))
        })?;

    Ok(())
}

/// Central context holding the Neo4j graph connection and project identity.
pub struct Neo4jContext {
    pub graph: Graph,
    pub project_id: String,
}

impl Neo4jContext {
    /// Connect to Neo4j using environment variable configuration,
    /// then ensure the schema (constraints, indexes, counters) exists.
    pub async fn connect() -> Result<Self> {
        let config = Neo4jConfig::from_env()?;

        let graph = Graph::new(&config.uri, &config.user, &config.password)
            .await
            .map_err(|e| {
                IntentError::OtherError(anyhow::anyhow!("Neo4j connection failed: {}", e))
            })?;

        // Ensure schema on every startup (idempotent)
        schema::ensure_schema(&graph, &config.project_id).await?;

        Ok(Self {
            graph,
            project_id: config.project_id,
        })
    }

    /// Create a Neo4jTaskManager from this context.
    pub fn task_manager(&self) -> Neo4jTaskManager {
        Neo4jTaskManager::new(self.graph.clone(), self.project_id.clone())
    }

    /// Create a Neo4jWorkspaceManager from this context.
    pub fn workspace_manager(&self) -> Neo4jWorkspaceManager {
        Neo4jWorkspaceManager::new(self.graph.clone(), self.project_id.clone())
    }

    /// Create a Neo4jEventManager from this context.
    pub fn event_manager(&self) -> Neo4jEventManager {
        Neo4jEventManager::new(self.graph.clone(), self.project_id.clone())
    }

    /// Create a Neo4jPlanExecutor from this context.
    pub fn plan_executor(&self) -> Neo4jPlanExecutor {
        Neo4jPlanExecutor::new(self.graph.clone(), self.project_id.clone())
    }

    /// Create a Neo4jSearchManager from this context.
    pub fn search_manager(&self) -> Neo4jSearchManager {
        Neo4jSearchManager::new(self.graph.clone(), self.project_id.clone())
    }
}
