use intent_engine::{
    error::Result,
    plan::{PlanExecutor, PlanRequest, TaskTree},
    project::ProjectContext,
    tasks::TaskManager,
    workspace::WorkspaceManager,
};
use tempfile::TempDir;

/// Helper to create temp project for testing
async fn setup_test_project() -> Result<(TempDir, ProjectContext)> {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().to_path_buf();
    let ctx = ProjectContext::initialize_project_at(project_path).await?;
    Ok((temp_dir, ctx))
}

#[tokio::test]
async fn test_cli_task_add_creates_ai_owned_task() -> Result<()> {
    let (_temp, ctx) = setup_test_project().await?;
    let task_mgr = TaskManager::new(&ctx.pool);

    // Simulate CLI task creation (should set owner='ai')
    let task = task_mgr
        .add_task("CLI Test Task", None, None, Some("ai"))
        .await?;

    assert_eq!(task.owner, "ai", "CLI-created task should have owner='ai'");

    Ok(())
}

#[tokio::test]
async fn test_dashboard_task_add_creates_human_owned_task() -> Result<()> {
    let (_temp, ctx) = setup_test_project().await?;
    let task_mgr = TaskManager::new(&ctx.pool);

    // Simulate Dashboard task creation (None defaults to 'human')
    let task = task_mgr
        .add_task("Dashboard Test Task", None, None, None)
        .await?;

    assert_eq!(
        task.owner, "human",
        "Dashboard-created task should have owner='human'"
    );

    Ok(())
}

#[tokio::test]
async fn test_plan_creates_ai_owned_tasks() -> Result<()> {
    let (_temp, ctx) = setup_test_project().await?;

    let plan_executor = PlanExecutor::new(&ctx.pool);
    let task_mgr = TaskManager::new(&ctx.pool);

    // Create tasks via plan (should set owner='ai')
    let plan = PlanRequest {
        tasks: vec![TaskTree {
            name: Some("Plan Test Task".to_string()),
            spec: None,
            priority: None,
            status: None,
            active_form: None,
            children: None,
            depends_on: None,
            id: None,
            parent_id: None,
            ..Default::default()
        }],
    };

    let result = plan_executor.execute(&plan).await?;
    assert_eq!(result.created_count, 1);

    // Verify the created task has owner='ai'
    let task_id = result.task_id_map.get("Plan Test Task").unwrap();
    let task = task_mgr.get_task(*task_id).await?;

    assert_eq!(task.owner, "ai", "Plan-created task should have owner='ai'");

    Ok(())
}

#[tokio::test]
async fn test_spawn_subtask_creates_ai_owned_task() -> Result<()> {
    let (_temp, ctx) = setup_test_project().await?;

    let task_mgr = TaskManager::new(&ctx.pool);

    // Create parent task
    let parent = task_mgr
        .add_task("Parent Task", None, None, Some("ai"))
        .await?;

    // Start parent task to set current_task_id
    task_mgr.start_task(parent.id, false).await?;

    // Spawn subtask (should set owner='ai')
    let result = task_mgr.spawn_subtask("Subtask", None).await?;

    // Verify the subtask has owner='ai'
    let subtask = task_mgr.get_task(result.subtask.id).await?;
    assert_eq!(
        subtask.owner, "ai",
        "Spawned subtask should have owner='ai'"
    );

    Ok(())
}

#[tokio::test]
async fn test_ai_cannot_complete_human_owned_task() -> Result<()> {
    let (_temp, ctx) = setup_test_project().await?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let workspace_mgr = WorkspaceManager::new(&ctx.pool);

    // Create human-owned task
    let task = task_mgr
        .add_task("Human Task", None, None, None) // None = human
        .await?;

    // Set as current task
    workspace_mgr.set_current_task(task.id, None).await?;

    // Try to complete as AI (is_ai_caller=true)
    let result = task_mgr.done_task(true).await;

    // Should fail
    assert!(
        result.is_err(),
        "AI should not be able to complete human-owned task"
    );

    // Verify error message contains expected text
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("human")
            || err_msg.contains("cannot")
            || err_msg.contains("AI")
            || err_msg.contains("permission"),
        "Error message should indicate permission issue: {}",
        err_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_human_can_complete_human_owned_task() -> Result<()> {
    let (_temp, ctx) = setup_test_project().await?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let workspace_mgr = WorkspaceManager::new(&ctx.pool);

    // Create human-owned task
    let task = task_mgr
        .add_task("Human Task", None, None, None) // None = human
        .await?;

    // Set as current task and doing status
    workspace_mgr.set_current_task(task.id, None).await?;
    task_mgr
        .update_task(
            task.id,
            None,
            None,
            None,
            Some("doing"),
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

    // Complete as human (is_ai_caller=false)
    let result = task_mgr.done_task(false).await;

    // Should succeed
    assert!(
        result.is_ok(),
        "Human should be able to complete human-owned task"
    );

    Ok(())
}

#[tokio::test]
async fn test_ai_can_complete_ai_owned_task() -> Result<()> {
    let (_temp, ctx) = setup_test_project().await?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let workspace_mgr = WorkspaceManager::new(&ctx.pool);

    // Create AI-owned task
    let task = task_mgr.add_task("AI Task", None, None, Some("ai")).await?;

    // Set as current task and doing status
    workspace_mgr.set_current_task(task.id, None).await?;
    task_mgr
        .update_task(
            task.id,
            None,
            None,
            None,
            Some("doing"),
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

    // Complete as AI (is_ai_caller=true)
    let result = task_mgr.done_task(true).await;

    // Should succeed
    assert!(
        result.is_ok(),
        "AI should be able to complete AI-owned task"
    );

    Ok(())
}

#[tokio::test]
async fn test_mixed_ownership_in_hierarchy() -> Result<()> {
    let (_temp, ctx) = setup_test_project().await?;

    let task_mgr = TaskManager::new(&ctx.pool);

    // Create human parent
    let parent = task_mgr.add_task("Human Parent", None, None, None).await?;

    // Create AI child under human parent
    let child = task_mgr
        .add_task("AI Child", None, Some(parent.id), Some("ai"))
        .await?;

    // Verify ownership is preserved
    let parent_check = task_mgr.get_task(parent.id).await?;
    let child_check = task_mgr.get_task(child.id).await?;

    assert_eq!(
        parent_check.owner, "human",
        "Parent should remain human-owned"
    );
    assert_eq!(child_check.owner, "ai", "Child should be AI-owned");

    Ok(())
}
