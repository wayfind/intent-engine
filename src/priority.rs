use crate::error::{IntentError, Result};
use std::str::FromStr;

/// Priority levels mapped to integers for storage and sorting
/// Lower number = Higher priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityLevel {
    Critical = 1,
    High = 2,
    Medium = 3,
    Low = 4,
}

impl FromStr for PriorityLevel {
    type Err = IntentError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Self::Critical),
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => Err(IntentError::InvalidInput(format!(
                "Invalid priority '{}'. Valid values: critical, high, medium, low",
                s
            ))),
        }
    }
}

impl PriorityLevel {
    /// Parse a priority string into integer value
    pub fn parse_to_int(s: &str) -> Result<i32> {
        let level: PriorityLevel = s.parse()?;
        Ok(level as i32)
    }

    /// Convert integer priority back to string
    pub fn to_str(priority: i32) -> &'static str {
        match priority {
            1 => "critical",
            2 => "high",
            3 => "medium",
            4 => "low",
            _ => "unknown",
        }
    }

    /// Parse optional priority string
    pub fn parse_optional(s: Option<&str>) -> Result<Option<i32>> {
        match s {
            Some(priority_str) => Ok(Some(Self::parse_to_int(priority_str)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_str() {
        assert_eq!(PriorityLevel::parse_to_int("critical").unwrap(), 1);
        assert_eq!(PriorityLevel::parse_to_int("high").unwrap(), 2);
        assert_eq!(PriorityLevel::parse_to_int("medium").unwrap(), 3);
        assert_eq!(PriorityLevel::parse_to_int("low").unwrap(), 4);
        assert_eq!(PriorityLevel::parse_to_int("CRITICAL").unwrap(), 1); // case insensitive
    }

    #[test]
    fn test_priority_from_str_invalid() {
        assert!(PriorityLevel::parse_to_int("invalid").is_err());
        assert!(PriorityLevel::parse_to_int("").is_err());
    }

    #[test]
    fn test_standard_fromstr_trait() {
        use std::str::FromStr;
        assert_eq!(
            PriorityLevel::from_str("critical").unwrap(),
            PriorityLevel::Critical
        );
        assert_eq!(
            PriorityLevel::from_str("high").unwrap(),
            PriorityLevel::High
        );
        assert_eq!(
            PriorityLevel::from_str("medium").unwrap(),
            PriorityLevel::Medium
        );
        assert_eq!(PriorityLevel::from_str("low").unwrap(), PriorityLevel::Low);
        assert!(PriorityLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_priority_to_str() {
        assert_eq!(PriorityLevel::to_str(1), "critical");
        assert_eq!(PriorityLevel::to_str(2), "high");
        assert_eq!(PriorityLevel::to_str(3), "medium");
        assert_eq!(PriorityLevel::to_str(4), "low");
        assert_eq!(PriorityLevel::to_str(999), "unknown");
    }

    #[test]
    fn test_parse_optional() {
        assert_eq!(
            PriorityLevel::parse_optional(Some("high")).unwrap(),
            Some(2)
        );
        assert_eq!(PriorityLevel::parse_optional(None).unwrap(), None);
        assert!(PriorityLevel::parse_optional(Some("invalid")).is_err());
    }
}
