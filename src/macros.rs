#[macro_export]
macro_rules! tagged_warn {
    ($tag:expr, $($arg:tt)*) => {
        if !$crate::env::HK_HIDE_WARNINGS.contains($tag) {
            warn!($($arg)*);
        }
    };
}
