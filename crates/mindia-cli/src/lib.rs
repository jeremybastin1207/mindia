/// Truncate a string to max_len characters, appending "..." if truncated.
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_string_short() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("", 5), "");
    }

    #[test]
    fn truncate_string_exact() {
        assert_eq!(truncate_string("hello", 5), "hello");
    }

    #[test]
    fn truncate_string_long() {
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("ab", 2), "ab");
        // max_len=2: 2-3=0 chars before "..."
        assert_eq!(truncate_string("abc", 2), "...");
    }

    #[test]
    fn truncate_string_very_short_max() {
        assert_eq!(truncate_string("hello", 0), "...");
        assert_eq!(truncate_string("hi", 1), "...");
    }
}

/// Initialize tracing for CLI binaries.
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}
