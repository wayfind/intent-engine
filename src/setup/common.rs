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
    // First, try to use the current executable path (most reliable in test/dev environments)
    // When setup is called, it's running inside the `ie` binary, so current_exe() returns the ie path
    if let Ok(current_exe) = env::current_exe() {
        // Verify the binary name ends with 'ie' or 'intent-engine'
        if let Some(file_name) = current_exe.file_name() {
            let name = file_name.to_string_lossy();
            if name == "ie"
                || name.starts_with("ie.")
                || name == "intent-engine"
                || name.starts_with("intent-engine.")
            {
                return Ok(current_exe);
            }
        }
    }

    // Try CARGO_BIN_EXE_ie environment variable (set by cargo test in some cases)
    if let Ok(path) = env::var("CARGO_BIN_EXE_ie") {
        let binary = PathBuf::from(path);
        if binary.exists() {
            return Ok(binary);
        }
    }

    // Try to find `ie` in PATH
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
            // Try relative paths for development/testing
            let candidate_paths = vec![
                PathBuf::from("./target/debug/ie"),
                PathBuf::from("./target/release/ie"),
                PathBuf::from("../target/debug/ie"),
                PathBuf::from("../target/release/ie"),
            ];

            for path in candidate_paths {
                if path.exists() {
                    return Ok(path);
                }
            }

            Err(IntentError::InvalidInput(
                "ie binary not found in relative paths".to_string(),
            ))
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

    // ========== resolve_absolute_path tests ==========

    #[test]
    fn test_resolve_absolute_path() {
        let _temp = TempDir::new().unwrap();
        let rel_path = PathBuf::from("test.txt");
        let abs_path = resolve_absolute_path(&rel_path).unwrap();
        assert!(abs_path.is_absolute());
    }

    #[test]
    fn test_resolve_absolute_path_already_absolute() {
        let abs_path = PathBuf::from("/tmp/test.txt");
        let result = resolve_absolute_path(&abs_path).unwrap();
        assert!(result.is_absolute());
    }

    #[test]
    fn test_resolve_absolute_path_relative() {
        let rel_path = PathBuf::from("./test.txt");
        let result = resolve_absolute_path(&rel_path).unwrap();
        assert!(result.is_absolute());
    }

    // ========== get_home_dir tests ==========

    #[test]
    fn test_get_home_dir() {
        let result = get_home_dir();
        assert!(result.is_ok());

        let home = result.unwrap();
        assert!(home.is_absolute());
    }

    // ========== backup and restore tests ==========

    #[test]
    fn test_create_backup_creates_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.json");
        fs::write(&file_path, "original content").unwrap();

        // Create backup
        let backup = create_backup(&file_path).unwrap();
        assert!(backup.is_some());
        let backup_path = backup.unwrap();
        assert!(backup_path.exists());

        // Verify backup contains original content
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, "original content");
    }

    #[test]
    fn test_create_backup_nonexistent_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("nonexistent.txt");

        // Backup of non-existent file should return None
        let backup = create_backup(&file_path).unwrap();
        assert!(backup.is_none());
    }

    #[test]
    fn test_create_backup_filename_format() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.json");
        fs::write(&file_path, "content").unwrap();

        let backup = create_backup(&file_path).unwrap();
        assert!(backup.is_some());

        let backup_path = backup.unwrap();
        let filename = backup_path.file_name().unwrap().to_string_lossy();

        // Should contain .backup. in the filename
        assert!(filename.contains(".backup."));
    }

    // ========== JSON config tests ==========

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

    #[test]
    fn test_read_json_config_invalid_json() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("invalid.json");

        // Write invalid JSON
        fs::write(&config_path, "{invalid json}").unwrap();

        // Should return error
        let result = read_json_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_json_config_creates_parent_dir() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("nested").join("dir").join("config.json");

        let test_config = serde_json::json!({"test": "value"});
        write_json_config(&config_path, &test_config).unwrap();

        // Parent directory should be created
        assert!(config_path.parent().unwrap().exists());
        assert!(config_path.exists());
    }

    #[test]
    fn test_write_json_config_pretty_format() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.json");

        let test_config = serde_json::json!({
            "key": "value",
            "nested": {
                "item": 123
            }
        });
        write_json_config(&config_path, &test_config).unwrap();

        // Read as string to verify pretty formatting
        let content = fs::read_to_string(&config_path).unwrap();

        // Pretty-printed JSON should have newlines
        assert!(content.contains('\n'));
        assert!(content.contains("  ")); // Should have indentation
    }

    #[test]
    fn test_json_config_complex_types() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("complex.json");

        let test_config = serde_json::json!({
            "string": "value",
            "number": 42,
            "boolean": true,
            "null": null,
            "array": [1, 2, 3],
            "object": {
                "nested": "value"
            }
        });

        write_json_config(&config_path, &test_config).unwrap();
        let read_config = read_json_config(&config_path).unwrap();

        assert_eq!(read_config, test_config);
    }

    // ========== find_ie_binary tests ==========

    #[test]
    fn test_find_ie_binary() {
        // This test depends on the binary being available
        let result = find_ie_binary();
        // Should either find it or return a descriptive error
        match result {
            Ok(path) => {
                // If found, should be a valid path
                assert!(!path.to_string_lossy().is_empty());
            },
            Err(e) => {
                // Error message should be descriptive
                let msg = format!("{:?}", e);
                assert!(
                    msg.contains("binary not found")
                        || msg.contains("intent-engine")
                        || msg.contains("ie")
                );
            },
        }
    }

    // ========== set_executable tests ==========

    #[cfg(unix)]
    #[test]
    fn test_set_executable_unix() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("script.sh");
        fs::write(&file_path, "#!/bin/bash\necho test").unwrap();

        // Set executable
        set_executable(&file_path).unwrap();

        // Check permissions
        let metadata = fs::metadata(&file_path).unwrap();
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        // Should have execute permissions (0o755)
        assert_ne!(mode & 0o111, 0); // At least one execute bit set
    }

    #[cfg(not(unix))]
    #[test]
    fn test_set_executable_non_unix() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("script.sh");
        fs::write(&file_path, "echo test").unwrap();

        // Should not error on non-Unix platforms
        let result = set_executable(&file_path);
        assert!(result.is_ok());
    }
}
