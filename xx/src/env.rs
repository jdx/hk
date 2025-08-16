//! Environment variable utilities
//!
//! This module provides convenient functions for working with environment variables,
//! including parsing boolean values, paths, and numeric types.
//!
//! ## Examples
//!
//! ```rust
//! use xx::env;
//!
//! // Check if an environment variable is truthy
//! unsafe { std::env::set_var("DEBUG", "1"); }
//! assert!(env::var_is_true("DEBUG"));
//!
//! // Parse a path with tilde expansion  
//! unsafe { std::env::set_var("CONFIG_PATH", "~/config"); }
//! let path = env::var_path("CONFIG_PATH");
//!
//! // Parse numeric values
//! unsafe { std::env::set_var("THREADS", "4"); }
//! let threads = env::var_u32("THREADS").unwrap_or(1);
//! ```

use std::env;
use std::path::PathBuf;

/// Check if an environment variable is set to a truthy value
///
/// Truthy values: "1", "true", "yes", "on" (case-insensitive)
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("ENABLED", "true"); }
/// assert!(env::var_is_true("ENABLED"));
/// ```
pub fn var_is_true<K: AsRef<str>>(key: K) -> bool {
    match env::var(key.as_ref()) {
        Ok(val) => {
            let val = val.to_lowercase();
            val == "1" || val == "true" || val == "yes" || val == "on"
        }
        Err(_) => false,
    }
}

/// Check if an environment variable is set to a falsy value
///
/// Falsy values: "0", "false", "no", "off" (case-insensitive)
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("DISABLED", "false"); }
/// assert!(env::var_is_false("DISABLED"));
/// ```
pub fn var_is_false<K: AsRef<str>>(key: K) -> bool {
    match env::var(key.as_ref()) {
        Ok(val) => {
            let val = val.to_lowercase();
            val == "0" || val == "false" || val == "no" || val == "off"
        }
        Err(_) => false,
    }
}

/// Parse an environment variable as an optional boolean
///
/// Returns Some(true) for truthy values, Some(false) for falsy values,
/// and None if the variable is not set or has an unrecognized value.
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("FEATURE", "yes"); }
/// assert_eq!(env::var_option_bool("FEATURE"), Some(true));
/// ```
pub fn var_option_bool<K: AsRef<str>>(key: K) -> Option<bool> {
    let key = key.as_ref();
    if var_is_true(key) {
        Some(true)
    } else if var_is_false(key) {
        Some(false)
    } else {
        None
    }
}

/// Parse an environment variable as a path with tilde expansion
///
/// Expands "~" to the home directory if present at the start of the path.
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("MY_PATH", "~/documents"); }
/// let path = env::var_path("MY_PATH");
/// // path will be Some(PathBuf) with expanded home directory
/// ```
pub fn var_path<K: AsRef<str>>(key: K) -> Option<PathBuf> {
    env::var(key.as_ref()).ok().map(|val| {
        if let Some(stripped) = val.strip_prefix("~/") {
            if let Some(home) = homedir::my_home().ok().flatten() {
                home.join(stripped)
            } else {
                PathBuf::from(val)
            }
        } else {
            PathBuf::from(val)
        }
    })
}

/// Parse an environment variable as a u8
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("LEVEL", "5"); }
/// assert_eq!(env::var_u8("LEVEL"), Some(5));
/// ```
pub fn var_u8<K: AsRef<str>>(key: K) -> Option<u8> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as a u32
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("COUNT", "42"); }
/// assert_eq!(env::var_u32("COUNT"), Some(42));
/// ```
pub fn var_u32<K: AsRef<str>>(key: K) -> Option<u32> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as a u64
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("SIZE", "1000000"); }
/// assert_eq!(env::var_u64("SIZE"), Some(1000000));
/// ```
pub fn var_u64<K: AsRef<str>>(key: K) -> Option<u64> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as an i32
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("OFFSET", "-10"); }
/// assert_eq!(env::var_i32("OFFSET"), Some(-10));
/// ```
pub fn var_i32<K: AsRef<str>>(key: K) -> Option<i32> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as an i64
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("TIMESTAMP", "1234567890"); }
/// assert_eq!(env::var_i64("TIMESTAMP"), Some(1234567890));
/// ```
pub fn var_i64<K: AsRef<str>>(key: K) -> Option<i64> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to ensure tests don't interfere with each other
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_var_is_true() {
        let _guard = TEST_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TEST_TRUE", "1");
        }
        assert!(var_is_true("TEST_TRUE"));

        unsafe {
            env::set_var("TEST_TRUE", "true");
        }
        assert!(var_is_true("TEST_TRUE"));

        unsafe {
            env::set_var("TEST_TRUE", "YES");
        }
        assert!(var_is_true("TEST_TRUE"));

        unsafe {
            env::set_var("TEST_TRUE", "on");
        }
        assert!(var_is_true("TEST_TRUE"));

        unsafe {
            env::set_var("TEST_TRUE", "false");
        }
        assert!(!var_is_true("TEST_TRUE"));

        assert!(!var_is_true("NONEXISTENT_VAR_TRUE"));
    }

    #[test]
    fn test_var_is_false() {
        let _guard = TEST_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TEST_FALSE", "0");
        }
        assert!(var_is_false("TEST_FALSE"));

        unsafe {
            env::set_var("TEST_FALSE", "false");
        }
        assert!(var_is_false("TEST_FALSE"));

        unsafe {
            env::set_var("TEST_FALSE", "NO");
        }
        assert!(var_is_false("TEST_FALSE"));

        unsafe {
            env::set_var("TEST_FALSE", "off");
        }
        assert!(var_is_false("TEST_FALSE"));

        unsafe {
            env::set_var("TEST_FALSE", "true");
        }
        assert!(!var_is_false("TEST_FALSE"));

        assert!(!var_is_false("NONEXISTENT_VAR_FALSE"));
    }

    #[test]
    fn test_var_option_bool() {
        let _guard = TEST_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TEST_BOOL", "true");
        }
        assert_eq!(var_option_bool("TEST_BOOL"), Some(true));

        unsafe {
            env::set_var("TEST_BOOL", "false");
        }
        assert_eq!(var_option_bool("TEST_BOOL"), Some(false));

        unsafe {
            env::set_var("TEST_BOOL", "maybe");
        }
        assert_eq!(var_option_bool("TEST_BOOL"), None);

        assert_eq!(var_option_bool("NONEXISTENT_VAR_BOOL"), None);
    }

    #[test]
    fn test_var_numeric() {
        let _guard = TEST_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TEST_U8", "255");
        }
        assert_eq!(var_u8("TEST_U8"), Some(255));

        unsafe {
            env::set_var("TEST_U32", "42");
        }
        assert_eq!(var_u32("TEST_U32"), Some(42));

        unsafe {
            env::set_var("TEST_U64", "1000000");
        }
        assert_eq!(var_u64("TEST_U64"), Some(1000000));

        unsafe {
            env::set_var("TEST_I32", "-100");
        }
        assert_eq!(var_i32("TEST_I32"), Some(-100));

        unsafe {
            env::set_var("TEST_I64", "9223372036854775807");
        }
        assert_eq!(var_i64("TEST_I64"), Some(9223372036854775807));

        unsafe {
            env::set_var("TEST_INVALID", "not_a_number");
        }
        assert_eq!(var_u32("TEST_INVALID"), None);
    }
}
