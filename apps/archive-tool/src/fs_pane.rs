//! Filesystem listing for dual-pane browsing.

use crate::archive::{format_size, ArchiveEntry};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

#[allow(dead_code)]
pub fn documents_dir() -> PathBuf {
    let home = home_dir();
    let documents = home.join("Documents");
    if documents.is_dir() {
        documents
    } else {
        home
    }
}

#[allow(dead_code)]
pub fn desktop_dir() -> PathBuf {
    let home = home_dir();
    let desktop = home.join("Desktop");
    if desktop.is_dir() {
        desktop
    } else {
        home
    }
}

pub fn downloads_dir() -> PathBuf {
    let home = home_dir();
    let downloads = home.join("Downloads");
    if downloads.is_dir() {
        downloads
    } else {
        home
    }
}

pub fn list_computer() -> Vec<ArchiveEntry> {
    #[cfg(windows)]
    {
        (b'A'..=b'Z')
            .filter_map(|letter| {
                let root = format!("{}:\\", letter as char);
                let path = PathBuf::from(&root);
                if !path.exists() {
                    return None;
                }
                Some(folder_entry(&path, &format!("{}:", letter as char)))
            })
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![folder_entry(Path::new("/"), "/")]
    }
}

pub fn list_directory(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let mut entries = Vec::new();
    let reader = fs::read_dir(path).map_err(|error| error.to_string())?;
    for entry in reader {
        let entry = entry.map_err(|error| error.to_string())?;
        let child = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if meta.is_symlink() {
            continue;
        }
        let is_dir = meta.is_dir();
        let modified = meta.modified().ok().map(format_system_time).unwrap_or_default();
        entries.push(ArchiveEntry {
            name,
            path: child.to_string_lossy().into_owned(),
            is_dir,
            size: if is_dir { 0 } else { meta.len() },
            compressed_size: None,
            crc: None,
            modified,
        });
    }
    Ok(entries)
}

pub fn parent_folder(path: &Path) -> Option<PathBuf> {
    path.parent().filter(|parent| !parent.as_os_str().is_empty()).map(Path::to_path_buf)
}

#[allow(dead_code)]
pub fn size_label(entry: &ArchiveEntry) -> String {
    if entry.is_dir {
        String::new()
    } else {
        format_size(entry.size)
    }
}

fn folder_entry(path: &Path, name: &str) -> ArchiveEntry {
    ArchiveEntry {
        name: name.to_string(),
        path: path.to_string_lossy().into_owned(),
        is_dir: true,
        size: 0,
        compressed_size: None,
        crc: None,
        modified: String::new(),
    }
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

    #[test]
    fn lists_existing_home_directory() {
        let home = home_dir();
        assert!(home.exists());
        let entries = list_directory(&home).expect("list home");
        assert!(!entries.is_empty());
    }
}
