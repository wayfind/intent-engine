use intent_engine::db::{create_pool, run_migrations};
use intent_engine::events::EventManager;
use intent_engine::report::ReportManager;
use intent_engine::tasks::TaskManager;
use std::time::Instant;
use tempfile::TempDir;

async fn setup_test_db() -> (TempDir, sqlx::SqlitePool) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("perf_test.db");
    let pool = create_pool(&db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();
    (temp_dir, pool)
}

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_deep_task_hierarchy() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let depth = 100;
    let start = Instant::now();

    // Create a 100-level deep hierarchy
    let mut parent_id = None;
    for i in 0..depth {
        let task = task_mgr
            .add_task(&format!("Level {}", i), None, parent_id)
            .await
            .unwrap();
        parent_id = Some(task.id);
    }

    let elapsed = start.elapsed();
    println!("Created {}-level deep hierarchy in {:?}", depth, elapsed);

    // Test retrieving leaf task
    let start = Instant::now();
    let leaf_task = task_mgr.get_task(depth).await.unwrap();
    let elapsed = start.elapsed();
    println!("Retrieved leaf task in {:?}", elapsed);

    assert_eq!(leaf_task.id, depth);
}

#[tokio::test]
#[ignore]
async fn test_very_deep_task_hierarchy() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let depth = 500;
    let start = Instant::now();

    let mut parent_id = None;
    for i in 0..depth {
        let task = task_mgr
            .add_task(&format!("Level {}", i), None, parent_id)
            .await
            .unwrap();
        parent_id = Some(task.id);
    }

    let elapsed = start.elapsed();
    println!("Created {}-level deep hierarchy in {:?}", depth, elapsed);
    assert!(elapsed.as_secs() < 10, "Should complete within 10 seconds");
}

#[tokio::test]
#[ignore]
async fn test_massive_tasks_10k() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let count = 10_000;
    let start = Instant::now();

    // Create 10,000 tasks
    for i in 0..count {
        task_mgr
            .add_task(&format!("Task {}", i), None, None)
            .await
            .unwrap();
    }

    let elapsed = start.elapsed();
    println!("Created {} tasks in {:?}", count, elapsed);
    println!(
        "Average: {:.2} ms per task",
        elapsed.as_millis() as f64 / count as f64
    );

    // Test finding all tasks
    let start = Instant::now();
    let tasks = task_mgr.find_tasks(None, None).await.unwrap();
    let elapsed = start.elapsed();

    println!("Found {} tasks in {:?}", tasks.len(), elapsed);
    assert_eq!(tasks.len(), count);
}

#[tokio::test]
#[ignore]
async fn test_massive_tasks_50k() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let count = 50_000;
    let start = Instant::now();

    // Batch insert for better performance
    for i in 0..count {
        task_mgr
            .add_task(&format!("Task {}", i), None, None)
            .await
            .unwrap();

        if (i + 1) % 10_000 == 0 {
            println!("Created {} tasks...", i + 1);
        }
    }

    let elapsed = start.elapsed();
    println!("Created {} tasks in {:?}", count, elapsed);
    println!(
        "Average: {:.2} ms per task",
        elapsed.as_millis() as f64 / count as f64
    );

    // Test report generation
    let start = Instant::now();
    let report_mgr = ReportManager::new(&pool);
    let report = report_mgr
        .generate_report(None, None, None, None, true)
        .await
        .unwrap();
    let elapsed = start.elapsed();

    println!(
        "Generated summary report for {} tasks in {:?}",
        count, elapsed
    );
    assert_eq!(report.summary.total_tasks, count as i64);
}

#[tokio::test]
#[ignore]
async fn test_massive_events() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);
    let event_mgr = EventManager::new(&pool);

    let task = task_mgr
        .add_task("Task with many events", None, None)
        .await
        .unwrap();

    let count = 10_000;
    let start = Instant::now();

    // Create 10,000 events
    for i in 0..count {
        event_mgr
            .add_event(task.id, "test", &format!("Event {}", i))
            .await
            .unwrap();
    }

    let elapsed = start.elapsed();
    println!("Created {} events in {:?}", count, elapsed);
    println!(
        "Average: {:.2} ms per event",
        elapsed.as_millis() as f64 / count as f64
    );

    // Test listing events with limit
    let start = Instant::now();
    let events = event_mgr.list_events(task.id, Some(100)).await.unwrap();
    let elapsed = start.elapsed();

    println!("Listed {} events (limited) in {:?}", events.len(), elapsed);
    assert_eq!(events.len(), 100);
}

#[tokio::test]
#[ignore]
async fn test_wide_task_hierarchy() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let parent = task_mgr.add_task("Parent task", None, None).await.unwrap();

    let children_count = 1000;
    let start = Instant::now();

    // Create 1000 children under one parent
    for i in 0..children_count {
        task_mgr
            .add_task(&format!("Child {}", i), None, Some(parent.id))
            .await
            .unwrap();
    }

    let elapsed = start.elapsed();
    println!(
        "Created {} children under one parent in {:?}",
        children_count, elapsed
    );

    // Test finding children
    let start = Instant::now();
    let children = task_mgr
        .find_tasks(None, Some(Some(parent.id)))
        .await
        .unwrap();
    let elapsed = start.elapsed();

    println!("Found {} children in {:?}", children.len(), elapsed);
    assert_eq!(children.len(), children_count);
}

#[tokio::test]
#[ignore]
async fn test_fts5_search_performance() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);
    let report_mgr = ReportManager::new(&pool);

    let count = 5_000;

    // Create tasks with various names
    let keywords = [
        "authentication",
        "database",
        "frontend",
        "backend",
        "testing",
    ];

    for i in 0..count {
        let keyword = keywords[i % keywords.len()];
        task_mgr
            .add_task(&format!("{} feature {}", keyword, i), None, None)
            .await
            .unwrap();
    }

    println!("Created {} tasks with keywords", count);

    // Test FTS5 search for each keyword
    for keyword in keywords.iter() {
        let start = Instant::now();
        let report = report_mgr
            .generate_report(None, None, Some(keyword.to_string()), None, false)
            .await
            .unwrap();
        let elapsed = start.elapsed();

        let tasks = report.tasks.unwrap();
        println!(
            "FTS5 search for '{}': found {} tasks in {:?}",
            keyword,
            tasks.len(),
            elapsed
        );

        assert!(tasks.len() >= count / keywords.len());
    }
}

#[tokio::test]
#[ignore]
async fn test_report_generation_performance() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);
    let event_mgr = EventManager::new(&pool);
    let report_mgr = ReportManager::new(&pool);

    let task_count = 5_000;

    // Create diverse workload
    for i in 0..task_count {
        let task = task_mgr
            .add_task(&format!("Task {}", i), None, None)
            .await
            .unwrap();

        // Add events to some tasks
        if i % 10 == 0 {
            event_mgr
                .add_event(task.id, "decision", &format!("Decision for task {}", i))
                .await
                .unwrap();
        }

        // Update status for some tasks
        match i % 4 {
            0 => {} // keep as todo
            1 => {
                task_mgr.start_task(task.id, false).await.unwrap();
            }
            2 => {
                task_mgr.start_task(task.id, false).await.unwrap();
                task_mgr.done_task().await.unwrap();
            }
            _ => {}
        }
    }

    println!("Created {} tasks with events", task_count);

    // Test summary-only report
    let start = Instant::now();
    let _summary_report = report_mgr
        .generate_report(None, None, None, None, true)
        .await
        .unwrap();
    let elapsed = start.elapsed();

    println!("Generated summary-only report in {:?}", elapsed);
    assert!(elapsed.as_millis() < 500, "Summary should be fast");

    // Test full report
    let start = Instant::now();
    let full_report = report_mgr
        .generate_report(None, None, None, None, false)
        .await
        .unwrap();
    let elapsed = start.elapsed();

    println!("Generated full report in {:?}", elapsed);
    assert!(full_report.tasks.is_some());
    assert!(full_report.events.is_some());
}

#[tokio::test]
#[ignore]
async fn test_concurrent_task_operations() {
    let (_temp_dir, pool) = setup_test_db().await;

    let start = Instant::now();

    // Spawn multiple concurrent operations
    let mut handles = vec![];

    for i in 0..100 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let task_mgr = TaskManager::new(&pool_clone);

            // Each task does multiple operations
            for j in 0..10 {
                let task = task_mgr
                    .add_task(&format!("Task {}-{}", i, j), None, None)
                    .await
                    .unwrap();

                task_mgr.start_task(task.id, false).await.unwrap();
                task_mgr.done_task().await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let elapsed = start.elapsed();
    println!(
        "Completed 100 concurrent workers (1000 total operations) in {:?}",
        elapsed
    );

    // Verify all tasks were created
    let task_mgr = TaskManager::new(&pool);
    let tasks = task_mgr.find_tasks(None, None).await.unwrap();
    assert_eq!(tasks.len(), 1000);
}

#[tokio::test]
#[ignore]
async fn test_stress_task_state_transitions() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let count = 1000;
    let start = Instant::now();

    // Create tasks and cycle through all states
    for i in 0..count {
        let task = task_mgr
            .add_task(&format!("Task {}", i), None, None)
            .await
            .unwrap();

        // todo -> doing -> done
        task_mgr.start_task(task.id, false).await.unwrap();
        task_mgr.done_task().await.unwrap();
    }

    let elapsed = start.elapsed();
    println!(
        "Completed {} full state transitions (3000 operations) in {:?}",
        count, elapsed
    );
    println!(
        "Average: {:.2} ms per transition",
        elapsed.as_millis() as f64 / (count * 3) as f64
    );
}
