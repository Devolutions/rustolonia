//! Jiffshot's one settings file, `%APPDATA%\Jiffshot\jiffshot.json`:
//!
//! ```json
//! { "library": "D:\\Captures", "theme": "Dark",
//!   "capture": { … }, "editor": { … } }
//! ```
//!
//! The capture side and the editor each own their section; the library
//! folder and the color theme are shared at the top. Reads and writes go
//! through one lock, so the two sides (on different threads) never undo
//! each other's changes.

use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const FILE: &str = "jiffshot.json";

/// Serializes every read-modify-write of the file in this process.
static LOCK: Mutex<()> = Mutex::new(());

fn read_object(path: &Path) -> Map<String, Value> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|v| match v {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

/// The settings in `dir` (empty when there are none yet).
fn read(dir: &Path) -> Map<String, Value> {
    read_object(&dir.join(FILE))
}

fn write(dir: &Path, root: &Map<String, Value>) {
    let _ = std::fs::create_dir_all(dir);
    if let Ok(json) = serde_json::to_vec_pretty(root) {
        // Through a temporary file, so a crash never leaves half a file.
        let tmp = dir.join(format!("{FILE}.tmp"));
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, dir.join(FILE));
        }
    }
}

fn with_dir<R>(f: impl FnOnce(&Path) -> R) -> Option<R> {
    let dir = crate::paths::config_dir()?;
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    Some(f(&dir))
}

/// One side's section (`"capture"` or `"editor"`), `Null` when there is none.
pub fn section(name: &str) -> Value {
    with_dir(|dir| read(dir).remove(name)).flatten().unwrap_or(Value::Null)
}

/// Replaces one side's section, keeping everything else.
pub fn set_section(name: &str, value: Value) {
    with_dir(|dir| {
        let mut root = read(dir);
        root.insert(name.to_owned(), value);
        write(dir, &root);
    });
}

fn top(key: &str) -> Option<String> {
    with_dir(|dir| read(dir).get(key).and_then(Value::as_str).map(str::to_owned)).flatten()
}

fn set_top(key: &str, value: Option<&str>) {
    with_dir(|dir| {
        let mut root = read(dir);
        match value {
            Some(v) => root.insert(key.to_owned(), Value::String(v.to_owned())),
            None => root.remove(key),
        };
        write(dir, &root);
    });
}

/// The library folder chosen in Preferences, if any. `JIFFSHOT_LIBRARY`
/// overrides it (for tests and portable use).
pub fn library() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("JIFFSHOT_LIBRARY").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    top("library").map(PathBuf::from)
}

/// Where captures are saved and listed: [`library`], or `Pictures\Jiffshot`.
pub fn library_root() -> PathBuf {
    library().unwrap_or_else(crate::library::Library::default_root)
}

pub fn set_library(folder: Option<&Path>) {
    set_top("library", folder.map(|f| f.to_string_lossy()).as_deref());
}

/// "Dark", "Light" or "System"; for every window.
pub fn theme() -> Option<String> {
    top("theme")
}

pub fn set_theme(theme: &str) {
    set_top("theme", Some(theme));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jiffshot-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_section_write_keeps_the_other_section() {
        let dir = temp("sections");
        let mut root = read(&dir);
        root.insert("editor".into(), serde_json::json!({ "recent": ["a.png"] }));
        write(&dir, &root);
        let mut root = read(&dir);
        root.insert("capture".into(), serde_json::json!({ "countdown": 1 }));
        write(&dir, &root);
        let root = read(&dir);
        assert_eq!(root["editor"]["recent"][0], "a.png");
        assert_eq!(root["capture"]["countdown"], 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
