use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

pub static DEPRECATED: LazyLock<Mutex<HashSet<&'static str>>> = LazyLock::new(Default::default);

/// Emit a deprecation warning once per process.
#[allow(unused_macros)]
macro_rules! deprecated {
    ($id:tt, $($arg:tt)*) => {{
        if $crate::output::DEPRECATED.lock().unwrap().insert($id) {
            warn!("deprecated [{}]: {}", $id, format!($($arg)*));
        }
    }};
}

/// Emits a deprecation warning when hk version >= warn_at, and fires a debug_assert
/// when version >= remove_at to remind developers to remove the deprecated code.
/// The removal version is automatically appended to the warning message.
///
/// # Example
/// ```ignore
/// deprecated_at!("1.37.0", "2.0.0", "hkrc-home",
///     "~/.hkrc.pkl is deprecated. Use ~/.config/hk/config.pkl (Linux) or ~/Library/Application Support/hk/config.pkl (macOS) instead.");
/// ```
macro_rules! deprecated_at {
    ($warn_at:tt, $remove_at:tt, $id:tt, $($arg:tt)*) => {{
        let warn_ver = semver::Version::parse($warn_at)
            .expect("invalid warn_at version in deprecated_at!");
        let remove_ver = semver::Version::parse($remove_at)
            .expect("invalid remove_at version in deprecated_at!");
        debug_assert!(
            *$crate::version::VERSION < remove_ver,
            "Deprecated code [{}] should have been removed in version {}. \
             Please remove this deprecated functionality.",
            $id, $remove_at
        );
        if *$crate::version::VERSION >= warn_ver {
            if $crate::output::DEPRECATED.lock().unwrap().insert($id) {
                warn!("deprecated [{}]: {} This will be removed in hk {}.", $id, format!($($arg)*), $remove_at);
            }
        }
    }};
}
