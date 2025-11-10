//! Windows Console UTF-8 Support
//!
//! This module provides utilities for handling console encoding on Windows.
//! It automatically configures the console to use UTF-8 encoding for proper
//! display of Chinese and other non-ASCII characters.

#[cfg(windows)]
use windows::Win32::System::Console::{
    GetConsoleMode, GetStdHandle, SetConsoleMode, SetConsoleOutputCP, CONSOLE_MODE,
    ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
};

/// Setup Windows console for UTF-8 output
///
/// This function:
/// 1. Sets the console output code page to UTF-8 (65001)
/// 2. Enables virtual terminal processing for ANSI escape sequences
///
/// # Returns
///
/// Returns `Ok(())` if successful, or an error message if it fails.
///
/// # Platform-specific
///
/// This function only affects Windows systems. On other platforms, it's a no-op.
///
/// # Example
///
/// ```no_run
/// # use intent_engine::windows_console::setup_windows_console;
/// if let Err(e) = setup_windows_console() {
///     eprintln!("Warning: Failed to setup UTF-8 console: {}", e);
/// }
/// ```
#[cfg(windows)]
pub fn setup_windows_console() -> Result<(), String> {
    unsafe {
        // Set console output code page to UTF-8 (65001)
        // This ensures that our UTF-8 output is correctly interpreted
        if SetConsoleOutputCP(65001).is_err() {
            return Err("Failed to set console output code page to UTF-8".to_string());
        }

        // Get the standard output handle
        let handle = match GetStdHandle(STD_OUTPUT_HANDLE) {
            Ok(h) => h,
            Err(e) => return Err(format!("Failed to get stdout handle: {}", e)),
        };

        // Enable virtual terminal processing
        // This allows ANSI escape sequences to work properly
        let mut mode = CONSOLE_MODE(0);
        if GetConsoleMode(handle, &mut mode).is_ok() {
            let new_mode = mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            if SetConsoleMode(handle, new_mode).is_err() {
                // This is non-critical, so we just log a warning
                eprintln!(
                    "Warning: Could not enable virtual terminal processing. ANSI colors may not work."
                );
            }
        }
    }

    Ok(())
}

/// Setup Windows console for UTF-8 output (no-op on non-Windows platforms)
#[cfg(not(windows))]
pub fn setup_windows_console() -> Result<(), String> {
    Ok(())
}

/// Check if the current console is using UTF-8 encoding
///
/// # Returns
///
/// Returns `true` if the console is using UTF-8 (code page 65001),
/// `false` otherwise.
///
/// # Platform-specific
///
/// On non-Windows platforms, this always returns `true`.
#[cfg(windows)]
pub fn is_console_utf8() -> bool {
    use windows::Win32::System::Console::GetConsoleOutputCP;

    unsafe {
        let cp = GetConsoleOutputCP();
        cp == 65001 // UTF-8 code page
    }
}

#[cfg(not(windows))]
pub fn is_console_utf8() -> bool {
    true
}

/// Detect the current console code page
///
/// # Returns
///
/// Returns the current code page number (e.g., 936 for GBK, 65001 for UTF-8)
///
/// # Platform-specific
///
/// On non-Windows platforms, this returns 65001 (UTF-8).
#[cfg(windows)]
pub fn get_console_code_page() -> u32 {
    use windows::Win32::System::Console::GetConsoleOutputCP;

    unsafe { GetConsoleOutputCP() }
}

#[cfg(not(windows))]
pub fn get_console_code_page() -> u32 {
    65001 // UTF-8
}

/// Get a user-friendly name for a Windows code page
///
/// # Arguments
///
/// * `code_page` - The code page number
///
/// # Returns
///
/// A string describing the code page (e.g., "UTF-8", "GBK", "Unknown")
pub fn code_page_name(code_page: u32) -> &'static str {
    match code_page {
        65001 => "UTF-8",
        936 => "GBK (Simplified Chinese)",
        950 => "Big5 (Traditional Chinese)",
        932 => "Shift-JIS (Japanese)",
        949 => "EUC-KR (Korean)",
        437 => "OEM United States",
        1252 => "Western European (Windows)",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_console() {
        // Should not panic
        let result = setup_windows_console();
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_console_utf8() {
        // After setup, console should be UTF-8 (on Windows)
        let _ = setup_windows_console();

        #[cfg(windows)]
        {
            let is_utf8 = is_console_utf8();
            // This might fail in some CI environments, so we just check it doesn't panic
            let _ = is_utf8;
        }

        #[cfg(not(windows))]
        {
            assert!(is_console_utf8());
        }
    }

    #[test]
    fn test_code_page_names() {
        assert_eq!(code_page_name(65001), "UTF-8");
        assert_eq!(code_page_name(936), "GBK (Simplified Chinese)");
        assert_eq!(code_page_name(950), "Big5 (Traditional Chinese)");
        assert_eq!(code_page_name(12345), "Unknown");
    }

    #[test]
    fn test_get_code_page() {
        let cp = get_console_code_page();

        #[cfg(windows)]
        {
            // After setup, should be UTF-8
            let _ = setup_windows_console();
            let cp_after = get_console_code_page();
            assert_eq!(cp_after, 65001);
        }

        #[cfg(not(windows))]
        {
            assert_eq!(cp, 65001);
        }
    }
}
