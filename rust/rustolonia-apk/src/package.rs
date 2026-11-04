//! Turns the prebuilt Rustolonia host APK into an application APK.

use crate::axml::Axml;
use crate::sign::{self, Signer};
use crate::zip::{self, Entry};
use std::collections::BTreeMap;

/// The host APK's own package name. Every manifest string equal to it or
/// starting with it (permissions, provider authorities) is renamed.
pub const HOST_PACKAGE: &str = "net.devolutions.rustolonia.hostapp";
/// The meta-data entry the host reads to find the Rust library.
pub const LIBRARY_META_DATA: &str = "net.devolutions.rustolonia.library";

/// What makes an APK this application's.
#[derive(Clone, Debug)]
pub struct AppIdentity {
    pub package: String,
    pub label: String,
    pub version_code: u32,
    pub version_name: String,
    /// The Rust library name without `lib` and `.so`.
    pub library: String,
}

/// Builds the signed application APK from `host_apk` and the Rust library
/// for each ABI (`"arm64-v8a"`, `"x86_64"`). ABIs without a Rust library are
/// dropped from the APK, so a device never installs a host it cannot start.
pub fn package(
    host_apk: &[u8],
    libraries: &BTreeMap<String, Vec<u8>>,
    identity: &AppIdentity,
    signer: &Signer,
) -> Result<Vec<u8>, String> {
    validate_package(&identity.package)?;
    if libraries.is_empty() {
        return Err("no Rust library to package".into());
    }
    let entries = zip::read(host_apk).map_err(|e| e.to_string())?;
    let host_abis: Vec<String> = entries
        .iter()
        .filter_map(|e| {
            e.name
                .strip_prefix("lib/")?
                .split_once('/')
                .map(|(abi, _)| abi.to_owned())
        })
        .collect();
    for abi in libraries.keys() {
        if !host_abis.contains(abi) {
            return Err(format!("the host APK has no {abi} runtime"));
        }
    }

    let mut output = Vec::with_capacity(entries.len() + libraries.len());
    let mut saw_manifest = false;
    for entry in entries {
        if is_signature_file(&entry.name) {
            continue;
        }
        if let Some((abi, _)) = entry
            .name
            .strip_prefix("lib/")
            .and_then(|r| r.split_once('/'))
        {
            if !libraries.contains_key(abi) {
                continue;
            }
        }
        if entry.name == "AndroidManifest.xml" {
            saw_manifest = true;
            let manifest =
                rewrite_manifest(&entry.contents().map_err(|e| e.to_string())?, identity)?;
            output.push(Entry::new(
                entry.name.clone(),
                &manifest,
                entry.method == zip::STORED,
            ));
            continue;
        }
        output.push(entry);
    }
    if !saw_manifest {
        return Err("the host APK has no AndroidManifest.xml".into());
    }
    for (abi, library) in libraries {
        output.push(Entry::new(
            format!("lib/{abi}/lib{}.so", identity.library),
            library,
            false,
        ));
    }
    sign::sign_v2(&zip::write(&output), signer).map_err(|e| e.to_string())
}

pub fn rewrite_manifest(manifest: &[u8], identity: &AppIdentity) -> Result<Vec<u8>, String> {
    let mut doc = Axml::parse(manifest).map_err(|e| e.to_string())?;
    let found = doc
        .get_string("manifest", "package")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    if found != HOST_PACKAGE {
        return Err(format!(
            "this is not a Rustolonia host APK (package {found:?}, expected {HOST_PACKAGE})"
        ));
    }
    doc.rename_prefix(HOST_PACKAGE, &identity.package);
    let set = |count: Result<usize, crate::axml::AxmlError>, what: &str| match count {
        Ok(0) => Err(format!("the host manifest has no {what}")),
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    };
    set(
        doc.set_int("manifest", "versionCode", identity.version_code),
        "versionCode",
    )?;
    set(
        doc.set_string("manifest", "versionName", &identity.version_name),
        "versionName",
    )?;
    set(
        doc.set_string("application", "label", &identity.label),
        "application label",
    )?;
    set(
        doc.set_string("activity", "label", &identity.label),
        "activity label",
    )?;
    set(
        doc.set_string_where(
            "meta-data",
            "name",
            LIBRARY_META_DATA,
            "value",
            &identity.library,
        ),
        "library meta-data",
    )?;
    Ok(doc.to_bytes())
}

fn is_signature_file(name: &str) -> bool {
    let Some(file) = name.strip_prefix("META-INF/") else {
        return false;
    };
    file == "MANIFEST.MF"
        || [".SF", ".RSA", ".DSA", ".EC"]
            .iter()
            .any(|suffix| file.to_ascii_uppercase().ends_with(suffix))
}

/// An Android application ID: two or more dot-separated segments of ASCII
/// letters, digits and underscores, each starting with a letter.
pub fn validate_package(package: &str) -> Result<(), String> {
    let segments: Vec<&str> = package.split('.').collect();
    let valid = segments.len() >= 2
        && segments.iter().all(|segment| {
            segment
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
    if valid {
        Ok(())
    } else {
        Err(format!(
            "{package:?} is not a valid Android package name (for example com.example.my_app)"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_names_are_validated() {
        assert!(validate_package("com.example.app_1").is_ok());
        for bad in ["app", "com..app", "com.1app", "com.my-app", ""] {
            assert!(validate_package(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn only_jar_signature_files_are_dropped() {
        assert!(is_signature_file("META-INF/MANIFEST.MF"));
        assert!(is_signature_file("META-INF/CERT.RSA"));
        assert!(is_signature_file("META-INF/BNDLTOOL.SF"));
        assert!(!is_signature_file("META-INF/androidx.core_core.version"));
        assert!(!is_signature_file("AndroidManifest.xml"));
    }
}
