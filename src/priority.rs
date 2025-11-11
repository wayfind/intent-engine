use crate::error::{IntentError, Result};

/// Priority levels mapped to integers for storage and sorting
/// Lower number = Higher priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityLevel {
    Critical = 1,
    High = 2,
    Medium = 3,
    Low = 4,
}

impl PriorityLevel {
    /// Parse a priority string into integer value
    pub fn from_str(s: &str) -> Result<i32> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Self::Critical as i32),
            "high" => Ok(Self::High as i32),
            "medium" => Ok(Self::Medium as i32),
            "low" => Ok(Self::Low as i32),
            _ => Err(IntentError::InvalidInput(format!(
                "Invalid priority '{}'. Valid values: critical, high, medium, low",
                s
            ))),
        }
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
            Some(priority_str) => Ok(Some(Self::from_str(priority_str)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_str() {
        assert_eq!(PriorityLevel::from_str("critical").unwrap(), 1);
        assert_eq!(PriorityLevel::from_str("high").unwrap(), 2);
        assert_eq!(PriorityLevel::from_str("medium").unwrap(), 3);
        assert_eq!(PriorityLevel::from_str("low").unwrap(), 4);
        assert_eq!(PriorityLevel::from_str("CRITICAL").unwrap(), 1); // case insensitive
    }

    #[test]
    fn test_priority_from_str_invalid() {
        assert!(PriorityLevel::from_str("invalid").is_err());
        assert!(PriorityLevel::from_str("").is_err());
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
