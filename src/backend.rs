//! Backend trait abstractions for SQLite and Neo4j.
//!
//! These traits define the shared interface that both storage backends implement.
//! Methods that exist only on one backend (e.g. SQLite's `spawn_subtask`, Neo4j's
//! `delete_task_cascade`) remain on their concrete structs.

use std::future::Future;

use crate::db::models::{
    DoneTaskResponse, Event, PaginatedTasks, PickNextResponse, StatusResponse, Task, TaskContext,
    TaskSortBy, TaskWithEvents,
};
use crate::error::Result;
use crate::plan::{PlanRequest, PlanResult};
use crate::tasks::TaskUpdate;
use crate::workspace::CurrentTaskResponse;

/// Task CRUD + lifecycle operations.
pub trait TaskBackend: Send + Sync {
    // ── Read ────────────────────────────────────────────────────────

    fn get_task(&self, id: i64) -> impl Future<Output = Result<Task>> + Send;

    fn get_task_with_events(&self, id: i64) -> impl Future<Output = Result<TaskWithEvents>> + Send;

    fn get_task_ancestry(&self, task_id: i64) -> impl Future<Output = Result<Vec<Task>>> + Send;

    fn get_task_context(&self, id: i64) -> impl Future<Output = Result<TaskContext>> + Send;

    fn get_siblings(
        &self,
        id: i64,
        parent_id: Option<i64>,
    ) -> impl Future<Output = Result<Vec<Task>>> + Send;

    fn get_children(&self, id: i64) -> impl Future<Output = Result<Vec<Task>>> + Send;

    fn get_blocking_tasks(&self, id: i64) -> impl Future<Output = Result<Vec<Task>>> + Send;

    fn get_blocked_by_tasks(&self, id: i64) -> impl Future<Output = Result<Vec<Task>>> + Send;

    fn get_descendants(&self, task_id: i64) -> impl Future<Output = Result<Vec<Task>>> + Send;

    fn get_status(
        &self,
        task_id: i64,
        with_events: bool,
    ) -> impl Future<Output = Result<StatusResponse>> + Send;

    fn get_root_tasks(&self) -> impl Future<Output = Result<Vec<Task>>> + Send;

    fn find_tasks(
        &self,
        status: Option<&str>,
        parent_id: Option<Option<i64>>,
        sort_by: Option<TaskSortBy>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> impl Future<Output = Result<PaginatedTasks>> + Send;

    // ── Write ───────────────────────────────────────────────────────

    fn add_task(
        &self,
        name: &str,
        spec: Option<&str>,
        parent_id: Option<i64>,
        owner: Option<&str>,
        priority: Option<i32>,
        metadata: Option<&str>,
    ) -> impl Future<Output = Result<Task>> + Send;

    fn update_task(
        &self,
        id: i64,
        update: TaskUpdate<'_>,
    ) -> impl Future<Output = Result<Task>> + Send;

    fn delete_task(&self, id: i64) -> impl Future<Output = Result<()>> + Send;

    // ── Lifecycle ───────────────────────────────────────────────────

    fn start_task(
        &self,
        id: i64,
        with_events: bool,
    ) -> impl Future<Output = Result<TaskWithEvents>> + Send;

    fn done_task(
        &self,
        is_ai_caller: bool,
    ) -> impl Future<Output = Result<DoneTaskResponse>> + Send;

    fn done_task_by_id(
        &self,
        id: i64,
        is_ai_caller: bool,
    ) -> impl Future<Output = Result<DoneTaskResponse>> + Send;

    fn pick_next(&self) -> impl Future<Output = Result<PickNextResponse>> + Send;
}

/// Session/workspace focus management.
pub trait WorkspaceBackend: Send + Sync {
    fn get_current_task(
        &self,
        session_id: Option<&str>,
    ) -> impl Future<Output = Result<CurrentTaskResponse>> + Send;

    fn set_current_task(
        &self,
        task_id: i64,
        session_id: Option<&str>,
    ) -> impl Future<Output = Result<CurrentTaskResponse>> + Send;

    fn clear_current_task(
        &self,
        session_id: Option<&str>,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// Event (decision log) operations.
pub trait EventBackend: Send + Sync {
    fn add_event(
        &self,
        task_id: i64,
        log_type: &str,
        discussion_data: &str,
    ) -> impl Future<Output = Result<Event>> + Send;

    fn list_events(
        &self,
        task_id: Option<i64>,
        limit: Option<i64>,
        log_type: Option<String>,
        since: Option<String>,
    ) -> impl Future<Output = Result<Vec<Event>>> + Send;
}

/// Batch plan execution.
pub trait PlanBackend: Send + Sync {
    fn execute(&self, request: &PlanRequest) -> impl Future<Output = Result<PlanResult>> + Send;
}
