//! A minimal reader for git's index file, for the one question
//! [`crate::git::Git::racily_clean_paths`] asks of every entry.
//!
//! libgit2 hashes the whole file to verify its checksum before parsing it,
//! which takes milliseconds for an index of a few thousand entries, and hk asks
//! before every staging `git add`. git itself only verifies the checksum in
//! `git fsck`. This reader skips it and reads only the fields it needs. It
//! handles index versions 2 to 4 with SHA-1 object names and returns `None`
//! for anything else, including a split index, so the caller can fall back to
//! libgit2. Each entry's recorded path length and padding are checked, so the
//! longer object names of a SHA-256 repository fail to parse rather than being
//! misread.
//!
//! The format is documented in git's `Documentation/gitformat-index.txt`.

use std::path::PathBuf;

const SIGNATURE: &[u8; 4] = b"DIRC";
const OID_LEN: usize = 20;
/// ctime, mtime, dev, ino, mode, uid, gid and size, then the object name and
/// the flags.
const ENTRY_FIXED_LEN: usize = 40 + OID_LEN + 2;
const MTIME_SECONDS_OFFSET: usize = 8;
const FILE_SIZE_OFFSET: usize = 36;
const FLAG_EXTENDED: u16 = 0x4000;
const NAME_MASK: u16 = 0x0fff;

/// Paths of the entries in the index `data` with a nonzero size and an mtime of
/// at least `index_secs` seconds.
///
/// Returns `None` if `data` is not an index this reader understands.
pub fn racily_clean_paths(data: &[u8], index_secs: i64) -> Option<Vec<PathBuf>> {
    if data.len() < 12 + OID_LEN || &data[..4] != SIGNATURE {
        return None;
    }
    let version = be_u32(data, 4)?;
    if !(2..=4).contains(&version) {
        return None;
    }
    let count = be_u32(data, 8)? as usize;
    // Entries and extensions end where the trailing checksum starts.
    let end = data.len() - OID_LEN;
    let mut pos = 12;
    let mut path: Vec<u8> = Vec::new();
    let mut racy = Vec::new();
    for _ in 0..count {
        let start = pos;
        let fixed = data.get(start..start + ENTRY_FIXED_LEN)?;
        let mtime_secs = i64::from(be_u32(fixed, MTIME_SECONDS_OFFSET)?);
        let file_size = be_u32(fixed, FILE_SIZE_OFFSET)?;
        let flags = be_u16(fixed, ENTRY_FIXED_LEN - 2)?;
        pos += ENTRY_FIXED_LEN;
        if flags & FLAG_EXTENDED != 0 {
            if version < 3 {
                return None;
            }
            pos += 2;
        }
        if version == 4 {
            // The path replaces the last `strip` bytes of the previous path.
            let (strip, len) = decode_varint(data.get(pos..end)?)?;
            pos += len;
            let keep = path.len().checked_sub(usize::try_from(strip).ok()?)?;
            path.truncate(keep);
            let suffix_len = nul_offset(data.get(pos..end)?)?;
            path.extend_from_slice(&data[pos..pos + suffix_len]);
            pos += suffix_len + 1;
        } else {
            let len = nul_offset(data.get(pos..end)?)?;
            path.clear();
            path.extend_from_slice(&data[pos..pos + len]);
            // One to eight NULs pad each entry to a multiple of eight bytes.
            let padding = pos + len..start + (pos - start + len + 8) / 8 * 8;
            if padding.end > end || data[padding.clone()].iter().any(|&b| b != 0) {
                return None;
            }
            pos = padding.end;
        }
        // Paths of 0xfff bytes or more store 0xfff as their length.
        let name_len = usize::from(flags & NAME_MASK);
        if name_len != path.len().min(usize::from(NAME_MASK)) {
            return None;
        }
        if file_size != 0 && mtime_secs >= index_secs {
            racy.push(path_from_bytes(&path));
        }
    }
    // A split index keeps most entries in a shared index file this reader
    // does not follow.
    while pos < end {
        let signature = data.get(pos..pos + 4)?;
        let size = be_u32(data, pos + 4)? as usize;
        if signature == b"link" {
            return None;
        }
        pos = pos.checked_add(size.checked_add(8)?)?;
    }
    (pos == end).then_some(racy)
}

fn be_u32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn be_u16(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

/// Offset of the first NUL in `data`.
fn nul_offset(data: &[u8]) -> Option<usize> {
    data.iter().position(|&b| b == 0)
}

/// git's variable-length integer (`varint.c`), which unlike LEB128 adds one
/// for each continuation byte. Returns the value and the bytes it used.
fn decode_varint(data: &[u8]) -> Option<(u64, usize)> {
    let mut bytes = data.iter();
    let mut c = *bytes.next()?;
    let mut value = u64::from(c & 0x7f);
    let mut len = 1;
    while c & 0x80 != 0 {
        value = value.checked_add(1)?;
        if value.leading_zeros() < 7 {
            return None;
        }
        c = *bytes.next()?;
        value = (value << 7) + u64::from(c & 0x7f);
        len += 1;
    }
    Some((value, len))
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::process::Command;
    use std::time::{Duration, SystemTime};

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    /// A repository whose index has entries with shared prefixes, an empty
    /// file, nested directories and mtimes an hour apart.
    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        git(root, &["init", "-q"]);
        std::fs::create_dir_all(root.join("src/nested")).unwrap();
        let files = [
            ("README.md", "readme\n"),
            ("empty.txt", ""),
            ("src/lib.rs", "lib\n"),
            ("src/library.rs", "library\n"),
            ("src/nested/mod.rs", "mod\n"),
            ("src/nested/mod_test.rs", "test\n"),
            ("with space.txt", "space\n"),
        ];
        for (hours, (path, content)) in files.into_iter().enumerate() {
            let path = root.join(path);
            std::fs::write(&path, content).unwrap();
            let mtime = SystemTime::now() - Duration::from_secs(3600 * hours as u64);
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(mtime)
                .unwrap();
        }
        git(root, &["add", "."]);
        dir
    }

    /// What `Git::racily_clean_paths` computed with libgit2.
    fn libgit2_racily_clean_paths(index: &Path, index_secs: i64) -> Vec<PathBuf> {
        git2::Index::open(index)
            .unwrap()
            .iter()
            .filter(|e| e.file_size != 0 && i64::from(e.mtime.seconds()) >= index_secs)
            .map(|e| path_from_bytes(&e.path))
            .collect()
    }

    fn assert_matches_libgit2(index: &Path) {
        let data = std::fs::read(index).unwrap();
        let mut mtimes: Vec<i64> = git2::Index::open(index)
            .unwrap()
            .iter()
            .map(|e| i64::from(e.mtime.seconds()))
            .collect();
        mtimes.sort();
        let middle = mtimes[mtimes.len() / 2];
        let newest = mtimes[mtimes.len() - 1];
        for secs in [0, middle, newest, newest + 1] {
            assert_eq!(
                racily_clean_paths(&data, secs),
                Some(libgit2_racily_clean_paths(index, secs)),
                "index_secs = {secs}"
            );
        }
        let all = racily_clean_paths(&data, 0).unwrap();
        assert_eq!(all.len(), 6, "{all:?}");
        assert!(!all.contains(&PathBuf::from("empty.txt")));
    }

    fn index_version(index: &Path) -> Option<u32> {
        be_u32(&std::fs::read(index).unwrap(), 4)
    }

    #[test]
    fn reads_version_2() {
        let dir = repo();
        git(dir.path(), &["update-index", "--index-version", "2"]);
        let index = dir.path().join(".git/index");
        assert_eq!(index_version(&index), Some(2));
        assert_matches_libgit2(&index);
    }

    #[test]
    fn reads_version_3_extended_entries() {
        let dir = repo();
        std::fs::write(dir.path().join("intent.txt"), "intent\n").unwrap();
        // An intent-to-add entry has the extended flag, which needs version 3.
        git(dir.path(), &["add", "--intent-to-add", "intent.txt"]);
        let index = dir.path().join(".git/index");
        assert_eq!(index_version(&index), Some(3));
        assert_matches_libgit2(&index);
    }

    #[test]
    fn reads_version_4_prefix_compressed_paths() {
        let dir = repo();
        git(dir.path(), &["update-index", "--index-version", "4"]);
        let index = dir.path().join(".git/index");
        assert_eq!(index_version(&index), Some(4));
        assert_matches_libgit2(&index);
    }

    #[test]
    fn reads_past_extensions() {
        let dir = repo();
        // Committing writes the cached-tree extension.
        git(
            dir.path(),
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-qm",
                "c",
            ],
        );
        let index = dir.path().join(".git/index");
        assert!(
            std::fs::read(&index)
                .unwrap()
                .windows(4)
                .any(|w| w == b"TREE")
        );
        assert_matches_libgit2(&index);
    }

    #[test]
    fn declines_a_split_index() {
        let dir = repo();
        git(dir.path(), &["update-index", "--split-index"]);
        let data = std::fs::read(dir.path().join(".git/index")).unwrap();
        assert_eq!(racily_clean_paths(&data, 0), None);
    }

    #[test]
    fn declines_sha256_object_names() {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q", "--object-format=sha256"]);
        std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
        git(dir.path(), &["add", "a.txt"]);
        let data = std::fs::read(dir.path().join(".git/index")).unwrap();
        assert_eq!(racily_clean_paths(&data, 0), None);
    }

    #[test]
    fn declines_malformed_data() {
        let dir = repo();
        let data = std::fs::read(dir.path().join(".git/index")).unwrap();
        assert_eq!(racily_clean_paths(&data[..data.len() / 2], 0), None);
        assert_eq!(racily_clean_paths(b"", 0), None);
        let mut other_version = data.clone();
        other_version[7] = 5;
        assert_eq!(racily_clean_paths(&other_version, 0), None);
        let mut bad_signature = data;
        bad_signature[0] = b'X';
        assert_eq!(racily_clean_paths(&bad_signature, 0), None);
    }

    #[test]
    fn decodes_git_varints() {
        assert_eq!(decode_varint(&[0x00]), Some((0, 1)));
        assert_eq!(decode_varint(&[0x7f]), Some((127, 1)));
        assert_eq!(decode_varint(&[0x80, 0x00]), Some((128, 2)));
        assert_eq!(decode_varint(&[0x80, 0x7f]), Some((255, 2)));
        assert_eq!(decode_varint(&[0x81, 0x00]), Some((256, 2)));
        assert_eq!(decode_varint(&[0x80]), None);
    }
}
