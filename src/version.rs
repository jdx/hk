use crate::Result;
use eyre::bail;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// How a `min_hk_version` string reads.
#[derive(Debug, PartialEq, Eq)]
pub enum MinVersion {
    /// The unrendered `{{version | truncate(length=1)}}.0.0` default that every
    /// published `Config.pkl` ships. It names no real requirement.
    Placeholder,
    Version(semver::Version),
    Invalid,
}

/// Parse a `min_hk_version`, accepting a leading `v` and a missing minor or
/// patch (`2`, `2.1`, `v2.1.0`) in addition to full semver.
pub fn parse_min_version(v: &str) -> MinVersion {
    let v = v.trim();
    if v.contains("{{") {
        return MinVersion::Placeholder;
    }
    let v = v.strip_prefix(['v', 'V']).unwrap_or(v);
    let core_len = v.find(['-', '+']).unwrap_or(v.len());
    let (core, rest) = v.split_at(core_len);
    let parts = core.split('.').count();
    let normalized = match parts {
        1 => format!("{core}.0.0{rest}"),
        2 => format!("{core}.0{rest}"),
        _ => v.to_string(),
    };
    match semver::Version::parse(&normalized) {
        Ok(version) => MinVersion::Version(version),
        Err(_) => MinVersion::Invalid,
    }
}

/// Fail when `v` is newer than this hk. A placeholder is ignored and an
/// unparseable value only warns, so a typo never blocks a config that an
/// older hk would have loaded.
pub fn version_cmp_or_bail(v: &str) -> Result<()> {
    match parse_min_version(v) {
        MinVersion::Placeholder => Ok(()),
        MinVersion::Invalid => {
            warn!("ignoring min_hk_version {v:?}: not a version number like \"1.0.0\"");
            Ok(())
        }
        MinVersion::Version(min) => {
            let current = semver::Version::parse(version())?;
            if min > current {
                bail!(
                    "hk version {} is less than the minimum required version {v}",
                    version()
                );
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u64, minor: u64, patch: u64) -> MinVersion {
        MinVersion::Version(semver::Version::new(major, minor, patch))
    }

    #[test]
    fn parses_loose_versions() {
        assert_eq!(parse_min_version("1.2.3"), v(1, 2, 3));
        assert_eq!(parse_min_version("v999.0.0"), v(999, 0, 0));
        assert_eq!(parse_min_version("999.0"), v(999, 0, 0));
        assert_eq!(parse_min_version(" 2 "), v(2, 0, 0));
        assert_eq!(
            parse_min_version("2.1-beta.1"),
            MinVersion::Version(semver::Version::parse("2.1.0-beta.1").unwrap())
        );
        assert_eq!(parse_min_version("latest"), MinVersion::Invalid);
        assert_eq!(parse_min_version(""), MinVersion::Invalid);
    }

    #[test]
    fn tolerates_the_unrendered_placeholder() {
        let placeholder = "{{version | truncate(length=1)}}.0.0";
        assert_eq!(parse_min_version(placeholder), MinVersion::Placeholder);
        version_cmp_or_bail(placeholder).unwrap();
    }

    #[test]
    fn rejects_newer_versions_in_any_spelling() {
        for newer in ["999.0.0", "999.0", "v999.0.0", "999"] {
            assert!(version_cmp_or_bail(newer).is_err(), "{newer}");
        }
        version_cmp_or_bail("1.0.0").unwrap();
        version_cmp_or_bail("garbage").unwrap();
    }
}
