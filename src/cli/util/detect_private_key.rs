use crate::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct DetectPrivateKey {
    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl DetectPrivateKey {
    pub async fn run(&self) -> Result<()> {
        let mut found_key = false;

        for file_path in &self.files {
            if has_private_key(file_path)? {
                println!("{}", file_path.display());
                found_key = true;
            }
        }

        if found_key {
            std::process::exit(1);
        }

        Ok(())
    }
}

fn has_private_key(path: &PathBuf) -> Result<bool> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(false), // File doesn't exist or can't be read as text
    };

    // Common private key patterns
    let key_patterns = [
        "BEGIN RSA PRIVATE KEY",
        "BEGIN DSA PRIVATE KEY",
        "BEGIN EC PRIVATE KEY",
        "BEGIN OPENSSH PRIVATE KEY",
        "BEGIN PGP PRIVATE KEY BLOCK",
        "BEGIN ENCRYPTED PRIVATE KEY",
        "BEGIN PRIVATE KEY",
        "PuTTY-User-Key-File-2",
        "PuTTY-User-Key-File-3",
    ];

    for line in content.lines() {
        for pattern in &key_patterns {
            if line.contains(pattern) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_rsa_private_key() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA...
-----END RSA PRIVATE KEY-----
"#,
        )
        .unwrap();

        let result = has_private_key(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_openssh_private_key() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAABlwAAAAdzc2gtcn
-----END OPENSSH PRIVATE KEY-----
"#,
        )
        .unwrap();

        let result = has_private_key(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_ec_private_key() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"-----BEGIN EC PRIVATE KEY-----
MHcCAQEEIIGLW...
-----END EC PRIVATE KEY-----
"#,
        )
        .unwrap();

        let result = has_private_key(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_pgp_private_key() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"-----BEGIN PGP PRIVATE KEY BLOCK-----
Version: GnuPG v1

lQHYBF...
-----END PGP PRIVATE KEY BLOCK-----
"#,
        )
        .unwrap();

        let result = has_private_key(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_putty_private_key() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"PuTTY-User-Key-File-2: ssh-rsa
Encryption: none
Comment: imported-openssh-key
"#,
        )
        .unwrap();

        let result = has_private_key(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_public_key_safe() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...
-----END PUBLIC KEY-----
"#,
        )
        .unwrap();

        let result = has_private_key(&file.path().to_path_buf()).unwrap();
        assert!(!result); // Public keys are safe
    }

    #[test]
    fn test_ssh_public_key_safe() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC... user@host",
        )
        .unwrap();

        let result = has_private_key(&file.path().to_path_buf()).unwrap();
        assert!(!result); // SSH public keys are safe
    }

    #[test]
    fn test_clean_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
def hello():
    print("Hello, world!")
"#,
        )
        .unwrap();

        let result = has_private_key(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "").unwrap();

        let result = has_private_key(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_nonexistent_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("nonexistent");

        let result = has_private_key(&file).unwrap();
        assert!(!result);
    }
}
