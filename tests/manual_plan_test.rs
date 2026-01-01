//! 手动测试Plan工具的status和active_form功能
//! 运行: cargo test --test manual_plan_test -- --nocapture --ignored

use intent_engine::plan::{PlanExecutor, PlanRequest, PriorityValue, TaskStatus, TaskTree};
use intent_engine::project::ProjectContext;
use intent_engine::tasks::TaskManager;

#[tokio::test]
#[ignore]
async fn manual_test_plan_with_status_and_active_form() {
    println!("\n🚀 手动测试: Plan工具的status和active_form功能\n");

    let ctx = ProjectContext::load()
        .await
        .expect("Failed to load project");

    // 测试1: 创建pending状态的任务（避免单一in_progress约束）
    println!("📝 测试1: 创建pending状态的任务...");
    let request = PlanRequest {
        tasks: vec![TaskTree {
            name: Some("【测试】实现用户认证功能".to_string()),
            spec: Some("完整的JWT认证系统".to_string()),
            priority: Some(PriorityValue::Critical),
            children: Some(vec![
                TaskTree {
                    name: Some("【测试】设计JWT Token结构".to_string()),
                    spec: Some("定义payload和claims".to_string()),
                    priority: Some(PriorityValue::High),
                    children: None,
                    depends_on: None,
                    id: None,
                    status: Some(TaskStatus::Todo),
                    active_form: Some("设计JWT Token结构中".to_string()),
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("【测试】实现登录API".to_string()),
                    spec: Some("POST /api/login endpoint".to_string()),
                    priority: Some(PriorityValue::High),
                    children: None,
                    depends_on: None,
                    id: None,
                    status: Some(TaskStatus::Todo),
                    active_form: Some("实现登录API中".to_string()),
                    parent_id: None,
                    ..Default::default()
                },
            ]),
            depends_on: None,
            id: None,
            status: Some(TaskStatus::Todo), // 使用Todo避免约束冲突
            active_form: Some("正在实现用户认证功能".to_string()),
            parent_id: None,
            ..Default::default()
        }],
    };

    let executor = PlanExecutor::new(&ctx.pool);
    let result = executor
        .execute(&request)
        .await
        .expect("Plan execution failed");

    println!("✅ Plan执行结果:");
    println!("   - 成功: {}", result.success);
    println!("   - 创建任务数: {}", result.created_count);

    assert!(result.success, "Plan should succeed");
    assert_eq!(result.created_count, 3);

    // 读取并验证active_form字段
    println!("\n📖 测试2: 验证active_form字段传递...");
    let task_mgr = TaskManager::new(&ctx.pool);
    let result = task_mgr
        .find_tasks(None, None, None, None, None)
        .await
        .expect("Failed to fetch");

    let test_tasks: Vec<_> = result
        .tasks
        .into_iter()
        .filter(|t| t.name.starts_with("【测试】"))
        .collect();

    for task in &test_tasks {
        println!("   📌 #{}: {}", task.id, task.name);
        println!("      status: {}", task.status);
        println!("      active_form: {:?}", task.active_form);
        assert!(task.active_form.is_some());
    }

    // JSON序列化测试
    println!("\n🔄 测试3: JSON序列化（MCP输出格式）...");
    if let Some(task) = test_tasks.first() {
        let json = serde_json::to_string_pretty(task).unwrap();
        println!("{}", json);
        assert!(json.contains("active_form"));
    }

    // 测试单一in_progress约束
    println!("\n🚫 测试4: 验证单一in_progress约束...");
    let invalid = PlanRequest {
        tasks: vec![TaskTree {
            name: Some("【测试】违反约束的任务".to_string()),
            spec: None,
            priority: None,
            children: None,
            depends_on: None,
            id: None,
            status: Some(TaskStatus::Doing),
            active_form: Some("尝试违反约束".to_string()),
            parent_id: None,
            ..Default::default()
        }],
    };

    let result = executor.execute(&invalid).await.unwrap();
    println!(
        "   约束检查: {}",
        if result.success {
            "❌ 失败"
        } else {
            "✅ 通过"
        }
    );
    if let Some(err) = &result.error {
        println!("   错误信息: {}", err);
    }
    assert!(!result.success, "Should reject multiple in_progress");

    println!("\n🎉 所有测试通过！");
    println!("\n💡 清理命令: ie task list | grep 【测试】");
}
