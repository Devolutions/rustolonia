//! `cargo rustolonia-apk`: build a code-first Rustolonia app into an Android
//! APK with Cargo alone.
//!
//! The managed side of a code-first app is identical for every app, so it
//! ships prebuilt: the Rustolonia host APK. This tool cross-compiles the
//! app's `cdylib` for each ABI with the NDK, adds it to the host APK, rewrites
//! the binary manifest (package, label, version, library name) and signs the
//! result (APK Signature Scheme v2). No .NET SDK, Android workload or JDK.
//! See `rust/PLATFORMS.md#android`.

mod axml;
mod host;
mod package;
mod sign;
mod zip;

use package::AppIdentity;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
Build a code-first Rustolonia app into an Android APK.

Usage: cargo rustolonia-apk [OPTIONS]

Options:
  --manifest-path <PATH>  The app's Cargo.toml (default: ./Cargo.toml)
  --abi <ABI>             arm64-v8a or x86_64; repeat for both (default: both)
  --debug                 Build the debug profile (default: release)
  --host-apk <PATH>       A local Rustolonia host APK (default:
                          $RUSTOLONIA_ANDROID_HOST_APK, else the release host
                          matching the app's rustolonia-sys, downloaded once,
                          verified against its host-checksums.txt and cached)
  --out <PATH>            Output APK (default: <target>/android/<package>.apk)
  --package <ID>          Override the Android package name from Cargo.toml
  --label <TEXT>          Override the launcher label from Cargo.toml
  --key <PATH>            PKCS#8 DER ECDSA P-256 signing key   } default: a debug
  --cert <PATH>           X.509 DER certificate for --key      } key in ~/.rustolonia
  --skip-build            Package the libraries Cargo already built
  --install               adb install -r the APK
  --launch                Install, then start the app
  -h, --help              Print this help

The app's Cargo.toml needs crate-type = [\"cdylib\", ...] and:

  [package.metadata.rustolonia.android]
  package = \"com.example.my_app\"   # required
  label = \"My App\"                 # default: the crate name
  version-code = 1                 # default: 1
  version-name = \"1.0\"             # default: the crate version
";

#[derive(Default)]
struct Options {
    manifest_path: Option<PathBuf>,
    abis: Vec<String>,
    debug: bool,
    host_apk: Option<PathBuf>,
    out: Option<PathBuf>,
    package: Option<String>,
    label: Option<String>,
    key: Option<PathBuf>,
    cert: Option<PathBuf>,
    skip_build: bool,
    install: bool,
    launch: bool,
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // `cargo rustolonia-apk ...` runs `cargo-rustolonia-apk rustolonia-apk ...`.
    if args.first().map(String::as_str) == Some("rustolonia-apk") {
        args.remove(0);
    }
    match parse(args).and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse(args: Vec<String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--manifest-path" => options.manifest_path = Some(value(&arg)?.into()),
            "--abi" => {
                let abi = value(&arg)?;
                triple(&abi)?;
                options.abis.push(abi);
            }
            "--debug" => options.debug = true,
            "--host-apk" => options.host_apk = Some(value(&arg)?.into()),
            "--out" => options.out = Some(value(&arg)?.into()),
            "--package" => options.package = Some(value(&arg)?),
            "--label" => options.label = Some(value(&arg)?),
            "--key" => options.key = Some(value(&arg)?.into()),
            "--cert" => options.cert = Some(value(&arg)?.into()),
            "--skip-build" => options.skip_build = true,
            "--install" => options.install = true,
            "--launch" => options.launch = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unexpected argument {other:?}\n\n{USAGE}")),
        }
    }
    if options.abis.is_empty() {
        options.abis = vec!["arm64-v8a".into(), "x86_64".into()];
    }
    if options.key.is_some() != options.cert.is_some() {
        return Err("--key and --cert go together".into());
    }
    Ok(options)
}

fn triple(abi: &str) -> Result<&'static str, String> {
    match abi {
        "arm64-v8a" => Ok("aarch64-linux-android"),
        "x86_64" => Ok("x86_64-linux-android"),
        other => Err(format!(
            "unsupported ABI {other:?}: the Rustolonia host (CoreCLR) runs on arm64-v8a and x86_64"
        )),
    }
}

/// What `cargo metadata` says about the app crate.
struct Crate {
    name: String,
    version: String,
    library: String,
    target_directory: PathBuf,
    android: serde_json::Value,
}

fn run(options: Options) -> Result<(), String> {
    let manifest = options
        .manifest_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("Cargo.toml"));
    let mut krate = read_crate(&manifest)?;
    if let Some(package) = &options.package {
        krate.android["package"] = package.clone().into();
    }
    if let Some(label) = &options.label {
        krate.android["label"] = label.clone().into();
    }
    let identity = identity(&krate)?;
    let profile = if options.debug { "debug" } else { "release" };

    let host_apk = match options
        .host_apk
        .clone()
        .or_else(|| std::env::var_os("RUSTOLONIA_ANDROID_HOST_APK").map(PathBuf::from))
    {
        // A local host (vendored, or built from a source checkout) is trusted as is.
        Some(path) => std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        None => {
            let bindings = host::read_bindings(&manifest)?;
            let (bytes, path) = host::Release::from_env(&bindings)?.fetch()?;
            eprintln!("Android host {} ({})", bindings.version, path.display());
            bytes
        }
    };

    let sdk = android_sdk();
    let mut libraries = BTreeMap::new();
    for abi in &options.abis {
        let triple = triple(abi)?;
        if !options.skip_build {
            cargo_build(&manifest, triple, options.debug, sdk.as_deref())?;
        }
        let path = krate
            .target_directory
            .join(triple)
            .join(profile)
            .join(format!("lib{}.so", identity.library));
        let library = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        libraries.insert(abi.clone(), library);
    }

    let signer = match (&options.key, &options.cert) {
        (Some(key), Some(cert)) => {
            let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
            sign::Signer::new(&read(key)?, read(cert)?).map_err(|e| e.to_string())?
        }
        _ => {
            let dir = dirs::home_dir()
                .ok_or("no home directory for the debug key; pass --key and --cert")?
                .join(".rustolonia")
                .join("android");
            sign::Signer::debug(&dir).map_err(|e| e.to_string())?
        }
    };

    let apk = package::package(&host_apk, &libraries, &identity, &signer)?;
    let out = options.out.clone().unwrap_or_else(|| {
        krate
            .target_directory
            .join("android")
            .join(format!("{}.apk", identity.package))
    });
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(&out, &apk).map_err(|e| format!("{}: {e}", out.display()))?;
    println!(
        "{} {} ({}) -> {} ({:.1} MB)",
        krate.name,
        identity.version_name,
        options.abis.join(", "),
        out.display(),
        apk.len() as f64 / 1_048_576.0
    );

    if options.install || options.launch {
        let adb = adb(sdk.as_deref());
        run_command(Command::new(&adb).arg("install").arg("-r").arg(&out))?;
        if options.launch {
            run_command(Command::new(&adb).args([
                "shell",
                "monkey",
                "-p",
                &identity.package,
                "-c",
                "android.intent.category.LAUNCHER",
                "1",
            ]))?;
            println!(
                "launched {}; logs: adb logcat -s rustolonia",
                identity.package
            );
        }
    }
    Ok(())
}

fn read_crate(manifest: &Path) -> Result<Crate, String> {
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
    let wanted =
        std::fs::canonicalize(manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let package = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| {
            p["manifest_path"]
                .as_str()
                .and_then(|path| std::fs::canonicalize(path).ok())
                .is_some_and(|path| path == wanted)
        })
        .ok_or_else(|| format!("{} is not a package manifest", manifest.display()))?;
    let str_field = |field: &str| package[field].as_str().unwrap_or_default().to_owned();
    let library = package["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|t| {
            t["kind"]
                .as_array()
                .is_some_and(|k| k.iter().any(|k| k == "cdylib"))
        })
        .and_then(|t| t["name"].as_str())
        .map(|name| name.replace('-', "_"))
        .ok_or(
            "the crate has no cdylib target; add crate-type = [\"cdylib\", \"rlib\"] under [lib]",
        )?;
    Ok(Crate {
        name: str_field("name"),
        version: str_field("version"),
        library,
        target_directory: metadata["target_directory"]
            .as_str()
            .map(PathBuf::from)
            .ok_or("cargo metadata has no target_directory")?,
        android: package["metadata"]["rustolonia"]["android"].clone(),
    })
}

fn identity(krate: &Crate) -> Result<AppIdentity, String> {
    let android = &krate.android;
    let package = android["package"].as_str().ok_or(
        "set the Android package name in Cargo.toml:\n\n  \
         [package.metadata.rustolonia.android]\n  package = \"com.example.my_app\"",
    )?;
    let version_code = match &android["version-code"] {
        serde_json::Value::Null => 1,
        value => value
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .filter(|&v| v > 0)
            .ok_or("version-code must be a positive integer")?,
    };
    Ok(AppIdentity {
        package: package.to_owned(),
        label: android["label"].as_str().unwrap_or(&krate.name).to_owned(),
        version_code,
        version_name: android["version-name"]
            .as_str()
            .unwrap_or(&krate.version)
            .to_owned(),
        library: krate.library.clone(),
    })
}

fn android_sdk() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .filter_map(|name| std::env::var_os(name).map(PathBuf::from))
        .collect();
    if cfg!(windows) {
        candidates.extend(dirs::data_local_dir().map(|d| d.join("Android").join("Sdk")));
    } else if cfg!(target_os = "macos") {
        candidates.extend(dirs::home_dir().map(|d| d.join("Library/Android/sdk")));
    } else {
        candidates.extend(dirs::home_dir().map(|d| d.join("Android/Sdk")));
    }
    candidates
        .into_iter()
        .find(|sdk| sdk.join("platform-tools").is_dir() || sdk.join("ndk").is_dir())
}

fn android_ndk(sdk: Option<&Path>) -> Result<PathBuf, String> {
    for name in ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT"] {
        if let Some(path) = std::env::var_os(name).map(PathBuf::from) {
            if path.join("toolchains").is_dir() {
                return Ok(path);
            }
        }
    }
    let version = |path: &Path| -> Vec<u64> {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .split('.')
            .filter_map(|part| part.parse().ok())
            .collect()
    };
    sdk.and_then(|sdk| std::fs::read_dir(sdk.join("ndk")).ok())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.join("toolchains").is_dir())
        .max_by_key(|path| version(path))
        .ok_or_else(|| {
            "no Android NDK: set ANDROID_NDK_HOME, or install one into the SDK \
             (sdkmanager \"ndk;28.2.13676358\")"
                .into()
        })
}

/// Cross-compiles the crate's library with the NDK's clang as linker.
fn cargo_build(
    manifest: &Path,
    triple: &str,
    debug: bool,
    sdk: Option<&Path>,
) -> Result<(), String> {
    const MIN_SDK: u32 = 24;
    let ndk = android_ndk(sdk)?;
    let host = if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else {
        "linux-x86_64"
    };
    let bin = ndk
        .join("toolchains")
        .join("llvm")
        .join("prebuilt")
        .join(host)
        .join("bin");
    let clang = bin.join(format!(
        "{triple}{MIN_SDK}-clang{}",
        if cfg!(windows) { ".cmd" } else { "" }
    ));
    let ar = bin.join(format!("llvm-ar{}", std::env::consts::EXE_SUFFIX));
    if !clang.exists() {
        return Err(format!(
            "the NDK at {} has no {}",
            ndk.display(),
            clang.display()
        ));
    }
    let env_triple = triple.to_uppercase().replace('-', "_");
    let cc_triple = triple.replace('-', "_");
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .args(["build", "--lib", "--target", triple, "--manifest-path"])
        .arg(manifest)
        .env(format!("CARGO_TARGET_{env_triple}_LINKER"), &clang)
        .env(format!("CC_{cc_triple}"), &clang)
        .env(format!("AR_{cc_triple}"), &ar);
    if !debug {
        command.arg("--release");
    }
    run_command(&mut command)
        .map_err(|e| format!("{e}\n(is the Rust target installed? rustup target add {triple})"))
}

fn adb(sdk: Option<&Path>) -> PathBuf {
    sdk.map(|sdk| {
        sdk.join("platform-tools")
            .join(format!("adb{}", std::env::consts::EXE_SUFFIX))
    })
    .filter(|adb| adb.exists())
    .unwrap_or_else(|| PathBuf::from("adb"))
}

fn run_command(command: &mut Command) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|e| format!("cannot run {:?}: {e}", command.get_program()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{:?} failed ({status})", command.get_program()))
    }
}
