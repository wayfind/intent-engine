//! Time utility functions
//!
//! Provides common time-related operations used across the codebase.

use crate::error::{IntentError, Result};
use chrono::{DateTime, Duration, Utc};

/// Parse a duration string (e.g., "7d", "24h", "30m") into a DateTime
///
/// Supported units:
/// - `d`: days
/// - `h`: hours
/// - `m`: minutes
/// - `s`: seconds
/// - `w`: weeks
///
/// # Arguments
/// * `duration` - Duration string in format like "7d", "24h", "30m", "5w"
///
/// # Returns
/// A DateTime representing the current time minus the specified duration
///
/// # Errors
/// Returns InvalidInput error if:
/// - Duration string is empty or too short
/// - Number part is not a valid integer
/// - Unit is not one of d/h/m/s/w
///
/// # Examples
/// ```ignore
/// use crate::time_utils::parse_duration;
///
/// let seven_days_ago = parse_duration("7d").unwrap();
/// let one_week_ago = parse_duration("1w").unwrap();
/// ```
pub fn parse_duration(duration: &str) -> Result<DateTime<Utc>> {
    let duration = duration.trim();

    if duration.len() < 2 {
        return Err(IntentError::InvalidInput(
            "Duration must be in format like '7d', '24h', '30m', '5w', or '10s'".to_string(),
        ));
    }

    let (num_str, unit) = duration.split_at(duration.len() - 1);
    let num: i64 = num_str.parse().map_err(|_| {
        IntentError::InvalidInput(format!("Invalid number in duration: '{}'", num_str))
    })?;

    let offset = match unit {
        "d" => Duration::days(num),
        "h" => Duration::hours(num),
        "m" => Duration::minutes(num),
        "s" => Duration::seconds(num),
        "w" => Duration::weeks(num),
        _ => {
            return Err(IntentError::InvalidInput(format!(
                "Invalid duration unit '{}'. Use 'd' (days), 'h' (hours), 'm' (minutes), 's' (seconds), or 'w' (weeks)",
                unit
            )))
        }
    };

    Ok(Utc::now() - offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_days() {
        let result = parse_duration("7d").unwrap();
        let expected_diff = Duration::days(7);
        let actual_diff = Utc::now() - result;

        // Allow 1 second tolerance for test execution time
        assert!((actual_diff - expected_diff).num_seconds().abs() <= 1);
    }

    #[test]
    fn test_parse_duration_hours() {
        let result = parse_duration("24h").unwrap();
        let expected_diff = Duration::hours(24);
        let actual_diff = Utc::now() - result;

        assert!((actual_diff - expected_diff).num_seconds().abs() <= 1);
    }

    #[test]
    fn test_parse_duration_minutes() {
        let result = parse_duration("30m").unwrap();
        let expected_diff = Duration::minutes(30);
        let actual_diff = Utc::now() - result;

        assert!((actual_diff - expected_diff).num_seconds().abs() <= 1);
    }

    #[test]
    fn test_parse_duration_seconds() {
        let result = parse_duration("10s").unwrap();
        let expected_diff = Duration::seconds(10);
        let actual_diff = Utc::now() - result;

        assert!((actual_diff - expected_diff).num_seconds().abs() <= 1);
    }

    #[test]
    fn test_parse_duration_weeks() {
        let result = parse_duration("2w").unwrap();
        let expected_diff = Duration::weeks(2);
        let actual_diff = Utc::now() - result;

        assert!((actual_diff - expected_diff).num_seconds().abs() <= 1);
    }

    #[test]
    fn test_parse_duration_with_whitespace() {
        let result = parse_duration("  7d  ").unwrap();
        let expected_diff = Duration::days(7);
        let actual_diff = Utc::now() - result;

        assert!((actual_diff - expected_diff).num_seconds().abs() <= 1);
    }

    #[test]
    fn test_parse_duration_invalid_number() {
        let result = parse_duration("abc d");
        assert!(matches!(result, Err(IntentError::InvalidInput(_))));
    }

    #[test]
    fn test_parse_duration_invalid_unit() {
        let result = parse_duration("7x");
        assert!(matches!(result, Err(IntentError::InvalidInput(_))));

        if let Err(IntentError::InvalidInput(msg)) = result {
            assert!(msg.contains("Invalid duration unit"));
        }
    }

    #[test]
    fn test_parse_duration_too_short() {
        let result = parse_duration("7");
        assert!(matches!(result, Err(IntentError::InvalidInput(_))));
    }

    #[test]
    fn test_parse_duration_empty() {
        let result = parse_duration("");
        assert!(matches!(result, Err(IntentError::InvalidInput(_))));
    }
}
