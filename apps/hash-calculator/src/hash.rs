//! Streaming file hashes and checksum comparison helpers.

use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Bytes hashed when the application starts with no files on the command line.
pub const SAMPLE_TEXT: &[u8] = b"Rustolonia Hash Calculator\n";
pub const SAMPLE_FILE_NAME: &str = "rustolonia-hash-calculator-sample.txt";

const CHUNK_SIZE: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HashOptions {
    pub md5: bool,
    pub sha1: bool,
    pub sha256: bool,
    pub sha512: bool,
    pub blake3: bool,
}

impl Default for HashOptions {
    fn default() -> Self {
        Self {
            md5: false,
            sha1: false,
            sha256: true,
            sha512: false,
            blake3: false,
        }
    }
}

impl HashOptions {
    pub fn any(self) -> bool {
        self.md5 || self.sha1 || self.sha256 || self.sha512 || self.blake3
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileHashes {
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub sha512: Option<String>,
    pub blake3: Option<String>,
}

impl FileHashes {
    pub fn clear_disabled(&mut self, options: HashOptions) {
        if !options.md5 {
            self.md5 = None;
        }
        if !options.sha1 {
            self.sha1 = None;
        }
        if !options.sha256 {
            self.sha256 = None;
        }
        if !options.sha512 {
            self.sha512 = None;
        }
        if !options.blake3 {
            self.blake3 = None;
        }
    }

    pub fn missing(&self, options: HashOptions) -> bool {
        (options.md5 && self.md5.is_none())
            || (options.sha1 && self.sha1.is_none())
            || (options.sha256 && self.sha256.is_none())
            || (options.sha512 && self.sha512.is_none())
            || (options.blake3 && self.blake3.is_none())
    }

    pub fn has_any(&self) -> bool {
        self.md5.is_some()
            || self.sha1.is_some()
            || self.sha256.is_some()
            || self.sha512.is_some()
            || self.blake3.is_some()
    }

    pub fn matched_algorithm(&self, expected: &str) -> Option<&'static str> {
        let expected = normalize_digest(expected);
        if expected.is_empty() {
            return None;
        }
        if self.md5.as_deref() == Some(expected.as_str()) {
            return Some("MD5");
        }
        if self.sha1.as_deref() == Some(expected.as_str()) {
            return Some("SHA-1");
        }
        if self.sha256.as_deref() == Some(expected.as_str()) {
            return Some("SHA-256");
        }
        if self.sha512.as_deref() == Some(expected.as_str()) {
            return Some("SHA-512");
        }
        if self.blake3.as_deref() == Some(expected.as_str()) {
            return Some("BLAKE3");
        }
        None
    }
}

pub fn normalize_digest(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

pub fn match_label(expected: &str, hashes: &FileHashes) -> String {
    if normalize_digest(expected).is_empty() {
        return String::new();
    }
    match hashes.matched_algorithm(expected) {
        Some(name) => format!("Matches {name}"),
        None if hashes.has_any() => "No match".to_string(),
        None => String::new(),
    }
}

pub fn format_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / KIB)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / MIB)
    } else {
        format!("{:.2} GB", bytes as f64 / GIB)
    }
}

pub fn hash_bytes(bytes: &[u8], options: HashOptions) -> FileHashes {
    hash_reader(&mut &bytes[..], options).expect("in-memory hashing cannot fail")
}

pub fn hash_file(path: &Path, options: HashOptions) -> Result<FileHashes, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    hash_reader(&mut file, options)
}

pub fn hash_reader<R: Read>(reader: &mut R, options: HashOptions) -> Result<FileHashes, String> {
    if !options.any() {
        return Ok(FileHashes::default());
    }

    let mut md5 = options.md5.then(Md5::new);
    let mut sha1 = options.sha1.then(Sha1::new);
    let mut sha256 = options.sha256.then(Sha256::new);
    let mut sha512 = options.sha512.then(Sha512::new);
    let mut blake3 = options.blake3.then(blake3::Hasher::new);
    let mut buffer = vec![0_u8; CHUNK_SIZE];

    loop {
        let read = reader.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        if let Some(hasher) = md5.as_mut() {
            hasher.update(chunk);
        }
        if let Some(hasher) = sha1.as_mut() {
            hasher.update(chunk);
        }
        if let Some(hasher) = sha256.as_mut() {
            hasher.update(chunk);
        }
        if let Some(hasher) = sha512.as_mut() {
            hasher.update(chunk);
        }
        if let Some(hasher) = blake3.as_mut() {
            hasher.update(chunk);
        }
    }

    Ok(FileHashes {
        md5: md5.map(|hasher| hex::encode(hasher.finalize())),
        sha1: sha1.map(|hasher| hex::encode(hasher.finalize())),
        sha256: sha256.map(|hasher| hex::encode(hasher.finalize())),
        sha512: sha512.map(|hasher| hex::encode(hasher.finalize())),
        blake3: blake3.map(|hasher| hex::encode(hasher.finalize().as_bytes())),
    })
}

pub fn write_sample_file(path: &Path) -> Result<(), String> {
    let mut file = File::create(path).map_err(|error| error.to_string())?;
    file.write_all(SAMPLE_TEXT)
        .map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())
}

pub fn format_row_report(name: &str, path: &str, size_label: &str, hashes: &FileHashes) -> String {
    let mut lines = vec![format!("{name}  ({size_label})"), path.to_string()];
    push_digest_line(&mut lines, "MD5", hashes.md5.as_deref());
    push_digest_line(&mut lines, "SHA-1", hashes.sha1.as_deref());
    push_digest_line(&mut lines, "SHA-256", hashes.sha256.as_deref());
    push_digest_line(&mut lines, "SHA-512", hashes.sha512.as_deref());
    push_digest_line(&mut lines, "BLAKE3", hashes.blake3.as_deref());
    lines.join("\n")
}

fn push_digest_line(lines: &mut Vec<String>, label: &str, digest: Option<&str>) {
    if let Some(digest) = digest {
        lines.push(format!("  {label}: {digest}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_abc_matches_fips_vector() {
        let hashes = hash_bytes(
            b"abc",
            HashOptions {
                md5: false,
                sha1: false,
                sha256: true,
                sha512: false,
                blake3: false,
            },
        );
        assert_eq!(
            hashes.sha256.as_deref(),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
    }

    #[test]
    fn md5_of_empty_matches_rfc_1321() {
        let hashes = hash_bytes(b"", HashOptions {
            md5: true,
            sha1: false,
            sha256: false,
            sha512: false,
            blake3: false,
        });
        assert_eq!(
            hashes.md5.as_deref(),
            Some("d41d8cd98f00b204e9800998ecf8427e")
        );
    }

    #[test]
    fn sample_text_has_stable_sha256() {
        let hashes = hash_bytes(SAMPLE_TEXT, HashOptions::default());
        let digest = hashes.sha256.expect("SHA-256 is enabled by default");
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(digest, hash_bytes(SAMPLE_TEXT, HashOptions::default()).sha256.unwrap());
    }

    #[test]
    fn one_pass_computes_requested_algorithms() {
        let hashes = hash_bytes(
            SAMPLE_TEXT,
            HashOptions {
                md5: true,
                sha1: true,
                sha256: true,
                sha512: true,
                blake3: true,
            },
        );
        assert_eq!(hashes.md5.as_ref().map(String::len), Some(32));
        assert_eq!(hashes.sha1.as_ref().map(String::len), Some(40));
        assert_eq!(hashes.sha256.as_ref().map(String::len), Some(64));
        assert_eq!(hashes.sha512.as_ref().map(String::len), Some(128));
        assert_eq!(hashes.blake3.as_ref().map(String::len), Some(64));
    }

    #[test]
    fn normalize_digest_strips_separators_and_case() {
        assert_eq!(normalize_digest("BA 78-16BF"), "ba7816bf");
        assert_eq!(normalize_digest("  DE AD  "), "dead");
    }

    #[test]
    fn match_label_identifies_algorithm() {
        let hashes = hash_bytes(b"abc", HashOptions {
            md5: false,
            sha1: false,
            sha256: true,
            sha512: false,
            blake3: false,
        });
        let digest = hashes.sha256.clone().unwrap();
        assert_eq!(match_label(&digest, &hashes), "Matches SHA-256");
        assert_eq!(
            match_label("BA78-16BF 8F01CFEA 414140DE 5DAE2223 B00361A3 96177A9C B410FF61 F20015AD", &hashes),
            "Matches SHA-256"
        );
        assert_eq!(match_label("deadbeef", &hashes), "No match");
        assert_eq!(match_label("", &hashes), "");
    }

    #[test]
    fn format_size_uses_binary_units() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(2 * 1024 * 1024), "2.0 MB");
    }

    #[test]
    fn sample_file_round_trips() {
        let dir = std::env::temp_dir().join("rustolonia-hash-calculator-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.txt");
        write_sample_file(&path).unwrap();
        let hashes = hash_file(&path, HashOptions::default()).unwrap();
        assert_eq!(hashes.sha256, hash_bytes(SAMPLE_TEXT, HashOptions::default()).sha256);
    }

    #[test]
    fn missing_detects_disabled_and_uncomputed() {
        let mut hashes = FileHashes::default();
        let options = HashOptions::default();
        assert!(hashes.missing(options));
        hashes.sha256 = Some("abc".into());
        assert!(!hashes.missing(options));
        hashes.clear_disabled(HashOptions {
            sha256: false,
            ..options
        });
        assert!(hashes.sha256.is_none());
    }
}
