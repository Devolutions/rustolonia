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

/// File listing the link inputs of a static host, one directive per line.
pub const LINK_FILE: &str = "rustolonia-host-link.txt";

/// File name of the host library for a RID: the shared library, or the
/// static archive when `static_host` is set.
pub fn host_file_name(rid: &str, static_host: bool) -> &'static str {
    if static_host {
        return if rid.starts_with("win-") {
            "rustolonia_host.lib"
        } else {
            "librustolonia_host.a"
        };
    }
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

pub fn validate_backend(target: &str, linux_drm: bool) -> Result<(), String> {
    if linux_drm && !matches!(rid_for_target(target), Some("linux-x64" | "linux-arm64")) {
        return Err(format!(
            "the `linux-drm` feature requires an x86_64/aarch64 linux-gnu target, not {target}"
        ));
    }
    Ok(())
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

/// The cache directory and release asset suffix for a host build:
/// `<rid>[-drm][-static]`.
pub fn host_variant(rid: &str, linux_drm: bool, static_host: bool) -> String {
    let mut variant = rid.to_owned();
    if linux_drm {
        variant.push_str("-drm");
    }
    if static_host {
        variant.push_str("-static");
    }
    variant
}

/// One line of `rustolonia-host-link.txt`, written by the host project's
/// `StageRustoloniaStaticHost` target in link order.
#[derive(Debug, PartialEq, Eq)]
pub enum LinkDirective {
    /// A static library file in the host directory.
    Static(String),
    /// A static library whose objects must all be kept (the NativeAOT
    /// bootstrapper, which nothing references by symbol).
    WholeStatic(String),
    /// A system library linked by name.
    System(String),
    /// An Apple framework.
    Framework(String),
    /// An extra native library search directory.
    Search(String),
}

/// Parses `rustolonia-host-link.txt`, ignoring blanks and `#` comments.
pub fn parse_link_file(text: &str) -> Result<Vec<LinkDirective>, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (kind, value) = line
                .split_once(char::is_whitespace)
                .map(|(kind, value)| (kind, value.trim()))
                .filter(|(_, value)| !value.is_empty())
                .ok_or_else(|| format!("invalid link directive `{line}`"))?;
            let value = value.to_owned();
            match kind {
                "static" => Ok(LinkDirective::Static(value)),
                "whole-static" => Ok(LinkDirective::WholeStatic(value)),
                "system" => Ok(LinkDirective::System(value)),
                "framework" => Ok(LinkDirective::Framework(value)),
                "search" => Ok(LinkDirective::Search(value)),
                _ => Err(format!("unknown link directive `{line}`")),
            }
        })
        .collect()
}

/// The cargo link name of a static library file: `+verbatim` is not
/// supported by every linker rustc drives (ld64), so strip the platform
/// prefix and extension instead.
pub fn static_link_name(file: &str) -> &str {
    if let Some(name) = file.strip_suffix(".lib") {
        name
    } else {
        let name = file.strip_suffix(".a").unwrap_or(file);
        name.strip_prefix("lib").unwrap_or(name)
    }
}

/// What the staged host must match.
pub struct Expected<'a> {
    pub version: &'a str,
    pub rid: &'a str,
    pub abi_fingerprint: &'a str,
    pub linux_drm: bool,
    pub static_host: bool,
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
    let host_file = host_file_name(expected.rid, expected.static_host);
    if !dir.join(host_file).is_file() {
        let hint = if expected.static_host {
            "; the `static-host` feature needs a host packaged with -Linkage Static"
        } else {
            ""
        };
        return Err(format!(
            "{} does not contain {host_file}{hint}",
            dir.display()
        ));
    }
    if expected.static_host {
        let link_path = dir.join(LINK_FILE);
        let link_file = fs::read_to_string(&link_path)
            .map_err(|error| format!("cannot read {}: {error}", link_path.display()))?;
        for directive in parse_link_file(&link_file)? {
            if let LinkDirective::Static(file) | LinkDirective::WholeStatic(file) = directive {
                if !dir.join(&file).is_file() {
                    return Err(format!("{} does not contain {file}", dir.display()));
                }
            }
        }
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
    let want = if expected.static_host {
        "Static"
    } else {
        "Shared"
    };
    let found = manifest_field(&manifest, "linkage");
    // Archives that predate static hosts did not record their linkage.
    if found != Some(want) && !(found.is_none() && !expected.static_host) {
        return Err(format!(
            "{} has linkage {}, expected {want}",
            manifest_path.display(),
            found.unwrap_or("<missing>")
        ));
    }
    if expected.rid.starts_with("linux-") {
        let want = if expected.linux_drm {
            "LinuxDrm"
        } else {
            "X11"
        };
        let found = manifest_field(&manifest, "platform");
        // Legacy desktop archives did not record their platform.
        if found != Some(want) && !(found.is_none() && !expected.linux_drm) {
            return Err(format!(
                "{} has platform {}, expected {want}",
                manifest_path.display(),
                found.unwrap_or("<missing>")
            ));
        }
    }
    Ok(())
}

/// Inputs to [`resolve`], gathered from the build environment.
pub struct Config {
    pub version: String,
    pub target: String,
    pub linux_drm: bool,
    /// Link the static host archive instead of loading a shared library.
    pub static_host: bool,
    pub abi_fingerprint: String,
    pub host_dir: Option<PathBuf>,
    pub skip_download: bool,
    pub offline: bool,
    /// Directory holding `<version>/<rid>[-drm][-static]` cache entries.
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

/// Android hosts are .NET Android assemblies packaged inside the APK, which
/// hand Rust their entry points at startup (`Host::from_in_process`); there is
/// no host library to stage next to a Rust binary.
pub fn is_embedded_host_target(target: &str) -> bool {
    target.contains("-linux-android")
}

pub fn resolve(config: &Config) -> Result<Resolution, String> {
    validate_backend(&config.target, config.linux_drm)?;
    // iOS links the host into the app. Its separate iOS app builder stages
    // the static-host archive; this resolver handles desktop hosts.
    if config.target.contains("-apple-ios") {
        return Ok(Resolution::Skipped("iOS uses an in-process host".into()));
    }

    if is_embedded_host_target(&config.target) {
        return Ok(Resolution::Skipped(
            "the Android host is packaged inside the APK".into(),
        ));
    }

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
        if config.static_host {
            return Err(
                "the `static-host` feature links the host at build time, so it \
                        needs RUSTOLONIA_HOST_DIR or a host download; it cannot be \
                        combined with `no-download` or RUSTOLONIA_NO_DOWNLOAD"
                    .into(),
            );
        }
        return Ok(Resolution::Skipped("host download disabled".into()));
    }

    let rid = rid.ok_or_else(|| unsupported_target(&config.target))?;
    let expected = expected(config, rid);
    let variant = host_variant(rid, config.linux_drm, config.static_host);
    let cache_dir = config.cache_root.join(&config.version).join(&variant);
    if validate_host_dir(&cache_dir, &expected, true).is_ok() {
        return Ok(Resolution::Found {
            dir: cache_dir,
            source: Source::Cache,
        });
    }

    let asset = asset_name(&config.version, &variant);
    let url = asset_url(&config.base_url, &config.version, &variant);
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
        linux_drm: config.linux_drm,
        static_host: config.static_host,
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
