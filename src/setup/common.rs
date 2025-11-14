//! Common utilities for setup operations

use crate::error::{IntentError, Result};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolve a path to absolute canonical form
pub fn resolve_absolute_path(path: &Path) -> Result<PathBuf> {
    path.canonicalize().or_else(|_| {
        // If canonicalize fails (e.g., file doesn't exist yet),
        // try to make it absolute relative to current dir
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            let current_dir = env::current_dir().map_err(IntentError::IoError)?;
            Ok(current_dir.join(path))
        }
    })
}

/// Get the home directory
pub fn get_home_dir() -> Result<PathBuf> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(PathBuf::from)
        .map_err(|_| IntentError::InvalidInput("Cannot determine home directory".to_string()))
}

/// Create a backup of a file if it exists
pub fn create_backup(file_path: &Path) -> Result<Option<PathBuf>> {
    if !file_path.exists() {
        return Ok(None);
    }

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = file_path.with_extension(format!(
        "{}.backup.{}",
        file_path.extension().and_then(|s| s.to_str()).unwrap_or(""),
        timestamp
    ));

    fs::copy(file_path, &backup_path).map_err(IntentError::IoError)?;
    Ok(Some(backup_path))
}

/// Restore from a backup file
pub fn restore_from_backup(backup_path: &Path, original_path: &Path) -> Result<()> {
    if backup_path.exists() {
        fs::copy(backup_path, original_path).map_err(IntentError::IoError)?;
    }
    Ok(())
}

/// Remove a backup file
pub fn remove_backup(backup_path: &Path) -> Result<()> {
    if backup_path.exists() {
        fs::remove_file(backup_path).map_err(IntentError::IoError)?;
    }
    Ok(())
}

/// Read a JSON config file, or return empty object if it doesn't exist
pub fn read_json_config(path: &Path) -> Result<Value> {
    if path.exists() {
        let content = fs::read_to_string(path).map_err(IntentError::IoError)?;
        serde_json::from_str(&content)
            .map_err(|e| IntentError::InvalidInput(format!("Failed to parse JSON config: {}", e)))
    } else {
        Ok(serde_json::json!({}))
    }
}

/// Write a JSON config file with pretty formatting
pub fn write_json_config(path: &Path, config: &Value) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(IntentError::IoError)?;
    }

    let content = serde_json::to_string_pretty(config)?;
    fs::write(path, content).map_err(IntentError::IoError)?;
    Ok(())
}

/// Find the ie binary path
pub fn find_ie_binary() -> Result<PathBuf> {
    // First try to find `ie` in PATH
    which::which("ie")
        .or_else(|_| {
            // Try ~/.cargo/bin/ie
            let home = get_home_dir()?;
            let cargo_bin = home.join(".cargo").join("bin").join("ie");
            if cargo_bin.exists() {
                Ok(cargo_bin)
            } else {
                Err(IntentError::InvalidInput(
                    "ie binary not found in PATH or ~/.cargo/bin".to_string(),
                ))
            }
        })
        .or_else(|_| {
            // Fallback: try old `intent-engine` name (for backward compatibility)
            which::which("intent-engine").map_err(|_| {
                IntentError::InvalidInput(
                    "intent-engine binary not found. Please install with: cargo install intent-engine".to_string()
                )
            })
        })
}

/// Set executable permissions on Unix platforms
#[cfg(unix)]
pub fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(IntentError::IoError)?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).map_err(IntentError::IoError)?;
    Ok(())
}

/// Set executable permissions on non-Unix platforms (no-op)
#[cfg(not(unix))]
pub fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_absolute_path() {
        let _temp = TempDir::new().unwrap();
        let rel_path = PathBuf::from("test.txt");
        let abs_path = resolve_absolute_path(&rel_path).unwrap();
        assert!(abs_path.is_absolute());
    }

    #[test]
    fn test_backup_and_restore() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.json");
        fs::write(&file_path, "original content").unwrap();

        // Create backup
        let backup = create_backup(&file_path).unwrap();
        assert!(backup.is_some());
        let backup_path = backup.unwrap();
        assert!(backup_path.exists());

        // Modify original
        fs::write(&file_path, "modified content").unwrap();

        // Restore from backup
        restore_from_backup(&backup_path, &file_path).unwrap();
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "original content");

        // Clean up
        remove_backup(&backup_path).unwrap();
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_json_config_ops() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.json");

        // Read non-existent file
        let config = read_json_config(&config_path).unwrap();
        assert_eq!(config, serde_json::json!({}));

        // Write config
        let test_config = serde_json::json!({
            "key": "value",
            "number": 42
        });
        write_json_config(&config_path, &test_config).unwrap();
        assert!(config_path.exists());

        // Read back
        let read_config = read_json_config(&config_path).unwrap();
        assert_eq!(read_config, test_config);
    }
}
