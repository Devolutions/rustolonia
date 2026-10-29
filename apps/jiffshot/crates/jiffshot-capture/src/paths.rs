//! Where Jiffshot keeps its settings, logs and caches.

use std::path::PathBuf;

/// The product's name, for folders and registry values.
pub const NAME: &str = "Jiffshot";

/// `%APPDATA%\Jiffshot`: settings, presets and stamps.
pub fn config_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join(NAME))
}

/// `%LOCALAPPDATA%\Jiffshot` (the temp folder without it): logs and caches,
/// which start fresh.
pub fn local_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join(NAME)
}
