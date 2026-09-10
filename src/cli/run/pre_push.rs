use std::io::IsTerminal;
use std::io::Read;

use crate::hook_options::HookOptions;
use crate::{
    Result,
    git::{Git, is_zero_sha},
};

/// Run the pre-push hook
#[derive(usage_rs::Args)]
pub struct PrePush {
    /// Remote name
    remote: Option<String>,
    /// Remote URL
    url: Option<String>,
    #[usage(flatten)]
    pub(super) hook: HookOptions,
}

#[derive(Debug)]
struct PrePushRefs {
    to: (String, String),
    from: (String, String),
}

impl From<&str> for PrePushRefs {
    fn from(line: &str) -> Self {
        let parts: Vec<&str> = line.split_whitespace().collect();
        PrePushRefs {
            to: (parts[0].to_string(), parts[1].to_string()),
            from: (parts[2].to_string(), parts[3].to_string()),
        }
    }
}

// Check that a string is a valid Git commit long hash (40 or 64 lowercase hexits)
fn is_valid_commit_hash(s: &str) -> bool {
    let length_is_valid = s.len() == 40 || s.len() == 64;
    let is_all_lowercase_hexits = s.chars().all(|c| ('0' <= c && c <= '9') || ('a' <= c && c <= 'f'));
    let is_valid = length_is_valid && is_all_lowercase_hexits;
    if !is_valid {
        eprintln!("Warning: \"{}\" is not a valid full Git hash (must be exactly 40 or 64 lowercase hexits)", s);
        return false;
    }
    return is_valid;
}

// Check that a string is a valid four-part stdin line.
// Silently ignores empty lines; prints warning for others.
fn validate_input_line(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    // Check that the line splits into four parts of which the second and fourth are commit hashes.
    let parts: Vec<&str> = line.split_whitespace().collect();
    let is_valid = parts.len() == 4
        && is_valid_commit_hash(parts[1])
        && is_valid_commit_hash(parts[3]);
    if !is_valid {
        eprintln!("Ignoring malformed line from stdin: {}", line);
    }
    return is_valid;
}

impl PrePush {
    pub async fn run(mut self) -> Result<()> {
        if self.hook.reads_file_list_from_stdin() {
            return Err(eyre::eyre!(
                "--files0-from - cannot be used with pre-push because the hook reads refs from stdin"
            ));
        }
        self.hook.tctx.insert(
            "hook_args",
            &format!(
                "{} {}",
                self.remote.as_deref().unwrap_or(""),
                self.url.as_deref().unwrap_or("")
            ),
        );
        let to_be_updated_refs = if std::io::stdin().is_terminal() {
            self.hook.tctx.insert("hook_stdin", "");
            vec![]
        } else {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            self.hook.tctx.insert("hook_stdin", &input);
            // Note: we deliberately keep deletions (local sha all-zeros) in
            // the list. The downstream EMPTY_REF guard in hook.rs detects
            // `to_ref` of all-zeros and short-circuits to an empty file set
            // — dropping deletions here would route them through
            // `files_between_refs(default_branch, "HEAD")` and lint
            // unrelated files.
            input
                .lines()
                .filter(|line| validate_input_line(&line))
                .map(PrePushRefs::from)
                .collect::<Vec<_>>()
        };
        trace!("to_be_updated_refs: {to_be_updated_refs:?}");

        self.hook.from_ref = Some(match &self.hook.from_ref {
            Some(to_ref) => to_ref.clone(),
            None if !to_be_updated_refs.is_empty()
                && !is_zero_sha(&to_be_updated_refs[0].from.1) =>
            {
                to_be_updated_refs[0].from.1.clone()
            }
            None => {
                // Either no refs were provided on stdin, or the first ref is
                // a new-branch push (remote sha is all-zeros). Fall back to
                // the remote-tracking branch if it exists, then to the
                // repository's default branch on the target remote.
                let remote = self.remote.as_deref().unwrap_or("origin");
                let repo = Git::new()?; // TODO: remove this extra repo creation
                if let Some(rb) = repo.matching_remote_branch(remote)? {
                    rb
                } else if remote == "origin" {
                    repo.resolve_default_branch()
                } else {
                    // resolve_default_branch is internally hardcoded to
                    // origin, so for a non-origin remote strip the known
                    // origin prefix and rebind onto the actual remote.
                    // Use strip_prefix (not rsplit) so multi-segment branch
                    // names like "release/v1" are preserved intact.
                    let default = repo.resolve_default_branch();
                    let bare = default
                        .strip_prefix("refs/remotes/origin/")
                        .or_else(|| default.strip_prefix("origin/"))
                        .unwrap_or(&default);
                    format!("refs/remotes/{remote}/{bare}")
                }
            }
        });
        self.hook.to_ref = Some(
            self.hook
                .to_ref
                .clone()
                .or(if !to_be_updated_refs.is_empty() {
                    Some(to_be_updated_refs[0].to.1.clone())
                } else {
                    None
                })
                .unwrap_or("HEAD".to_string()),
        );
        debug!(
            "from_ref: {}, to_ref: {}",
            self.hook.from_ref.as_ref().unwrap(),
            self.hook.to_ref.as_ref().unwrap()
        );

        self.hook.run("pre-push").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA1: &str = "0123456789abcdef0123456789abcdef01234567";
    const SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const ZERO_SHA1: &str = "0000000000000000000000000000000000000000";

    #[test]
    fn test_accepts_sha1_hash() {
        assert!(is_valid_commit_hash(SHA1));
    }

    #[test]
    fn test_accepts_sha256_hash() {
        assert!(is_valid_commit_hash(SHA256));
    }

    #[test]
    fn test_accepts_zero_sha() {
        // Deletions and new branches arrive with an all-zeros sha. These must
        // survive the filter so the EMPTY_REF guard in hook.rs can
        // short-circuit them to an empty file set.
        assert!(is_valid_commit_hash(ZERO_SHA1));
    }

    #[test]
    fn test_rejects_abbreviated_hash() {
        assert!(!is_valid_commit_hash("abc1234"));
    }

    #[test]
    fn test_rejects_uppercase_hash() {
        assert!(!is_valid_commit_hash(&SHA1.to_uppercase()));
    }

    #[test]
    fn test_rejects_non_hex_hash() {
        // Right length, wrong alphabet.
        assert!(!is_valid_commit_hash(&"z".repeat(40)));
    }

    #[test]
    fn test_accepts_well_formed_line() {
        let line = format!("refs/heads/main {SHA1} refs/heads/main {SHA1}");
        assert!(validate_input_line(&line));
    }

    #[test]
    fn test_accepts_deletion_line() {
        // git sends the all-zeros local sha when deleting a remote branch.
        let line = format!("(delete) {ZERO_SHA1} refs/heads/gone {SHA1}");
        assert!(validate_input_line(&line));
    }

    #[test]
    fn test_rejects_line_with_too_few_fields() {
        // The crash case: PrePushRefs::from indexes parts[3] unconditionally,
        // so a short line used to panic instead of being skipped.
        assert!(!validate_input_line("refs/heads/main"));
        assert!(!validate_input_line(&format!("refs/heads/main {SHA1}")));
        assert!(!validate_input_line(&format!(
            "refs/heads/main {SHA1} refs/heads/main"
        )));
    }

    #[test]
    fn test_rejects_line_with_too_many_fields() {
        let line = format!("refs/heads/main {SHA1} refs/heads/main {SHA1} extra");
        assert!(!validate_input_line(&line));
    }

    #[test]
    fn test_rejects_line_with_abbreviated_hashes() {
        assert!(!validate_input_line("refs/heads/main abc refs/heads/main def"));
    }

    #[test]
    fn test_rejects_blank_lines() {
        assert!(!validate_input_line(""));
        assert!(!validate_input_line("   "));
        assert!(!validate_input_line("\t"));
    }

    #[test]
    fn test_accepted_lines_are_safe_to_parse() {
        // The filter/map pairing in run() means anything validate_input_line
        // accepts is handed straight to PrePushRefs::from, which indexes
        // parts[0..=3] without bounds checks.
        let line = format!("refs/heads/main {SHA1} refs/heads/main {ZERO_SHA1}");
        assert!(validate_input_line(&line));
        let refs = PrePushRefs::from(line.as_str());
        assert_eq!(refs.to, ("refs/heads/main".to_string(), SHA1.to_string()));
        assert_eq!(
            refs.from,
            ("refs/heads/main".to_string(), ZERO_SHA1.to_string())
        );
    }
}
