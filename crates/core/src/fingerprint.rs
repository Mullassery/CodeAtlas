//! Content fingerprinting: per-file truncated SHA-256 and the whole-tree hash.
//!
//! Matches upstream `agents/content_hash.py` so incremental runs can reuse the
//! same version keys: per-file digests are SHA-256 truncated to 16 hex chars;
//! the tree hash is a full 64-hex SHA-256 over sorted `"path:digest"` lines.

use std::collections::BTreeMap;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::discovery::SourceFile;

/// Lowercase hex encoding of `bytes`.
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// SHA-256 of `bytes`, truncated to the first 16 hex characters.
pub fn sha256_hex16(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_encode(&digest)[..16].to_string()
}

/// Full SHA-256 of `bytes` as 64 hex characters.
pub fn sha256_hex64(bytes: &[u8]) -> String {
    hex_encode(&Sha256::digest(bytes))
}

/// Whole-tree version key over sorted `"path:digest"` lines (skipping empty
/// digests), matching upstream `tree_hash_from_file_hashes`.
pub fn tree_hash(file_hashes: &BTreeMap<String, String>) -> String {
    let mut acc = String::new();
    for (path, digest) in file_hashes {
        if digest.is_empty() {
            continue;
        }
        acc.push_str(path);
        acc.push(':');
        acc.push_str(digest);
        acc.push('\n');
    }
    sha256_hex64(acc.as_bytes())
}

/// Hash every discovered file into a `{posix_path: sha16}` map.
pub fn fingerprint(files: &[SourceFile], root: &Path) -> BTreeMap<String, String> {
    files
        .iter()
        .filter_map(|f| {
            let Ok(bytes) = std::fs::read(root.join(&f.rel_path)) else {
                return None;
            };
            Some((f.rel_path.clone(), sha256_hex16(&bytes)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_sha256_vectors() {
        // sha256("") and sha256("abc") are well-known vectors.
        assert_eq!(
            sha256_hex64(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex64(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // Truncated form keeps the first 16 chars.
        assert_eq!(sha256_hex16(b"abc"), "ba7816bf8f01cfea");
    }

    #[test]
    fn tree_hash_is_deterministic_and_sorted() {
        let a = BTreeMap::from([
            ("b.py".to_string(), "1111".to_string()),
            ("a.py".to_string(), "2222".to_string()),
        ]);
        let b = BTreeMap::from([
            ("a.py".to_string(), "2222".to_string()),
            ("b.py".to_string(), "1111".to_string()),
        ]);
        assert_eq!(tree_hash(&a), tree_hash(&b));
        // Explicit expected hash over "a.py:2222\nb.py:1111\n".
        let expected = sha256_hex64(b"a.py:2222\nb.py:1111\n");
        assert_eq!(tree_hash(&a), expected);
    }

    #[test]
    fn tree_hash_skips_empty_digests() {
        let map = BTreeMap::from([("a.py".to_string(), String::new())]);
        assert_eq!(tree_hash(&map), sha256_hex64(b""));
    }
}
