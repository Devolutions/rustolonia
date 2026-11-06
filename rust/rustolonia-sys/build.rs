#[path = "build_support/host.rs"]
mod host;

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let header = Path::new("include").join("avalonia-rust-abi.h");
    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rerun-if-changed=build_support/host.rs");
    let bytes = std::fs::read(&header)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", header.display()));
    let abi_fingerprint = host::sha256_hex(&bytes);
    println!("cargo:rustc-env=RUSTOLONIA_ABI_FINGERPRINT={abi_fingerprint}");

    let target = env::var("TARGET").expect("TARGET");
    let linux_drm = env::var_os("CARGO_FEATURE_LINUX_DRM").is_some();
    let static_host = env::var_os("CARGO_FEATURE_STATIC_HOST").is_some();
    println!("cargo:rustc-check-cfg=cfg(rustolonia_static_host)");
    host::validate_backend(&target, linux_drm).unwrap_or_else(|error| panic!("{error}"));

    // Browser builds link the Rust app into the .NET WebAssembly host (see
    // host/Browser), so there is no prebuilt host to stage.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten") {
        return;
    }

    let config = host::Config {
        version: env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION"),
        target: target.clone(),
        linux_drm,
        static_host,
        abi_fingerprint,
        host_dir: env_var("RUSTOLONIA_HOST_DIR").map(PathBuf::from),
        skip_download: env::var_os("DOCS_RS").is_some()
            || env::var_os("CARGO_FEATURE_NO_DOWNLOAD").is_some()
            || env_var("RUSTOLONIA_NO_DOWNLOAD").is_some_and(|value| is_truthy(&value)),
        offline: env_var("CARGO_NET_OFFLINE").is_some_and(|value| is_truthy(&value)),
        cache_root: cache_root(),
        base_url: env_var("RUSTOLONIA_HOST_BASE_URL")
            .unwrap_or_else(|| host::DEFAULT_BASE_URL.to_owned()),
        checksums: checksums(),
    };

    // docs.rs only documents the crate, so there is nothing to link.
    if static_host && env::var_os("DOCS_RS").is_some() {
        return;
    }

    match host::resolve(&config) {
        Ok(host::Resolution::Found { dir, .. }) => {
            println!("cargo:host_dir={}", dir.display());
            if static_host && !target.contains("-apple-ios") {
                link_static_host(&dir);
                println!("cargo:rustc-cfg=rustolonia_static_host");
            }
            if env::var_os("CARGO_FEATURE_DEV_HOST_PATH").is_some() {
                println!(
                    "cargo:rustc-env=RUSTOLONIA_BUILD_HOST_DIR={}",
                    dir.display()
                );
            }
        }
        Ok(host::Resolution::Skipped(_)) => {}
        Err(error) => {
            eprintln!("error: {error}");
            panic!("rustolonia-sys: {error}");
        }
    }
}

/// Links the static host archive and the NativeAOT runtime listed in its
/// link file. Directives from a dependency's build script reach the final
/// link only as `rustc-link-lib`/`rustc-link-search`, which is why the
/// bootstrapper object ships wrapped in an archive.
fn link_static_host(dir: &Path) {
    let link_path = dir.join(host::LINK_FILE);
    println!("cargo:rerun-if-changed={}", link_path.display());
    let text = std::fs::read_to_string(&link_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", link_path.display()));
    let directives = host::parse_link_file(&text).unwrap_or_else(|error| {
        panic!("rustolonia-sys: {}: {error}", link_path.display());
    });
    println!("cargo:rustc-link-search=native={}", dir.display());
    for directive in directives {
        match directive {
            host::LinkDirective::Static(file) => {
                let name = host::static_link_name(&file);
                println!("cargo:rustc-link-lib=static:-bundle={name}");
            }
            host::LinkDirective::WholeStatic(file) => {
                let name = host::static_link_name(&file);
                println!("cargo:rustc-link-lib=static:+whole-archive,-bundle={name}");
            }
            host::LinkDirective::System(name) => println!("cargo:rustc-link-lib=dylib={name}"),
            host::LinkDirective::Framework(name) => {
                println!("cargo:rustc-link-lib=framework={name}")
            }
            host::LinkDirective::Search(path) => {
                println!("cargo:rustc-link-search=native={path}")
            }
        }
    }
}

/// Reads an env var, treating empty as unset, and tracks it for reruns.
fn env_var(name: &str) -> Option<String> {
    println!("cargo:rerun-if-env-changed={name}");
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn is_truthy(value: &str) -> bool {
    !matches!(
        value.to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

fn cache_root() -> PathBuf {
    if let Some(dir) = env_var("RUSTOLONIA_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    dirs::cache_dir()
        .map(|dir| dir.join("rustolonia").join("host"))
        .unwrap_or_else(|| {
            PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("rustolonia-host")
        })
}

/// The checksums shipped with this crate, or `RUSTOLONIA_HOST_CHECKSUMS`
/// (a path) to verify a host from a custom `RUSTOLONIA_HOST_BASE_URL`.
fn checksums() -> String {
    let path = env_var("RUSTOLONIA_HOST_CHECKSUMS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("host-checksums.txt"));
    println!("cargo:rerun-if-changed={}", path.display());
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}
