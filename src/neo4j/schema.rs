use crate::error::{IntentError, Result};
use neo4rs::{query, Graph};

/// Current schema version. Bump this when schema changes.
const SCHEMA_VERSION: &str = "0.1.0";

/// Ensure all Neo4j constraints, indexes, and counter nodes exist.
///
/// Uses a `SchemaVersion` marker node to skip redundant DDL on repeated invocations.
/// This is idempotent — safe to call on every startup, but avoids paying the cost
/// of 7+ DDL statements when the schema is already current.
pub async fn ensure_schema(graph: &Graph, project_id: &str) -> Result<()> {
    // Check if schema is already at current version
    if is_schema_current(graph, project_id).await? {
        return Ok(());
    }

    // ── Uniqueness constraints ──────────────────────────────────
    run_ddl(
        graph,
        "CREATE CONSTRAINT task_unique IF NOT EXISTS \
         FOR (t:Task) REQUIRE (t.project_id, t.id) IS UNIQUE",
    )
    .await?;

    run_ddl(
        graph,
        "CREATE CONSTRAINT event_unique IF NOT EXISTS \
         FOR (e:Event) REQUIRE (e.project_id, e.id) IS UNIQUE",
    )
    .await?;

    run_ddl(
        graph,
        "CREATE CONSTRAINT session_unique IF NOT EXISTS \
         FOR (s:Session) REQUIRE (s.project_id, s.session_id) IS UNIQUE",
    )
    .await?;

    run_ddl(
        graph,
        "CREATE CONSTRAINT counter_unique IF NOT EXISTS \
         FOR (c:Counter) REQUIRE (c.project_id, c.entity) IS UNIQUE",
    )
    .await?;

    // ── Fulltext indexes ────────────────────────────────────────
    run_ddl(
        graph,
        "CREATE FULLTEXT INDEX task_fulltext IF NOT EXISTS \
         FOR (t:Task) ON EACH [t.name, t.spec]",
    )
    .await?;

    run_ddl(
        graph,
        "CREATE FULLTEXT INDEX event_fulltext IF NOT EXISTS \
         FOR (e:Event) ON EACH [e.discussion_data]",
    )
    .await?;

    // Note: Vector index is NOT auto-created. The embedding dimension depends on
    // the model used (768 for MiniLM, 1024 for larger models). Create it explicitly
    // via `ie-neo4j init --vector-dims <N>` when the embedding pipeline is configured.

    // ── Ensure counter nodes exist ──────────────────────────────
    graph
        .run(
            query(
                "MERGE (c:Counter {project_id: $pid, entity: 'task'}) \
                 ON CREATE SET c.next_id = 1",
            )
            .param("pid", project_id.to_string()),
        )
        .await
        .map_err(|e| neo4j_err("create task counter", e))?;

    graph
        .run(
            query(
                "MERGE (c:Counter {project_id: $pid, entity: 'event'}) \
                 ON CREATE SET c.next_id = 1",
            )
            .param("pid", project_id.to_string()),
        )
        .await
        .map_err(|e| neo4j_err("create event counter", e))?;

    // ── Stamp schema version ────────────────────────────────────
    graph
        .run(
            query(
                "MERGE (sv:SchemaVersion {project_id: $pid}) \
                 SET sv.version = $version",
            )
            .param("pid", project_id.to_string())
            .param("version", SCHEMA_VERSION.to_string()),
        )
        .await
        .map_err(|e| neo4j_err("stamp schema version", e))?;

    Ok(())
}

/// Check if the schema version marker matches the current version.
async fn is_schema_current(graph: &Graph, project_id: &str) -> Result<bool> {
    let mut result = graph
        .execute(
            query(
                "OPTIONAL MATCH (sv:SchemaVersion {project_id: $pid}) \
                 RETURN sv.version AS version",
            )
            .param("pid", project_id.to_string()),
        )
        .await
        .map_err(|e| neo4j_err("check schema version", e))?;

    match result
        .next()
        .await
        .map_err(|e| neo4j_err("read schema version", e))?
    {
        Some(row) => {
            let version: Option<String> = row.get("version").ok();
            Ok(version.as_deref() == Some(SCHEMA_VERSION))
        },
        None => Ok(false),
    }
}

/// Run a DDL statement, failing hard on real errors.
/// `IF NOT EXISTS` in the Cypher handles idempotency — if Neo4j returns an error
/// despite that clause, it's a real problem (permissions, syntax, server version).
async fn run_ddl(graph: &Graph, cypher: &str) -> Result<()> {
    graph.run(query(cypher)).await.map_err(|e| {
        neo4j_err(
            &format!("schema DDL: {}", &cypher[..cypher.len().min(60)]),
            e,
        )
    })
}

/// Convert a neo4rs error into an IntentError with context.
fn neo4j_err(context: &str, e: impl std::fmt::Display) -> IntentError {
    IntentError::OtherError(anyhow::anyhow!("Neo4j {}: {}", context, e))
}
