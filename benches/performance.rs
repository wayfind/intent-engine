use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use intent_engine::db::{create_pool, run_migrations};
use intent_engine::events::EventManager;
use intent_engine::report::ReportManager;
use intent_engine::tasks::TaskManager;
use std::hint::black_box;
use tempfile::TempDir;
use tokio::runtime::Runtime;

async fn setup_test_db() -> (TempDir, sqlx::SqlitePool) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let pool = create_pool(&db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();
    (temp_dir, pool)
}

fn bench_task_add(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("task_add", |b| {
        b.to_async(&rt).iter(|| async {
            let (_temp_dir, pool) = setup_test_db().await;
            let task_mgr = TaskManager::new(&pool);

            task_mgr
                .add_task("Benchmark task", None, None)
                .await
                .unwrap();
        });
    });
}

fn bench_task_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("task_get", |b| {
        b.to_async(&rt).iter(|| async {
            let (_temp_dir, pool) = setup_test_db().await;
            let task_mgr = TaskManager::new(&pool);

            let task = task_mgr.add_task("Test task", None, None).await.unwrap();

            black_box(task_mgr.get_task(task.id).await.unwrap());
        });
    });
}

fn bench_task_update(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("task_update", |b| {
        b.to_async(&rt).iter(|| async {
            let (_temp_dir, pool) = setup_test_db().await;
            let task_mgr = TaskManager::new(&pool);

            let task = task_mgr
                .add_task("Original name", None, None)
                .await
                .unwrap();

            task_mgr
                .update_task(
                    task.id,
                    Some("New name"),
                    None,
                    None,
                    Some("doing"),
                    None,
                    None,
                )
                .await
                .unwrap();
        });
    });
}

fn bench_task_find(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("task_find");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let (_temp_dir, pool) = setup_test_db().await;
                let task_mgr = TaskManager::new(&pool);

                // Create tasks
                for i in 0..size {
                    task_mgr
                        .add_task(&format!("Task {}", i), None, None)
                        .await
                        .unwrap();
                }

                // Benchmark find
                black_box(task_mgr.find_tasks(None, None).await.unwrap());
            });
        });
    }
    group.finish();
}

fn bench_event_add(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("event_add", |b| {
        b.to_async(&rt).iter(|| async {
            let (_temp_dir, pool) = setup_test_db().await;
            let task_mgr = TaskManager::new(&pool);
            let event_mgr = EventManager::new(&pool);

            let task = task_mgr.add_task("Test task", None, None).await.unwrap();

            event_mgr
                .add_event(task.id, "decision", "Benchmark decision")
                .await
                .unwrap();
        });
    });
}

fn bench_event_list(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("event_list");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let (_temp_dir, pool) = setup_test_db().await;
                let task_mgr = TaskManager::new(&pool);
                let event_mgr = EventManager::new(&pool);

                let task = task_mgr.add_task("Test task", None, None).await.unwrap();

                // Create events
                for i in 0..size {
                    event_mgr
                        .add_event(task.id, "test", &format!("Event {}", i))
                        .await
                        .unwrap();
                }

                // Benchmark list
                black_box(
                    event_mgr
                        .list_events(Some(task.id), None, None, None)
                        .await
                        .unwrap(),
                );
            });
        });
    }
    group.finish();
}

fn bench_report_summary(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("report_summary");

    for size in [100, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let (_temp_dir, pool) = setup_test_db().await;
                let task_mgr = TaskManager::new(&pool);
                let report_mgr = ReportManager::new(&pool);

                // Create tasks with different statuses
                for i in 0..size {
                    let task = task_mgr
                        .add_task(&format!("Task {}", i), None, None)
                        .await
                        .unwrap();

                    match i % 3 {
                        0 => {}, // keep as todo
                        1 => {
                            task_mgr.start_task(task.id, false).await.unwrap();
                        },
                        _ => {
                            task_mgr.start_task(task.id, false).await.unwrap();
                            task_mgr.done_task().await.unwrap();
                        },
                    }
                }

                // Benchmark report
                black_box(
                    report_mgr
                        .generate_report(None, None, None, None, true)
                        .await
                        .unwrap(),
                );
            });
        });
    }
    group.finish();
}

fn bench_task_hierarchy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("task_hierarchy");

    for depth in [5, 10, 20].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, &depth| {
            b.to_async(&rt).iter(|| async move {
                let (_temp_dir, pool) = setup_test_db().await;
                let task_mgr = TaskManager::new(&pool);

                // Create deep hierarchy
                let mut parent_id = None;
                for i in 0..depth {
                    let task = task_mgr
                        .add_task(&format!("Level {}", i), None, parent_id)
                        .await
                        .unwrap();
                    parent_id = Some(task.id);
                }
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_task_add,
    bench_task_get,
    bench_task_update,
    bench_task_find,
    bench_event_add,
    bench_event_list,
    bench_report_summary,
    bench_task_hierarchy,
);
criterion_main!(benches);
