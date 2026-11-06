//! Finds the prebuilt Rustolonia Android host APK.
//!
//! In order:
//! 1. `--host-apk` or `RUSTOLONIA_ANDROID_HOST_APK`: a local file, used as is
//!    (a host built from a source checkout, or a vendored copy).
//! 2. The download cache, re-verified against the pinned checksum.
//! 3. A download of `rustolonia-host-<version>-android.apk` from the GitHub
//!    release, verified against the checksum before it is cached or used.
//!
//! `<version>` and the checksum come from the `rustolonia-sys` crate the app
//! actually resolves (its `host-checksums.txt`), so the host always matches
//! the bindings compiled into the app, whatever version of this tool runs.
//! The mirror and cache settings are the ones `rustolonia-sys` uses for the
//! desktop hosts: `RUSTOLONIA_HOST_BASE_URL`, `RUSTOLONIA_HOST_CHECKSUMS` and
//! `RUSTOLONIA_CACHE_DIR`.

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_BASE_URL: &str = "https://github.com/Devolutions/rustolonia/releases/download";

/// The release asset name of the Android host for a rustolonia version.
pub fn asset_name(version: &str) -> String {
    format!("rustolonia-host-{version}-android.apk")
}

pub fn asset_url(base_url: &str, version: &str) -> String {
    format!(
        "{}/v{version}/{}",
        base_url.trim_end_matches('/'),
        asset_name(version)
    )
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Looks `asset` up in `sha256sum`-style lines (`<hex>  <file>`), ignoring
/// blanks and `#` comments, like `rustolonia-sys`'s `host-checksums.txt`.
pub fn checksum_for(checksums: &str, asset: &str) -> Option<String> {
    checksums
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let file = parts.next()?.trim_start_matches('*');
            (file == asset && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| hash.to_ascii_lowercase())
        })
}

/// The `rustolonia-sys` package the app resolves: its version and the
/// directory holding its `host-checksums.txt`.
#[derive(Debug, PartialEq)]
pub struct Bindings {
    pub version: String,
    pub root: PathBuf,
}

/// Finds the single `rustolonia-sys` package in full `cargo metadata` output.
pub fn bindings_from_metadata(metadata: &serde_json::Value) -> Result<Bindings, String> {
    let found: Vec<&serde_json::Value> = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["name"] == "rustolonia-sys")
        .collect();
    let package =
        match found.as_slice() {
            [package] => package,
            [] => return Err(
                "the app does not depend on rustolonia (no rustolonia-sys in its dependency graph)"
                    .into(),
            ),
            _ => {
                return Err(
                    "the app's dependency graph has several rustolonia-sys versions; \
                 depend on a single rustolonia release"
                        .into(),
                )
            }
        };
    let version = package["version"]
        .as_str()
        .ok_or("cargo metadata: rustolonia-sys has no version")?;
    let root = package["manifest_path"]
        .as_str()
        .map(Path::new)
        .and_then(Path::parent)
        .ok_or("cargo metadata: rustolonia-sys has no manifest_path")?;
    Ok(Bindings {
        version: version.to_owned(),
        root: root.to_owned(),
    })
}

/// Runs `cargo metadata` (with dependencies) for the app.
pub fn read_bindings(manifest: &Path) -> Result<Bindings, String> {
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(manifest)
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
    bindings_from_metadata(&metadata)
}

/// `RUSTOLONIA_HOST_CHECKSUMS` as an absolute path.
///
/// `rustolonia-sys`'s build script reads this variable relative to its own
/// crate directory, so a relative value must be resolved against the
/// directory this tool runs in before it is passed to the app's build.
pub fn checksums_override() -> Result<Option<PathBuf>, String> {
    let Some(value) = std::env::var_os("RUSTOLONIA_HOST_CHECKSUMS").filter(|v| !v.is_empty())
    else {
        return Ok(None);
    };
    std::path::absolute(&value).map(Some).map_err(|e| {
        format!(
            "RUSTOLONIA_HOST_CHECKSUMS ({}): {e}",
            Path::new(&value).display()
        )
    })
}

/// Where and how to fetch a release host.
pub struct Release {
    pub version: String,
    /// Contents of the checksum list to verify against.
    pub checksums: String,
    pub base_url: String,
    pub cache_root: PathBuf,
    pub offline: bool,
}

impl Release {
    /// The settings for `bindings`, honouring the `rustolonia-sys` variables.
    pub fn from_env(bindings: &Bindings) -> Result<Self, String> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let checksums_path =
            checksums_override()?.unwrap_or_else(|| bindings.root.join("host-checksums.txt"));
        let checksums = std::fs::read_to_string(&checksums_path)
            .map_err(|e| format!("{}: {e}", checksums_path.display()))?;
        let cache_root = match var("RUSTOLONIA_CACHE_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => dirs::cache_dir()
                .ok_or("no cache directory; set RUSTOLONIA_CACHE_DIR")?
                .join("rustolonia")
                .join("host"),
        };
        Ok(Self {
            version: bindings.version.clone(),
            checksums,
            base_url: var("RUSTOLONIA_HOST_BASE_URL").unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            cache_root,
            offline: var("CARGO_NET_OFFLINE").is_some_and(|v| {
                !matches!(
                    v.to_ascii_lowercase().as_str(),
                    "0" | "false" | "no" | "off"
                )
            }),
        })
    }

    pub fn cache_path(&self) -> PathBuf {
        self.cache_root
            .join(&self.version)
            .join("android")
            .join(asset_name(&self.version))
    }

    /// Returns the verified host APK, from the cache or downloaded into it.
    pub fn fetch(&self) -> Result<(Vec<u8>, PathBuf), String> {
        let asset = asset_name(&self.version);
        let expected = checksum_for(&self.checksums, &asset).ok_or_else(|| {
            format!(
                "no checksum for {asset}: rustolonia-sys {} was released without an \
                 Android host. Use a release that has one, or pass --host-apk with a \
                 host built by rust/package-android-host.ps1",
                self.version
            )
        })?;
        let cached = self.cache_path();
        if let Ok(bytes) = std::fs::read(&cached) {
            if sha256_hex(&bytes) == expected {
                return Ok((bytes, cached));
            }
        }
        let url = asset_url(&self.base_url, &self.version);
        if self.offline {
            return Err(format!(
                "{asset} is not cached in {} and cargo is offline; download {url} and \
                 pass it with --host-apk",
                cached.display()
            ));
        }
        eprintln!("Downloading {url}");
        let bytes = download(&url)?;
        let actual = sha256_hex(&bytes);
        if actual != expected {
            return Err(format!(
                "checksum mismatch for {url}: expected {expected}, got {actual}"
            ));
        }
        store(&cached, &bytes)?;
        Ok((bytes, cached))
    }
}

/// Downloads `url`; `file://` URLs are read directly (mirrors and tests).
fn download(url: &str) -> Result<Vec<u8>, String> {
    if let Some(path) = url.strip_prefix("file://") {
        // file:///C:/x on Windows, file:///x elsewhere.
        let path = if cfg!(windows) {
            path.trim_start_matches('/')
        } else {
            path
        };
        return std::fs::read(path).map_err(|e| format!("cannot read {url}: {e}"));
    }
    let agent = ureq::AgentBuilder::new().try_proxy_from_env(true).build();
    let response = agent
        .get(url)
        .call()
        .map_err(|e| format!("failed to download {url}: {e}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(1024 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("failed to download {url}: {e}"))?;
    Ok(bytes)
}

/// Writes `bytes` beside `path`, then renames it into place, so concurrent
/// builds never read a partial file.
fn store(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid cache path {}", path.display()))?;
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let temp = parent.join(format!(".tmp-{}-{nonce}", std::process::id()));
    std::fs::write(&temp, bytes).map_err(|e| format!("{}: {e}", temp.display()))?;
    if let Err(error) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        // Another build may have stored the same verified file meanwhile.
        if std::fs::read(path).is_ok_and(|existing| existing == bytes) {
            return Ok(());
        }
        return Err(format!("cannot store {}: {error}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rustolonia-apk-{name}-{nonce}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn file_url(dir: &Path) -> String {
        let path = dir.to_string_lossy().replace('\\', "/");
        if path.starts_with('/') {
            format!("file://{path}")
        } else {
            format!("file:///{path}")
        }
    }

    #[test]
    fn asset_names_follow_the_host_convention() {
        assert_eq!(asset_name("12.1.5"), "rustolonia-host-12.1.5-android.apk");
        assert_eq!(
            asset_url("https://example.test/dl/", "12.1.5"),
            "https://example.test/dl/v12.1.5/rustolonia-host-12.1.5-android.apk"
        );
    }

    #[test]
    fn checksums_are_looked_up_by_exact_asset_name() {
        let hash = "AB".repeat(32);
        let text = format!(
            "# comment\n\n{}  rustolonia-host-1.0.0-linux-x64.tar.gz\n{hash} *rustolonia-host-1.0.0-android.apk\n",
            "cd".repeat(32)
        );
        assert_eq!(
            checksum_for(&text, "rustolonia-host-1.0.0-android.apk"),
            Some(hash.to_ascii_lowercase())
        );
        assert_eq!(
            checksum_for(&text, "rustolonia-host-1.0.1-android.apk"),
            None
        );
        assert_eq!(
            checksum_for(
                "xyz  rustolonia-host-1.0.0-android.apk",
                "rustolonia-host-1.0.0-android.apk"
            ),
            None
        );
    }

    #[test]
    fn bindings_come_from_the_single_rustolonia_sys_package() {
        let metadata = serde_json::json!({ "packages": [
            { "name": "rustolonia", "version": "1.0.0", "manifest_path": "/a/rustolonia/Cargo.toml" },
            { "name": "rustolonia-sys", "version": "1.0.0", "manifest_path": "/a/rustolonia-sys/Cargo.toml" },
        ]});
        assert_eq!(
            bindings_from_metadata(&metadata).unwrap(),
            Bindings {
                version: "1.0.0".into(),
                root: PathBuf::from("/a/rustolonia-sys")
            }
        );
        let none = serde_json::json!({ "packages": [] });
        assert!(bindings_from_metadata(&none).is_err());
        let two = serde_json::json!({ "packages": [
            { "name": "rustolonia-sys", "version": "1.0.0", "manifest_path": "/a/Cargo.toml" },
            { "name": "rustolonia-sys", "version": "2.0.0", "manifest_path": "/b/Cargo.toml" },
        ]});
        assert!(bindings_from_metadata(&two).is_err());
    }

    #[test]
    fn downloads_are_verified_then_cached() {
        let mirror = temp_dir("mirror");
        let apk = b"not really an apk".to_vec();
        std::fs::create_dir_all(mirror.join("v9.9.9")).unwrap();
        std::fs::write(mirror.join("v9.9.9").join(asset_name("9.9.9")), &apk).unwrap();
        let cache = temp_dir("cache");
        let mut release = Release {
            version: "9.9.9".into(),
            checksums: format!("{}  {}\n", sha256_hex(&apk), asset_name("9.9.9")),
            base_url: file_url(&mirror),
            cache_root: cache.clone(),
            offline: false,
        };

        let (bytes, path) = release.fetch().unwrap();
        assert_eq!(bytes, apk);
        assert_eq!(path, release.cache_path());
        assert_eq!(std::fs::read(&path).unwrap(), apk);

        // A cache hit needs no mirror, even offline.
        std::fs::remove_dir_all(&mirror).unwrap();
        release.offline = true;
        assert_eq!(release.fetch().unwrap().0, apk);

        // A corrupted cache entry is not used.
        std::fs::write(&path, b"tampered").unwrap();
        assert!(release.fetch().unwrap_err().contains("offline"));
        std::fs::remove_dir_all(&cache).unwrap();
    }

    #[test]
    fn mismatched_or_unlisted_hosts_are_rejected() {
        let mirror = temp_dir("bad-mirror");
        std::fs::create_dir_all(mirror.join("v9.9.8")).unwrap();
        std::fs::write(mirror.join("v9.9.8").join(asset_name("9.9.8")), b"evil").unwrap();
        let cache = temp_dir("bad-cache");
        let release = Release {
            version: "9.9.8".into(),
            checksums: format!("{}  {}\n", "0".repeat(64), asset_name("9.9.8")),
            base_url: file_url(&mirror),
            cache_root: cache.clone(),
            offline: false,
        };
        assert!(release.fetch().unwrap_err().contains("checksum mismatch"));
        assert!(!release.cache_path().exists());

        let unlisted = Release {
            checksums: "# desktop-only release\n".into(),
            ..release
        };
        assert!(unlisted
            .fetch()
            .unwrap_err()
            .contains("without an Android host"));
        std::fs::remove_dir_all(&mirror).unwrap();
        std::fs::remove_dir_all(&cache).unwrap();
    }
}
