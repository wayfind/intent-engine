use crate::error::Result;
use crate::workspace::CurrentTaskResponse;
use neo4rs::{query, Graph};

use super::task_manager::neo4j_err;

/// Workspace/session management backed by Neo4j Session nodes.
///
/// Reuses `crate::workspace::CurrentTaskResponse` to avoid type duplication.
/// Session ID resolution delegates to `crate::workspace::resolve_session_id`
/// to ensure consistent behavior with the SQLite backend.
pub struct Neo4jWorkspaceManager {
    graph: Graph,
    project_id: String,
}

impl Neo4jWorkspaceManager {
    pub fn new(graph: Graph, project_id: String) -> Self {
        Self { graph, project_id }
    }

    /// Get the current focused task for a session.
    pub async fn get_current_task(&self, session_id: Option<&str>) -> Result<CurrentTaskResponse> {
        let session_id = crate::workspace::resolve_session_id(session_id);

        let mut result = self
            .graph
            .execute(
                query(
                    "OPTIONAL MATCH (s:Session {project_id: $pid, session_id: $sid}) \
                     OPTIONAL MATCH (t:Task {project_id: $pid, id: s.current_task_id}) \
                     RETURN s.current_task_id AS current_task_id, t",
                )
                .param("pid", self.project_id.clone())
                .param("sid", session_id.clone()),
            )
            .await
            .map_err(|e| neo4j_err("get_current_task query", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("get_current_task fetch", e))?
        {
            Some(row) => {
                let current_task_id: Option<i64> = row.get("current_task_id").ok();
                let task = row
                    .get::<neo4rs::Node>("t")
                    .ok()
                    .and_then(|node| super::task_manager::node_to_task(&node).ok());

                Ok(CurrentTaskResponse {
                    current_task_id,
                    task,
                    session_id: Some(session_id),
                })
            },
            None => Ok(CurrentTaskResponse {
                current_task_id: None,
                task: None,
                session_id: Some(session_id),
            }),
        }
    }

    /// Set the current focused task for a session.
    pub async fn set_current_task(&self, task_id: i64, session_id: Option<&str>) -> Result<()> {
        let session_id = crate::workspace::resolve_session_id(session_id);

        self.graph
            .run(
                query(
                    "MERGE (s:Session {project_id: $pid, session_id: $sid}) \
                     ON CREATE SET s.created_at = datetime(), s.last_active_at = datetime() \
                     ON MATCH SET s.last_active_at = datetime() \
                     SET s.current_task_id = $tid",
                )
                .param("pid", self.project_id.clone())
                .param("sid", session_id)
                .param("tid", task_id),
            )
            .await
            .map_err(|e| neo4j_err("set_current_task", e))?;

        Ok(())
    }

    /// Clear the current focused task for a session.
    pub async fn clear_current_task(&self, session_id: Option<&str>) -> Result<()> {
        let session_id = crate::workspace::resolve_session_id(session_id);

        self.graph
            .run(
                query(
                    "MATCH (s:Session {project_id: $pid, session_id: $sid}) \
                     SET s.current_task_id = null, s.last_active_at = datetime()",
                )
                .param("pid", self.project_id.clone())
                .param("sid", session_id),
            )
            .await
            .map_err(|e| neo4j_err("clear_current_task", e))?;

        Ok(())
    }
}
