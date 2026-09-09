use crate::error::GenerationError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const BINDGEN_GENERATOR_ID: &str = "avalonia-bindgen";

#[derive(Debug, Default)]
pub struct CheckReport {
    pub mismatches: Vec<String>,
}

impl CheckReport {
    pub fn success(&self) -> bool {
        self.mismatches.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    generator: String,
    files: Vec<ManifestFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawManifest {
    generator: String,
    files: Option<Vec<Option<ManifestFile>>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    path: String,
    sha256: String,
}

pub fn manifest_file_name(generator_id: &str) -> String {
    format!(".{generator_id}.owned.json")
}

pub fn hash_content(content: &str) -> String {
    hex_encode(&Sha256::digest(content.as_bytes()))
}

pub fn is_safe_generator_id(generator_id: &str) -> bool {
    let mut chars = generator_id.chars();
    matches!(chars.next(), Some('a'..='z'))
        && generator_id.len() <= 64
        && chars.all(|character| matches!(character, 'a'..='z' | '0'..='9' | '-'))
}

pub fn is_safe_leaf_file_name(path: &str) -> bool {
    !path.contains('/')
        && !path.contains('\\')
        && !path.contains(':')
        && is_safe_relative_path(path)
        && Path::new(path).file_name().and_then(|name| name.to_str()) == Some(path)
}

pub fn is_safe_relative_path(path: &str) -> bool {
    if path.trim().is_empty() {
        return false;
    }
    let normalized = path.replace('\\', "/");
    if Path::new(&normalized).is_absolute()
        || normalized.starts_with('/')
        || normalized.contains(':')
    {
        return false;
    }
    let parts: Vec<_> = normalized.split('/').collect();
    !parts.is_empty()
        && parts
            .iter()
            .all(|part| !part.is_empty() && *part != "." && *part != "..")
}

pub fn check_outputs(
    generator_id: &str,
    files: &[(PathBuf, String)],
) -> Result<CheckReport, GenerationError> {
    let map = destination_map(generator_id, files)?;
    let mut report = CheckReport::default();
    for (path, expected) in &map {
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let leaf = path.file_name().and_then(|name| name.to_str()).unwrap();
        let others = other_owners(directory, leaf, generator_id)?;
        if !others.is_empty() {
            report.mismatches.push(format!(
                "CONFLICT: {} is owned by {}",
                path.display(),
                others.join(", ")
            ));
        }
        if !path.exists() {
            report
                .mismatches
                .push(format!("MISSING: {}", path.display()));
            continue;
        }
        let actual = fs::read_to_string(path)?.replace("\r\n", "\n");
        if actual != *expected {
            report
                .mismatches
                .push(format!("DIFFERENT: {}", path.display()));
        }
    }

    for directory in group_directories(&map).into_keys() {
        if !directory.is_dir() {
            continue;
        }
        let Some(manifest) = read_manifest(&directory, generator_id)? else {
            continue;
        };
        let expected = expected_names(&directory, &map);
        for owned in manifest.files {
            if expected.contains(&owned.path) {
                continue;
            }
            let full = directory.join(&owned.path);
            if !full.exists() {
                report
                    .mismatches
                    .push(format!("OBSOLETE: {}", full.display()));
                continue;
            }
            let on_disk = hex_encode(&Sha256::digest(fs::read(&full)?));
            report.mismatches.push(if on_disk == owned.sha256 {
                format!("OBSOLETE: {}", full.display())
            } else {
                format!("OBSOLETE-MODIFIED: {}", full.display())
            });
        }
    }
    Ok(report)
}

pub fn write_outputs(
    generator_id: &str,
    files: &[(PathBuf, String)],
) -> Result<(), GenerationError> {
    let map = destination_map(generator_id, files)?;
    let groups = group_directories(&map);
    let mut obsolete = Vec::new();
    for path in map.keys() {
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let leaf = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let others = other_owners(directory, leaf, generator_id)?;
        if !others.is_empty() {
            return Err(GenerationError::invalid(format!(
                "Refusing to overwrite '{leaf}' owned by {}.",
                others.join(", ")
            )));
        }
    }
    for directory in groups.keys() {
        let expected = expected_names(directory, &map);
        let Some(manifest) = read_manifest(directory, generator_id)? else {
            continue;
        };
        for owned in manifest.files {
            if expected.contains(&owned.path) {
                continue;
            }
            let full = directory.join(&owned.path);
            if !full.exists() {
                continue;
            }
            if !other_owners(directory, &owned.path, generator_id)?.is_empty() {
                continue;
            }
            let on_disk = hex_encode(&Sha256::digest(fs::read(&full)?));
            if on_disk != owned.sha256 {
                return Err(GenerationError::invalid(format!(
                    "Refusing to delete '{}' owned by {generator_id}: file was modified outside the generator.",
                    owned.path
                )));
            }
            obsolete.push(full);
        }
    }

    let mut staged = Vec::new();
    let stage_result: Result<(), GenerationError> = (|| {
        for (path, content) in &map {
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            fs::create_dir_all(parent)?;
            let unique = create_exclusive_temp(parent, content.as_bytes())?;
            staged.push((path.clone(), unique));
        }
        Ok(())
    })();
    if let Err(error) = stage_result {
        delete_temps(&staged);
        return Err(error);
    }

    let mut committed = Vec::new();
    for (destination, temp) in &staged {
        if let Err(error) = replace_file(temp, destination) {
            delete_temps(&staged);
            return Err(GenerationError::invalid(format!(
                "Partial write failure after updating {} file(s): {error}",
                committed.len()
            )));
        }
        committed.push(destination.clone());
    }

    for path in obsolete {
        fs::remove_file(path)?;
    }

    let mut manifest_temps = Vec::new();
    let manifest_result: Result<(), GenerationError> = (|| {
        for (directory, directory_files) in &groups {
            fs::create_dir_all(directory)?;
            let mut files: Vec<_> = directory_files
                .iter()
                .map(|(path, content)| ManifestFile {
                    path: path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default()
                        .to_string(),
                    sha256: hash_content(content),
                })
                .collect();
            files.sort_by(|left, right| {
                left.path
                    .to_ascii_lowercase()
                    .cmp(&right.path.to_ascii_lowercase())
            });
            let manifest = Manifest {
                generator: generator_id.to_string(),
                files,
            };
            let mut json = serde_json::to_string_pretty(&manifest)?;
            json.push('\n');
            let json = json.replace("\r\n", "\n");
            let destination = directory.join(manifest_file_name(generator_id));
            let temp = create_exclusive_temp(directory, json.as_bytes())?;
            manifest_temps.push((destination, temp));
        }
        Ok(())
    })();
    if let Err(error) = manifest_result {
        delete_temps(&manifest_temps);
        return Err(error);
    }
    for (destination, temp) in &manifest_temps {
        if let Err(error) = replace_file(temp, destination) {
            delete_temps(&manifest_temps);
            return Err(error);
        }
    }
    Ok(())
}

fn destination_map(
    generator_id: &str,
    files: &[(PathBuf, String)],
) -> Result<BTreeMap<PathBuf, String>, GenerationError> {
    if !is_safe_generator_id(generator_id) {
        return Err(GenerationError::invalid(format!(
            "Unsafe generator id '{generator_id}'."
        )));
    }
    if files.is_empty() {
        return Err(GenerationError::invalid("Generation produced no outputs."));
    }
    let mut map = BTreeMap::new();
    let mut physical = HashMap::new();
    for (path, content) in files {
        let full = normalize_path(path)?;
        let name = full
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| GenerationError::invalid("Generated output path must not be empty."))?;
        if !is_safe_leaf_file_name(name) {
            return Err(GenerationError::invalid(format!(
                "Generated output '{}' is not a safe file name.",
                full.display()
            )));
        }
        if name.eq_ignore_ascii_case(&manifest_file_name(generator_id)) {
            return Err(GenerationError::invalid(format!(
                "Generator '{generator_id}' cannot emit its ownership manifest as content."
            )));
        }
        let key = physical_key(&full);
        if let Some(existing) = physical.insert(key, full.clone()) {
            return Err(GenerationError::invalid(format!(
                "Duplicate generated output '{}' collides with '{}'.",
                full.display(),
                existing.display()
            )));
        }
        map.insert(full, content.clone());
    }
    Ok(map)
}

fn group_directories(
    files: &BTreeMap<PathBuf, String>,
) -> HashMap<PathBuf, Vec<(PathBuf, String)>> {
    let mut groups = HashMap::new();
    for (path, content) in files {
        let directory = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        groups
            .entry(directory)
            .or_insert_with(Vec::new)
            .push((path.clone(), content.clone()));
    }
    groups
}

fn expected_names(directory: &Path, files: &BTreeMap<PathBuf, String>) -> HashSet<String> {
    files
        .keys()
        .filter(|path| path.parent() == Some(directory))
        .filter_map(|path| path.file_name()?.to_str().map(str::to_string))
        .collect()
}

fn read_manifest(
    directory: &Path,
    generator_id: &str,
) -> Result<Option<Manifest>, GenerationError> {
    let path = directory.join(manifest_file_name(generator_id));
    if !path.exists() {
        return Ok(None);
    }
    let parsed: RawManifest = serde_json::from_str(&fs::read_to_string(path)?)?;
    if !is_safe_generator_id(&parsed.generator) {
        return Err(GenerationError::invalid(format!(
            "Ownership manifest in '{}' has an unsafe generator id '{}'.",
            directory.display(),
            parsed.generator
        )));
    }
    if parsed.generator != generator_id {
        return Err(GenerationError::invalid(format!(
            "Ownership manifest belongs to '{}', not '{generator_id}'.",
            parsed.generator
        )));
    }
    let Some(raw_files) = parsed.files else {
        return Err(GenerationError::invalid(format!(
            "Ownership manifest in '{}' has a null files list.",
            directory.display()
        )));
    };
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for (index, owned) in raw_files.into_iter().enumerate() {
        let Some(owned) = owned else {
            return Err(GenerationError::invalid(format!(
                "Ownership manifest in '{}' contains a null files[{index}] element.",
                directory.display()
            )));
        };
        validate_manifest_entry(directory, &owned)?;
        if !seen.insert(owned.path.to_ascii_lowercase()) {
            return Err(GenerationError::invalid(format!(
                "Ownership manifest in '{}' contains a duplicate file name '{}'.",
                directory.display(),
                owned.path
            )));
        }
        files.push(owned);
    }
    Ok(Some(Manifest {
        generator: parsed.generator,
        files,
    }))
}

fn validate_manifest_entry(directory: &Path, owned: &ManifestFile) -> Result<(), GenerationError> {
    if !is_safe_leaf_file_name(&owned.path) {
        return Err(GenerationError::invalid(format!(
            "Ownership manifest in '{}' contains an unsafe path '{}'.",
            directory.display(),
            owned.path
        )));
    }
    if !is_sha256(&owned.sha256) {
        return Err(GenerationError::invalid(format!(
            "Ownership manifest in '{}' contains an invalid hash for '{}'.",
            directory.display(),
            owned.path
        )));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|character| matches!(character, b'0'..=b'9' | b'a'..=b'f'))
}

fn other_owners(
    directory: &Path,
    leaf: &str,
    except_generator: &str,
) -> Result<Vec<String>, GenerationError> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut owners = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(id) = name
            .strip_prefix('.')
            .and_then(|name| name.strip_suffix(".owned.json"))
        else {
            continue;
        };
        if !is_safe_generator_id(id) || id == except_generator {
            continue;
        }
        let Some(manifest) = read_manifest(directory, id)? else {
            continue;
        };
        if manifest
            .files
            .iter()
            .any(|file| file.path.eq_ignore_ascii_case(leaf))
        {
            owners.push(id.to_string());
        }
    }
    Ok(owners)
}

fn normalize_path(path: &Path) -> Result<PathBuf, GenerationError> {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in full.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn physical_key(path: &Path) -> String {
    let value = path.to_string_lossy();
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value.into_owned()
    }
}

fn create_exclusive_temp(directory: &Path, bytes: &[u8]) -> Result<PathBuf, GenerationError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    for attempt in 0..32 {
        let path = directory.join(format!(
            ".avalonia-owned-tmp-{}-{}-{attempt}",
            std::process::id(),
            stamp
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let result = file.write_all(bytes).and_then(|()| file.sync_all());
                drop(file);
                if let Err(error) = result {
                    if let Err(cleanup_error) = fs::remove_file(&path) {
                        return Err(GenerationError::invalid(format!(
                            "Failed to stage '{}': {error}; cleanup also failed: {cleanup_error}",
                            path.display()
                        )));
                    }
                    return Err(error.into());
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(GenerationError::invalid(format!(
        "Could not allocate an exclusive staging file in '{}'.",
        directory.display()
    )))
}

fn replace_file(temp: &Path, destination: &Path) -> Result<(), GenerationError> {
    fs::rename(temp, destination)?;
    Ok(())
}

fn delete_temps(staged: &[(PathBuf, PathBuf)]) {
    for (_, temp) in staged {
        let _ = fs::remove_file(temp);
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rustolonia-bindgen-owned-{stamp}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn check_mode_does_not_create_missing_directories() {
        let root = scratch();
        let missing_root = root.join("absent");
        let path = missing_root.join("generated.rs");
        let report = check_outputs(
            BINDGEN_GENERATOR_ID,
            &[(path.clone(), "pub struct A;\n".into())],
        )
        .unwrap();
        assert!(!report.success());
        assert!(report
            .mismatches
            .iter()
            .any(|item| item.contains("MISSING:")));
        assert!(!missing_root.exists());
        cleanup(&root);
    }

    #[test]
    fn check_reports_another_generators_claim_without_mutating_it() {
        let root = scratch();
        let output = root.join("shared.rs");
        write_outputs("first-generator", &[(output.clone(), "same\n".into())]).unwrap();
        let report =
            check_outputs("second-generator", &[(output.clone(), "same\n".into())]).unwrap();
        assert!(!report.success());
        assert!(report
            .mismatches
            .iter()
            .any(|message| message.contains("CONFLICT")));
        assert_eq!(fs::read_to_string(output).unwrap(), "same\n");
        assert!(!root.join(".second-generator.owned.json").exists());
        cleanup(&root);
    }

    #[test]
    fn replacement_preserves_existing_file_when_source_is_missing() {
        let root = scratch();
        let destination = root.join("generated.rs");
        fs::write(&destination, "previous\n").unwrap();
        assert!(replace_file(&root.join("missing-temp"), &destination).is_err());
        assert_eq!(fs::read_to_string(destination).unwrap(), "previous\n");
        cleanup(&root);
    }

    #[test]
    fn write_renders_all_outputs_before_replacing_and_keeps_unrelated_files() {
        let root = scratch();
        let sys = root.join("sys").join("generated.rs");
        let safe = root.join("safe").join("generated.rs");
        let unrelated = root.join("sys").join("hand.rs");
        fs::create_dir_all(sys.parent().unwrap()).unwrap();
        fs::write(&unrelated, "leave me\n").unwrap();
        write_outputs(
            BINDGEN_GENERATOR_ID,
            &[
                (sys.clone(), "sys-one\n".into()),
                (safe.clone(), "safe-one\n".into()),
            ],
        )
        .unwrap();
        write_outputs(
            BINDGEN_GENERATOR_ID,
            &[
                (sys.clone(), "sys-two\n".into()),
                (safe.clone(), "safe-two\n".into()),
            ],
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&sys).unwrap(), "sys-two\n");
        assert_eq!(fs::read_to_string(&safe).unwrap(), "safe-two\n");
        assert_eq!(fs::read_to_string(&unrelated).unwrap(), "leave me\n");
        cleanup(&root);
    }

    #[test]
    fn write_prunes_only_owned_obsolete_files() {
        let root = scratch();
        let kept = root.join("generated.rs");
        let stale = root.join("old.rs");
        write_outputs(
            BINDGEN_GENERATOR_ID,
            &[
                (kept.clone(), "keep\n".into()),
                (stale.clone(), "stale\n".into()),
            ],
        )
        .unwrap();
        write_outputs(
            BINDGEN_GENERATOR_ID,
            &[(kept.clone(), "keep-updated\n".into())],
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&kept).unwrap(), "keep-updated\n");
        assert!(!stale.exists());
        cleanup(&root);
    }

    #[test]
    fn write_refuses_modified_obsolete_file_and_leaves_bytes() {
        let root = scratch();
        let kept = root.join("generated.rs");
        let stale = root.join("old.rs");
        write_outputs(
            BINDGEN_GENERATOR_ID,
            &[
                (kept.clone(), "keep\n".into()),
                (stale.clone(), "stale\n".into()),
            ],
        )
        .unwrap();
        fs::write(&stale, "edited\n").unwrap();
        let kept_before = fs::read(&kept).unwrap();
        let stale_before = fs::read(&stale).unwrap();
        let error = write_outputs(
            BINDGEN_GENERATOR_ID,
            &[(kept.clone(), "keep-updated\n".into())],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("modified outside the generator"));
        assert_eq!(fs::read(&kept).unwrap(), kept_before);
        assert_eq!(fs::read(&stale).unwrap(), stale_before);
        cleanup(&root);
    }

    #[cfg(windows)]
    #[test]
    fn case_alias_destinations_are_rejected_as_collisions() {
        let root = scratch();
        let error = write_outputs(
            BINDGEN_GENERATOR_ID,
            &[
                (root.join("generated.rs"), "one\n".into()),
                (root.join("GENERATED.rs"), "two\n".into()),
            ],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Duplicate generated output"), "{error}");
        cleanup(&root);
    }

    #[test]
    fn write_does_not_delete_a_file_still_owned_by_another_generator() {
        let root = scratch();
        let shared = root.join("shared.rs");
        write_outputs(BINDGEN_GENERATOR_ID, &[(shared.clone(), "same\n".into())]).unwrap();
        let extra = root.join("extra.rs");
        let hash = hash_content("same\n");
        fs::write(
            root.join(".other-gen.owned.json"),
            format!(
                "{{\n  \"generator\": \"other-gen\",\n  \"files\": [\n    {{ \"path\": \"shared.rs\", \"sha256\": \"{hash}\" }}\n  ]\n}}\n"
            ),
        )
        .unwrap();
        write_outputs("other-gen", &[(extra.clone(), "extra\n".into())]).unwrap();
        assert_eq!(fs::read_to_string(&shared).unwrap(), "same\n");
        cleanup(&root);
    }

    #[test]
    fn malformed_ownership_paths_are_rejected() {
        let root = scratch();
        let owned = root.join("generated.rs");
        fs::write(&owned, "original\n").unwrap();
        fs::write(
            root.join(manifest_file_name(BINDGEN_GENERATOR_ID)),
            r#"{
  "generator": "avalonia-bindgen",
  "files": [{ "path": "../escape.rs", "sha256": "00" }]
}
"#,
        )
        .unwrap();
        let before = fs::read(&owned).unwrap();
        let error = write_outputs(BINDGEN_GENERATOR_ID, &[(owned.clone(), "updated\n".into())])
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsafe path"));
        assert_eq!(fs::read(&owned).unwrap(), before);
        cleanup(&root);
    }

    #[test]
    fn check_reports_changed_missing_and_obsolete() {
        let root = scratch();
        let changed = root.join("changed.rs");
        let missing = root.join("missing.rs");
        let obsolete = root.join("obsolete.rs");
        write_outputs(
            BINDGEN_GENERATOR_ID,
            &[
                (changed.clone(), "old\n".into()),
                (obsolete.clone(), "stale\n".into()),
            ],
        )
        .unwrap();
        let report = check_outputs(
            BINDGEN_GENERATOR_ID,
            &[
                (changed.clone(), "new\n".into()),
                (missing.clone(), "fresh\n".into()),
            ],
        )
        .unwrap();
        assert!(report
            .mismatches
            .iter()
            .any(|item| item.starts_with("DIFFERENT:")));
        assert!(report
            .mismatches
            .iter()
            .any(|item| item.starts_with("MISSING:")));
        assert!(report
            .mismatches
            .iter()
            .any(|item| item.starts_with("OBSOLETE:")));
        assert!(!missing.exists());
        cleanup(&root);
    }
}
