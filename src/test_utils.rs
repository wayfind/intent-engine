#[cfg(test)]
pub mod test_helpers {
    use crate::db::{create_pool, run_migrations};
    use sqlx::SqlitePool;
    use std::path::PathBuf;
    use tempfile::TempDir;

    pub struct TestContext {
        pub pool: SqlitePool,
        pub _temp_dir: TempDir,
    }

    impl TestContext {
        pub async fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");

            let pool = create_pool(&db_path).await.unwrap();
            run_migrations(&pool).await.unwrap();

            Self {
                pool,
                _temp_dir: temp_dir,
            }
        }

        pub fn pool(&self) -> &SqlitePool {
            &self.pool
        }
    }

    pub async fn setup_test_project() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();
        let intent_dir = path.join(".intent-engine");
        std::fs::create_dir_all(&intent_dir).unwrap();

        let db_path = intent_dir.join("project.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        (temp_dir, path)
    }
}
