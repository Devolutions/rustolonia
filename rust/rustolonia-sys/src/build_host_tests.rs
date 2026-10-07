//! Tests for `build_support/host.rs`, the host fetcher used by `build.rs`.

#[path = "../build_support/host.rs"]
#[allow(dead_code)]
mod host;

use host::*;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const VERSION: &str = "1.2.3";
const TARGET: &str = "x86_64-unknown-linux-gnu";
const RID: &str = "linux-x64";
const FINGERPRINT: &str = "abc123";

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "rustolonia-host-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn manifest(version: &str, rid: &str, fingerprint: &str) -> String {
    format!(
        "{{\n  \"schemaVersion\": 1,\n  \"version\": \"{version}\",\n  \"rid\": \"{rid}\",\n  \
         \"abiFingerprint\": \"{fingerprint}\"\n}}\n"
    )
}

fn write_host_dir(dir: &Path, manifest: Option<&str>) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(host_file_name(RID)), b"host").unwrap();
    if let Some(manifest) = manifest {
        fs::write(dir.join(MANIFEST_FILE), manifest).unwrap();
    }
}

fn tarball(manifest: &str) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut builder = tar::Builder::new(encoder);
    for (name, contents) in [
        (host_file_name(RID), b"host".as_slice()),
        (MANIFEST_FILE, manifest.as_bytes()),
        ("THIRD-PARTY-NOTICES.txt", b"notices".as_slice()),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, name, contents).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

fn config(cache_root: &Path) -> Config {
    Config {
        version: VERSION.into(),
        target: TARGET.into(),
        abi_fingerprint: FINGERPRINT.into(),
        host_dir: None,
        skip_download: false,
        offline: false,
        cache_root: cache_root.to_path_buf(),
        base_url: "http://127.0.0.1:1/unused".into(),
        checksums: String::new(),
    }
}

fn expected() -> Expected<'static> {
    Expected {
        version: VERSION,
        rid: RID,
        abi_fingerprint: FINGERPRINT,
    }
}

/// Serves `body` for every request and returns the base URL plus the
/// number of requests served so far.
fn serve(body: Vec<u8>) -> (String, std::sync::Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!(
        "http://{}/releases/download",
        listener.local_addr().unwrap()
    );
    let hits = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            let expected_path =
                format!("/releases/download/v{VERSION}/{}", asset_name(VERSION, RID));
            let found = request_line.split_whitespace().nth(1) == Some(expected_path.as_str());
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            counter.fetch_add(1, Ordering::SeqCst);
            if found {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            } else {
                stream
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
            }
        }
    });
    (base, hits)
}

#[test]
fn maps_supported_targets_to_rids() {
    for (target, rid) in [
        ("x86_64-pc-windows-msvc", "win-x64"),
        ("aarch64-pc-windows-msvc", "win-arm64"),
        ("x86_64-apple-darwin", "osx-x64"),
        ("aarch64-apple-darwin", "osx-arm64"),
        ("x86_64-unknown-linux-gnu", "linux-x64"),
        ("aarch64-unknown-linux-gnu", "linux-arm64"),
    ] {
        assert_eq!(rid_for_target(target), Some(rid), "{target}");
    }
    for target in [
        "x86_64-pc-windows-gnu",
        "x86_64-unknown-linux-musl",
        "i686-pc-windows-msvc",
        "wasm32-unknown-unknown",
    ] {
        assert_eq!(rid_for_target(target), None, "{target}");
    }
}

#[test]
fn builds_asset_urls() {
    assert_eq!(
        asset_url("https://example.test/dl/", "0.1.0", "osx-arm64"),
        "https://example.test/dl/v0.1.0/rustolonia-host-0.1.0-osx-arm64.tar.gz"
    );
    assert_eq!(host_file_name("win-arm64"), "rustolonia_host.dll");
    assert_eq!(host_file_name("osx-x64"), "librustolonia_host.dylib");
    assert_eq!(host_file_name("linux-arm64"), "librustolonia_host.so");
}

#[test]
fn parses_sha256sum_files() {
    let parsed = parse_checksums(
        "# comment\n\nABCDEF  rustolonia-host-1.2.3-win-x64.tar.gz\n012345 *rustolonia-host-1.2.3-osx-x64.tar.gz\n",
    );
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed["rustolonia-host-1.2.3-win-x64.tar.gz"], "abcdef");
    assert_eq!(parsed["rustolonia-host-1.2.3-osx-x64.tar.gz"], "012345");
}

#[test]
fn reads_manifest_fields() {
    let text = manifest(VERSION, RID, FINGERPRINT);
    assert_eq!(manifest_field(&text, "version"), Some(VERSION));
    assert_eq!(manifest_field(&text, "rid"), Some(RID));
    assert_eq!(manifest_field(&text, "abiFingerprint"), Some(FINGERPRINT));
    assert_eq!(manifest_field(&text, "schemaVersion"), None);
    assert_eq!(manifest_field(&text, "missing"), None);
}

#[test]
fn validates_host_directories() {
    let temp = TempDir::new();
    let dir = temp.0.join("host");

    assert!(validate_host_dir(&dir, &expected(), false)
        .unwrap_err()
        .contains("does not contain"));

    write_host_dir(&dir, None);
    validate_host_dir(&dir, &expected(), false).unwrap();
    assert!(validate_host_dir(&dir, &expected(), true).is_err());

    fs::write(dir.join(MANIFEST_FILE), manifest(VERSION, RID, FINGERPRINT)).unwrap();
    validate_host_dir(&dir, &expected(), true).unwrap();

    for (bad, field) in [
        (manifest("9.9.9", RID, FINGERPRINT), "version"),
        (manifest(VERSION, "osx-x64", FINGERPRINT), "rid"),
        (manifest(VERSION, RID, "other"), "abiFingerprint"),
    ] {
        fs::write(dir.join(MANIFEST_FILE), bad).unwrap();
        let error = validate_host_dir(&dir, &expected(), false).unwrap_err();
        assert!(error.contains(field), "{error}");
    }
}

#[test]
fn host_dir_wins_over_skip_and_cache() {
    let temp = TempDir::new();
    let local = temp.0.join("local");
    write_host_dir(&local, None);
    write_host_dir(
        &temp.0.join("cache").join(VERSION).join(RID),
        Some(&manifest(VERSION, RID, FINGERPRINT)),
    );
    let mut config = config(&temp.0.join("cache"));
    config.host_dir = Some(local.clone());
    config.skip_download = true;
    assert_eq!(
        resolve(&config).unwrap(),
        Resolution::Found {
            dir: local,
            source: Source::HostDir
        }
    );

    config.host_dir = Some(temp.0.join("missing"));
    assert!(resolve(&config)
        .unwrap_err()
        .contains("RUSTOLONIA_HOST_DIR"));
}

#[test]
fn skip_download_ignores_target_and_cache() {
    let temp = TempDir::new();
    let mut config = config(&temp.0);
    config.skip_download = true;
    config.target = "wasm32-unknown-unknown".into();
    assert!(matches!(resolve(&config).unwrap(), Resolution::Skipped(_)));
}

#[test]
fn unsupported_targets_fail_when_downloading() {
    let temp = TempDir::new();
    let mut config = config(&temp.0);
    config.target = "x86_64-unknown-linux-musl".into();
    assert!(resolve(&config).unwrap_err().contains("no prebuilt host"));
}

#[test]
fn uses_a_valid_cache_entry_without_network() {
    let temp = TempDir::new();
    let cached = temp.0.join(VERSION).join(RID);
    write_host_dir(&cached, Some(&manifest(VERSION, RID, FINGERPRINT)));
    let mut config = config(&temp.0);
    config.offline = true;
    assert_eq!(
        resolve(&config).unwrap(),
        Resolution::Found {
            dir: cached,
            source: Source::Cache
        }
    );
}

#[test]
fn offline_without_cache_fails_clearly() {
    let temp = TempDir::new();
    let mut config = config(&temp.0);
    config.offline = true;
    let error = resolve(&config).unwrap_err();
    assert!(error.contains("offline"), "{error}");
    assert!(error.contains(&asset_name(VERSION, RID)), "{error}");
}

#[test]
fn missing_checksum_entry_fails_before_downloading() {
    let temp = TempDir::new();
    let (base, hits) = serve(tarball(&manifest(VERSION, RID, FINGERPRINT)));
    let mut config = config(&temp.0);
    config.base_url = base;
    config.checksums = "00  rustolonia-host-1.2.3-win-x64.tar.gz\n".into();
    assert!(resolve(&config).unwrap_err().contains("no checksum"));
    assert_eq!(hits.load(Ordering::SeqCst), 0);
}

#[test]
fn downloads_verifies_and_caches_from_http() {
    let temp = TempDir::new();
    let archive = tarball(&manifest(VERSION, RID, FINGERPRINT));
    let (base, hits) = serve(archive.clone());
    let mut config = config(&temp.0);
    config.base_url = base;
    config.checksums = format!("{}  {}\n", sha256_hex(&archive), asset_name(VERSION, RID));

    let cached = temp.0.join(VERSION).join(RID);
    assert_eq!(
        resolve(&config).unwrap(),
        Resolution::Found {
            dir: cached.clone(),
            source: Source::Download
        }
    );
    assert_eq!(fs::read(cached.join(host_file_name(RID))).unwrap(), b"host");
    assert!(cached.join("THIRD-PARTY-NOTICES.txt").is_file());
    let leftovers: Vec<_> = fs::read_dir(temp.0.join(VERSION))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, [std::ffi::OsString::from(RID)]);

    assert_eq!(
        resolve(&config).unwrap(),
        Resolution::Found {
            dir: cached,
            source: Source::Cache
        }
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn rejects_checksum_mismatch_without_caching() {
    let temp = TempDir::new();
    let (base, _) = serve(tarball(&manifest(VERSION, RID, FINGERPRINT)));
    let mut config = config(&temp.0);
    config.base_url = base;
    config.checksums = format!("{}  {}\n", "0".repeat(64), asset_name(VERSION, RID));
    assert!(resolve(&config).unwrap_err().contains("checksum mismatch"));
    assert!(!temp.0.join(VERSION).join(RID).exists());
}

#[test]
fn rejects_archives_for_another_abi() {
    let temp = TempDir::new();
    let archive = tarball(&manifest(VERSION, RID, "different"));
    let (base, _) = serve(archive.clone());
    let mut config = config(&temp.0);
    config.base_url = base;
    config.checksums = format!("{}  {}\n", sha256_hex(&archive), asset_name(VERSION, RID));
    let error = resolve(&config).unwrap_err();
    assert!(error.contains("abiFingerprint"), "{error}");
    assert!(!temp.0.join(VERSION).join(RID).exists());
}

#[test]
fn reports_http_errors() {
    let temp = TempDir::new();
    let (base, _) = serve(Vec::new());
    let mut config = config(&temp.0);
    config.base_url = format!("{base}/elsewhere");
    config.checksums = format!("{}  {}\n", "0".repeat(64), asset_name(VERSION, RID));
    assert!(resolve(&config).unwrap_err().contains("failed to download"));
}

#[test]
fn downloads_from_file_urls() {
    let temp = TempDir::new();
    let archive = tarball(&manifest(VERSION, RID, FINGERPRINT));
    let release = temp.0.join("release").join(format!("v{VERSION}"));
    fs::create_dir_all(&release).unwrap();
    fs::write(release.join(asset_name(VERSION, RID)), &archive).unwrap();

    let mut config = config(&temp.0.join("cache"));
    let release_root = temp.0.join("release").to_string_lossy().replace('\\', "/");
    config.base_url = format!("file:///{}", release_root.trim_start_matches('/'));
    config.checksums = format!("{}  {}\n", sha256_hex(&archive), asset_name(VERSION, RID));
    assert!(matches!(
        resolve(&config).unwrap(),
        Resolution::Found {
            source: Source::Download,
            ..
        }
    ));
}
