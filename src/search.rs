/// CJK (Chinese, Japanese, Korean) search utilities
///
/// This module provides utilities for detecting CJK characters and determining
/// when to use LIKE fallback vs FTS5 trigram search.
///
/// **Background**: SQLite FTS5 with trigram tokenizer requires at least 3 consecutive
/// characters to match. This is problematic for CJK languages where single-character
/// or two-character searches are common (e.g., "用户", "认证").
///
/// **Solution**: For short CJK queries, we fallback to LIKE search which supports
/// any length substring matching, albeit slower.

/// Check if a character is a CJK character
pub fn is_cjk_char(c: char) -> bool {
    let code = c as u32;
    matches!(code,
        // CJK Unified Ideographs (most common Chinese characters)
        0x4E00..=0x9FFF |
        // CJK Extension A
        0x3400..=0x4DBF |
        // CJK Extension B-F (less common, but included for completeness)
        0x20000..=0x2A6DF |
        0x2A700..=0x2B73F |
        0x2B740..=0x2B81F |
        0x2B820..=0x2CEAF |
        0x2CEB0..=0x2EBEF |
        // Hiragana (Japanese)
        0x3040..=0x309F |
        // Katakana (Japanese)
        0x30A0..=0x30FF |
        // Hangul Syllables (Korean)
        0xAC00..=0xD7AF
    )
}

/// Determine if a query should use LIKE fallback instead of FTS5 trigram
///
/// Returns `true` if:
/// - Query is a single CJK character, OR
/// - Query is two CJK characters
///
/// Trigram tokenizer requires 3+ characters for matching, so we use LIKE
/// for shorter CJK queries to ensure they work.
pub fn needs_like_fallback(query: &str) -> bool {
    let chars: Vec<char> = query.chars().collect();

    // Single-character CJK
    if chars.len() == 1 && is_cjk_char(chars[0]) {
        return true;
    }

    // Two-character all-CJK
    // This is optional - could also let trigram handle it, but trigram
    // needs minimum 3 chars so two-char CJK won't work well
    if chars.len() == 2 && chars.iter().all(|c| is_cjk_char(*c)) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cjk_char() {
        // Chinese characters
        assert!(is_cjk_char('中'));
        assert!(is_cjk_char('文'));
        assert!(is_cjk_char('认'));
        assert!(is_cjk_char('证'));

        // Japanese Hiragana
        assert!(is_cjk_char('あ'));
        assert!(is_cjk_char('い'));

        // Japanese Katakana
        assert!(is_cjk_char('ア'));
        assert!(is_cjk_char('イ'));

        // Korean Hangul
        assert!(is_cjk_char('가'));
        assert!(is_cjk_char('나'));

        // Non-CJK
        assert!(!is_cjk_char('a'));
        assert!(!is_cjk_char('A'));
        assert!(!is_cjk_char('1'));
        assert!(!is_cjk_char(' '));
        assert!(!is_cjk_char('.'));
    }

    #[test]
    fn test_needs_like_fallback() {
        // Single CJK character - needs fallback
        assert!(needs_like_fallback("中"));
        assert!(needs_like_fallback("认"));
        assert!(needs_like_fallback("あ"));
        assert!(needs_like_fallback("가"));

        // Two CJK characters - needs fallback
        assert!(needs_like_fallback("中文"));
        assert!(needs_like_fallback("认证"));
        assert!(needs_like_fallback("用户"));

        // Three+ CJK characters - can use FTS5
        assert!(!needs_like_fallback("用户认"));
        assert!(!needs_like_fallback("用户认证"));

        // English - can use FTS5
        assert!(!needs_like_fallback("JWT"));
        assert!(!needs_like_fallback("auth"));
        assert!(!needs_like_fallback("a")); // Single ASCII char, not CJK

        // Mixed - can use FTS5
        assert!(!needs_like_fallback("JWT认证"));
        assert!(!needs_like_fallback("API接口"));
    }
}
