use crate::db::models::{DateRange, Event, Report, ReportSummary, StatusBreakdown, Task};
use crate::error::Result;
use chrono::Utc;
use sqlx::SqlitePool;

pub struct ReportManager<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ReportManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Generate a report with optional filters
    pub async fn generate_report(
        &self,
        since: Option<String>,
        status: Option<String>,
        filter_name: Option<String>,
        filter_spec: Option<String>,
        summary_only: bool,
    ) -> Result<Report> {
        // Parse duration if provided
        let since_datetime = since.and_then(|s| crate::time_utils::parse_duration(&s).ok());

        // Build task query
        let mut task_query = String::from("SELECT id FROM tasks WHERE 1=1");
        let mut task_conditions = Vec::new();

        if let Some(ref status) = status {
            task_query.push_str(" AND status = ?");
            task_conditions.push(status.clone());
        }

        if let Some(ref dt) = since_datetime {
            task_query.push_str(" AND first_todo_at >= ?");
            task_conditions.push(dt.to_rfc3339());
        }

        // Add FTS5 filters
        let task_ids = if filter_name.is_some() || filter_spec.is_some() {
            self.filter_tasks_by_fts(&filter_name, &filter_spec).await?
        } else {
            Vec::new()
        };

        // If FTS filters were applied, intersect with other filters
        let tasks = if !task_ids.is_empty() {
            task_query.push_str(&format!(
                " AND id IN ({})",
                task_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
            ));
            let full_query = task_query.replace("SELECT id", "SELECT id, parent_id, name, NULL as spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner");
            let mut q = sqlx::query_as::<_, Task>(&full_query);
            for cond in &task_conditions {
                q = q.bind(cond);
            }
            for id in &task_ids {
                q = q.bind(id);
            }
            q.fetch_all(self.pool).await?
        } else if filter_name.is_none() && filter_spec.is_none() {
            let full_query = task_query.replace("SELECT id", "SELECT id, parent_id, name, NULL as spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner");
            let mut q = sqlx::query_as::<_, Task>(&full_query);
            for cond in &task_conditions {
                q = q.bind(cond);
            }
            q.fetch_all(self.pool).await?
        } else {
            Vec::new()
        };

        // Count tasks by status from filtered results
        let todo_count = tasks.iter().filter(|t| t.status == "todo").count() as i64;
        let doing_count = tasks.iter().filter(|t| t.status == "doing").count() as i64;
        let done_count = tasks.iter().filter(|t| t.status == "done").count() as i64;

        let total_tasks = tasks.len() as i64;

        // Get events
        let events = if !summary_only {
            let mut event_query = String::from(crate::sql_constants::SELECT_EVENT_BASE);
            let mut event_conditions = Vec::new();

            if let Some(ref dt) = since_datetime {
                event_query.push_str(" AND timestamp >= ?");
                event_conditions.push(dt.to_rfc3339());
            }

            event_query.push_str(" ORDER BY timestamp DESC");

            let mut q = sqlx::query_as::<_, Event>(&event_query);
            for cond in &event_conditions {
                q = q.bind(cond);
            }

            Some(q.fetch_all(self.pool).await?)
        } else {
            None
        };

        let total_events = if let Some(ref evts) = events {
            evts.len() as i64
        } else {
            sqlx::query_scalar::<_, i64>(crate::sql_constants::COUNT_EVENTS_TOTAL)
                .fetch_one(self.pool)
                .await?
        };

        let date_range = since_datetime.map(|from| DateRange {
            from,
            to: Utc::now(),
        });

        Ok(Report {
            summary: ReportSummary {
                total_tasks,
                tasks_by_status: StatusBreakdown {
                    todo: todo_count,
                    doing: doing_count,
                    done: done_count,
                },
                total_events,
                date_range,
            },
            tasks: if summary_only { None } else { Some(tasks) },
            events,
        })
    }

    /// Filter tasks using FTS5
    async fn filter_tasks_by_fts(
        &self,
        filter_name: &Option<String>,
        filter_spec: &Option<String>,
    ) -> Result<Vec<i64>> {
        let mut query = String::from("SELECT rowid FROM tasks_fts WHERE ");
        let mut conditions = Vec::new();

        if let Some(name_filter) = filter_name {
            conditions.push(format!(
                "name MATCH '{}'",
                crate::search::escape_fts5(name_filter)
            ));
        }

        if let Some(spec_filter) = filter_spec {
            conditions.push(format!(
                "spec MATCH '{}'",
                crate::search::escape_fts5(spec_filter)
            ));
        }

        if conditions.is_empty() {
            return Ok(Vec::new());
        }

        query.push_str(&conditions.join(" AND "));

        let ids: Vec<i64> = sqlx::query_scalar(&query).fetch_all(self.pool).await?;

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventManager;
    use crate::tasks::TaskManager;
    use crate::test_utils::test_helpers::TestContext;

    #[tokio::test]
    async fn test_generate_report_summary_only() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        // Create tasks with different statuses
        task_mgr
            .add_task("Todo task", None, None, None)
            .await
            .unwrap();
        let doing = task_mgr
            .add_task("Doing task", None, None, None)
            .await
            .unwrap();
        task_mgr.start_task(doing.id, false).await.unwrap();
        let done = task_mgr
            .add_task("Done task", None, None, None)
            .await
            .unwrap();
        task_mgr.start_task(done.id, false).await.unwrap();
        task_mgr.done_task(false).await.unwrap();

        let report = report_mgr
            .generate_report(None, None, None, None, true)
            .await
            .unwrap();

        assert_eq!(report.summary.total_tasks, 3);
        assert_eq!(report.summary.tasks_by_status.todo, 1);
        assert_eq!(report.summary.tasks_by_status.doing, 1);
        assert_eq!(report.summary.tasks_by_status.done, 1);
        assert!(report.tasks.is_none());
        assert!(report.events.is_none());
    }

    #[tokio::test]
    async fn test_generate_report_full() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr.add_task("Task 1", None, None, None).await.unwrap();
        task_mgr.add_task("Task 2", None, None, None).await.unwrap();

        let report = report_mgr
            .generate_report(None, None, None, None, false)
            .await
            .unwrap();

        assert!(report.tasks.is_some());
        assert_eq!(report.tasks.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_generate_report_filter_by_status() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr
            .add_task("Todo task", None, None, None)
            .await
            .unwrap();
        let doing = task_mgr
            .add_task("Doing task", None, None, None)
            .await
            .unwrap();
        task_mgr.start_task(doing.id, false).await.unwrap();

        let report = report_mgr
            .generate_report(None, Some("doing".to_string()), None, None, false)
            .await
            .unwrap();

        let tasks = report.tasks.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, "doing");
    }

    #[tokio::test]
    async fn test_generate_report_with_events() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        let task = task_mgr.add_task("Task 1", None, None, None).await.unwrap();
        event_mgr
            .add_event(task.id, "decision", "Test event")
            .await
            .unwrap();

        let report = report_mgr
            .generate_report(None, None, None, None, false)
            .await
            .unwrap();

        assert!(report.events.is_some());
        assert_eq!(report.summary.total_events, 1);
    }

    #[tokio::test]
    async fn test_parse_duration_days() {
        let result = crate::time_utils::parse_duration("7d").ok();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_parse_duration_hours() {
        let result = crate::time_utils::parse_duration("24h").ok();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_parse_duration_invalid() {
        let result = crate::time_utils::parse_duration("invalid").ok();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_filter_tasks_by_fts_name() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr
            .add_task("Authentication feature", None, None, None)
            .await
            .unwrap();
        task_mgr
            .add_task("Database migration", None, None, None)
            .await
            .unwrap();

        let report = report_mgr
            .generate_report(None, None, Some("Authentication".to_string()), None, false)
            .await
            .unwrap();

        let tasks = report.tasks.unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].name.contains("Authentication"));
    }

    #[tokio::test]
    async fn test_empty_report() {
        let ctx = TestContext::new().await;
        let report_mgr = ReportManager::new(ctx.pool());

        let report = report_mgr
            .generate_report(None, None, None, None, true)
            .await
            .unwrap();

        assert_eq!(report.summary.total_tasks, 0);
        assert_eq!(report.summary.total_events, 0);
    }

    #[tokio::test]
    async fn test_report_filter_consistency() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        // Create tasks with different statuses
        task_mgr.add_task("Task A", None, None, None).await.unwrap();
        task_mgr.add_task("Task B", None, None, None).await.unwrap();
        let doing = task_mgr.add_task("Task C", None, None, None).await.unwrap();
        task_mgr.start_task(doing.id, false).await.unwrap();

        // Filter with non-existent spec should return consistent summary
        let report = report_mgr
            .generate_report(None, None, None, Some("JWT".to_string()), true)
            .await
            .unwrap();

        // All counts should be 0 since no tasks match the filter
        assert_eq!(report.summary.total_tasks, 0);
        assert_eq!(report.summary.tasks_by_status.todo, 0);
        assert_eq!(report.summary.tasks_by_status.doing, 0);
        assert_eq!(report.summary.tasks_by_status.done, 0);
    }

    #[tokio::test]
    async fn test_generate_report_with_since() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        // Create some tasks
        task_mgr
            .add_task("Old task", None, None, None)
            .await
            .unwrap();
        task_mgr
            .add_task("Recent task", None, None, None)
            .await
            .unwrap();

        // Query with since parameter (should include all tasks created just now)
        let report = report_mgr
            .generate_report(Some("1h".to_string()), None, None, None, true)
            .await
            .unwrap();

        // Should include recent tasks
        assert!(report.summary.total_tasks >= 2);
        assert!(report.summary.date_range.is_some());
    }

    #[tokio::test]
    async fn test_generate_report_filter_by_spec() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr
            .add_task(
                "Task 1",
                Some("Implement authentication using JWT"),
                None,
                None,
            )
            .await
            .unwrap();
        task_mgr
            .add_task("Task 2", Some("Setup database migrations"), None, None)
            .await
            .unwrap();

        let report = report_mgr
            .generate_report(None, None, None, Some("authentication".to_string()), false)
            .await
            .unwrap();

        let tasks = report.tasks.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "Task 1");
    }

    #[tokio::test]
    async fn test_generate_report_combined_status_and_since() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr
            .add_task("Todo task", None, None, None)
            .await
            .unwrap();
        let doing = task_mgr
            .add_task("Doing task", None, None, None)
            .await
            .unwrap();
        task_mgr.start_task(doing.id, false).await.unwrap();

        // Filter by status + since
        let report = report_mgr
            .generate_report(
                Some("1d".to_string()),
                Some("doing".to_string()),
                None,
                None,
                false,
            )
            .await
            .unwrap();

        let tasks = report.tasks.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, "doing");
    }

    #[tokio::test]
    async fn test_filter_tasks_by_fts_spec() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr
            .add_task(
                "Feature A",
                Some("Implement JWT authentication"),
                None,
                None,
            )
            .await
            .unwrap();
        task_mgr
            .add_task("Feature B", Some("Setup OAuth2 integration"), None, None)
            .await
            .unwrap();

        let ids = report_mgr
            .filter_tasks_by_fts(&None, &Some("JWT".to_string()))
            .await
            .unwrap();

        assert_eq!(ids.len(), 1);
    }

    #[tokio::test]
    async fn test_filter_tasks_by_fts_both_name_and_spec() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr
            .add_task("Auth feature", Some("Implement authentication"), None, None)
            .await
            .unwrap();
        task_mgr
            .add_task(
                "Database setup",
                Some("Configure authentication database"),
                None,
                None,
            )
            .await
            .unwrap();

        // Both name and spec contain "auth"
        let ids = report_mgr
            .filter_tasks_by_fts(
                &Some("Auth".to_string()),
                &Some("authentication".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(ids.len(), 1);
    }

    #[tokio::test]
    async fn test_filter_tasks_by_fts_empty() {
        let ctx = TestContext::new().await;
        let report_mgr = ReportManager::new(ctx.pool());

        // Empty filters should return empty vec
        let ids = report_mgr.filter_tasks_by_fts(&None, &None).await.unwrap();

        assert_eq!(ids.len(), 0);
    }

    #[tokio::test]
    async fn test_report_date_range_present() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr.add_task("Task", None, None, None).await.unwrap();

        let report = report_mgr
            .generate_report(Some("7d".to_string()), None, None, None, true)
            .await
            .unwrap();

        // date_range should be present when since is specified
        assert!(report.summary.date_range.is_some());
        let date_range = report.summary.date_range.unwrap();
        assert!(date_range.to > date_range.from);
    }

    #[tokio::test]
    async fn test_report_date_range_absent() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr.add_task("Task", None, None, None).await.unwrap();

        let report = report_mgr
            .generate_report(None, None, None, None, true)
            .await
            .unwrap();

        // date_range should be None when since is not specified
        assert!(report.summary.date_range.is_none());
    }

    #[tokio::test]
    async fn test_report_events_count_consistency() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        let task = task_mgr.add_task("Task", None, None, None).await.unwrap();
        event_mgr
            .add_event(task.id, "decision", "Event 1")
            .await
            .unwrap();
        event_mgr
            .add_event(task.id, "note", "Event 2")
            .await
            .unwrap();

        // summary_only should still count events
        let summary_report = report_mgr
            .generate_report(None, None, None, None, true)
            .await
            .unwrap();
        assert_eq!(summary_report.summary.total_events, 2);
        assert!(summary_report.events.is_none());

        // Full report should include events
        let full_report = report_mgr
            .generate_report(None, None, None, None, false)
            .await
            .unwrap();
        assert_eq!(full_report.summary.total_events, 2);
        assert_eq!(full_report.events.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_generate_report_all_filters_combined() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let report_mgr = ReportManager::new(ctx.pool());

        task_mgr
            .add_task("Auth feature", Some("JWT implementation"), None, None)
            .await
            .unwrap();
        let doing = task_mgr
            .add_task("Auth testing", Some("Write JWT tests"), None, None)
            .await
            .unwrap();
        task_mgr.start_task(doing.id, false).await.unwrap();

        // Combine all filters: since + status + name + spec
        let report = report_mgr
            .generate_report(
                Some("1h".to_string()),
                Some("doing".to_string()),
                Some("Auth".to_string()),
                Some("JWT".to_string()),
                false,
            )
            .await
            .unwrap();

        let tasks = report.tasks.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, "doing");
        assert!(tasks[0].name.contains("Auth"));
    }
}
