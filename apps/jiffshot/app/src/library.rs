//! The library: recent captures with thumbnails made on a background
//! thread and cached under `%LOCALAPPDATA%\Jiffshot\thumbs`, shown in the
//! tray strip or the full library view, searched, filtered, sorted, pinned,
//! tagged, renamed, closed from the tray and deleted.

use crate::app::{self, ItemSink};
use crate::vm::{LibraryItemViewModel, LibraryItemViewModelSink};
use rustolonia::Result;
use jiffshot_capture::library::CaptureMeta;
use jiffshot_doc::{io, raster, Document, Pixmap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

const THUMB_HEIGHT: u32 = 120;

/// What the library knows about one capture.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub meta: Option<CaptureMeta>,
    pub modified: SystemTime,
}

impl Entry {
    pub fn read(path: PathBuf) -> Entry {
        let meta = jiffshot_capture::library::read_meta(&path);
        let modified = modified(&path).unwrap_or(SystemTime::UNIX_EPOCH);
        Entry { path, meta, modified }
    }

    pub fn is_video(&self) -> bool {
        self.path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
    }

    pub fn pinned(&self) -> bool {
        self.meta.as_ref().is_some_and(|m| m.pinned)
    }

    pub fn tags(&self) -> &[String] {
        self.meta.as_ref().map_or(&[], |m| m.tags.as_slice())
    }

    pub fn app(&self) -> Option<&str> {
        self.meta.as_ref()?.process.as_deref()
    }

    /// The web site's host, for browser captures.
    pub fn site(&self) -> Option<&str> {
        let url = self.meta.as_ref()?.url.as_deref()?;
        let rest = url.split_once("://").map_or(url, |(_, r)| r);
        rest.split(['/', '?', '#']).next().filter(|h| !h.is_empty())
    }

    fn name(&self) -> String {
        self.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }

    /// Everything a search matches: name, window title, app, address, tags.
    fn haystack(&self) -> String {
        let mut s = self.name();
        if let Some(m) = &self.meta {
            for part in [m.window_title.as_deref(), m.process.as_deref(), m.url.as_deref()].into_iter().flatten() {
                s.push(' ');
                s.push_str(part);
            }
            for t in &m.tags {
                s.push(' ');
                s.push_str(t);
            }
            s.push(' ');
            s.push_str(&m.captured_at);
        }
        s.to_lowercase()
    }

    pub fn info(&self) -> String {
        let mut parts = vec![self.name()];
        if let Some(m) = &self.meta {
            parts.extend(m.window_title.clone());
            parts.extend(m.process.clone());
            parts.extend(m.url.clone());
            parts.push(m.captured_at.replace('T', " "));
            parts.push(format!("{} × {}", m.width, m.height));
            if !m.tags.is_empty() {
                parts.push(format!("Tags: {}", m.tags.join(", ")));
            }
        }
        parts.join("\n")
    }
}

/// The filters offered, from what the entries have: fixed ones, then
/// each app, web site and tag.
pub fn filters(entries: &[Entry]) -> Vec<String> {
    let mut out: Vec<String> = ["All", "Images", "Videos", "Pinned", "Today", "Last 7 days", "Tagged"].iter().map(|s| s.to_string()).collect();
    let mut add = |prefix: &str, values: Vec<&str>| {
        let mut v: Vec<&str> = values;
        v.sort_unstable_by_key(|s| s.to_lowercase());
        v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        out.extend(v.into_iter().map(|x| format!("{prefix}: {x}")));
    };
    add("App", entries.iter().filter_map(Entry::app).collect());
    add("Site", entries.iter().filter_map(Entry::site).collect());
    add("Tag", entries.iter().flat_map(|e| e.tags().iter().map(String::as_str)).collect());
    out
}

pub const SORTS: &[&str] = &["Newest", "Oldest", "Name", "App", "Size"];

/// The entries a search, filter and sort leave, pinned ones first.
pub fn select(entries: &[Entry], query: &str, filter: &str, sort: &str, now: SystemTime) -> Vec<Entry> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let age = |e: &Entry| now.duration_since(e.modified).unwrap_or_default();
    let mut out: Vec<Entry> = entries
        .iter()
        .filter(|e| {
            let hay = e.haystack();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .filter(|e| match filter {
            "Images" => !e.is_video(),
            "Videos" => e.is_video(),
            "Pinned" => e.pinned(),
            "Today" => age(e) < Duration::from_secs(24 * 3600),
            "Last 7 days" => age(e) < Duration::from_secs(7 * 24 * 3600),
            "Tagged" => !e.tags().is_empty(),
            f => match f.split_once(": ") {
                Some(("App", a)) => e.app().is_some_and(|x| x.eq_ignore_ascii_case(a)),
                Some(("Site", s)) => e.site().is_some_and(|x| x.eq_ignore_ascii_case(s)),
                Some(("Tag", t)) => e.tags().iter().any(|x| x.eq_ignore_ascii_case(t)),
                _ => true,
            },
        })
        .cloned()
        .collect();
    let pixels = |e: &Entry| e.meta.as_ref().map_or(0, |m| m.width as u64 * m.height as u64);
    match sort {
        "Oldest" => out.sort_by_key(|e| e.modified),
        "Name" => out.sort_by_key(|e| e.name().to_lowercase()),
        "App" => out.sort_by(|a, b| a.app().unwrap_or("~").to_lowercase().cmp(&b.app().unwrap_or("~").to_lowercase()).then(b.modified.cmp(&a.modified))),
        "Size" => out.sort_by_key(|e| std::cmp::Reverse(pixels(e))),
        _ => out.sort_by_key(|e| std::cmp::Reverse(e.modified)),
    }
    // Stable: pinned first, in the chosen order.
    out.sort_by_key(|e| !e.pinned());
    out
}

/// "a, b ,, c" → ["a", "b", "c"].
pub fn parse_tags(text: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for t in text.split([',', ';']).map(str::trim).filter(|t| !t.is_empty()) {
        if !tags.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            tags.push(t.to_owned());
        }
    }
    tags
}

#[derive(Clone)]
pub struct Item {
    pub entry: Entry,
    pub path: PathBuf,
    current: Arc<Mutex<bool>>,
    selected: Arc<Mutex<bool>>,
    unsaved: Arc<Mutex<bool>>,
    new_name: String,
    tag_text: String,
    thumb_height: f64,
    sink: ItemSink,
}

impl Item {
    pub fn new(entry: Entry, current: bool, selected: bool, thumb_height: f64) -> Item {
        let tags = entry.tags().join(", ");
        Item {
            path: entry.path.clone(),
            new_name: entry.name(),
            tag_text: tags,
            entry,
            current: Arc::new(Mutex::new(current)),
            selected: Arc::new(Mutex::new(selected)),
            unsaved: Arc::new(Mutex::new(false)),
            thumb_height,
            sink: Arc::new(Mutex::new(None)),
        }
    }

    fn sink(&self) -> Option<LibraryItemViewModelSink> {
        // Never call a sink while holding the lock: another thread may be
        // waiting on the UI thread with it.
        self.sink.lock().unwrap().clone()
    }

    pub fn set_current(&self, value: bool) {
        *self.current.lock().unwrap() = value;
        if let Some(sink) = self.sink() {
            let _ = sink.set_is_current(value);
        }
    }

    pub fn set_selected(&self, value: bool) {
        *self.selected.lock().unwrap() = value;
        if let Some(sink) = self.sink() {
            let _ = sink.set_is_selected(value);
        }
    }

    pub fn set_unsaved(&self, value: bool) {
        let changed = std::mem::replace(&mut *self.unsaved.lock().unwrap(), value) != value;
        if let (true, Some(sink)) = (changed, self.sink()) {
            let _ = sink.set_unsaved(value);
        }
    }

    fn label(&self) -> String {
        // "Capture 2026-10-28 141503" → "Oct 28 14:15"; renamed captures
        // keep their name.
        let stem = self.entry.name();
        let video = stem.starts_with("Recording ");
        let parts: Vec<&str> = stem.trim_start_matches("Capture ").trim_start_matches("Recording ").split(' ').collect();
        if let [date, time, ..] = parts.as_slice() {
            let d: Vec<&str> = date.split('-').collect();
            if d.len() == 3 && time.len() >= 4 && time[..4].bytes().all(|b| b.is_ascii_digit()) {
                const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                if let Ok(m) = d[1].parse::<usize>() {
                    return format!("{}{} {} {}:{}", if video { "▶ " } else { "" }, MONTHS[(m.max(1) - 1).min(11)], d[2].trim_start_matches('0'), &time[..2], &time[2..4]);
                }
            }
        }
        stem
    }

    fn act(&self, what: &'static str, value: Option<String>) {
        let path = self.path.clone();
        app::post(move |app| app.library_action(&path, what, value));
    }
}

impl LibraryItemViewModel for Item {
    fn attach(&mut self, sink: LibraryItemViewModelSink) -> Result<()> {
        sink.set_path(self.path.to_string_lossy())?;
        sink.set_label(self.label())?;
        sink.set_is_current(*self.current.lock().unwrap())?;
        sink.set_is_selected(*self.selected.lock().unwrap())?;
        sink.set_unsaved(*self.unsaved.lock().unwrap())?;
        sink.set_kind(if self.entry.is_video() { "Video" } else { "Image" })?;
        sink.set_is_pinned(self.entry.pinned())?;
        sink.set_tags(self.entry.tags().join(", "))?;
        sink.set_info(self.entry.info())?;
        sink.set_new_name(self.new_name.clone())?;
        sink.set_tag_text(self.tag_text.clone())?;
        sink.set_thumb_height(self.thumb_height)?;
        let thumb = thumbnail_path(&self.path);
        if thumb.exists() {
            sink.set_thumbnail(thumb.to_string_lossy())?;
        }
        *self.sink.lock().unwrap() = Some(sink);
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        *self.sink.lock().unwrap() = None;
        Ok(())
    }

    fn open(&mut self) -> Result<()> {
        let path = self.path.clone();
        app::post(move |app| app.open_path(&path));
        Ok(())
    }

    fn toggle_pin(&mut self) -> Result<()> {
        self.act("pin", None);
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        self.act("close", None);
        Ok(())
    }

    fn delete(&mut self) -> Result<()> {
        self.act("delete", None);
        Ok(())
    }

    fn rename(&mut self, value: String) -> Result<()> {
        self.act("rename", Some(value));
        Ok(())
    }

    fn set_tags(&mut self, value: String) -> Result<()> {
        self.act("tags", Some(value));
        Ok(())
    }

    fn toggle_select(&mut self) -> Result<()> {
        self.act("select", None);
        Ok(())
    }

    fn show_in_folder(&mut self) -> Result<()> {
        self.act("reveal", None);
        Ok(())
    }
}

fn cache_dir() -> PathBuf {
    jiffshot_capture::paths::local_dir().join("thumbs")
}

/// Where the thumbnail for a capture lives. The name hashes the path, so
/// moving the library never shows stale thumbnails.
pub fn thumbnail_path(capture: &Path) -> PathBuf {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in capture.to_string_lossy().to_lowercase().bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    cache_dir().join(format!("{h:016x}.png"))
}

fn modified(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Whether the thumbnail is older than the capture or its saved edits.
fn stale(capture: &Path) -> bool {
    let thumb = modified(&thumbnail_path(capture));
    let edits = jiffshot_capture::library::edits_of(capture);
    let newest = [modified(capture), edits.as_deref().and_then(modified)].into_iter().flatten().max();
    match (thumb, newest) {
        (Some(t), Some(n)) => t < n,
        (None, _) => true,
        _ => false,
    }
}

fn shrink(pm: &Pixmap) -> Option<Pixmap> {
    let h = THUMB_HEIGHT.min(pm.height());
    let w = ((pm.width() as f32 * h as f32 / pm.height() as f32).round() as u32).clamp(1, 400);
    raster::resample(pm, w, h)
}

/// Writes the thumbnail of an edited document for its capture.
pub fn write_thumbnail(doc: &Document, capture: &Path) {
    if let Some(small) = shrink(&doc.flatten()) {
        let _ = std::fs::create_dir_all(cache_dir());
        if let Ok(bytes) = io::encode(&small, io::Format::Png, 0) {
            let _ = std::fs::write(thumbnail_path(capture), bytes);
        }
    }
}

fn render_source(capture: &Path) -> Option<Pixmap> {
    if capture.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4")) {
        return crate::video::thumbnail(capture);
    }
    if let Some(edits) = jiffshot_capture::library::edits_of(capture) {
        if let Ok(doc) = io::load_document(&edits) {
            return Some(doc.flatten());
        }
    }
    io::load_image(capture).ok()
}

/// Makes missing or stale thumbnails off the UI thread and shows each one
/// as it is ready.
pub fn make_thumbnails(items: Vec<Item>) {
    std::thread::spawn(move || {
        let _ = std::fs::create_dir_all(cache_dir());
        for item in items {
            if !stale(&item.path) {
                continue;
            }
            let Some(small) = render_source(&item.path).as_ref().and_then(shrink) else { continue };
            let thumb = thumbnail_path(&item.path);
            if io::encode(&small, io::Format::Png, 0).ok().and_then(|b| std::fs::write(&thumb, b).ok()).is_some() {
                // Sinks dispatch to the UI thread themselves.
                if let Some(sink) = item.sink() {
                    let _ = sink.set_thumbnail(thumb.to_string_lossy());
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiffshot_capture::library::CaptureKind;
    use jiffshot_capture::Rect;

    fn entry(name: &str, age_hours: u64, app: Option<&str>, url: Option<&str>, tags: &[&str], pinned: bool) -> Entry {
        let meta = CaptureMeta {
            kind: CaptureKind::Region,
            rect: Rect::new(0, 0, 10, 10),
            width: 10,
            height: 10,
            captured_at: "2026-10-28T10:00:00".into(),
            window_title: Some(format!("{name} window")),
            process: app.map(str::to_owned),
            cursor_included: false,
            url: url.map(str::to_owned),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            pinned,
        };
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        Entry { path: PathBuf::from(format!("C:/lib/{name}.png")), meta: Some(meta), modified: now - Duration::from_secs(age_hours * 3600) }
    }

    #[test]
    fn search_filter_sort_and_pins() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let all = vec![
            entry("alpha", 1, Some("chrome.exe"), Some("https://docs.example.com/a?b"), &["docs"], false),
            entry("beta", 30, Some("code.exe"), None, &[], true),
            entry("gamma", 200, Some("chrome.exe"), None, &["Docs", "release"], false),
        ];
        let names = |v: Vec<Entry>| v.iter().map(|e| e.name()).collect::<Vec<_>>();
        assert_eq!(names(select(&all, "", "All", "Newest", now)), ["beta", "alpha", "gamma"], "pinned first");
        assert_eq!(names(select(&all, "", "Today", "Newest", now)), ["alpha"]);
        assert_eq!(names(select(&all, "", "Last 7 days", "Oldest", now)), ["beta", "alpha"]);
        assert_eq!(names(select(&all, "CHROME window", "All", "Name", now)), ["alpha", "gamma"]);
        assert_eq!(names(select(&all, "", "Tag: docs", "Newest", now)), ["alpha", "gamma"]);
        assert_eq!(names(select(&all, "", "Site: docs.example.com", "Newest", now)), ["alpha"]);
        let f = filters(&all);
        assert!(f.contains(&"App: chrome.exe".to_string()) && f.contains(&"App: code.exe".to_string()));
        assert_eq!(f.iter().filter(|x| x.starts_with("Tag: ")).count(), 2, "tags deduplicated ignoring case: {f:?}");
        assert_eq!(parse_tags(" a, b ;; c ,A"), ["a", "b", "c"]);
    }
}
