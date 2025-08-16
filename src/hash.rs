use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::Path;

use sha2::Digest;
use sha2::digest::Output;

use crate::file::display_path;
use crate::{XXError, XXResult, bail, file};

/// Calculate the hash of a value
/// # Arguments
/// * `t` - A value to hash
/// # Returns
/// A hash as a string
/// # Example
/// ```
/// use xx::hash::hash_to_str;
/// let hash = hash_to_str(&"foo"); // 3e8b8c44c3ca73b7
/// ```
pub fn hash_to_str<T: Hash>(t: &T) -> String {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    let bytes = s.finish();
    format!("{bytes:x}")
}

/// Calculate the SHA256 checksum of a file
/// # Arguments
/// * `path` - A path to a file
/// # Returns
/// A SHA256 checksum
/// # Errors
/// Returns an error if the file cannot be read
/// # Example
/// ```
/// use std::path::Path;
/// use xx::hash::file_hash_sha256;
/// let hash = file_hash_sha256(Path::new("test/data/foo.txt")).unwrap();
/// ```
pub fn file_hash_sha256(path: impl AsRef<Path>) -> XXResult<String> {
    let path = path.as_ref();
    debug!("Calculating SHA256 checksum for {}", display_path(path));
    let h = file_hash::<sha2::Sha256>(path)?;
    Ok(format!("{h:x}"))
}

/// Calculate the SHA512 checksum of a file
/// # Arguments
/// * `path` - A path to a file
/// # Returns
/// A SHA512 checksum
/// # Errors
/// Returns an error if the file cannot be read
/// # Example
/// ```
/// use std::path::Path;
/// use xx::hash::file_hash_sha512;
/// let hash = file_hash_sha512(Path::new("test/data/foo.txt")).unwrap();
/// ```
pub fn file_hash_sha512(path: impl AsRef<Path>) -> XXResult<String> {
    let path = path.as_ref();
    debug!("Calculating SHA512 checksum for {}", display_path(path));
    let h = file_hash::<sha2::Sha512>(path)?;
    Ok(format!("{h:x}"))
}

pub fn file_hash<H>(path: &Path) -> XXResult<Output<H>>
where
    H: Digest + Write,
{
    let mut file = file::open(path)?;
    // if let Some(pr) = pr {
    //     pr.set_length(file.metadata()?.len());
    // }
    let mut hasher = H::new();
    let mut buf = [0; 32 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
        if n == 0 {
            break;
        }
        hasher
            .write_all(&buf[..n])
            .map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
        // if let Some(pr) = pr {
        //     pr.inc(n as u64);
        // }
    }
    std::io::copy(&mut file, &mut hasher)
        .map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(hasher.finalize())
}

/// Ensure that a file has a specific SHA256 checksum
/// # Arguments
/// * `path` - A path to a file
/// * `checksum` - A SHA256 checksum
/// # Errors
/// Returns an error if the checksum does not match
/// # Example
/// ```
/// use std::path::Path;
/// use xx::hash::ensure_checksum_sha256;
/// ensure_checksum_sha256(Path::new("test/data/foo.txt"), "aec070645fe53ee3b3763059376134f058cc337247c978add178b6ccdfb0019f").unwrap();
/// ```
pub fn ensure_checksum_sha256(path: &Path, checksum: &str) -> XXResult<()> {
    let actual = file_hash_sha256(path)?;
    if actual != checksum {
        bail!(
            "Checksum mismatch for file {}:\nExpected: {checksum}\nActual:   {actual}",
            display_path(path),
        );
    }
    Ok(())
}

/// Ensure that a file has a specific SHA512 checksum
/// # Arguments
/// * `path` - A path to a file
/// * `checksum` - A SHA512 checksum
/// # Errors
/// Returns an error if the checksum does not match
/// # Example
/// ```
/// use std::path::Path;
/// use xx::hash::ensure_checksum_sha512;
/// ensure_checksum_sha512(Path::new("test/data/foo.txt"), "e79b8ad22b34a54be999f4eadde2ee895c208d4b3d83f1954b61255d2556a8b73773c0dc0210aa044ffcca6834839460959cbc9f73d3079262fc8bc935d46262").unwrap();
/// ```
pub fn ensure_checksum_sha512(path: &Path, checksum: &str) -> XXResult<()> {
    let actual = file_hash_sha512(path)?;
    if actual != checksum {
        bail!(
            "Checksum mismatch for file {}:\nExpected: {checksum}\nActual:   {actual}",
            display_path(path),
        );
    }
    Ok(())
}

pub fn parse_shasums(text: &str) -> HashMap<String, String> {
    text.lines()
        .map(|l| {
            let mut parts = l.split_whitespace();
            let hash = parts.next().unwrap();
            let name = parts.next().unwrap();
            (name.into(), hash.into())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_to_str() {
        assert_eq!(hash_to_str(&"foo"), "3e8b8c44c3ca73b7");
    }

    #[test]
    fn test_hash_sha256() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.as_file().write_all(b"Hello, world!").unwrap();
        let hash = file_hash_sha256(tmp.path()).unwrap();
        insta::assert_snapshot!(hash, @"315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3");
    }
}
