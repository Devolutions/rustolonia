//! Rust-owned state behind stage 31 command surfaces.
//!
//! Menus themselves are presentation: the declared menu model is projected into
//! generated managed factories that build real Avalonia `NativeMenu`,
//! `ContextMenu` and `KeyBinding` objects bound to already generated commands
//! and properties. Nothing about a menu crosses the ABI.
//!
//! What *is* state, and therefore lives here, is the recent-file list. It is a
//! most-recently-used list of stage 29 storage URIs - the only member a storage
//! item is guaranteed to have - so it publishes through the already published
//! string-collection transport and needs no new ABI and nothing
//! platform-specific (this is not a Windows jump list).

use crate::storage::StorageItem;

/// Whether a recent-file mutation actually changed the list.
///
/// Publishing is a real cross-ABI call, so the caller is told when it can be
/// skipped: re-opening the file that is already at the front is the common case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecentFilesChange {
    Unchanged,
    Changed,
}

impl RecentFilesChange {
    pub fn is_changed(self) -> bool {
        matches!(self, Self::Changed)
    }
}

/// A bounded most-recently-used list of storage URIs.
///
/// Entries are ordered most recent first, de-duplicated by exact URI, and
/// trimmed to the declared capacity. The generated projection publishes this
/// list into the schema-declared string collection, and the generated menu
/// factory turns each entry into a menu item whose header is derived from the
/// URI and whose command parameter is the URI itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentFileList {
    capacity: usize,
    entries: Vec<String>,
}

impl RecentFileList {
    /// Creates an empty list. A capacity of zero is rejected as a programming
    /// error, because a zero-capacity MRU list can never hold anything.
    ///
    /// # Panics
    /// Panics if `capacity` is zero.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "a recent-file list needs a positive capacity");
        Self {
            capacity,
            entries: Vec::with_capacity(capacity),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The URIs, most recent first.
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, uri: &str) -> bool {
        self.entries.iter().any(|entry| entry == uri)
    }

    /// Records a use of `uri`, moving it to the front.
    ///
    /// A blank URI is ignored. Re-recording the entry that is already at the
    /// front reports [`RecentFilesChange::Unchanged`], so a consumer can skip
    /// republishing.
    pub fn push(&mut self, uri: impl Into<String>) -> RecentFilesChange {
        let uri = uri.into();
        if uri.trim().is_empty() {
            return RecentFilesChange::Unchanged;
        }
        if self.entries.first().is_some_and(|first| *first == uri) {
            return RecentFilesChange::Unchanged;
        }
        self.entries.retain(|entry| *entry != uri);
        self.entries.insert(0, uri);
        self.entries.truncate(self.capacity);
        RecentFilesChange::Changed
    }

    /// Records a use of a storage item, keyed by its guaranteed URI.
    pub fn push_item(&mut self, item: &StorageItem) -> RecentFilesChange {
        self.push(item.uri())
    }

    /// Records several items in the order given; the last one ends up first.
    pub fn extend_items<'a>(
        &mut self,
        items: impl IntoIterator<Item = &'a StorageItem>,
    ) -> RecentFilesChange {
        let mut change = RecentFilesChange::Unchanged;
        for item in items {
            if self.push_item(item).is_changed() {
                change = RecentFilesChange::Changed;
            }
        }
        change
    }

    pub fn remove(&mut self, uri: &str) -> RecentFilesChange {
        let before = self.entries.len();
        self.entries.retain(|entry| entry != uri);
        if self.entries.len() == before {
            RecentFilesChange::Unchanged
        } else {
            RecentFilesChange::Changed
        }
    }

    pub fn clear(&mut self) -> RecentFilesChange {
        if self.entries.is_empty() {
            return RecentFilesChange::Unchanged;
        }
        self.entries.clear();
        RecentFilesChange::Changed
    }

    /// Replaces the whole list, preserving order and applying de-duplication
    /// and the capacity limit. Used to restore a persisted list at startup.
    pub fn replace<S: Into<String>>(&mut self, entries: impl IntoIterator<Item = S>) {
        self.entries.clear();
        for entry in entries {
            let entry = entry.into();
            if entry.trim().is_empty() || self.entries.contains(&entry) {
                continue;
            }
            self.entries.push(entry);
            if self.entries.len() == self.capacity {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushing_moves_an_existing_entry_to_the_front() {
        let mut list = RecentFileList::with_capacity(4);
        assert!(list.push("file:///a").is_changed());
        assert!(list.push("file:///b").is_changed());
        assert!(list.push("file:///a").is_changed());
        assert_eq!(["file:///a", "file:///b"], list.entries());
    }

    #[test]
    fn re_pushing_the_front_entry_is_not_a_change() {
        let mut list = RecentFileList::with_capacity(4);
        list.push("file:///a");
        assert_eq!(RecentFilesChange::Unchanged, list.push("file:///a"));
        assert_eq!(1, list.len());
    }

    #[test]
    fn capacity_drops_the_oldest_entry() {
        let mut list = RecentFileList::with_capacity(2);
        list.push("file:///a");
        list.push("file:///b");
        list.push("file:///c");
        assert_eq!(["file:///c", "file:///b"], list.entries());
    }

    #[test]
    fn blank_entries_are_ignored() {
        let mut list = RecentFileList::with_capacity(2);
        assert_eq!(RecentFilesChange::Unchanged, list.push("   "));
        assert!(list.is_empty());
    }

    #[test]
    fn replace_deduplicates_and_truncates() {
        let mut list = RecentFileList::with_capacity(2);
        list.replace(["file:///a", "file:///a", "file:///b", "file:///c"]);
        assert_eq!(["file:///a", "file:///b"], list.entries());
    }

    #[test]
    fn remove_and_clear_report_whether_anything_changed() {
        let mut list = RecentFileList::with_capacity(3);
        list.push("file:///a");
        assert_eq!(RecentFilesChange::Unchanged, list.remove("file:///z"));
        assert_eq!(RecentFilesChange::Changed, list.remove("file:///a"));
        assert_eq!(RecentFilesChange::Unchanged, list.clear());
    }
}
