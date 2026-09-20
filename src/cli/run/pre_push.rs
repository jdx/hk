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

/// Check that a string is a valid Git commit long hash (40 or 64 lowercase hexits).
/// This function is only called against the second and fourth parts of an input line,
/// which must be full-length hashes (the first and third can be any commit-like expressions).
fn is_valid_commit_hash(s: &str) -> bool {
    let length_is_valid = s.len() == 40 || s.len() == 64;
    let is_all_lowercase_hexits = s
        .chars()
        .all(|c| ('0' <= c && c <= '9') || ('a' <= c && c <= 'f'));
    return length_is_valid && is_all_lowercase_hexits;
}

#[derive(Debug, Eq, PartialEq)]
enum RejectionReason {
    Empty,
    NotFourParts,
    FirstHashInvalid,
    SecondHashInvalid,
    BothHashesInvalid,
}

fn format_rejection_reason(reason: RejectionReason) -> &'static str {
    match reason {
        RejectionReason::Empty => "empty",
        RejectionReason::NotFourParts => "must be four whitespace-separated parts",
        RejectionReason::FirstHashInvalid => "second part must be 40 or 64 lowercase hexits",
        RejectionReason::SecondHashInvalid => "fourth part must be 40 or 64 lowercase hexits",
        RejectionReason::BothHashesInvalid => {
            "second and fourth parts must be 40 or 64 lowercase hexits"
        }
    }
}

/// Check that a string is a valid four-part stdin line. Return Ok(()) for a pass, Err(RejectionReason)
/// with an error message for a failure.
fn validate_input_line(line: &str) -> Result<(), RejectionReason> {
    let line = line.trim();
    if line.is_empty() {
        return Err(RejectionReason::Empty);
    }
    // Check that the line splits into four parts of which the second and fourth are commit hashes.
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 4 {
        return Err(RejectionReason::NotFourParts);
    }
    let first_hash_is_valid = is_valid_commit_hash(parts[1]);
    let second_hash_is_valid = is_valid_commit_hash(parts[3]);
    if first_hash_is_valid {
        if second_hash_is_valid {
            return Ok(());
        } else {
            return Err(RejectionReason::SecondHashInvalid);
        }
    } else {
        if second_hash_is_valid {
            return Err(RejectionReason::FirstHashInvalid);
        } else {
            return Err(RejectionReason::BothHashesInvalid);
        }
    }
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
                .filter(|line| {
                    let result = validate_input_line(&line);
                    match result {
                        Ok(()) => true,
                        Err(reason) => {
                            if reason == RejectionReason::Empty {
                                // use different format so we don't print "Ignoring malformed line : empty"
                                eprintln!("Ignoring empty stdin line");
                            } else {
                                let reason_str = format_rejection_reason(reason);
                                eprintln!("Ignoring malformed stdin line \"{line}\": {reason_str}");
                            }
                            false
                        }
                    }
                })
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
    const OK: Result<(), RejectionReason> = Ok(());

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
        assert_eq!(validate_input_line(&line), OK);
    }

    #[test]
    fn test_accepts_deletion_line() {
        // git sends the all-zeros local sha when deleting a remote branch.
        let line = format!("(delete) {ZERO_SHA1} refs/heads/gone {SHA1}");
        assert_eq!(validate_input_line(&line), OK);
    }

    #[test]
    fn test_rejects_line_with_too_few_fields() {
        // The crash case: PrePushRefs::from indexes parts[3] unconditionally,
        // so a short line used to panic instead of being skipped.
        assert_eq!(
            validate_input_line("refs/heads/main"),
            Err(RejectionReason::NotFourParts)
        );
        assert_eq!(
            validate_input_line(&format!("refs/heads/main {SHA1}")),
            Err(RejectionReason::NotFourParts)
        );
        assert_eq!(
            validate_input_line(&format!("refs/heads/main {SHA1} refs/heads/main")),
            Err(RejectionReason::NotFourParts)
        );
    }

    #[test]
    fn test_rejects_line_with_too_many_fields() {
        let line = format!("refs/heads/main {SHA1} refs/heads/main {SHA1} extra");
        assert!(validate_input_line(&line) == Err(RejectionReason::NotFourParts));
    }

    #[test]
    fn test_rejects_line_with_abbreviated_hashes() {
        let first_invalid_line = format!("refs/heads/main abc refs/heads/main {SHA1}");
        let second_invalid_line = format!("refs/heads/main {SHA1} refs/heads/main def");
        let both_invalid_line = "refs/heads/main abc refs/heads/main def";
        assert_eq!(
            validate_input_line(&first_invalid_line),
            Err(RejectionReason::FirstHashInvalid)
        );
        assert_eq!(
            validate_input_line(&second_invalid_line),
            Err(RejectionReason::SecondHashInvalid)
        );
        assert_eq!(
            validate_input_line(both_invalid_line),
            Err(RejectionReason::BothHashesInvalid)
        );
    }

    #[test]
    fn test_rejects_blank_lines() {
        assert_eq!(validate_input_line(""), Err(RejectionReason::Empty));
        assert_eq!(validate_input_line("   "), Err(RejectionReason::Empty));
        assert_eq!(validate_input_line("\t"), Err(RejectionReason::Empty));
    }

    #[test]
    fn test_accepted_lines_are_safe_to_parse() {
        // The filter/map pairing in run() means anything validate_input_line
        // accepts is handed straight to PrePushRefs::from, which indexes
        // parts[0..=3] without bounds checks.
        let line = format!("refs/heads/main {SHA1} refs/heads/main {ZERO_SHA1}");
        assert!(validate_input_line(&line) == Ok(()));
        let refs = PrePushRefs::from(line.as_str());
        assert_eq!(refs.to, ("refs/heads/main".to_string(), SHA1.to_string()));
        assert_eq!(
            refs.from,
            ("refs/heads/main".to_string(), ZERO_SHA1.to_string())
        );
    }
}
