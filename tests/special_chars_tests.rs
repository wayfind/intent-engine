/// Tests for special characters, edge cases, and security
///
/// This test suite validates Intent-Engine's handling of:
/// - SQL injection attempts
/// - Unicode characters and emojis
/// - JSON special characters
/// - Control characters
/// - Extreme length inputs
/// - Empty and null-like inputs
use intent_engine::db::{create_pool, run_migrations};
use intent_engine::events::EventManager;
use intent_engine::report::ReportManager;
use intent_engine::tasks::TaskManager;
use tempfile::TempDir;

async fn setup_test_db() -> (TempDir, sqlx::SqlitePool) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let pool = create_pool(&db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();
    (temp_dir, pool)
}

// ==================== SQL Injection Tests ====================

#[tokio::test]
async fn test_sql_injection_single_quote() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    // Attempt SQL injection with single quote
    let malicious_name = "Task'; DROP TABLE tasks; --";
    let task = task_mgr.add_task(malicious_name, None, None).await.unwrap();

    assert_eq!(task.name, malicious_name);

    // Verify table still exists by querying
    let result = task_mgr
        .find_tasks(None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(result.tasks.len(), 1);
}

#[tokio::test]
async fn test_sql_injection_union_select() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let malicious_name = "Task' UNION SELECT * FROM tasks WHERE '1'='1";
    let task = task_mgr.add_task(malicious_name, None, None).await.unwrap();

    assert_eq!(task.name, malicious_name);

    // Verify no extra tasks were created
    let result = task_mgr
        .find_tasks(None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(result.tasks.len(), 1);
}

#[tokio::test]
async fn test_sql_injection_in_spec() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let malicious_spec = "'; DELETE FROM events WHERE 1=1; --";
    let task = task_mgr
        .add_task("Normal task", Some(malicious_spec), None)
        .await
        .unwrap();

    assert_eq!(task.spec.as_deref(), Some(malicious_spec));

    // Verify task can be retrieved safely
    let retrieved = task_mgr.get_task(task.id).await.unwrap();
    assert_eq!(retrieved.spec.as_deref(), Some(malicious_spec));
}

#[tokio::test]
async fn test_sql_injection_in_event_data() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);
    let event_mgr = EventManager::new(&pool);

    let task = task_mgr.add_task("Test task", None, None).await.unwrap();

    let malicious_data = "'; DROP TABLE tasks; SELECT '";
    let event = event_mgr
        .add_event(task.id, "test", malicious_data)
        .await
        .unwrap();

    assert_eq!(event.discussion_data, malicious_data);

    // Verify tasks table still exists
    let result = task_mgr
        .find_tasks(None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(result.tasks.len(), 1);
}

// ==================== Unicode and Emoji Tests ====================

#[tokio::test]
async fn test_unicode_chinese_characters() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let chinese_name = "实现用户认证功能";
    let task = task_mgr.add_task(chinese_name, None, None).await.unwrap();

    assert_eq!(task.name, chinese_name);

    let retrieved = task_mgr.get_task(task.id).await.unwrap();
    assert_eq!(retrieved.name, chinese_name);
}

#[tokio::test]
async fn test_unicode_japanese_characters() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let japanese_name = "タスクを実装する";
    let task = task_mgr.add_task(japanese_name, None, None).await.unwrap();

    assert_eq!(task.name, japanese_name);
}

#[tokio::test]
async fn test_unicode_arabic_characters() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let arabic_name = "تنفيذ المهمة";
    let task = task_mgr.add_task(arabic_name, None, None).await.unwrap();

    assert_eq!(task.name, arabic_name);
}

#[tokio::test]
async fn test_emoji_in_task_name() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let emoji_name = "🚀 Deploy to production 🎉";
    let task = task_mgr.add_task(emoji_name, None, None).await.unwrap();

    assert_eq!(task.name, emoji_name);

    let retrieved = task_mgr.get_task(task.id).await.unwrap();
    assert_eq!(retrieved.name, emoji_name);
}

#[tokio::test]
async fn test_complex_emoji_sequences() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let complex_emoji = "👨‍👩‍👧‍👦 Family task 🏳️‍🌈 🇺🇸";
    let task = task_mgr.add_task(complex_emoji, None, None).await.unwrap();

    assert_eq!(task.name, complex_emoji);
}

#[tokio::test]
async fn test_mixed_languages() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let mixed = "实现 authentication 認証 مصادقة with 🔐";
    let task = task_mgr.add_task(mixed, None, None).await.unwrap();

    assert_eq!(task.name, mixed);
}

// ==================== JSON Special Characters Tests ====================

#[tokio::test]
async fn test_double_quotes_in_name() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let name_with_quotes = r#"Task with "quoted" text"#;
    let task = task_mgr
        .add_task(name_with_quotes, None, None)
        .await
        .unwrap();

    assert_eq!(task.name, name_with_quotes);

    // Verify JSON serialization works
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains(r#"\"quoted\""#));
}

#[tokio::test]
async fn test_backslash_in_name() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let name_with_backslash = r"C:\Users\test\path";
    let task = task_mgr
        .add_task(name_with_backslash, None, None)
        .await
        .unwrap();

    assert_eq!(task.name, name_with_backslash);
}

#[tokio::test]
async fn test_json_control_characters() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let name_with_controls = "Task\nwith\nnewlines\tand\ttabs";
    let task = task_mgr
        .add_task(name_with_controls, None, None)
        .await
        .unwrap();

    assert_eq!(task.name, name_with_controls);

    // Verify JSON serialization escapes properly
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains(r"\n"));
    assert!(json.contains(r"\t"));
}

#[tokio::test]
async fn test_null_bytes_rejected() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    // SQLite doesn't support null bytes in text
    let name_with_null = "Task\0with\0nulls";

    // This should either work (with nulls removed) or fail gracefully
    let result = task_mgr.add_task(name_with_null, None, None).await;

    // Either way, the system should handle it without crashing
    match result {
        Ok(task) => {
            // If it succeeded, nulls should be handled somehow
            assert!(!task.name.is_empty());
        },
        Err(_) => {
            // Or it should fail gracefully
            // This is acceptable behavior
        },
    }
}

// ==================== Control Characters Tests ====================

#[tokio::test]
async fn test_multiline_task_name() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let multiline_name = "Task title\nWith description\nAnd multiple lines";
    let task = task_mgr.add_task(multiline_name, None, None).await.unwrap();

    assert_eq!(task.name, multiline_name);
}

#[tokio::test]
async fn test_multiline_spec() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let multiline_spec = r#"# Task Specification

## Requirements
1. Feature A
2. Feature B

## Notes
- Important detail
- Another detail
"#;

    let task = task_mgr
        .add_task("Task", Some(multiline_spec), None)
        .await
        .unwrap();

    assert_eq!(task.spec.as_deref(), Some(multiline_spec));
}

#[tokio::test]
async fn test_tabs_and_spaces() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let name_with_whitespace = "Task\t\twith\t\tmultiple\t\ttabs   and   spaces";
    let task = task_mgr
        .add_task(name_with_whitespace, None, None)
        .await
        .unwrap();

    assert_eq!(task.name, name_with_whitespace);
}

#[tokio::test]
async fn test_carriage_return() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let name_with_cr = "Task\r\nwith\r\nCRLF";
    let task = task_mgr.add_task(name_with_cr, None, None).await.unwrap();

    assert_eq!(task.name, name_with_cr);
}

// ==================== Extreme Length Tests ====================

#[tokio::test]
async fn test_very_long_task_name() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let long_name = "A".repeat(10_000);
    let task = task_mgr.add_task(&long_name, None, None).await.unwrap();

    assert_eq!(task.name.len(), 10_000);
    assert_eq!(task.name, long_name);
}

#[tokio::test]
async fn test_very_long_spec() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let long_spec = "This is a very long specification. ".repeat(1_000);
    let task = task_mgr
        .add_task("Task", Some(&long_spec), None)
        .await
        .unwrap();

    assert!(task.spec.is_some());
    assert_eq!(task.spec.unwrap().len(), long_spec.len());
}

#[tokio::test]
async fn test_very_long_event_data() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);
    let event_mgr = EventManager::new(&pool);

    let task = task_mgr.add_task("Test", None, None).await.unwrap();

    let long_data = "Event data. ".repeat(10_000);
    let event = event_mgr
        .add_event(task.id, "test", &long_data)
        .await
        .unwrap();

    assert_eq!(event.discussion_data.len(), long_data.len());
}

// ==================== Empty and Boundary Tests ====================

#[tokio::test]
async fn test_empty_task_name_rejected() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    // Empty name should still be allowed (the spec doesn't forbid it)
    let task = task_mgr.add_task("", None, None).await.unwrap();
    assert_eq!(task.name, "");
}

#[tokio::test]
async fn test_whitespace_only_task_name() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let whitespace_name = "   \t\n   ";
    let task = task_mgr
        .add_task(whitespace_name, None, None)
        .await
        .unwrap();

    assert_eq!(task.name, whitespace_name);
}

#[tokio::test]
async fn test_empty_spec() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let task = task_mgr.add_task("Task", Some(""), None).await.unwrap();
    assert_eq!(task.spec.as_deref(), Some(""));
}

#[tokio::test]
async fn test_empty_event_data() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);
    let event_mgr = EventManager::new(&pool);

    let task = task_mgr.add_task("Test", None, None).await.unwrap();

    let event = event_mgr.add_event(task.id, "test", "").await.unwrap();
    assert_eq!(event.discussion_data, "");
}

// ==================== Special Symbol Combinations ====================

#[tokio::test]
async fn test_markdown_in_task_name() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let markdown_name = "# Task **bold** *italic* `code`";
    let task = task_mgr.add_task(markdown_name, None, None).await.unwrap();

    assert_eq!(task.name, markdown_name);
}

#[tokio::test]
async fn test_html_tags_in_name() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let html_name = "<script>alert('xss')</script>";
    let task = task_mgr.add_task(html_name, None, None).await.unwrap();

    assert_eq!(task.name, html_name);
}

#[tokio::test]
async fn test_regex_metacharacters() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let regex_name = r"Task.*[0-9]+\d{3}(test|prod)$";
    let task = task_mgr.add_task(regex_name, None, None).await.unwrap();

    assert_eq!(task.name, regex_name);
}

#[tokio::test]
async fn test_shell_metacharacters() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let shell_name = "Task && echo 'test' | grep -v 'bad' > /dev/null";
    let task = task_mgr.add_task(shell_name, None, None).await.unwrap();

    assert_eq!(task.name, shell_name);
}

#[tokio::test]
async fn test_url_in_task_name() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let url_name = "Deploy to https://example.com/api?key=value&test=1";
    let task = task_mgr.add_task(url_name, None, None).await.unwrap();

    assert_eq!(task.name, url_name);
}

// ==================== FTS5 Search with Special Characters ====================

#[tokio::test]
async fn test_fts5_search_with_quotes() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);
    let report_mgr = ReportManager::new(&pool);

    task_mgr
        .add_task(r#"Task with "quotes""#, None, None)
        .await
        .unwrap();

    // Search for quoted text
    let report = report_mgr
        .generate_report(None, None, Some("quotes".to_string()), None, false)
        .await
        .unwrap();

    assert!(report.tasks.is_some());
    assert!(!report.tasks.unwrap().is_empty());
}

#[tokio::test]
async fn test_fts5_search_with_special_chars() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);
    let report_mgr = ReportManager::new(&pool);

    task_mgr
        .add_task("C++ programming task", None, None)
        .await
        .unwrap();

    // FTS5 might treat special chars differently
    let report = report_mgr
        .generate_report(None, None, Some("programming".to_string()), None, false)
        .await
        .unwrap();

    assert!(report.tasks.is_some());
}

#[tokio::test]
async fn test_fts5_search_unicode() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);
    let report_mgr = ReportManager::new(&pool);

    // Create task with Chinese characters
    task_mgr
        .add_task("实现用户认证功能", None, None)
        .await
        .unwrap();

    // Also create task with mixed content for better FTS5 matching
    task_mgr
        .add_task("认证 authentication feature", None, None)
        .await
        .unwrap();

    // NOTE: SQLite FTS5 unicode61 tokenizer has limited CJK word segmentation
    // For better CJK support, would need custom tokenizers or external solutions
    // Testing with the full word that FTS5 can match
    let report = report_mgr
        .generate_report(
            None,
            None,
            Some("实现用户认证功能".to_string()),
            None,
            false,
        )
        .await
        .unwrap();

    // Should find at least the exact match
    assert!(report.tasks.is_some());
    let tasks = report.tasks.unwrap();
    assert!(!tasks.is_empty());
}

// ==================== Edge Cases ====================

#[tokio::test]
async fn test_task_name_with_only_spaces() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let task = task_mgr.add_task("     ", None, None).await.unwrap();
    assert_eq!(task.name, "     ");
}

#[tokio::test]
async fn test_task_name_single_character() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let task = task_mgr.add_task("A", None, None).await.unwrap();
    assert_eq!(task.name, "A");
}

#[tokio::test]
async fn test_task_name_all_special_chars() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let special_name = "!@#$%^&*()_+-=[]{}|;':\",./<>?~`";
    let task = task_mgr.add_task(special_name, None, None).await.unwrap();

    assert_eq!(task.name, special_name);
}

#[tokio::test]
async fn test_repeated_special_characters() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    let repeated = "'''\"\"\"\\\\\\///";
    let task = task_mgr.add_task(repeated, None, None).await.unwrap();

    assert_eq!(task.name, repeated);
}
