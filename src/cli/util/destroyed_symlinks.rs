use crate::Result;
use std::path::PathBuf;
use std::process::Command;

const PERMS_LINK: u32 = 0o120_000;
const PERMS_NONEXIST: u32 = 0;

/// Bytes a staged blob may grow and still match the old symlink target, which
/// covers formatters that append a newline or rewrite LF as CRLF.
const SIZE_TOLERANCE: u64 = 2;

#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct DestroyedSymlinks {
    /// Files to check. Defaults to the whole repository.
    #[usage(arg)]
    pub files: Option<Vec<PathBuf>>,
}

impl DestroyedSymlinks {
    pub async fn run(&self) -> Result<()> {
        let destroyed = find_destroyed_symlinks(self.files.as_deref().unwrap_or_default())?;
        if destroyed.is_empty() {
            return Ok(());
        }

        println!("Destroyed symlinks:");
        for path in &destroyed {
            println!("- {path}");
        }
        println!("You should unstage affected files:");
        println!("\tgit reset HEAD -- {}", shell_words::join(&destroyed));
        println!(
            "And retry commit. As a long term solution you may try to explicitly tell git that your environment does not support symlinks:"
        );
        println!("\tgit config core.symlinks false");

        Err(eyre::eyre!("Destroyed symlinks found"))
    }
}

fn find_destroyed_symlinks(files: &[PathBuf]) -> Result<Vec<String>> {
    let mut args = vec![
        "status".to_string(),
        "--porcelain=v2".to_string(),
        "-z".to_string(),
        "--no-renames".to_string(),
        "--".to_string(),
    ];
    if files.is_empty() {
        args.push(".".to_string());
    } else {
        args.extend(files.iter().map(|f| f.to_string_lossy().into_owned()));
    }

    let stdout = git(&args)?;
    let mut destroyed = Vec::new();
    for record in String::from_utf8_lossy(&stdout).split('\0') {
        let Some(entry) = ChangedEntry::parse(record) else {
            continue;
        };
        if entry.head_mode != PERMS_LINK
            || entry.index_mode == PERMS_LINK
            || entry.index_mode == PERMS_NONEXIST
        {
            continue;
        }
        if is_destroyed_symlink(&entry)? {
            destroyed.push(entry.path.to_string());
        }
    }

    Ok(destroyed)
}

/// An ordinary changed entry from `git status --porcelain=v2`:
/// `1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>`
#[derive(Debug, PartialEq, Eq)]
struct ChangedEntry<'a> {
    head_mode: u32,
    index_mode: u32,
    head_hash: &'a str,
    index_hash: &'a str,
    path: &'a str,
}

impl<'a> ChangedEntry<'a> {
    /// `None` for any other record type, which callers skip.
    fn parse(record: &'a str) -> Option<Self> {
        let mut fields = record.splitn(9, ' ');
        if fields.next()? != "1" {
            return None;
        }
        let _xy = fields.next()?;
        let _sub = fields.next()?;
        let head_mode = u32::from_str_radix(fields.next()?, 8).ok()?;
        let index_mode = u32::from_str_radix(fields.next()?, 8).ok()?;
        let _worktree_mode = fields.next()?;
        Some(Self {
            head_mode,
            index_mode,
            head_hash: fields.next()?,
            index_hash: fields.next()?,
            path: fields.next()?,
        })
    }
}

fn is_destroyed_symlink(entry: &ChangedEntry<'_>) -> Result<bool> {
    if entry.head_hash == entry.index_hash {
        return Ok(true);
    }

    let index_size = object_size(entry.index_hash)?;
    let head_size = object_size(entry.head_hash)?;
    if index_size > head_size.saturating_add(SIZE_TOLERANCE) {
        return Ok(false);
    }

    let head = object_content(entry.head_hash)?;
    let index = object_content(entry.index_hash)?;
    Ok(head.trim_ascii_end() == index.trim_ascii_end())
}

fn object_size(object: &str) -> Result<u64> {
    let stdout = git(&["cat-file".to_string(), "-s".to_string(), object.to_string()])?;
    Ok(String::from_utf8_lossy(&stdout).trim().parse()?)
}

fn object_content(object: &str) -> Result<Vec<u8>> {
    git(&["cat-file".to_string(), "-p".to_string(), object.to_string()])
}

fn git(args: &[String]) -> Result<Vec<u8>> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err(eyre::eyre!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_destroyed_symlink_entry() {
        let entry = ChangedEntry::parse(
            "1 T. N... 120000 100644 100644 80c552ec 80c552ec path with spaces.txt",
        )
        .unwrap();

        assert_eq!(entry.head_mode, PERMS_LINK);
        assert_eq!(entry.index_mode, 0o100_644);
        assert_eq!(entry.head_hash, "80c552ec");
        assert_eq!(entry.index_hash, "80c552ec");
        assert_eq!(entry.path, "path with spaces.txt");
    }

    #[test]
    fn test_parse_skips_untracked_entries() {
        assert_eq!(ChangedEntry::parse("? untracked.txt"), None);
    }

    #[test]
    fn test_parse_skips_renamed_entries() {
        assert_eq!(
            ChangedEntry::parse("2 R. N... 100644 100644 100644 abc abc R100 new.txt\told.txt"),
            None
        );
    }

    #[test]
    fn test_parse_skips_empty_record() {
        assert_eq!(ChangedEntry::parse(""), None);
    }
}
