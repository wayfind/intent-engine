// Performance tests for large datasets
// These tests are designed to validate performance and accuracy with 100k+ tasks

use intent_engine::db::{create_pool, run_migrations};
use intent_engine::events::EventManager;
use intent_engine::report::ReportManager;
use intent_engine::tasks::TaskManager;
use rand::Rng;
use std::time::Instant;
use tempfile::TempDir;

const TOTAL_TASKS_100K: usize = 100_000;
const TOTAL_TASKS_10K: usize = 10_000;

#[tokio::test]
#[ignore] // Run with: cargo test --test performance_large_dataset_tests test_large_dataset_report_and_search -- --ignored --nocapture
async fn test_large_dataset_report_and_search() {
    run_dataset_test(TOTAL_TASKS_100K).await;
}

#[tokio::test]
#[ignore] // Run with: cargo test --test performance_large_dataset_tests test_medium_dataset_report_and_search -- --ignored --nocapture
async fn test_medium_dataset_report_and_search() {
    run_dataset_test(TOTAL_TASKS_10K).await;
}

async fn run_dataset_test(total_tasks: usize) {
    println!(
        "\n🚀 Starting dataset performance test with {} tasks",
        total_tasks
    );

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("large_test.db");
    let pool = create_pool(&db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let task_mgr = TaskManager::new(&pool);
    let event_mgr = EventManager::new(&pool);
    let report_mgr = ReportManager::new(&pool);

    // Phase 1: Insert tasks
    println!("\n📝 Phase 1: Inserting {} tasks...", total_tasks);
    let start = Instant::now();

    let keywords = vec![
        "authentication",
        "database",
        "frontend",
        "backend",
        "api",
        "security",
        "optimization",
        "refactor",
        "feature",
        "bugfix",
        "documentation",
        "testing",
        "deployment",
        "migration",
        "performance",
    ];

    let mut task_ids = Vec::with_capacity(total_tasks);

    for i in 0..total_tasks {
        let keyword_idx = i % keywords.len();
        let keyword = keywords[keyword_idx];

        let name = format!("{} task #{}", keyword, i);
        let spec = format!("Implement {} functionality for module {}", keyword, i / 100);

        let task = task_mgr.add_task(&name, Some(&spec), None).await.unwrap();

        task_ids.push(task.id);

        if (i + 1) % 10000 == 0 {
            println!(
                "  ✓ Inserted {} tasks ({:.1}%)",
                i + 1,
                (i + 1) as f64 / total_tasks as f64 * 100.0
            );
        }
    }

    let insert_duration = start.elapsed();
    println!(
        "✅ Inserted {} tasks in {:.2}s ({:.0} tasks/sec)",
        total_tasks,
        insert_duration.as_secs_f64(),
        total_tasks as f64 / insert_duration.as_secs_f64()
    );

    // Phase 2: Randomly start and complete tasks
    println!("\n🎲 Phase 2: Randomly starting and completing tasks...");
    let start = Instant::now();

    #[allow(deprecated)]
    let mut rng = rand::thread_rng();
    let mut started_count = 0;
    let mut completed_count = 0;

    // Start 30% of tasks
    let tasks_to_start = total_tasks * 30 / 100;
    for _ in 0..tasks_to_start {
        #[allow(deprecated)]
        let random_idx = rng.gen_range(0..task_ids.len());
        let task_id = task_ids[random_idx];

        if task_mgr.start_task(task_id, false).await.is_ok() {
            started_count += 1;
        }
    }

    println!("  ✓ Started {} tasks", started_count);

    // Complete 50% of started tasks
    let tasks_to_complete = started_count / 2;
    for _ in 0..tasks_to_complete {
        #[allow(deprecated)]
        let random_idx = rng.gen_range(0..task_ids.len());
        let task_id = task_ids[random_idx];

        // Try to set as current and complete
        if sqlx::query(
            "INSERT OR REPLACE INTO workspace_state (key, value) VALUES ('current_task_id', ?)",
        )
        .bind(task_id.to_string())
        .execute(&pool)
        .await
        .is_ok()
        {
            // Check if task is in doing status
            let status: Option<String> =
                sqlx::query_scalar("SELECT status FROM tasks WHERE id = ?")
                    .bind(task_id)
                    .fetch_optional(&pool)
                    .await
                    .unwrap();

            if status == Some("doing".to_string()) {
                // Check if it has no uncompleted children
                let uncompleted_children: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM tasks WHERE parent_id = ? AND status != 'done'",
                )
                .bind(task_id)
                .fetch_one(&pool)
                .await
                .unwrap();

                if uncompleted_children == 0 && task_mgr.done_task().await.is_ok() {
                    completed_count += 1;
                }
            }
        }
    }

    println!("  ✓ Completed {} tasks", completed_count);

    let state_change_duration = start.elapsed();
    println!(
        "✅ State changes completed in {:.2}s",
        state_change_duration.as_secs_f64()
    );

    // Add some events to random tasks
    println!("\n📋 Adding events to sample tasks...");
    let start = Instant::now();
    let events_count = 1000;

    for _ in 0..events_count {
        #[allow(deprecated)]
        let random_idx = rng.gen_range(0..task_ids.len());
        let task_id = task_ids[random_idx];

        let _ = event_mgr
            .add_event(
                task_id,
                "progress",
                "Made significant progress on this task",
            )
            .await;
    }

    let events_duration = start.elapsed();
    println!(
        "✅ Added {} events in {:.2}s",
        events_count,
        events_duration.as_secs_f64()
    );

    // Phase 3: Test Report Performance
    println!("\n📊 Phase 3: Testing Report Performance...");

    // Test 1: Summary-only report
    let start = Instant::now();
    let report = report_mgr
        .generate_report(None, None, None, None, true)
        .await
        .unwrap();
    let summary_duration = start.elapsed();

    println!("  Report Summary:");
    println!("    Total tasks: {}", report.summary.total_tasks);
    println!("    Todo: {}", report.summary.tasks_by_status.todo);
    println!("    Doing: {}", report.summary.tasks_by_status.doing);
    println!("    Done: {}", report.summary.tasks_by_status.done);
    println!("    Total events: {}", report.summary.total_events);
    println!(
        "  ⏱️  Summary report generated in {:.3}s",
        summary_duration.as_secs_f64()
    );

    // Verify counts
    assert_eq!(report.summary.total_tasks, total_tasks as i64);
    let total_status = report.summary.tasks_by_status.todo
        + report.summary.tasks_by_status.doing
        + report.summary.tasks_by_status.done;
    assert_eq!(total_status, total_tasks as i64);

    // Test 2: Status filter report
    let start = Instant::now();
    let done_report = report_mgr
        .generate_report(None, Some("done".to_string()), None, None, true)
        .await
        .unwrap();
    let status_filter_duration = start.elapsed();

    println!("  Status Filter Report (done only):");
    println!("    Total tasks: {}", done_report.summary.total_tasks);
    println!(
        "  ⏱️  Status filter report generated in {:.3}s",
        status_filter_duration.as_secs_f64()
    );

    assert_eq!(
        done_report.summary.total_tasks,
        report.summary.tasks_by_status.done
    );

    // Test 3: Time-based report
    let start = Instant::now();
    let time_report = report_mgr
        .generate_report(Some("24h".to_string()), None, None, None, true)
        .await
        .unwrap();
    let time_filter_duration = start.elapsed();

    println!("  Time Filter Report (last 24h):");
    println!("    Total tasks: {}", time_report.summary.total_tasks);
    println!(
        "  ⏱️  Time filter report generated in {:.3}s",
        time_filter_duration.as_secs_f64()
    );

    // Phase 4: Test Search Performance and Accuracy
    println!("\n🔍 Phase 4: Testing Search Performance and Accuracy...");

    // Test search for each keyword
    let mut total_search_duration = std::time::Duration::ZERO;
    for keyword in &keywords[0..5] {
        // Test first 5 keywords
        let start = Instant::now();
        let results = task_mgr.search_tasks(keyword).await.unwrap();
        let search_duration = start.elapsed();
        total_search_duration += search_duration;

        // Calculate expected count (approximate)
        let expected_min = total_tasks / keywords.len() - 100; // Allow some margin
        let expected_max = total_tasks / keywords.len() + 100;

        println!(
            "  Search '{}': {} results in {:.3}s",
            keyword,
            results.len(),
            search_duration.as_secs_f64()
        );

        // Verify accuracy - check that results actually contain the keyword
        let mut accurate = 0;
        for result in results.iter().take(100) {
            // Check first 100 results
            if result.task.name.contains(keyword)
                || result
                    .task
                    .spec
                    .as_ref()
                    .is_some_and(|s| s.contains(keyword))
            {
                accurate += 1;
            }
        }

        let accuracy = accurate as f64 / results.len().min(100) as f64 * 100.0;
        println!(
            "    Accuracy: {:.1}% (checked {} results)",
            accuracy,
            results.len().min(100)
        );

        // Verify search returned reasonable number of results
        assert!(
            results.len() >= expected_min,
            "Search for '{}' returned too few results: {} < {}",
            keyword,
            results.len(),
            expected_min
        );
        assert!(
            results.len() <= expected_max,
            "Search for '{}' returned too many results: {} > {}",
            keyword,
            results.len(),
            expected_max
        );

        // Verify high accuracy
        assert!(
            accuracy >= 95.0,
            "Search accuracy too low: {:.1}% < 95%",
            accuracy
        );
    }

    // Test complex search with AND operator
    let start = Instant::now();
    let complex_results = task_mgr
        .search_tasks("authentication AND module")
        .await
        .unwrap();
    let complex_duration = start.elapsed();

    println!(
        "  Complex search 'authentication AND module': {} results in {:.3}s",
        complex_results.len(),
        complex_duration.as_secs_f64()
    );

    // Verify complex search accuracy
    let mut accurate = 0;
    for result in complex_results.iter().take(100) {
        let text = format!(
            "{} {}",
            result.task.name,
            result.task.spec.as_ref().unwrap_or(&String::new())
        );
        if text.contains("authentication") && text.contains("module") {
            accurate += 1;
        }
    }

    if !complex_results.is_empty() {
        let accuracy = accurate as f64 / complex_results.len().min(100) as f64 * 100.0;
        println!("    Accuracy: {:.1}%", accuracy);
        assert!(accuracy >= 95.0);
    }

    // Phase 5: Test find_tasks performance
    println!("\n🔎 Phase 5: Testing find_tasks Performance...");

    let start = Instant::now();
    let todo_tasks = task_mgr.find_tasks(Some("todo"), None).await.unwrap();
    let find_duration = start.elapsed();

    println!(
        "  Find todo tasks: {} results in {:.3}s",
        todo_tasks.len(),
        find_duration.as_secs_f64()
    );

    assert_eq!(todo_tasks.len() as i64, report.summary.tasks_by_status.todo);

    let avg_search_duration = total_search_duration.as_secs_f64() / 5.0;

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("📈 PERFORMANCE SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Dataset: {} tasks, {} events", total_tasks, events_count);
    println!(
        "Insert:  {:.2}s ({:.0} tasks/sec)",
        insert_duration.as_secs_f64(),
        total_tasks as f64 / insert_duration.as_secs_f64()
    );
    println!("Report:  {:.3}s (summary)", summary_duration.as_secs_f64());
    println!(
        "Search:  ~{:.3}s (average per keyword)",
        avg_search_duration
    );
    println!(
        "Find:    {:.3}s (status filter)",
        find_duration.as_secs_f64()
    );
    println!("{}", "=".repeat(60));

    // Performance assertions
    assert!(
        summary_duration.as_secs_f64() < 5.0,
        "Summary report too slow: {:.2}s > 5s",
        summary_duration.as_secs_f64()
    );
    assert!(
        avg_search_duration < 1.0,
        "Search too slow: {:.2}s > 1s",
        avg_search_duration
    );
    assert!(
        find_duration.as_secs_f64() < 2.0,
        "Find tasks too slow: {:.2}s > 2s",
        find_duration.as_secs_f64()
    );

    println!("✅ All performance tests passed!");
}

#[tokio::test]
#[ignore] // Run with: cargo test --test performance_large_dataset_tests test_search_accuracy -- --ignored --nocapture
async fn test_search_accuracy_detailed() {
    println!("\n🎯 Testing search accuracy with controlled dataset...");

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("accuracy_test.db");
    let pool = create_pool(&db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let task_mgr = TaskManager::new(&pool);

    // Create controlled test dataset
    let test_cases = vec![
        ("JWT Authentication", "Implement JWT token authentication"),
        ("User Authentication", "Add user authentication with OAuth2"),
        (
            "Database Optimization",
            "Optimize database query performance",
        ),
        ("API Endpoint", "Create new API endpoint for authentication"),
        ("Frontend Component", "Build authentication form component"),
        ("Backend Service", "Authentication service implementation"),
        ("Security Audit", "Perform security audit on authentication"),
        ("Performance Testing", "Load test authentication endpoints"),
        ("Documentation", "Document authentication flow"),
        ("Migration Script", "Database migration for auth tables"),
    ];

    println!("Creating {} test tasks...", test_cases.len() * 100);

    for i in 0..100 {
        for (name_template, spec_template) in &test_cases {
            let name = format!("{} #{}", name_template, i);
            let spec = format!("{} - iteration {}", spec_template, i);
            task_mgr.add_task(&name, Some(&spec), None).await.unwrap();
        }
    }

    println!("Testing search accuracy...\n");

    // Test 1: Single keyword search
    let results = task_mgr.search_tasks("authentication").await.unwrap();
    println!("Search 'authentication': {} results", results.len());

    // Should find all tasks with "authentication" in name or spec
    let expected = test_cases
        .iter()
        .filter(|(n, s)| {
            n.to_lowercase().contains("authentication")
                || s.to_lowercase().contains("authentication")
        })
        .count()
        * 100;

    println!("  Expected: ~{} tasks", expected);
    println!("  Actual:   {} tasks", results.len());
    println!(
        "  Accuracy: {:.1}%",
        results.len() as f64 / expected as f64 * 100.0
    );

    assert!(
        results.len() >= expected - 10 && results.len() <= expected + 10,
        "Search returned {} tasks, expected ~{}",
        results.len(),
        expected
    );

    // Test 2: AND operator
    let results = task_mgr
        .search_tasks("authentication AND JWT")
        .await
        .unwrap();
    println!(
        "\nSearch 'authentication AND JWT': {} results",
        results.len()
    );

    let expected = 100; // Only "JWT Authentication" tasks
    println!("  Expected: ~{} tasks", expected);
    println!("  Actual:   {} tasks", results.len());

    assert!(results.len() >= expected - 10 && results.len() <= expected + 10);

    // Test 3: OR operator
    let results = task_mgr.search_tasks("JWT OR OAuth2").await.unwrap();
    println!("\nSearch 'JWT OR OAuth2': {} results", results.len());

    let expected = 200; // "JWT Authentication" + "User Authentication"
    println!("  Expected: ~{} tasks", expected);
    println!("  Actual:   {} tasks", results.len());

    assert!(results.len() >= expected - 20 && results.len() <= expected + 20);

    // Test 4: NOT operator
    let results = task_mgr
        .search_tasks("authentication NOT JWT")
        .await
        .unwrap();
    println!(
        "\nSearch 'authentication NOT JWT': {} results",
        results.len()
    );

    // Should exclude "JWT Authentication" tasks
    let expected = 600; // All auth tasks except JWT ones
    println!("  Expected: ~{} tasks", expected);
    println!("  Actual:   {} tasks", results.len());

    println!("\n✅ Search accuracy tests completed!");
}

#[tokio::test]
#[ignore]
async fn test_concurrent_search_performance() {
    println!("\n⚡ Testing concurrent search performance...");

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("concurrent_test.db");
    let pool = create_pool(&db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let task_mgr = TaskManager::new(&pool);

    // Create dataset
    println!("Creating 10,000 tasks...");
    for i in 0..10000 {
        let keyword = match i % 5 {
            0 => "authentication",
            1 => "database",
            2 => "frontend",
            3 => "backend",
            _ => "api",
        };

        task_mgr
            .add_task(
                &format!("{} task #{}", keyword, i),
                Some(&format!("Implementation for {}", keyword)),
                None,
            )
            .await
            .unwrap();
    }

    println!("Running concurrent searches...");

    let start = Instant::now();
    let mut handles = vec![];

    // Spawn 10 concurrent search tasks
    for keyword in &["authentication", "database", "frontend", "backend", "api"] {
        for _ in 0..2 {
            let keyword = keyword.to_string();
            let temp_dir_path = temp_dir.path().to_path_buf();

            let handle = tokio::spawn(async move {
                let db_path = temp_dir_path.join("concurrent_test.db");
                let pool = create_pool(&db_path).await.unwrap();
                let task_mgr = TaskManager::new(&pool);

                let start = Instant::now();
                let results = task_mgr.search_tasks(&keyword).await.unwrap();
                let duration = start.elapsed();

                (keyword, results.len(), duration)
            });

            handles.push(handle);
        }
    }

    // Wait for all searches to complete
    let mut total_results = 0;
    for handle in handles {
        let (keyword, count, duration) = handle.await.unwrap();
        println!(
            "  '{}': {} results in {:.3}s",
            keyword,
            count,
            duration.as_secs_f64()
        );
        total_results += count;
    }

    let total_duration = start.elapsed();

    println!("\n✅ Concurrent searches completed:");
    println!("  Total time: {:.3}s", total_duration.as_secs_f64());
    println!("  Total results: {}", total_results);
    println!(
        "  Average time per search: {:.3}s",
        total_duration.as_secs_f64() / 10.0
    );

    // Should complete all searches within reasonable time
    assert!(
        total_duration.as_secs_f64() < 10.0,
        "Concurrent searches too slow: {:.2}s > 10s",
        total_duration.as_secs_f64()
    );
}
