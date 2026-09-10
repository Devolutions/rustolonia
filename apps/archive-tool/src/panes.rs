//! Dual-pane location and listing.

use crate::archive::{children_of, parent_dir, ArchiveEntry, ArchiveKind};
use crate::fs_pane::{
    home_dir, list_computer, list_directory, parent_folder,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

pub enum Location {
    Computer,
    Folder(PathBuf),
    Archive {
        path: PathBuf,
        name: String,
        kind: ArchiveKind,
        entries: Vec<ArchiveEntry>,
        current_dir: String,
        parent: PathBuf,
    },
}

pub struct Pane {
    pub location: Location,
    pub selected_paths: HashSet<String>,
    pub selected_index: i64,
    pub selected_key: String,
    pub sort_column: String,
    pub sort_descending: bool,
    fs_entries: Vec<ArchiveEntry>,
}

impl Pane {
    pub fn folder(path: PathBuf) -> Self {
        let fs_entries = list_directory(&path).unwrap_or_default();
        Self {
            location: Location::Folder(path),
            selected_paths: HashSet::new(),
            selected_index: -1,
            selected_key: String::new(),
            sort_column: "Name".to_string(),
            sort_descending: false,
            fs_entries,
        }
    }

    pub fn computer() -> Self {
        Self {
            location: Location::Computer,
            selected_paths: HashSet::new(),
            selected_index: -1,
            selected_key: String::new(),
            sort_column: "Name".to_string(),
            sort_descending: false,
            fs_entries: list_computer(),
        }
    }

    pub fn home() -> Self {
        Self::folder(home_dir())
    }

    pub fn path_label(&self) -> String {
        match &self.location {
            Location::Computer => "Computer".to_string(),
            Location::Folder(path) => path.to_string_lossy().into_owned(),
            Location::Archive {
                name, current_dir, ..
            } if current_dir.is_empty() => name.clone(),
            Location::Archive {
                name, current_dir, ..
            } => format!("{} / {}", name, current_dir.replace('/', " / ")),
        }
    }

    pub fn kind_label(&self) -> String {
        match &self.location {
            Location::Computer => "Computer".to_string(),
            Location::Folder(_) => "Folder".to_string(),
            Location::Archive { kind, .. } => kind.label().to_string(),
        }
    }

    pub fn can_go_up(&self) -> bool {
        match &self.location {
            Location::Computer => false,
            Location::Folder(path) => parent_folder(path).is_some() || path.exists(),
            Location::Archive { .. } => true,
        }
    }

    pub fn is_archive(&self) -> bool {
        matches!(self.location, Location::Archive { .. })
    }

    pub fn archive_path(&self) -> Option<&Path> {
        match &self.location {
            Location::Archive { path, .. } => Some(path),
            _ => None,
        }
    }

    pub fn archive_entries(&self) -> Option<&[ArchiveEntry]> {
        match &self.location {
            Location::Archive { entries, .. } => Some(entries),
            _ => None,
        }
    }

    pub fn copy_destination(&self) -> Option<PathBuf> {
        match &self.location {
            Location::Folder(path) => Some(path.clone()),
            Location::Archive { parent, .. } => Some(parent.clone()),
            Location::Computer => None,
        }
    }

    pub fn visible(&self, filter: &str) -> Vec<ArchiveEntry> {
        let needle = filter.trim().to_ascii_lowercase();
        let mut entries = match &self.location {
            Location::Computer | Location::Folder(_) => self.fs_entries.clone(),
            Location::Archive {
                entries,
                current_dir,
                ..
            } => {
                if needle.is_empty() {
                    children_of(entries, current_dir)
                        .into_iter()
                        .cloned()
                        .collect()
                } else {
                    entries
                        .iter()
                        .filter(|entry| {
                            (current_dir.is_empty()
                                || entry.path == *current_dir
                                || entry.path.starts_with(&format!("{current_dir}/")))
                                && entry.path.to_ascii_lowercase().contains(&needle)
                        })
                        .cloned()
                        .collect()
                }
            }
        };
        if !needle.is_empty() && !matches!(self.location, Location::Archive { .. }) {
            entries.retain(|entry| entry.name.to_ascii_lowercase().contains(&needle));
        }
        entries.sort_by(|left, right| compare_entries(left, right, &self.sort_column, self.sort_descending));
        entries
    }

    pub fn go_up(&mut self) {
        match &self.location {
            Location::Computer => return,
            Location::Folder(path) => {
                if let Some(parent) = parent_folder(path) {
                    self.fs_entries = list_directory(&parent).unwrap_or_default();
                    self.location = Location::Folder(parent);
                } else {
                    self.fs_entries = list_computer();
                    self.location = Location::Computer;
                }
            }
            Location::Archive { current_dir, parent, .. } => {
                if current_dir.is_empty() {
                    let parent = parent.clone();
                    self.fs_entries = list_directory(&parent).unwrap_or_default();
                    self.location = Location::Folder(parent);
                } else if let Location::Archive { current_dir, .. } = &mut self.location {
                    *current_dir = parent_dir(current_dir);
                }
            }
        }
        self.clear_selection();
    }

    pub fn open_folder(&mut self, path: PathBuf) {
        self.fs_entries = list_directory(&path).unwrap_or_default();
        self.location = Location::Folder(path);
        self.clear_selection();
    }

    pub fn open_archive(
        &mut self,
        path: PathBuf,
        kind: ArchiveKind,
        entries: Vec<ArchiveEntry>,
    ) {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("archive")
            .to_string();
        let parent = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        self.location = Location::Archive {
            path,
            name,
            kind,
            entries,
            current_dir: String::new(),
            parent,
        };
        self.clear_selection();
    }

    pub fn enter_dir(&mut self, path: String) {
        match &mut self.location {
            Location::Archive { current_dir, .. } => {
                *current_dir = path;
            }
            Location::Folder(_) | Location::Computer => {
                let folder = PathBuf::from(path);
                self.fs_entries = list_directory(&folder).unwrap_or_default();
                self.location = Location::Folder(folder);
            }
        }
        self.clear_selection();
    }

    pub fn clear_selection(&mut self) {
        self.selected_paths.clear();
        self.selected_index = -1;
        self.selected_key.clear();
    }
}

pub fn compare_entries(
    left: &ArchiveEntry,
    right: &ArchiveEntry,
    column: &str,
    descending: bool,
) -> std::cmp::Ordering {
    let folder_order = right.is_dir.cmp(&left.is_dir);
    if folder_order != std::cmp::Ordering::Equal {
        return folder_order;
    }
    let ordering = match column {
        "Kind" => left.kind_label().cmp(right.kind_label()),
        "Size" => left.size.cmp(&right.size),
        "Packed" => left
            .compressed_size
            .unwrap_or(0)
            .cmp(&right.compressed_size.unwrap_or(0)),
        "Ratio" => left.ratio_label().cmp(&right.ratio_label()),
        "Modified" => left.modified.cmp(&right.modified),
        "Crc" => left.crc.cmp(&right.crc),
        _ => left
            .name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase()),
    };
    if descending {
        ordering.reverse()
    } else {
        ordering
    }
}

