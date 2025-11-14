/// Comprehensive CJK (Chinese, Japanese, Korean) search tests
///
/// Tests the trigram + LIKE fallback search implementation for CJK languages
use intent_engine::db::{create_pool, run_migrations};
use intent_engine::tasks::TaskManager;
use tempfile::TempDir;

async fn setup_test_db() -> (TempDir, sqlx::SqlitePool) {
    let temp_dir = TempDir::new().unwrap();
    let intent_dir = temp_dir.path().join(".intent-engine");
    std::fs::create_dir_all(&intent_dir).unwrap();

    let db_path = intent_dir.join("project.db");
    let pool = create_pool(&db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();
    (temp_dir, pool)
}

// ================================
// 1. 单字搜索测试 (LIKE fallback)
// ================================

#[tokio::test]
async fn test_chinese_single_char_search() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    // 创建任务
    task_mgr
        .add_task("实现用户认证功能", Some("使用JWT实现登录"), None)
        .await
        .unwrap();

    // 单字搜索测试
    let results = task_mgr.search_tasks("用").await.unwrap();
    assert_eq!(results.len(), 1, "应该找到1个包含'用'的任务");
    assert!(results[0].task.name.contains("用户"));

    let results = task_mgr.search_tasks("认").await.unwrap();
    assert_eq!(results.len(), 1, "应该找到1个包含'认'的任务");

    let results = task_mgr.search_tasks("证").await.unwrap();
    assert_eq!(results.len(), 1, "应该找到1个包含'证'的任务");

    let results = task_mgr.search_tasks("功").await.unwrap();
    assert_eq!(results.len(), 1, "应该找到1个包含'功'的任务");
}

// ================================
// 2. 双字词搜索测试 (LIKE fallback)
// ================================

#[tokio::test]
async fn test_chinese_two_char_search() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    task_mgr.add_task("实现用户认证", None, None).await.unwrap();
    task_mgr
        .add_task("添加数据库索引", None, None)
        .await
        .unwrap();
    task_mgr.add_task("优化查询性能", None, None).await.unwrap();

    // 测试常见双字词
    let test_cases = vec![
        ("用户", "实现用户认证"),
        ("认证", "实现用户认证"),
        ("数据", "添加数据库索引"),
        ("索引", "添加数据库索引"),
        ("查询", "优化查询性能"),
        ("性能", "优化查询性能"),
    ];

    for (query, expected_name) in test_cases {
        let results = task_mgr.search_tasks(query).await.unwrap();
        assert!(
            results.iter().any(|r| r.task.name.contains(expected_name)),
            "搜索'{}'应该找到'{}'",
            query,
            expected_name
        );
    }
}

// =====================================
// 3. 多字符搜索测试 (FTS5 trigram)
// =====================================

#[tokio::test]
async fn test_chinese_multi_char_search() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    task_mgr
        .add_task("实现JWT用户认证功能", Some("基于Token的认证机制"), None)
        .await
        .unwrap();
    task_mgr
        .add_task("优化数据库查询性能", Some("添加索引提升查询速度"), None)
        .await
        .unwrap();

    // 三字词 (FTS5 trigram)
    let results = task_mgr.search_tasks("用户认").await.unwrap();
    assert_eq!(results.len(), 1, "三字词搜索应该使用FTS5");

    // 四字词
    let results = task_mgr.search_tasks("用户认证").await.unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].task.name.contains("认证"));

    // 五字及以上
    let results = task_mgr.search_tasks("用户认证功能").await.unwrap();
    assert_eq!(results.len(), 1);

    let results = task_mgr.search_tasks("数据库查询").await.unwrap();
    assert_eq!(results.len(), 1);

    let results = task_mgr.search_tasks("查询性能").await.unwrap();
    assert_eq!(results.len(), 1);
}

// ================================
// 4. 中英文混合搜索
// ================================

#[tokio::test]
async fn test_mixed_language_search() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    task_mgr
        .add_task("实现JWT认证", Some("JSON Web Token认证"), None)
        .await
        .unwrap();
    task_mgr
        .add_task("添加API接口", Some("RESTful API设计"), None)
        .await
        .unwrap();
    task_mgr
        .add_task("配置OAuth2流程", None, None)
        .await
        .unwrap();

    // 搜索英文部分
    let results = task_mgr.search_tasks("JWT").await.unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].task.name.contains("JWT"));

    let results = task_mgr.search_tasks("API").await.unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].task.name.contains("API"));

    let results = task_mgr.search_tasks("OAuth2").await.unwrap();
    assert_eq!(results.len(), 1);

    // 搜索中文部分（双字词，LIKE）
    let results = task_mgr.search_tasks("认证").await.unwrap();
    assert_eq!(results.len(), 1);

    let results = task_mgr.search_tasks("接口").await.unwrap();
    assert_eq!(results.len(), 1);

    let results = task_mgr.search_tasks("流程").await.unwrap();
    assert_eq!(results.len(), 1);
}

// ================================
// 5. 日文搜索测试
// ================================

#[tokio::test]
async fn test_japanese_search() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    task_mgr
        .add_task("ユーザー認証を実装", Some("JWTトークンを使用"), None)
        .await
        .unwrap();
    task_mgr
        .add_task("データベース索引", Some("性能向上"), None)
        .await
        .unwrap();

    // 单字（平假名）
    let results = task_mgr.search_tasks("ユ").await.unwrap();
    assert_eq!(results.len(), 1, "应该找到包含'ユ'的任务");

    // 双字（片假名）
    let results = task_mgr.search_tasks("認証").await.unwrap();
    assert_eq!(results.len(), 1);

    // 多字
    let results = task_mgr.search_tasks("ユーザー").await.unwrap();
    assert_eq!(results.len(), 1);

    let results = task_mgr.search_tasks("データベース").await.unwrap();
    assert_eq!(results.len(), 1);

    // 平假名
    let results = task_mgr.search_tasks("を").await.unwrap();
    assert_eq!(results.len(), 1);
}

// ================================
// 6. 特殊场景测试
// ================================

#[tokio::test]
async fn test_edge_cases() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    // 标点符号
    task_mgr
        .add_task("实现：用户认证", None, None)
        .await
        .unwrap();
    let results = task_mgr.search_tasks("用户").await.unwrap();
    assert_eq!(results.len(), 1, "应该忽略标点符号");

    // 数字混合
    task_mgr
        .add_task("实现OAuth2认证", None, None)
        .await
        .unwrap();
    let results = task_mgr.search_tasks("认证").await.unwrap();
    assert_eq!(results.len(), 2, "应该找到2个包含'认证'的任务");

    // 空格
    task_mgr
        .add_task("实现 用户 认证", None, None)
        .await
        .unwrap();
    let results = task_mgr.search_tasks("用户").await.unwrap();
    assert!(results.len() >= 1, "应该能处理空格");

    // Spec中的搜索
    task_mgr
        .add_task("任务标题", Some("描述中包含用户信息"), None)
        .await
        .unwrap();
    let results = task_mgr.search_tasks("用户").await.unwrap();
    assert!(results.len() >= 1, "应该能搜索spec字段");
}

// ================================
// 7. 性能测试
// ================================

#[tokio::test]
async fn test_search_performance() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    // 创建1000个任务
    for i in 0..1000 {
        task_mgr
            .add_task(
                &format!("任务{}: 实现功能{}", i, i % 10),
                Some(&format!("这是任务{}的详细说明，包含各种关键词", i)),
                None,
            )
            .await
            .unwrap();
    }

    // 测试FTS5 trigram搜索性能
    let start = std::time::Instant::now();
    let _results = task_mgr.search_tasks("功能").await.unwrap();
    let duration = start.elapsed();
    assert!(
        duration.as_millis() < 100,
        "FTS5搜索耗时{}ms，超过100ms阈值",
        duration.as_millis()
    );

    // 测试LIKE fallback搜索性能
    let start = std::time::Instant::now();
    let _results = task_mgr.search_tasks("任").await.unwrap(); // 单字CJK，使用LIKE
    let duration = start.elapsed();
    assert!(
        duration.as_millis() < 500,
        "LIKE搜索耗时{}ms，超过500ms阈值",
        duration.as_millis()
    );
}

// ================================
// 8. 边界情况和回归测试
// ================================

#[tokio::test]
async fn test_empty_and_special_queries() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    task_mgr.add_task("测试任务", None, None).await.unwrap();

    // 空查询
    let results = task_mgr.search_tasks("").await;
    assert!(results.is_ok(), "空查询应该返回空结果而不是错误");

    // 仅空格
    let results = task_mgr.search_tasks("   ").await;
    assert!(results.is_ok());

    // 特殊字符
    let results = task_mgr.search_tasks("@#$%").await;
    assert!(results.is_ok());
}

#[tokio::test]
async fn test_case_sensitivity() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    task_mgr
        .add_task("Implement API", None, None)
        .await
        .unwrap();

    // 英文大小写不敏感（FTS5特性）
    let results_upper = task_mgr.search_tasks("API").await.unwrap();
    let results_lower = task_mgr.search_tasks("api").await.unwrap();

    // FTS5默认不区分大小写
    assert_eq!(results_upper.len(), results_lower.len());
}

#[tokio::test]
async fn test_korean_search() {
    let (_temp_dir, pool) = setup_test_db().await;
    let task_mgr = TaskManager::new(&pool);

    task_mgr
        .add_task("사용자 인증 구현", Some("JWT 토큰 사용"), None)
        .await
        .unwrap();

    // 单字韩文
    let results = task_mgr.search_tasks("사").await.unwrap();
    assert_eq!(results.len(), 1, "应该找到包含韩文的任务");

    // 双字韩文
    let results = task_mgr.search_tasks("사용").await.unwrap();
    assert_eq!(results.len(), 1);

    // 多字韩文
    let results = task_mgr.search_tasks("사용자").await.unwrap();
    assert_eq!(results.len(), 1);
}
