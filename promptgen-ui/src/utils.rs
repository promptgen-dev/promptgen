//! Utility functions for the promptgen UI.

/// Maximum characters before truncation
pub const TRUNCATE_MAX_CHARS: usize = 200;

/// Truncate a string to `TRUNCATE_MAX_CHARS` characters, appending "..." if truncated.
/// Handles unicode character boundaries safely.
pub fn truncate(s: &str) -> std::borrow::Cow<'_, str> {
    if s.chars().count() <= TRUNCATE_MAX_CHARS {
        std::borrow::Cow::Borrowed(s)
    } else {
        let truncated: String = s.chars().take(TRUNCATE_MAX_CHARS).collect();
        std::borrow::Cow::Owned(format!("{}...", truncated))
    }
}
