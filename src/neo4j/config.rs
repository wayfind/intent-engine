use crate::error::{IntentError, Result};

/// Neo4j connection configuration parsed from environment variables.
///
/// Required:
///   NEO4J_URI       — e.g. "neo4j+s://xxx.databases.neo4j.io"
///   NEO4J_PASSWORD   — database password
///
/// Optional:
///   NEO4J_USER       — defaults to "neo4j"
///   NEO4J_PROJECT_ID — project isolation key, defaults to working directory
#[derive(Debug)]
pub struct Neo4jConfig {
    pub uri: String,
    pub user: String,
    pub password: String,
    pub project_id: String,
}

impl Neo4jConfig {
    /// Parse configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let uri = std::env::var("NEO4J_URI").map_err(|_| {
            IntentError::InvalidInput("NEO4J_URI environment variable not set".into())
        })?;

        let password = std::env::var("NEO4J_PASSWORD").map_err(|_| {
            IntentError::InvalidInput("NEO4J_PASSWORD environment variable not set".into())
        })?;

        let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".into());

        let project_id = std::env::var("NEO4J_PROJECT_ID").unwrap_or_else(|_| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "default".into())
        });

        Ok(Self {
            uri,
            user,
            password,
            project_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env_missing_uri() {
        std::env::remove_var("NEO4J_URI");
        std::env::remove_var("NEO4J_PASSWORD");
        let result = Neo4jConfig::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("NEO4J_URI"));
    }

    #[test]
    fn test_from_env_missing_password() {
        std::env::set_var("NEO4J_URI", "neo4j+s://test.example.com");
        std::env::remove_var("NEO4J_PASSWORD");
        let result = Neo4jConfig::from_env();
        std::env::remove_var("NEO4J_URI");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("NEO4J_PASSWORD"));
    }

    #[test]
    fn test_from_env_defaults() {
        std::env::set_var("NEO4J_URI", "neo4j+s://test.example.com");
        std::env::set_var("NEO4J_PASSWORD", "secret");
        std::env::remove_var("NEO4J_USER");
        std::env::remove_var("NEO4J_PROJECT_ID");

        let config = Neo4jConfig::from_env().unwrap();

        std::env::remove_var("NEO4J_URI");
        std::env::remove_var("NEO4J_PASSWORD");

        assert_eq!(config.uri, "neo4j+s://test.example.com");
        assert_eq!(config.user, "neo4j"); // default
        assert_eq!(config.password, "secret");
        // project_id falls back to cwd
        assert!(!config.project_id.is_empty());
    }

    #[test]
    fn test_from_env_custom_user_and_project() {
        std::env::set_var("NEO4J_URI", "neo4j+s://test.example.com");
        std::env::set_var("NEO4J_PASSWORD", "secret");
        std::env::set_var("NEO4J_USER", "admin");
        std::env::set_var("NEO4J_PROJECT_ID", "my-project");

        let config = Neo4jConfig::from_env().unwrap();

        std::env::remove_var("NEO4J_URI");
        std::env::remove_var("NEO4J_PASSWORD");
        std::env::remove_var("NEO4J_USER");
        std::env::remove_var("NEO4J_PROJECT_ID");

        assert_eq!(config.user, "admin");
        assert_eq!(config.project_id, "my-project");
    }
}
