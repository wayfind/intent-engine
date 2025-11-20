use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Get PID file path for a Dashboard instance
pub fn pid_file_path(port: u16) -> PathBuf {
    // Use temp directory for PID files
    let temp_dir = std::env::temp_dir();
    temp_dir.join(format!("ie-dashboard-{}.pid", port))
}

/// Write PID to file
pub fn write_pid_file(port: u16, pid: u32) -> Result<()> {
    let path = pid_file_path(port);
    fs::write(&path, pid.to_string())
        .with_context(|| format!("Failed to write PID file: {}", path.display()))?;
    Ok(())
}

/// Read PID from file
pub fn read_pid_file(port: u16) -> Result<Option<u32>> {
    let path = pid_file_path(port);

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read PID file: {}", path.display()))?;

    let pid = content
        .trim()
        .parse::<u32>()
        .context("Invalid PID in file")?;

    Ok(Some(pid))
}

/// Delete PID file
pub fn delete_pid_file(port: u16) -> Result<()> {
    let path = pid_file_path(port);

    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete PID file: {}", path.display()))?;
    }

    Ok(())
}

/// Check if a process is running
#[cfg(unix)]
pub fn is_process_running(pid: u32) -> bool {
    use std::process::Command;

    // Use kill -0 to check if process exists
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
pub fn is_process_running(pid: u32) -> bool {
    use std::process::Command;

    // Use tasklist to check if process exists
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid)])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

/// Stop a process by PID
#[cfg(unix)]
pub fn stop_process(pid: u32) -> Result<()> {
    use std::process::Command;

    // Send SIGTERM
    let output = Command::new("kill")
        .arg(pid.to_string())
        .output()
        .context("Failed to send SIGTERM")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to stop process {}: {}",
            pid,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

#[cfg(windows)]
pub fn stop_process(pid: u32) -> Result<()> {
    use std::process::Command;

    // Use taskkill
    let output = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output()
        .context("Failed to kill process")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to stop process {}: {}",
            pid,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Open URL in default browser
pub fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .context("Failed to open browser")?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context("Failed to open browser")?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("Failed to open browser")?;
    }

    Ok(())
}

/// Daemonize the current process (Unix only)
/// NOTE: This is a placeholder for Step 3. Actual daemon implementation
/// will be done when we implement the HTTP server.
#[cfg(unix)]
pub fn daemonize() -> Result<()> {
    // TODO: Step 3 will implement proper daemonization
    // For now, this is just a placeholder
    Ok(())
}

/// On Windows, daemonize placeholder
#[cfg(windows)]
pub fn daemonize() -> Result<()> {
    // TODO: Step 3 will implement Windows service/detach
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_file_operations() {
        let port = 9999;
        let pid = std::process::id();

        // Write PID
        write_pid_file(port, pid).unwrap();

        // Read PID
        let read_pid = read_pid_file(port).unwrap();
        assert_eq!(read_pid, Some(pid));

        // Delete PID
        delete_pid_file(port).unwrap();

        // Verify deleted
        let read_pid_after = read_pid_file(port).unwrap();
        assert_eq!(read_pid_after, None);
    }

    #[test]
    fn test_is_process_running() {
        let current_pid = std::process::id();

        // Current process should be running
        assert!(is_process_running(current_pid));

        // Invalid PID should not be running
        assert!(!is_process_running(999999));
    }
}
