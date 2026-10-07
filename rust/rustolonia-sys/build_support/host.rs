//! Host resolution shared by `build.rs` and the crate's unit tests.
//!
//! `build.rs` stages the prebuilt rustolonia NativeAOT host for the target
//! RID, in this order:
//! 1. `RUSTOLONIA_HOST_DIR`: a local directory (dev, CI, vendored, distro).
//! 2. `DOCS_RS`, the `no-download` feature or `RUSTOLONIA_NO_DOWNLOAD`: skip.
//! 3. The download cache.
//! 4. A download of the release tarball, verified against `host-checksums.txt`.
//!
//! This file must stay self-contained: it is compiled both as part of the
//! build script and (through `#[path]`) as a test module of the library.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_BASE_URL: &str = "https://github.com/Devolutions/rustolonia/releases/download";
pub const MANIFEST_FILE: &str = "host-manifest.json";

/// Maps a Rust target triple to the .NET RID of the matching host.
pub fn rid_for_target(target: &str) -> Option<&'static str> {
    let arm64 = match target.split('-').next()? {
        "x86_64" => false,
        "aarch64" => true,
        _ => return None,
    };
    Some(
        match (
            target.contains("-windows-msvc"),
            target.contains("-apple-darwin"),
            target.contains("-linux-gnu"),
            arm64,
        ) {
            (true, _, _, false) => "win-x64",
            (true, _, _, true) => "win-arm64",
            (_, true, _, false) => "osx-x64",
            (_, true, _, true) => "osx-arm64",
            (_, _, true, false) => "linux-x64",
            (_, _, true, true) => "linux-arm64",
            _ => return None,
        },
    )
}

/// File name of the host library for a RID.
pub fn host_file_name(rid: &str) -> &'static str {
    if rid.starts_with("win-") {
        "rustolonia_host.dll"
    } else if rid.starts_with("osx-") {
        "librustolonia_host.dylib"
    } else {
        "librustolonia_host.so"
    }
}

pub fn asset_name(version: &str, rid: &str) -> String {
    format!("rustolonia-host-{version}-{rid}.tar.gz")
}

pub fn asset_url(base_url: &str, version: &str, rid: &str) -> String {
    format!(
        "{}/v{version}/{}",
        base_url.trim_end_matches('/'),
        asset_name(version, rid)
    )
}

/// Parses `sha256sum`-style lines (`<hex>  <file>`), ignoring blanks and
/// `#` comments. File names may carry the binary-mode `*` prefix.
pub fn parse_checksums(text: &str) -> HashMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let file = parts.next()?.trim_start_matches('*');
            Some((file.to_owned(), hash.to_ascii_lowercase()))
        })
        .collect()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Reads one string field from the flat `host-manifest.json` written by
/// `package-host.ps1`. Values are plain identifiers, versions and hashes,
/// so escapes are not supported.
pub fn manifest_field<'a>(manifest: &'a str, field: &str) -> Option<&'a str> {
    let key = format!("\"{field}\"");
    let rest = &manifest[manifest.find(&key)? + key.len()..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    Some(&rest[..rest.find('"')?])
}

/// What the staged host must match.
pub struct Expected<'a> {
    pub version: &'a str,
    pub rid: &'a str,
    pub abi_fingerprint: &'a str,
}

/// Checks a staged host directory. `host-manifest.json` is required when
/// `require_manifest` is set (release tarballs); a local directory without
/// one only needs the host library, and the load-time ABI check still
/// applies.
pub fn validate_host_dir(
    dir: &Path,
    expected: &Expected<'_>,
    require_manifest: bool,
) -> Result<(), String> {
    let host = dir.join(host_file_name(expected.rid));
    if !host.is_file() {
        return Err(format!(
            "{} does not contain {}",
            dir.display(),
            host_file_name(expected.rid)
        ));
    }
    let manifest_path = dir.join(MANIFEST_FILE);
    let manifest = match fs::read_to_string(&manifest_path) {
        Ok(manifest) => manifest,
        Err(_) if !require_manifest => return Ok(()),
        Err(error) => return Err(format!("cannot read {}: {error}", manifest_path.display())),
    };
    for (field, want) in [
        ("version", expected.version),
        ("rid", expected.rid),
        ("abiFingerprint", expected.abi_fingerprint),
    ] {
        match manifest_field(&manifest, field) {
            Some(found) if found == want => {}
            found => {
                return Err(format!(
                    "{} has {field} {}, expected {want}",
                    manifest_path.display(),
                    found.unwrap_or("<missing>")
                ))
            }
        }
    }
    Ok(())
}

/// Inputs to [`resolve`], gathered from the build environment.
pub struct Config {
    pub version: String,
    pub target: String,
    pub abi_fingerprint: String,
    pub host_dir: Option<PathBuf>,
    pub skip_download: bool,
    pub offline: bool,
    /// Directory holding `<version>/<rid>` cache entries.
    pub cache_root: PathBuf,
    pub base_url: String,
    /// Contents of the checksum file to verify downloads against.
    pub checksums: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    /// The host is staged in this directory.
    Found { dir: PathBuf, source: Source },
    /// No host is staged; the runtime falls back to its other search paths.
    Skipped(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Source {
    HostDir,
    Cache,
    Download,
}

pub fn resolve(config: &Config) -> Result<Resolution, String> {
    let rid = rid_for_target(&config.target);

    if let Some(dir) = &config.host_dir {
        let rid = rid.ok_or_else(|| unsupported_target(&config.target))?;
        validate_host_dir(dir, &expected(config, rid), false)
            .map_err(|error| format!("RUSTOLONIA_HOST_DIR is not usable: {error}"))?;
        return Ok(Resolution::Found {
            dir: dir.clone(),
            source: Source::HostDir,
        });
    }

    if config.skip_download {
        return Ok(Resolution::Skipped("host download disabled".into()));
    }

    let rid = rid.ok_or_else(|| unsupported_target(&config.target))?;
    let expected = expected(config, rid);
    let cache_dir = config.cache_root.join(&config.version).join(rid);
    if validate_host_dir(&cache_dir, &expected, true).is_ok() {
        return Ok(Resolution::Found {
            dir: cache_dir,
            source: Source::Cache,
        });
    }

    let asset = asset_name(&config.version, rid);
    let url = asset_url(&config.base_url, &config.version, rid);
    if config.offline {
        return Err(format!(
            "the rustolonia host {asset} is not cached in {} and cargo is offline; \
             build once online, or download {url}, extract it and set \
             RUSTOLONIA_HOST_DIR, or enable the `no-download` feature",
            cache_dir.display()
        ));
    }
    let checksum = parse_checksums(&config.checksums)
        .remove(&asset)
        .ok_or_else(|| {
            format!(
                "no checksum for {asset}; this rustolonia-sys release does not ship \
                 a host for {rid}. Set RUSTOLONIA_HOST_DIR to a host you built, or \
                 enable the `no-download` feature"
            )
        })?;

    let archive = fetch(&url)?;
    let actual = sha256_hex(&archive);
    if actual != checksum {
        return Err(format!(
            "checksum mismatch for {url}: expected {checksum}, got {actual}"
        ));
    }
    install(&archive, &cache_dir, &expected)?;
    Ok(Resolution::Found {
        dir: cache_dir,
        source: Source::Download,
    })
}

fn expected<'a>(config: &'a Config, rid: &'a str) -> Expected<'a> {
    Expected {
        version: &config.version,
        rid,
        abi_fingerprint: &config.abi_fingerprint,
    }
}

fn unsupported_target(target: &str) -> String {
    format!(
        "rustolonia has no prebuilt host for target {target}; supported targets are \
         x86_64/aarch64 windows-msvc, apple-darwin and linux-gnu. Set \
         RUSTOLONIA_HOST_DIR or enable the `no-download` feature"
    )
}

/// Downloads `url`. `file://` URLs are read directly so release smoke tests
/// can run without a server.
fn fetch(url: &str) -> Result<Vec<u8>, String> {
    if let Some(path) = url.strip_prefix("file://") {
        // file:///C:/x on Windows, file:///x elsewhere.
        let path = if cfg!(windows) {
            path.trim_start_matches('/')
        } else {
            path
        };
        return fs::read(path).map_err(|error| format!("cannot read {url}: {error}"));
    }
    let agent = ureq::AgentBuilder::new().try_proxy_from_env(true).build();
    let response = agent
        .get(url)
        .call()
        .map_err(|error| format!("failed to download {url}: {error}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(512 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to download {url}: {error}"))?;
    Ok(bytes)
}

/// Extracts `archive` into a private temporary directory beside
/// `destination`, validates it, then renames it into place so concurrent
/// builds never observe a partial host.
fn install(archive: &[u8], destination: &Path, expected: &Expected<'_>) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("invalid cache directory {}", destination.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let staging = parent.join(format!(".tmp-{}-{nonce}", std::process::id()));
    let result = (|| {
        tar::Archive::new(flate2::read::GzDecoder::new(archive))
            .unpack(&staging)
            .map_err(|error| format!("cannot extract host archive: {error}"))?;
        validate_host_dir(&staging, expected, true)
            .map_err(|error| format!("downloaded host is invalid: {error}"))?;
        if destination.exists() {
            // A concurrent build won the race, or an older entry is broken.
            if validate_host_dir(destination, expected, true).is_ok() {
                return Ok(());
            }
            fs::remove_dir_all(destination)
                .map_err(|error| format!("cannot replace {}: {error}", destination.display()))?;
        }
        match fs::rename(&staging, destination) {
            Ok(()) => Ok(()),
            Err(_) if validate_host_dir(destination, expected, true).is_ok() => Ok(()),
            Err(error) => Err(format!(
                "cannot move host into {}: {error}",
                destination.display()
            )),
        }
    })();
    let _ = fs::remove_dir_all(&staging);
    result
}
