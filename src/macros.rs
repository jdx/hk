#[macro_export]
macro_rules! tagged_warn {
    ($tag:expr, $($arg:tt)*) => {
        if !$crate::settings::Settings::get().hide_warnings.contains($tag) {
            warn!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! tagged_warn_missing_profiles {
    ($($arg:tt)*) => {
        $crate::tagged_warn!("missing-profiles", $($arg)*);
    };
}
