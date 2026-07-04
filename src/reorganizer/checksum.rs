use std::fs::File;
use std::io::{self, Read};

const BUFFER_SIZE: usize = 8 * 1024;

/// Compute the Blake3 hex digest of a file.
pub fn file_hash(path: &std::path::Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_matches_content() {
        let dir = std::env::temp_dir().join(format!("alex-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hash.txt");
        std::fs::write(&path, b"hello world").unwrap();
        let h1 = file_hash(&path).unwrap();
        std::fs::write(&path, b"hello world").unwrap();
        let h2 = file_hash(&path).unwrap();
        assert_eq!(h1, h2);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
