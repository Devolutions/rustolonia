//! The on-disk capture library: one PNG per capture plus a JSON sidecar with
//! where it came from, grouped into monthly folders under
//! `Pictures\Jiffshot`.

use crate::{Frame, Rect};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::Shell::{FOLDERID_Pictures, SHGetKnownFolderPath};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureKind {
    Region,
    Window,
    Monitor,
    Desktop,
    Scrolling,
    Panoramic,
    Menu,
    Video,
    Steps,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptureMeta {
    pub kind: CaptureKind,
    /// Screen rectangle that was captured.
    pub rect: Rect,
    pub width: u32,
    pub height: u32,
    /// Local time, `YYYY-MM-DDTHH:MM:SS`.
    pub captured_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
    pub cursor_included: bool,
    /// The address of the web page, for captures of a browser window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Kept at the front of the tray.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
}

/// The sidecar of a capture (or recording): `<stem>.json`.
pub fn sidecar(capture: &Path) -> PathBuf {
    capture.with_extension("json")
}

/// A capture's metadata; recordings and imported files may have none.
pub fn read_meta(capture: &Path) -> Option<CaptureMeta> {
    serde_json::from_slice(&std::fs::read(sidecar(capture)).ok()?).ok()
}

pub fn write_meta(capture: &Path, meta: &CaptureMeta) -> io::Result<()> {
    std::fs::write(sidecar(capture), serde_json::to_vec_pretty(meta).map_err(io::Error::other)?)
}

/// The extension of a capture's saved edits (a Jiffshot document).
pub const EDITS_EXT: &str = "jsd";

/// A capture's saved edits, if any: the `.jsd` beside it.
pub fn edits_of(capture: &Path) -> Option<PathBuf> {
    Some(capture.with_extension(EDITS_EXT)).filter(|p| p.exists())
}

/// Files that belong with a capture: its sidecar, edits and pointer.
pub fn companions(capture: &Path) -> Vec<PathBuf> {
    let stem = capture.with_extension("");
    let mut out = vec![capture.with_extension("json"), capture.with_extension(EDITS_EXT)];
    out.push(PathBuf::from(format!("{}.cursor.png", stem.display())));
    out.push(PathBuf::from(format!("{}.cursor.json", stem.display())));
    out
}

/// Renames a capture and its companions to `name` (no extension) in the
/// same folder; returns the new path.
pub fn rename(capture: &Path, name: &str) -> io::Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() || name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "a file name cannot contain \\ / : * ? \" < > |"));
    }
    let ext = capture.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    let target = capture.with_file_name(format!("{name}.{ext}"));
    if target.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "a capture with that name exists"));
    }
    let new_stem = target.with_extension("");
    let old_stem = capture.with_extension("");
    for c in companions(capture) {
        if c.exists() {
            let suffix = c.to_string_lossy()[old_stem.to_string_lossy().len()..].to_owned();
            std::fs::rename(&c, format!("{}{suffix}", new_stem.display()))?;
        }
    }
    std::fs::rename(capture, &target)?;
    Ok(target)
}

/// Local wall-clock time as `(year, month, day, hour, minute, second)`.
pub fn local_time() -> (u16, u16, u16, u16, u16, u16) {
    // SAFETY: out-parameter only.
    let t = unsafe {
        let mut t = std::mem::zeroed();
        GetLocalTime(&mut t);
        t
    };
    (t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
}

pub fn timestamp() -> String {
    let (y, mo, d, h, mi, s) = local_time();
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}")
}

pub struct Library {
    pub root: PathBuf,
}

impl Library {
    pub fn new(root: PathBuf) -> Library {
        Library { root }
    }

    /// `Pictures\Jiffshot`, following OneDrive or other folder redirection.
    pub fn default_root() -> PathBuf {
        // SAFETY: the returned string is freed with CoTaskMemFree as
        // documented, after being copied.
        let pictures = unsafe {
            let mut path = std::ptr::null_mut();
            let hr = SHGetKnownFolderPath(&FOLDERID_Pictures, 0, std::ptr::null_mut(), &mut path);
            let result = if hr == 0 && !path.is_null() {
                let len = (0..).take_while(|&i| *path.add(i) != 0).count();
                Some(PathBuf::from(String::from_utf16_lossy(std::slice::from_raw_parts(path, len))))
            } else {
                None
            };
            CoTaskMemFree(path as *const _);
            result
        };
        let pictures = pictures
            .or_else(|| std::env::var_os("USERPROFILE").map(|home| Path::new(&home).join("Pictures")))
            .unwrap_or_else(std::env::temp_dir);
        pictures.join(crate::paths::NAME)
    }

    /// Saves a capture and its sidecar; returns the PNG path.
    pub fn save(&self, frame: &Frame, meta: &CaptureMeta) -> io::Result<PathBuf> {
        let (y, mo, d, h, mi, s) = local_time();
        let folder = self.root.join(format!("{y:04}-{mo:02}"));
        std::fs::create_dir_all(&folder)?;
        let stem = format!("Capture {y:04}-{mo:02}-{d:02} {h:02}{mi:02}{s:02}");
        let mut path = folder.join(format!("{stem}.png"));
        for n in 2.. {
            if !path.exists() {
                break;
            }
            path = folder.join(format!("{stem} ({n}).png"));
        }
        frame.save_png(&path)?;
        let json = serde_json::to_vec_pretty(meta).map_err(io::Error::other)?;
        std::fs::write(path.with_extension("json"), json)?;
        Ok(path)
    }

    /// A new, unused path for a screen recording.
    pub fn recording_path(&self) -> io::Result<PathBuf> {
        let (y, mo, d, h, mi, s) = local_time();
        let folder = self.root.join(format!("{y:04}-{mo:02}"));
        std::fs::create_dir_all(&folder)?;
        let stem = format!("Recording {y:04}-{mo:02}-{d:02} {h:02}{mi:02}{s:02}");
        let mut path = folder.join(format!("{stem}.mp4"));
        for n in 2.. {
            if !path.exists() {
                break;
            }
            path = folder.join(format!("{stem} ({n}).mp4"));
        }
        Ok(path)
    }

    /// Captures in the library (images and recordings), newest first.
    pub fn recent(&self, limit: usize) -> Vec<PathBuf> {
        let mut items: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|month| month.path().is_dir())
            .flat_map(|month| std::fs::read_dir(month.path()).into_iter().flatten().flatten())
            .filter(|entry| entry.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("png") || e.eq_ignore_ascii_case("mp4")))
            .filter(|entry| !entry.path().to_string_lossy().ends_with(".thumb.png") && !entry.path().to_string_lossy().ends_with(".cursor.png"))
            .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
            .collect();
        items.sort_by(|a, b| b.0.cmp(&a.0));
        items.into_iter().take(limit).map(|(_, path)| path).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_writes_png_and_sidecar_without_overwriting() {
        let root = std::env::temp_dir().join(format!("jiffshot-library-test-{}", std::process::id()));
        let library = Library::new(root.clone());
        let frame = Frame::new(4, 3);
        let meta = CaptureMeta {
            kind: CaptureKind::Region,
            rect: Rect::new(1, 2, 4, 3),
            width: 4,
            height: 3,
            captured_at: timestamp(),
            window_title: None,
            process: None,
            cursor_included: false,
            url: None,
            tags: vec!["docs".into()],
            pinned: false,
        };
        let first = library.save(&frame, &meta).unwrap();
        let second = library.save(&frame, &meta).unwrap();
        assert_ne!(first, second);
        assert!(first.with_extension("json").exists());
        let recent = library.recent(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(read_meta(&first).unwrap().tags, ["docs"]);
        std::fs::write(first.with_extension("jsd"), b"{}").unwrap();
        let renamed = rename(&first, "Settings page").unwrap();
        assert!(renamed.ends_with("Settings page.png") && renamed.exists());
        assert!(renamed.with_extension("json").exists() && renamed.with_extension("jsd").exists());
        assert!(!first.exists() && !first.with_extension("jsd").exists());
        assert_eq!(edits_of(&renamed), Some(renamed.with_extension("jsd")));
        assert!(rename(&renamed, "a/b").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
