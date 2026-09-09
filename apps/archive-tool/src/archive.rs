//! Zip/tar listing and extraction. Rust owns archive IO; the UI only presents it.

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tar::{Builder, EntryType, Header};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    Tar,
    TarGz,
}

impl ArchiveKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Zip => "ZIP",
            Self::Tar => "TAR",
            Self::TarGz => "TAR.GZ",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub compressed_size: Option<u64>,
    pub crc: Option<u32>,
    pub modified: String,
}

impl ArchiveEntry {
    pub fn kind_label(&self) -> &'static str {
        if self.is_dir {
            "Folder"
        } else {
            "File"
        }
    }

    pub fn size_label(&self) -> String {
        if self.is_dir {
            "—".to_string()
        } else {
            format_size(self.size)
        }
    }

    pub fn packed_label(&self) -> String {
        if self.is_dir {
            "—".to_string()
        } else {
            self.compressed_size
                .map(format_size)
                .unwrap_or_else(|| "—".to_string())
        }
    }

    pub fn crc_label(&self) -> String {
        self.crc
            .map(|value| format!("{value:08X}"))
            .unwrap_or_else(|| "—".to_string())
    }

    pub fn ratio_label(&self) -> String {
        if self.is_dir {
            return "—".to_string();
        }
        match (self.size, self.compressed_size) {
            (0, _) => "—".to_string(),
            (size, Some(packed)) => format!("{}%", packed.saturating_mul(100) / size),
            _ => "—".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractReport {
    pub extracted: usize,
    pub destination: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestReport {
    pub files: usize,
    pub failed: usize,
}

impl TestReport {
    pub fn summary(&self) -> String {
        if self.failed == 0 {
            format!("Test OK · {} file{}", self.files, if self.files == 1 { "" } else { "s" })
        } else {
            format!("Test failed · {} of {} file(s) damaged", self.failed, self.files)
        }
    }
}

pub fn detect_kind(path: &Path) -> Result<ArchiveKind, String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Ok(ArchiveKind::TarGz);
    }
    if name.ends_with(".tar") {
        return Ok(ArchiveKind::Tar);
    }
    if name.ends_with(".zip") {
        return Ok(ArchiveKind::Zip);
    }
    sniff_kind(path)
}

pub fn list_entries(path: &Path) -> Result<(ArchiveKind, Vec<ArchiveEntry>), String> {
    let kind = detect_kind(path)?;
    let mut entries = match kind {
        ArchiveKind::Zip => list_zip(path)?,
        ArchiveKind::Tar | ArchiveKind::TarGz => list_tar(path, kind)?,
    };
    synthesize_directories(&mut entries);
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((kind, entries))
}

pub fn extract_entries(
    path: &Path,
    destination: &Path,
    selected: &[String],
) -> Result<ExtractReport, String> {
    let (kind, entries) = list_entries(path)?;
    let names = expand_selection(&entries, selected);
    if names.is_empty() {
        return Err("Nothing to extract.".to_string());
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    match kind {
        ArchiveKind::Zip => extract_zip(path, destination, &names)?,
        ArchiveKind::Tar | ArchiveKind::TarGz => extract_tar(path, kind, destination, &names)?,
    }
    Ok(ExtractReport {
        extracted: names.len(),
        destination: destination.to_path_buf(),
    })
}

pub fn expand_selection(entries: &[ArchiveEntry], selected: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    for selected_path in selected {
        let normalized = normalize_archive_path(selected_path);
        if normalized.is_empty() {
            continue;
        }
        let prefix = format!("{normalized}/");
        let is_dir = entries
            .iter()
            .any(|entry| entry.path == normalized && entry.is_dir);
        if is_dir {
            for entry in entries {
                if !entry.is_dir && (entry.path == normalized || entry.path.starts_with(&prefix)) {
                    names.push(entry.path.clone());
                }
            }
        } else if entries.iter().any(|entry| entry.path == normalized && !entry.is_dir)
        {
            names.push(normalized);
        }
    }
    names.sort();
    names.dedup();
    names
}

pub fn parent_dir(dir: &str) -> String {
    match dir.rsplit_once('/') {
        Some((parent, _)) => parent.to_string(),
        None => String::new(),
    }
}

pub fn children_of<'a>(entries: &'a [ArchiveEntry], dir: &str) -> Vec<&'a ArchiveEntry> {
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    entries
        .iter()
        .filter(|entry| {
            let rest = if prefix.is_empty() {
                entry.path.as_str()
            } else if let Some(rest) = entry.path.strip_prefix(&prefix) {
                rest
            } else {
                return false;
            };
            !rest.is_empty() && !rest.contains('/')
        })
        .collect()
}

pub fn top_level_names(entries: &[ArchiveEntry]) -> Vec<String> {
    let mut names = BTreeSet::new();
    for entry in entries {
        let name = entry.path.split('/').next().unwrap_or(&entry.path);
        if !name.is_empty() {
            names.insert(name.to_string());
        }
    }
    names.into_iter().collect()
}

pub fn smart_extract_destination(
    dest: &Path,
    archive_stem: &str,
    entries: &[ArchiveEntry],
) -> PathBuf {
    if top_level_names(entries).len() <= 1 {
        dest.to_path_buf()
    } else {
        dest.join(archive_stem)
    }
}

pub fn archive_totals(entries: &[ArchiveEntry]) -> (usize, u64, Option<u64>) {
    let files: Vec<_> = entries.iter().filter(|entry| !entry.is_dir).collect();
    let size = files.iter().map(|entry| entry.size).sum();
    let packed = files
        .iter()
        .map(|entry| entry.compressed_size)
        .collect::<Option<Vec<_>>>()
        .map(|values| values.into_iter().sum());
    (files.len(), size, packed)
}

pub fn is_nested_archive(path: &str) -> bool {
    let name = path.to_ascii_lowercase();
    name.ends_with(".zip")
        || name.ends_with(".tar")
        || name.ends_with(".tar.gz")
        || name.ends_with(".tgz")
}

pub fn test_archive(path: &Path) -> Result<TestReport, String> {
    let kind = detect_kind(path)?;
    match kind {
        ArchiveKind::Zip => test_zip(path),
        ArchiveKind::Tar | ArchiveKind::TarGz => test_tar(path, kind),
    }
}

pub fn create_archive_from_folder(
    folder: &Path,
    destination: &Path,
    kind: ArchiveKind,
) -> Result<(), String> {
    let files = collect_folder_files(folder)?;
    if files.is_empty() {
        return Err("The folder is empty.".to_string());
    }
    let payload: Vec<(String, Vec<u8>)> = files
        .into_iter()
        .map(|(name, path)| {
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            Ok((name, bytes))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let refs: Vec<(&str, &[u8])> = payload
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    match kind {
        ArchiveKind::Zip => write_zip(destination, &refs),
        ArchiveKind::Tar => write_tar(destination, false, &refs),
        ArchiveKind::TarGz => write_tar(destination, true, &refs),
    }
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn safe_output_path(destination: &Path, entry_path: &str) -> Result<PathBuf, String> {
    let relative = relative_entry_path(entry_path)?;
    Ok(destination.join(relative))
}

fn relative_entry_path(entry_path: &str) -> Result<PathBuf, String> {
    let raw = entry_path.replace('\\', "/");
    if raw.starts_with('/') || has_windows_drive_prefix(&raw) {
        return Err(format!("unsafe archive path: {entry_path}"));
    }
    let normalized = normalize_archive_path(entry_path);
    if normalized.is_empty() {
        return Err("archive entry path is empty".to_string());
    }
    let mut output = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(name) => output.push(name),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(format!("unsafe archive path: {entry_path}"));
            }
        }
    }
    if output.as_os_str().is_empty() {
        return Err(format!("unsafe archive path: {entry_path}"));
    }
    Ok(output)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let mut chars = path.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic()
    )
}

fn normalize_archive_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_matches('/')
        .trim()
        .to_string()
}

fn entry_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn sniff_kind(path: &Path) -> Result<ArchiveKind, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut header = [0_u8; 265];
    let read = file.read(&mut header).map_err(|error| error.to_string())?;
    if read >= 2 && header[0] == 0x50 && header[1] == 0x4B {
        return Ok(ArchiveKind::Zip);
    }
    if read >= 2 && header[0] == 0x1F && header[1] == 0x8B {
        return Ok(ArchiveKind::TarGz);
    }
    if read >= 262 && &header[257..262] == b"ustar" {
        return Ok(ArchiveKind::Tar);
    }
    Err(format!(
        "Unsupported archive: {}",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file")
    ))
}

fn list_zip(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(|error| error.to_string())?;
        if file.is_symlink() {
            continue;
        }
        let raw_name = file.name();
        let is_dir = file.is_dir() || raw_name.ends_with('/');
        let normalized = normalize_archive_path(raw_name);
        if normalized.is_empty() {
            continue;
        }
        relative_entry_path(&normalized)?;
        entries.push(ArchiveEntry {
            name: entry_name(&normalized),
            path: normalized,
            is_dir,
            size: file.size(),
            compressed_size: Some(file.compressed_size()),
            crc: if is_dir { None } else { Some(file.crc32()) },
            modified: file
                .last_modified()
                .map(format_zip_datetime)
                .unwrap_or_default(),
        });
    }
    Ok(entries)
}

fn list_tar(path: &Path, kind: ArchiveKind) -> Result<Vec<ArchiveEntry>, String> {
    let mut archive = open_tar(path, kind)?;
    let mut entries = Vec::new();
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let header = entry.header();
        let entry_type = header.entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .into_owned();
        let normalized = normalize_archive_path(&path);
        if normalized.is_empty() {
            continue;
        }
        relative_entry_path(&normalized)?;
        let modified = header
            .mtime()
            .ok()
            .map(format_unix_mtime)
            .unwrap_or_default();
        entries.push(ArchiveEntry {
            name: entry_name(&normalized),
            is_dir: entry_type.is_dir() || normalized.ends_with('/'),
            path: normalized,
            size: header.size().unwrap_or(0),
            compressed_size: None,
            crc: None,
            modified,
        });
    }
    Ok(entries)
}

fn extract_zip(path: &Path, destination: &Path, names: &[String]) -> Result<(), String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
    for name in names {
        let mut file = archive.by_name(name).map_err(|error| error.to_string())?;
        if file.is_symlink() || file.is_dir() {
            continue;
        }
        let output = safe_output_path(destination, file.name())?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut created = File::create(&output).map_err(|error| error.to_string())?;
        io::copy(&mut file, &mut created).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn extract_tar(
    path: &Path,
    kind: ArchiveKind,
    destination: &Path,
    names: &[String],
) -> Result<(), String> {
    let wanted: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let mut archive = open_tar(path, kind)?;
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        let header = entry.header().clone();
        if !header.entry_type().is_file() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .into_owned();
        let normalized = normalize_archive_path(&path);
        if !wanted.contains(normalized.as_str()) {
            continue;
        }
        let output = safe_output_path(destination, &normalized)?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut created = File::create(&output).map_err(|error| error.to_string())?;
        io::copy(&mut entry, &mut created).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn open_tar(path: &Path, kind: ArchiveKind) -> Result<tar::Archive<Box<dyn Read>>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let reader: Box<dyn Read> = match kind {
        ArchiveKind::Tar => Box::new(file),
        ArchiveKind::TarGz => Box::new(GzDecoder::new(file)),
        ArchiveKind::Zip => return Err("not a tar archive".to_string()),
    };
    Ok(tar::Archive::new(reader))
}

fn synthesize_directories(entries: &mut Vec<ArchiveEntry>) {
    let mut directories = BTreeSet::new();
    for entry in entries.iter() {
        let mut parent = Path::new(&entry.path);
        while let Some(next) = parent.parent() {
            if next.as_os_str().is_empty() {
                break;
            }
            directories.insert(next.to_string_lossy().replace('\\', "/"));
            parent = next;
        }
    }
    for path in directories {
        if entries.iter().any(|entry| entry.path == path) {
            continue;
        }
        entries.push(ArchiveEntry {
            name: entry_name(&path),
            path,
            is_dir: true,
            size: 0,
            compressed_size: None,
            crc: None,
            modified: String::new(),
        });
    }
}

fn test_zip(path: &Path) -> Result<TestReport, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut files = 0;
    let mut failed = 0;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        if file.is_dir() || file.is_symlink() {
            continue;
        }
        files += 1;
        if io::copy(&mut file, &mut io::sink()).is_err() {
            failed += 1;
        }
    }
    Ok(TestReport { files, failed })
}

fn test_tar(path: &Path, kind: ArchiveKind) -> Result<TestReport, String> {
    let mut archive = open_tar(path, kind)?;
    let mut files = 0;
    let mut failed = 0;
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        if !entry.header().entry_type().is_file() {
            continue;
        }
        files += 1;
        if io::copy(&mut entry, &mut io::sink()).is_err() {
            failed += 1;
        }
    }
    Ok(TestReport { files, failed })
}

fn collect_folder_files(folder: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut files = Vec::new();
    collect_folder_files_inner(folder, folder, &mut files)?;
    Ok(files)
}

fn collect_folder_files_inner(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    let entries = fs::read_dir(current).map_err(|error| error.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            collect_folder_files_inner(root, &path, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "file is outside the source folder".to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        relative_entry_path(&relative)?;
        files.push((relative, path));
    }
    Ok(())
}

fn write_zip(path: &Path, files: &[(&str, &[u8])]) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in files {
        zip.start_file(*name, options)
            .map_err(|error| error.to_string())?;
        zip.write_all(bytes).map_err(|error| error.to_string())?;
    }
    zip.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn write_tar(path: &Path, gzip: bool, files: &[(&str, &[u8])]) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    if gzip {
        append_tar(GzEncoder::new(file, Compression::default()), files)
    } else {
        append_tar(file, files)
    }
}

fn append_tar<W: Write>(writer: W, files: &[(&str, &[u8])]) -> Result<(), String> {
    let mut builder = Builder::new(writer);
    for (name, bytes) in files {
        let mut header = Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(EntryType::Regular);
        header.set_mtime(1_700_000_000);
        header.set_cksum();
        builder
            .append_data(&mut header, name, *bytes)
            .map_err(|error| error.to_string())?;
    }
    builder.finish().map_err(|error| error.to_string())
}

fn format_zip_datetime(datetime: zip::DateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        datetime.year(),
        datetime.month(),
        datetime.day(),
        datetime.hour(),
        datetime.minute()
    )
}

fn format_unix_mtime(mtime: u64) -> String {
    let Some(time) = UNIX_EPOCH.checked_add(std::time::Duration::from_secs(mtime)) else {
        return String::new();
    };
    format_system_time(time)
}

fn format_system_time(time: SystemTime) -> String {
    let Ok(duration) = time.duration_since(UNIX_EPOCH) else {
        return String::new();
    };
    let total = duration.as_secs();
    let seconds = total % 60;
    let minutes = (total / 60) % 60;
    let hours = (total / 3600) % 24;
    let days = total / 86400;
    // 1970-01-01 plus whole days; good enough for archive listing tests.
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02}")
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "rustolonia-archive-{}-{}-{}",
            name,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        );
        env::temp_dir().join(unique)
    }

    fn sample_files() -> [(&'static str, &'static [u8]); 3] {
        [
            ("readme.txt", b"Rustolonia Archive Explorer\n"),
            ("docs/notes.txt", b"Sample section 2\n"),
            ("data/hello.txt", b"Hello from a sample zip.\n"),
        ]
    }

    #[test]
    fn lists_zip_entries_and_synthesizes_folders() {
        let path = temp_path("sample.zip");
        write_zip(&path, &sample_files()).expect("write zip");
        let (kind, entries) = list_entries(&path).expect("list zip");
        assert_eq!(kind, ArchiveKind::Zip);
        let paths: Vec<_> = entries.iter().map(|entry| entry.path.as_str()).collect();
        assert!(paths.contains(&"readme.txt"));
        assert!(paths.contains(&"docs"));
        assert!(paths.contains(&"docs/notes.txt"));
        assert!(paths.contains(&"data/hello.txt"));
        let notes = entries
            .iter()
            .find(|entry| entry.path == "docs/notes.txt")
            .expect("notes");
        assert!(!notes.is_dir);
        assert!(notes.size > 0);
        assert!(notes.compressed_size.is_some());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn lists_and_extracts_tar_gz() {
        let path = temp_path("sample.tar.gz");
        write_tar(&path, true, &sample_files()).expect("write tar.gz");
        let (kind, entries) = list_entries(&path).expect("list tar.gz");
        assert_eq!(kind, ArchiveKind::TarGz);
        assert!(entries.iter().any(|entry| entry.path == "readme.txt"));

        let dest = temp_path("out");
        let report = extract_entries(&path, &dest, &["docs".to_string()]).expect("extract");
        assert_eq!(report.extracted, 1);
        let notes = fs::read_to_string(dest.join("docs").join("notes.txt")).expect("notes");
        assert!(notes.contains("Sample section 2"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn extract_selected_files_from_zip() {
        let path = temp_path("extract.zip");
        write_zip(&path, &sample_files()).expect("write zip");
        let dest = temp_path("extract-out");
        let report =
            extract_entries(&path, &dest, &["readme.txt".to_string()]).expect("extract readme");
        assert_eq!(report.extracted, 1);
        let body = fs::read_to_string(dest.join("readme.txt")).expect("readme");
        assert!(body.contains("Archive Explorer"));
        assert!(!dest.join("data").join("hello.txt").exists());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn rejects_zip_slip_parent_paths() {
        let path = temp_path("slip.zip");
        write_zip(&path, &[("../evil.txt", b"nope\n")]).expect("write slip zip");
        let listed = list_entries(&path);
        assert!(listed.is_err(), "listing should reject parent paths");
        let dest = temp_path("slip-out");
        fs::create_dir_all(&dest).expect("dest");
        let extracted = extract_entries(&path, &dest, &["../evil.txt".to_string()]);
        assert!(extracted.is_err());
        assert!(!dest.join("evil.txt").exists());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn rejects_absolute_entry_paths() {
        assert!(relative_entry_path("/tmp/evil.txt").is_err());
        assert!(relative_entry_path("C:/Windows/evil.txt").is_err());
        assert!(relative_entry_path("\\\\server\\share\\evil.txt").is_err());
    }

    #[test]
    fn expand_selection_includes_folder_children() {
        let entries = vec![
            ArchiveEntry {
                path: "docs".to_string(),
                name: "docs".to_string(),
                is_dir: true,
                size: 0,
                compressed_size: None,
                crc: None,
                modified: String::new(),
            },
            ArchiveEntry {
                path: "docs/notes.txt".to_string(),
                name: "notes.txt".to_string(),
                is_dir: false,
                size: 4,
                compressed_size: Some(4),
                crc: Some(1),
                modified: String::new(),
            },
            ArchiveEntry {
                path: "readme.txt".to_string(),
                name: "readme.txt".to_string(),
                is_dir: false,
                size: 4,
                compressed_size: Some(4),
                crc: Some(1),
                modified: String::new(),
            },
        ];
        let names = expand_selection(&entries, &["docs".to_string()]);
        assert_eq!(names, vec!["docs/notes.txt".to_string()]);
    }

    #[test]
    fn children_of_lists_one_directory_level() {
        let path = temp_path("children.zip");
        write_zip(&path, &sample_files()).expect("write zip");
        let (_, entries) = list_entries(&path).expect("list");
        let root: Vec<_> = children_of(&entries, "")
            .into_iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert!(root.contains(&"readme.txt"));
        assert!(root.contains(&"docs"));
        assert!(!root.contains(&"docs/notes.txt"));
        let docs: Vec<_> = children_of(&entries, "docs")
            .into_iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(docs, vec!["docs/notes.txt"]);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn smart_extract_uses_subfolder_when_multiple_roots() {
        let entries = vec![
            ArchiveEntry {
                path: "a.txt".to_string(),
                name: "a.txt".to_string(),
                is_dir: false,
                size: 1,
                compressed_size: Some(1),
                crc: None,
                modified: String::new(),
            },
            ArchiveEntry {
                path: "b.txt".to_string(),
                name: "b.txt".to_string(),
                is_dir: false,
                size: 1,
                compressed_size: Some(1),
                crc: None,
                modified: String::new(),
            },
        ];
        let dest = PathBuf::from("out");
        assert_eq!(
            smart_extract_destination(&dest, "bundle", &entries),
            PathBuf::from("out").join("bundle")
        );
    }

    #[test]
    fn test_and_create_round_trip() {
        let folder = temp_path("src-folder");
        fs::create_dir_all(folder.join("docs")).expect("dir");
        fs::write(folder.join("readme.txt"), b"hello\n").expect("write");
        fs::write(folder.join("docs").join("notes.txt"), b"notes\n").expect("write");
        let zip_path = temp_path("created.zip");
        create_archive_from_folder(&folder, &zip_path, ArchiveKind::Zip).expect("create");
        let report = test_archive(&zip_path).expect("test");
        assert_eq!(report.failed, 0);
        assert_eq!(report.files, 2);
        let (_, entries) = list_entries(&zip_path).expect("list");
        assert!(entries.iter().any(|entry| entry.path == "docs/notes.txt"));
        let _ = fs::remove_file(&zip_path);
        let _ = fs::remove_dir_all(&folder);
    }
}
