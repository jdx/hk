#[macro_export]
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

#[cfg(test)]
mod tests {
    use test_log::test;

    #[test]
    fn test_regex() {
        let re = regex!("^\\d{4}-\\d{2}-\\d{2}$");
        assert!(re.is_match("2021-01-01"));
        assert!(!re.is_match("2021-01-01-01"));
    }
}
