//! The editor application: view-model state, commands, files, the library
//! tray and quick styles. Runs on the UI thread; background work posts its
//! results back with [`post`].
//!
//! The state lives in a thread-local `RefCell`. Model calls change state
//! and queue one [`App::sync`] on the dispatcher instead of publishing
//! directly: the window's model is busy during a call, and a published
//! value can make a control write a coerced value straight back (a slider
//! clamping to a new range), which would re-enter it.
//! Writes that arrive while a sync is publishing find the state borrowed
//! and are dropped, which is right for echoes.

use crate::canvas::{with_editor, EDITOR, PENDING_EVENTS};
use crate::vm::*;
use crate::{library, settings, winutil};
use rustolonia::storage::{FileTypeFilter, OpenFilePickerOptions, PickerOutcome, SaveFilePickerOptions, WellKnownFolder};
use rustolonia::{AppScope, Result};
use jiffshot_capture::library::{edits_of, Library, EDITS_EXT};
use jiffshot_doc::editor::{self as ed, Tool, EVENT_COLOR_PICKED};
use jiffshot_doc::object::{Cap, Dash, Kind, Object, Style};
use jiffshot_doc::text::Align;
use jiffshot_doc::{combine, effects, io, Document, Rgba};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` on the app state, unless it is already in use further up the
/// stack (a re-entrant echo from a window).
pub fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| {
        let mut guard = cell.try_borrow_mut().ok()?;
        guard.as_mut().map(f)
    })
}

/// Like [`with`], then queues publishing whatever changed.
pub fn act(f: impl FnOnce(&mut App)) -> Result<()> {
    let _t = crate::trace::span("act");
    with(f);
    schedule_sync();
    Ok(())
}

static SYNC_QUEUED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Publishes changes once the current dispatcher job (and the model lock
/// it may hold) is done. Several requests before then share one sync.
pub fn schedule_sync() {
    if SYNC_QUEUED.swap(true, Ordering::AcqRel) {
        return;
    }
    let scope = SCOPE.lock().unwrap().clone();
    if let Some(scope) = scope {
        let _ = scope.post(|| {
            SYNC_QUEUED.store(false, Ordering::Release);
            with(|app| app.sync());
        });
    }
}

/// Runs `f` on the UI thread with the app state, from any thread.
pub fn post(f: impl FnOnce(&mut App) + Send + 'static) {
    let scope = SCOPE.lock().unwrap().clone();
    if let Some(scope) = scope {
        let _ = scope.post(move || {
            let _ = act(f);
        });
    }
}

/// The capture side's latest HUD, not yet shown.
static PENDING_HUD: Mutex<Option<String>> = Mutex::new(None);

/// Shows the capture side's HUD (`json`), from any thread. While recording
/// it changes every tick: only the latest is shown, so a UI thread that
/// falls behind skips states instead of queueing them, and showing one does
/// not sync the editor.
pub fn post_hud(json: String) {
    let queued = PENDING_HUD.lock().unwrap().replace(json).is_some();
    if queued {
        return;
    }
    let scope = SCOPE.lock().unwrap().clone();
    if let Some(scope) = scope {
        let _ = scope.post(|| {
            let Some(json) = PENDING_HUD.lock().unwrap().take() else { return };
            if with(|app| app.set_hud(&json)).is_none() {
                // The state is in use further up the stack: next turn. The
                // last HUD (all panels gone) must not be lost.
                post_hud(json);
            }
        });
    }
}

/// Runs `f` on the UI thread without syncing the editor afterwards, for
/// changes that only reach other windows (the overlay, Screen Draw).
pub fn post_quiet(f: impl FnOnce(&mut App) + Send + 'static) {
    let scope = SCOPE.lock().unwrap().clone();
    if let Some(scope) = scope {
        let _ = scope.post(move || {
            with(f);
        });
    }
}

static SCOPE: Mutex<Option<AppScope>> = Mutex::new(None);

pub fn install(scope: AppScope, opening: Option<PathBuf>) {
    *SCOPE.lock().unwrap() = Some(scope.clone());
    let settings = settings::Settings::load();
    let root = jiffshot_capture::config::library_root();
    let library = Library::new(root);
    APP.with(|cell| {
        *cell.borrow_mut() = Some(App {
            scope,
            sink: None,
            shown: Default::default(),
            path: None,
            source: None,
            library,
            exiting: false,
            toasts: 0,
            settings,
            frame: 0,
            focus: 0,
            zoom: 0,
            status: String::new(),
            object_clipboard: None,
            library_items: Vec::new(),
            library_selected: Default::default(),
            library_entries: Vec::new(),
            shown_filters: Vec::new(),
            canvas_drop: None,
            opening,
            last_tool: None,
            last_kind: None,
            video: None,
            narration: None,
        })
    });
}

/// Values last sent to the windows, so unchanged ones are not resent.
#[derive(Default)]
pub struct Shown {
    strings: std::collections::HashMap<&'static str, String>,
    numbers: std::collections::HashMap<&'static str, f64>,
    flags: std::collections::HashMap<&'static str, bool>,
}

pub struct App {
    scope: AppScope,
    /// Set once Exit started, so the capture side stopping does not start it again.
    exiting: bool,
    /// Toasts shown so far, so each one is a new value of `Toast`.
    toasts: u64,
    pub(crate) sink: Option<MainViewModelSink>,
    shown: Shown,
    /// Where Save writes: a `.jsd` file, or `None` for a new document.
    path: Option<PathBuf>,
    /// The image or capture the document was opened from.
    source: Option<PathBuf>,
    pub library: Library,
    pub settings: settings::Settings,
    frame: i64,
    focus: i64,
    zoom: i64,
    status: String,
    object_clipboard: Option<Object>,
    library_items: Vec<library::Item>,
    library_selected: std::collections::HashSet<PathBuf>,
    library_entries: Vec<library::Entry>,
    shown_filters: Vec<String>,
    /// When images were last dropped onto the canvas.
    canvas_drop: Option<std::time::Instant>,
    opening: Option<PathBuf>,
    last_tool: Option<Tool>,
    last_kind: Option<String>,
    pub video: Option<crate::video::Session>,
    pub narration: Option<crate::video::Narration>,
}

fn opacity_percent(style: &Style) -> f64 {
    (style.opacity * 100.0).round() as f64
}

fn align_name(a: Align) -> &'static str {
    match a {
        Align::Left => "Left",
        Align::Center => "Center",
        Align::Right => "Right",
    }
}

fn cap_name(c: Cap) -> &'static str {
    c.name()
}

fn dash_name(d: Dash) -> &'static str {
    match d {
        Dash::Solid => "Solid",
        Dash::Dash => "Dash",
        Dash::Dot => "Dot",
    }
}

/// Sections of the properties panel for a tool or selected object kind.
fn panel_for(kind: &str) -> &'static str {
    match kind {
        "Arrow" | "Line" => "stroke;width;caps;dash;shadow;opacity",
        "Shape" => "variant;stroke;fill;gradient;width;dash;shadow;opacity;rotation",
        "Text" => "text;font;textopts;textcolor;fill;gradient;stroke;width;shadow;opacity;rotation",
        "Callout" => "variant;tails;text;font;textopts;textcolor;fill;gradient;stroke;width;shadow;opacity;rotation",
        "Step" => "variant;text;font;textcolor;fill;gradient;stroke;width;shadow",
        "Pen" | "Marker" => "variant;amount;stroke;width;opacity;shadow",
        "Highlight" => "variant;fill;width",
        "Blur" => "variant;amount;fill",
        "Magnify" => "variant;amount;stroke;width;shadow",
        "Spotlight" => "variant;amount",
        "Stamp" => "variant;shadow;opacity;rotation",
        "Image" => "stroke;width;shadow;opacity;rotation",
        "Drop zone" => "zone;stroke;width;shadow",
        "Fill" => "fill;amount",
        "Cut Out" => "variant",
        "Selection" => "variant;backfill",
        "Magic Wand" => "amount;backfill",
        "Smart Move" => "backfill",
        "Eraser" => "variant;width",
        "Group" => "shadow;opacity",
        _ => "",
    }
}

fn amount_label(kind: &str) -> &'static str {
    match kind {
        "Blur" => "Intensity",
        "Magnify" => "Zoom",
        "Spotlight" => "Dim",
        "Magic Wand" | "Fill" => "Tolerance",
        "Pen" | "Marker" => "Smoothing",
        _ => "",
    }
}

/// What the properties panel shows, read from the editor in one go.
struct Snapshot {
    tool: Tool,
    kind: Option<String>,
    style: Style,
    text: String,
    variant: String,
    variants: Vec<String>,
    amount: f64,
    rotation: f64,
    size: (u32, u32),
    objects: usize,
    undo: Option<String>,
    redo: Option<String>,
    modified: bool,
    damage: bool,
    picked: Option<Rgba>,
    selected_count: usize,
    background_fill: &'static str,
}

impl App {
    pub fn attach(&mut self, sink: MainViewModelSink) -> Result<()> {
        for f in jiffshot_doc::text::FAMILIES {
            sink.add_fonts(*f)?;
        }
        for (name, _) in effects::catalog(Rgba::BLACK) {
            sink.add_effects(name)?;
        }
        for name in effects::INTERACTIVE {
            sink.add_effects(*name)?;
        }
        for name in crate::share::BUILT_IN {
            sink.add_shares(*name)?;
        }
        for (name, _) in &self.settings.programs {
            sink.add_shares(name)?;
        }
        sink.add_shares("Add program…")?;
        for path in self.settings.recent.iter().filter(|p| p.exists()) {
            sink.add_recent_files(file_uri(path))?;
        }
        for t in ["Combine side by side", "Combine stacked", "Combine grid"].iter().chain(combine::TEMPLATES) {
            sink.add_templates(*t)?;
        }
        for t in combine::THEMES {
            sink.add_template_themes(t.name)?;
        }
        self.sink = Some(sink);
        if let Some(window) = self.scope.main_window() {
            use rustolonia::storage::{DragDropEffects, FileDropEvent};
            let _ = self.scope.on_file_drop(&window, DragDropEffects::COPY, |event| {
                if let FileDropEvent::Drop { items, .. } = event {
                    let paths: Vec<PathBuf> = items.iter().filter_map(|i| i.local_path().map(Path::to_path_buf)).collect();
                    post(move |app| {
                        // Files dropped on the canvas were placed there.
                        if app.canvas_drop.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(2)) {
                            return;
                        }
                        if let Some(first) = paths.first() {
                            app.open_path(first);
                        }
                    });
                }
            });
        }
        if let Some(path) = self.opening.take() {
            self.open_path(&path);
        } else {
            self.set_status("Capture with Print Screen or the tray icon, open an image, or paste one.");
        }
        self.refresh_library();
        self.load_stamps();
        schedule_sync();
        Ok(())
    }

    pub fn detach(&mut self) {
        self.autosave();
        self.sink = None;
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = text.into();
    }

    // ---- publishing ----------------------------------------------------

    fn put_str(&mut self, name: &'static str, value: &str, set: impl Fn(&MainViewModelSink, &str) -> Result<()>) {
        if self.shown.strings.get(name).map(String::as_str) != Some(value) {
            if let Some(sink) = &self.sink {
                let _ = set(sink, value);
            }
            self.shown.strings.insert(name, value.to_owned());
        }
    }

    fn put_num(&mut self, name: &'static str, value: f64, set: impl Fn(&MainViewModelSink, f64) -> Result<()>) {
        if self.shown.numbers.get(name) != Some(&value) {
            if let Some(sink) = &self.sink {
                let _ = set(sink, value);
            }
            self.shown.numbers.insert(name, value);
        }
    }

    fn put_flag(&mut self, name: &'static str, value: bool, set: impl Fn(&MainViewModelSink, bool) -> Result<()>) {
        if self.shown.flags.get(name) != Some(&value) {
            if let Some(sink) = &self.sink {
                let _ = set(sink, value);
            }
            self.shown.flags.insert(name, value);
        }
    }

    fn snapshot(&self) -> Option<Snapshot> {
        let _t = crate::trace::span("snapshot");
        let mut guard = EDITOR.lock();
        let e = guard.as_mut()?;
        let selected = e.selected().cloned();
        let tool = e.tool();
        let kind = selected.as_ref().map(|o| if o.zone.is_some() { "Drop zone".to_owned() } else { o.kind_name().to_owned() });
        let variant_tool = selected.as_ref().map(ed::tool_for).unwrap_or(tool);
        let variant = selected.as_ref().and_then(ed::variant_of).unwrap_or_else(|| e.variant(tool));
        Some(Snapshot {
            tool,
            kind,
            style: e.current_style(),
            text: selected.as_ref().and_then(|o| o.text().map(str::to_owned)).unwrap_or_default(),
            variant,
            variants: e.variants_for(variant_tool),
            amount: e.selected_amount().unwrap_or_else(|| e.tool_amount(tool)) as f64,
            rotation: selected.as_ref().map_or(0.0, |o| o.rotation as f64),
            size: e.document().size(),
            objects: e.document().objects.len(),
            undo: e.undo_label().map(str::to_owned),
            redo: e.redo_label().map(str::to_owned),
            modified: e.is_modified(),
            damage: e.has_damage(),
            picked: e.picked_color(),
            selected_count: e.selected_ids().len(),
            background_fill: e.background_fill.name(),
        })
    }

    /// Publishes everything that changed since the last call.
    pub fn sync(&mut self) {
        let _t = crate::trace::span("sync");
        let events = PENDING_EVENTS.swap(0, Ordering::Relaxed) | with_editor(|e| e.take_events()).unwrap_or(0);
        let Some(s) = self.snapshot() else { return };
        if let Some(color) = s.picked {
            let hex = color.to_hex();
            self.set_status(format!("Picked {hex} — it is now the stroke color."));
            with_editor(|e| {
                let prev = e.tool();
                e.set_tool(Tool::Arrow);
                e.update_style("stroke", |st| st.stroke = color);
                e.set_tool(prev);
            });
        }
        let _ = events & EVENT_COLOR_PICKED;
        let kind_name = s.kind.clone().unwrap_or_else(|| s.tool.name().to_owned());
        self.put_str("Tool", s.tool.name(), |k, v| k.set_tool(v));
        self.put_str("Selection", s.kind.as_deref().unwrap_or(""), |k, v| k.set_selection(v));
        self.put_str("Panel", panel_for(&kind_name), |k, v| k.set_panel(v));
        self.put_str("AmountLabel", amount_label(&kind_name), |k, v| k.set_amount_label(v));
        if self.last_tool != Some(s.tool) || self.last_kind != s.kind {
            self.last_tool = Some(s.tool);
            self.last_kind = s.kind.clone();
            if let Some(sink) = &self.sink {
                let _ = sink.clear_variants();
                for v in &s.variants {
                    let _ = sink.add_variants(v);
                }
            }
            self.shown.strings.remove("Variant");
            self.publish_quick_styles(&kind_name);
        }
        let st = &s.style;
        self.put_str("StrokeColor", &st.stroke.to_hex(), |k, v| k.set_stroke_color(v));
        self.put_str("FillColor", &st.fill.to_hex(), |k, v| k.set_fill_color(v));
        self.put_str("TextColor", &st.text_color.to_hex(), |k, v| k.set_text_color(v));
        self.put_num("StrokeWidth", st.width as f64, |k, v| k.set_stroke_width(v));
        self.put_num("FontSize", st.font_size.round() as f64, |k, v| k.set_font_size(v));
        self.put_str("FontFamily", &st.font, |k, v| k.set_font_family(v));
        self.put_flag("Bold", st.bold, |k, v| k.set_bold(v));
        self.put_flag("Italic", st.italic, |k, v| k.set_italic(v));
        self.put_flag("Shadow", st.shadow, |k, v| k.set_shadow(v));
        self.put_num("Opacity", opacity_percent(st), |k, v| k.set_opacity(v));
        self.put_str("Align", align_name(st.align), |k, v| k.set_align(v));
        self.put_str("Dash", dash_name(st.dash), |k, v| k.set_dash(v));
        self.put_str("StartCap", cap_name(st.start_cap), |k, v| k.set_start_cap(v));
        self.put_str("EndCap", cap_name(st.end_cap), |k, v| k.set_end_cap(v));
        self.put_str("Text", &s.text, |k, v| k.set_text(v));
        self.put_str("Variant", &s.variant, |k, v| k.set_variant(v));
        self.put_num("Amount", (s.amount * 100.0).round() / 100.0, |k, v| k.set_amount(v));
        self.put_num("Rotation", s.rotation.round(), |k, v| k.set_rotation(v));
        let info = format!("{} × {} px · {} objects", s.size.0, s.size.1, s.objects);
        self.put_str("DocumentInfo", &info, |k, v| k.set_document_info(v));
        let name = self
            .path
            .as_ref()
            .or(self.source.as_ref())
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_owned());
        let title = format!("{}{name} — Jiffshot Editor", if s.modified { "• " } else { "" });
        self.put_str("Title", &title, |k, v| k.set_title(v));
        self.mark_unsaved();
        let status = self.status.clone();
        self.put_str("Status", &status, |k, v| k.set_status(v));
        let has_sel = s.kind.is_some();
        self.put_flag("UndoEnabled", s.undo.is_some(), |k, v| k.set_undo_enabled(v));
        self.put_flag("RedoEnabled", s.redo.is_some(), |k, v| k.set_redo_enabled(v));
        for (name, set) in [
            ("DeleteEnabled", MainViewModelSink::set_delete_enabled as fn(&MainViewModelSink, bool) -> Result<()>),
            ("DuplicateEnabled", MainViewModelSink::set_duplicate_enabled),
            ("CutEnabled", MainViewModelSink::set_cut_enabled),
            ("BringToFrontEnabled", MainViewModelSink::set_bring_to_front_enabled),
            ("BringForwardEnabled", MainViewModelSink::set_bring_forward_enabled),
            ("SendBackwardEnabled", MainViewModelSink::set_send_backward_enabled),
            ("SendToBackEnabled", MainViewModelSink::set_send_to_back_enabled),
        ] {
            self.put_flag(name, has_sel, set);
        }
        self.put_str("BackgroundFill", s.background_fill, |k, v| k.set_background_fill(v));
        let theme = self.settings.theme.clone().unwrap_or_else(|| "Dark".to_owned());
        self.put_str("Theme", &theme, |k, v| k.set_theme(v));
        let tools = self.settings.toolbar.join(",");
        self.put_str("ToolOrder", &tools, |k, v| k.set_tool_order(v));
        self.publish_layout();
        let tp = self.settings.template.clone();
        self.put_str("TemplateTheme", combine::theme(&tp.theme).name, |k, v| k.set_template_theme(v));
        self.put_flag("TemplateCaptions", tp.captions, |k, v| k.set_template_captions(v));
        self.put_flag("TemplateNumbers", tp.numbers, |k, v| k.set_template_numbers(v));
        self.put_str("TemplateFooter", &tp.footer, |k, v| k.set_template_footer(v));
        let fit = with_editor(|e| e.selected().and_then(|o| o.zone).is_some_and(|z| z.fit)).unwrap_or(false);
        self.put_flag("ZoneFit", fit, |k, v| k.set_zone_fit(v));
        let vp = self.settings.video.clone();
        self.put_str("GifPreset", &vp.gif_preset, |k, v| k.set_gif_preset(v));
        self.put_flag("GifLoop", vp.gif_loop, |k, v| k.set_gif_loop(v));
        self.put_flag("GifFade", vp.gif_fade, |k, v| k.set_gif_fade(v));
        self.put_flag("GifDither", vp.gif_dither, |k, v| k.set_gif_dither(v));
        self.put_num("VideoVolume", vp.volume, |k, v| k.set_video_volume(v));
        self.put_flag("Muted", vp.muted, |k, v| k.set_muted(v));
        self.put_num("SelectionCount", s.selected_count as f64, |k, v| k.set_selection_count(v as i64));
        if s.selected_count > 1 {
            let note = format!("{} objects selected", s.selected_count);
            self.put_str("Selection", &note, |k, v| k.set_selection(v));
        }
        self.put_num("HeadSize", st.head_size as f64, |k, v| k.set_head_size(v));
        self.put_num("ShadowAngle", st.shadow_angle as f64, |k, v| k.set_shadow_angle(v));
        self.put_num("ShadowDistance", st.shadow_distance as f64, |k, v| k.set_shadow_distance(v));
        self.put_num("ShadowBlur", st.shadow_blur as f64, |k, v| k.set_shadow_blur(v));
        self.put_str("ShadowColor", &st.shadow_color.to_hex(), |k, v| k.set_shadow_color(v));
        self.put_str("OutlineColor", &st.outline.to_hex(), |k, v| k.set_outline_color(v));
        self.put_num("OutlineWidth", st.outline_width as f64, |k, v| k.set_outline_width(v));
        self.put_flag("Underline", st.underline, |k, v| k.set_underline(v));
        self.put_flag("Strike", st.strike, |k, v| k.set_strike(v));
        let valign = match st.valign {
            jiffshot_doc::object::VAlign::Top => "Top",
            jiffshot_doc::object::VAlign::Middle => "Middle",
            jiffshot_doc::object::VAlign::Bottom => "Bottom",
        };
        self.put_str("VAlign", valign, |k, v| k.set_v_align(v));
        self.put_num("Padding", st.padding as f64, |k, v| k.set_padding(v));
        self.put_str("Fill2", &st.fill2.to_hex(), |k, v| k.set_fill2(v));
        self.put_flag("GradientRadial", st.gradient_radial, |k, v| k.set_gradient_radial(v));
        self.put_num("GradientAngle", st.gradient_angle as f64, |k, v| k.set_gradient_angle(v));
        let video = self.video.is_some();
        self.put_str("Mode", if video { "Video" } else { "Image" }, |k, v| k.set_mode(v));
        if events & ed::EVENT_FOCUS_TEXT != 0 {
            self.focus += 1;
            let focus = self.focus;
            if let Some(sink) = &self.sink {
                let _ = sink.set_focus_text(focus);
            }
        }
        if s.damage {
            self.bump_frame();
        }
    }

    /// Tells the canvas to fetch pending damage.
    pub fn bump_frame(&mut self) {
        self.frame += 1;
        if let Some(sink) = &self.sink {
            let _ = sink.set_frame_id(self.frame);
        }
    }

    pub(crate) fn zoom(&mut self, what: &str) {
        self.zoom += 1;
        let request = format!("{what}:{}", self.zoom);
        if let Some(sink) = &self.sink {
            let _ = sink.set_zoom_request(request);
        }
    }

    // ---- documents --------------------------------------------------------

    /// Saves edits of library captures automatically, next to the capture.
    pub fn autosave(&mut self) {
        let modified = with_editor(|e| e.is_modified()).unwrap_or(false);
        if !modified {
            return;
        }
        let target = match (&self.path, &self.source) {
            (Some(p), _) => Some(p.clone()),
            (None, Some(src)) if src.starts_with(&self.library.root) => Some(src.with_extension(EDITS_EXT)),
            _ => None,
        };
        if let Some(path) = target {
            self.save_to(&path);
        }
    }

    fn save_to(&mut self, path: &Path) -> bool {
        let doc = with_editor(|e| e.document().clone());
        let Some(doc) = doc else { return false };
        let is_doc = io::is_document(path);
        let result = if is_doc { io::save_document(&doc, path) } else { io::export(&doc, path) };
        match result {
            Ok(()) => {
                if is_doc {
                    self.path = Some(path.to_path_buf());
                    with_editor(|e| e.mark_saved());
                    library::write_thumbnail(&doc, self.source.as_deref().unwrap_or(path));
                    self.refresh_library();
                }
                self.set_status(format!("Saved {}", path.display()));
                true
            }
            Err(error) => {
                self.set_status(format!("Could not save {}: {error}", path.display()));
                false
            }
        }
    }

    pub(crate) fn load(&mut self, doc: Document, path: Option<PathBuf>, source: Option<PathBuf>) {
        self.autosave();
        self.stop_video();
        let size = doc.size();
        with_editor(|e| e.load(doc));
        self.path = path;
        self.source = source;
        self.zoom("fit");
        self.bump_frame();
        self.mark_current();
        self.set_status(format!("{} × {} px", size.0, size.1));
    }

    /// The Share list: the built-in destinations, the user's programs, and
    /// Add program.
    pub(crate) fn publish_shares(&mut self) {
        let Some(sink) = &self.sink else { return };
        let _ = sink.clear_shares();
        for n in crate::share::BUILT_IN {
            let _ = sink.add_shares(*n);
        }
        for (n, _) in &self.settings.programs {
            let _ = sink.add_shares(n);
        }
        let _ = sink.add_shares("Add program…");
    }

    /// Puts `path` first in File ▸ Open recent.
    fn remember_recent(&mut self, path: &Path) {
        let recent = &mut self.settings.recent;
        recent.retain(|p| p != path);
        recent.insert(0, path.to_path_buf());
        recent.truncate(10);
        self.settings.save();
        self.publish_recent();
    }

    pub(crate) fn publish_recent(&mut self) {
        let Some(sink) = &self.sink else { return };
        let _ = sink.clear_recent_files();
        for path in self.settings.recent.iter().filter(|p| p.exists()) {
            let _ = sink.add_recent_files(file_uri(path));
        }
    }

    pub fn open_recent(&mut self, uri: &str) {
        let path = path_of_uri(uri);
        if path.exists() {
            self.open_path(&path);
        } else {
            self.settings.recent.retain(|p| *p != path);
            self.settings.save();
            self.publish_recent();
            self.set_status(format!("{} is gone", path.display()));
        }
    }

    /// Image ▸ Auto trim: removes edges of one solid color.
    pub fn auto_trim(&mut self) {
        let Some(r) = with_editor(|e| trim_bounds(e.document().background())) else { return };
        match r {
            Some(r) => self.document_op("Auto trim", |d| d.crop(r)),
            None => self.set_status("No solid-colored edges to trim"),
        }
    }

    /// Resize Image: to `w` x `h` pixels.
    pub fn resize_to(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 || w > 16384 || h > 16384 {
            return self.set_status("Sizes go from 1 to 16384 pixels");
        }
        self.document_op("Resize", |d| {
            if d.size() == (w, h) {
                return false;
            }
            d.scale(w, h);
            true
        });
    }

    /// Trim: removes this many pixels from each side.
    pub fn trim(&mut self, left: u32, top: u32, right: u32, bottom: u32) {
        let (w, h) = with_editor(|e| e.document().size()).unwrap_or_default();
        if left + right >= w || top + bottom >= h {
            return self.set_status("That would trim away the whole image");
        }
        let r = jiffshot_doc::IRect::new(left as i32, top as i32, (w - left - right) as i32, (h - top - bottom) as i32);
        self.document_op("Trim", |d| d.crop(r));
    }

    pub fn open_path(&mut self, path: &Path) {
        let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
        if ext == "mp4" {
            return self.open_video(path);
        }
        // A library capture with saved edits opens the edits.
        let is_doc = io::is_document(path);
        let (file, doc_path) = match (!is_doc).then(|| edits_of(path)).flatten() {
            Some(edits) => (edits.clone(), Some(edits)),
            None => (path.to_path_buf(), is_doc.then(|| path.to_path_buf())),
        };
        let started = std::time::Instant::now();
        match io::open(&file) {
            Ok(mut doc) => {
                if doc_path.is_none() {
                    add_cursor_object(&mut doc, path);
                }
                let source = if is_doc { None } else { Some(path.to_path_buf()) };
                self.load(doc, doc_path, source);
                self.remember_recent(path);
                let ms = started.elapsed().as_secs_f64() * 1000.0;
                let (w, h) = with_editor(|e| e.document().size()).unwrap_or_default();
                self.set_status(format!("Opened {} ({w} × {h}) in {ms:.0} ms", file.display()));
            }
            Err(error) => self.set_status(format!("Could not open {}: {error}", file.display())),
        }
    }

    fn window(&self) -> Option<rustolonia::Window> {
        self.scope.main_window()
    }

    /// Runs a file picker and hands the chosen paths to `then` on the UI
    /// thread.
    fn pick_open(&mut self, title: &str, multiple: bool, images_only: bool, then: impl FnOnce(&mut App, Vec<PathBuf>) + Send + 'static) {
        let Some(window) = self.window() else { return };
        let mut filter = FileTypeFilter::new(if images_only { "Images" } else { "Jiffshot and image files" });
        let exts: &[&str] = if images_only { &["png", "jpg", "jpeg", "bmp", "gif", "webp"] } else { io::OPEN_EXTENSIONS };
        for e in exts.iter().chain((!images_only).then_some(&"mp4")) {
            filter = filter.with_extension(e);
        }
        let options = OpenFilePickerOptions::new().title(title).allow_multiple(multiple).start_in(WellKnownFolder::Pictures).file_type(filter);
        let op = match self.scope.open_file_picker(&window, &options) {
            Ok(op) => op,
            Err(error) => return self.set_status(format!("File picker failed: {error}")),
        };
        let _ = self.scope.spawn(async move {
            if let Ok(PickerOutcome::Selected(items)) = op.await {
                let paths: Vec<PathBuf> = items.iter().filter_map(|i| i.local_path().map(Path::to_path_buf)).collect();
                post(move |app| then(app, paths));
            }
        });
    }

    pub(crate) fn pick_save(&mut self, title: &str, name: &str, kinds: &[(&str, &str)], then: impl FnOnce(&mut App, PathBuf) + Send + 'static) {
        let Some(window) = self.window() else { return };
        let mut options = SaveFilePickerOptions::new().title(title).suggested_file_name(name).show_overwrite_prompt(true).start_in(WellKnownFolder::Pictures);
        for (label, ext) in kinds {
            options = options.file_type(FileTypeFilter::new(*label).with_extension(ext));
        }
        if let Some((_, ext)) = kinds.first() {
            options = options.default_extension(*ext);
        }
        let op = match self.scope.save_file_picker(&window, &options) {
            Ok(op) => op,
            Err(error) => return self.set_status(format!("File picker failed: {error}")),
        };
        let _ = self.scope.spawn(async move {
            if let Ok(PickerOutcome::Selected(items)) = op.await {
                if let Some(path) = items.first().and_then(|i| i.local_path().map(Path::to_path_buf)) {
                    post(move |app| then(app, path));
                }
            }
        });
    }

    fn stem(&self) -> String {
        self.path
            .as_ref()
            .or(self.source.as_ref())
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into())
    }

    // ---- commands ----------------------------------------------------------

    pub fn new_image(&mut self) {
        self.load(Document::blank(1280, 720, Rgba::WHITE), None, None);
    }

    pub fn open(&mut self) {
        self.pick_open("Open", false, false, |app, paths| {
            if let Some(p) = paths.first() {
                app.open_path(p);
            }
        });
    }

    pub fn save(&mut self) {
        let target = match (&self.path, &self.source) {
            (Some(p), _) => Some(p.clone()),
            (None, Some(src)) if src.starts_with(&self.library.root) => Some(src.with_extension(EDITS_EXT)),
            _ => None,
        };
        match target {
            Some(path) => {
                self.save_to(&path);
            }
            None => self.save_as(),
        }
    }

    pub fn save_as(&mut self) {
        let name = format!("{}.{EDITS_EXT}", self.stem());
        let kinds = [("Jiffshot document", EDITS_EXT), ("PNG image", "png"), ("JPEG image", "jpg"), ("BMP image", "bmp"), ("GIF image", "gif"), ("WebP image", "webp")];
        self.pick_save("Save as", &name, &kinds, |app, path| {
            app.save_to(&path);
        });
    }

    pub fn export(&mut self) {
        let mut kinds = vec![
            ("PNG image", "png"),
            ("JPEG image", "jpg"),
            ("PDF document", "pdf"),
            ("BMP image", "bmp"),
            ("GIF image", "gif"),
            ("WebP image", "webp"),
        ];
        // The format used last comes first.
        if let Some(i) = self.settings.export_format.as_deref().and_then(|f| kinds.iter().position(|k| k.1 == f)) {
            let k = kinds.remove(i);
            kinds.insert(0, k);
        }
        let name = format!("{}.{}", self.stem(), kinds[0].1);
        self.pick_save("Export image", &name, &kinds, |app, path| {
            let doc = with_editor(|e| e.document().clone());
            let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
            if let Some(doc) = doc {
                match io::export(&doc, &path) {
                    Ok(()) => {
                        app.settings.export_format = Some(ext);
                        app.settings.save();
                        app.set_status(format!("Exported {}", path.display()));
                    }
                    Err(error) => app.set_status(format!("Export failed: {error}")),
                }
            }
        });
    }

    pub fn copy_image(&mut self) {
        let Some(flat) = with_editor(|e| e.document().flatten()) else { return };
        let frame = winutil::frame_from_pixmap(&flat);
        match jiffshot_capture::clipboard::copy(&frame, winutil::main_hwnd()) {
            Ok(()) => self.set_status(format!("Copied the image ({} × {}) to the clipboard", flat.width(), flat.height())),
            Err(error) => self.set_status(format!("Copy failed: {error}")),
        }
    }

    pub fn copy(&mut self) {
        match with_editor(|e| e.selected().cloned()).flatten() {
            Some(o) => {
                self.set_status(format!("Copied the {}", o.kind_name().to_lowercase()));
                self.object_clipboard = Some(o);
            }
            None => self.copy_image(),
        }
    }

    pub fn cut(&mut self) {
        self.copy();
        with_editor(|e| e.delete_selected());
    }

    pub fn paste(&mut self) {
        use jiffshot_capture::clipboard::Pasted;
        match jiffshot_capture::clipboard::read(winutil::main_hwnd()) {
            Ok(Some(Pasted::Image { width, height, rgba })) => {
                let img = image::RgbaImage::from_raw(width, height, rgba).and_then(|i| io::from_rgba_image(&i));
                if let Some(pm) = img {
                    with_editor(|e| e.paste_image(pm));
                    self.object_clipboard = None;
                    self.set_status(format!("Pasted a {width} × {height} image"));
                }
            }
            Ok(Some(Pasted::Files(files))) => {
                let mut n = 0;
                for f in files {
                    if let Ok(pm) = io::load_image(&f) {
                        with_editor(|e| e.paste_image(pm));
                        n += 1;
                    }
                }
                self.set_status(format!("Pasted {n} image file(s)"));
            }
            Ok(Some(Pasted::Text(text))) if self.object_clipboard.is_none() => {
                with_editor(|e| {
                    let (w, h) = e.document().size();
                    let style = Style { ..e.tool_style(Tool::Text) };
                    let rect = jiffshot_doc::Box2::from_xywh(w as f32 * 0.1, h as f32 * 0.1, w as f32 * 0.5, style.font_size * 2.0);
                    let id = e.add_object("Paste text", Kind::Text { rect, text: String::new() }, style);
                    let _ = id;
                    e.set_selected_text(&text);
                });
            }
            _ => match self.object_clipboard.clone() {
                Some(o) => {
                    with_editor(|e| e.paste_object(o));
                }
                None => self.set_status("Nothing to paste"),
            },
        }
    }

    pub fn new_capture(&mut self) {
        match winutil::start_capture() {
            Ok(()) => self.set_status("Select what to capture…"),
            Err(error) => self.set_status(error),
        }
    }

    /// Opens the Capture window; the editor stays open.
    pub fn open_capture_window(&mut self) {
        if let Err(error) = winutil::open_capture_window() {
            self.set_status(error);
        }
    }

    pub fn combine_images(&mut self) {
        self.template("Combine grid");
    }

    /// Images (or a capture to open) dropped at a canvas point:
    /// "x,y|path|path…". Several images fill the next empty drop zones.
    fn drop_files(&mut self, arg: &str) {
        self.canvas_drop = Some(std::time::Instant::now());
        let mut parts = arg.split('|');
        let at = parts.next().and_then(|xy| xy.split_once(',')).and_then(|(x, y)| Some(jiffshot_doc::Pt::new(x.parse().ok()?, y.parse().ok()?)));
        let paths: Vec<PathBuf> = parts.filter(|p| !p.is_empty()).map(PathBuf::from).collect();
        let Some(at) = at else { return };
        let images: Vec<_> = paths.iter().filter_map(|p| io::load_image(p).ok()).collect();
        if images.is_empty() {
            if let Some(first) = paths.first() {
                self.open_path(first);
            }
            return;
        }
        let n = images.len();
        with_editor(|e| {
            let mut images = images.into_iter();
            if let Some(first) = images.next() {
                e.drop_image(at, first);
            }
            for img in images {
                let empty = e.document().objects.iter().find(|o| o.zone.is_some() && matches!(&o.kind, jiffshot_doc::object::Kind::Image { image, .. } if image.pixmap.is_none())).map(|o| o.id);
                match empty {
                    Some(id) => e.fill_zone(id, img),
                    None => {
                        e.drop_image(jiffshot_doc::Pt::new(at.x + 24.0, at.y + 24.0), img);
                    }
                }
            }
        });
        self.set_status(format!("Placed {n} image{}", if n == 1 { "" } else { "s" }));
    }

    /// Chooses a picture for the selected drop zone.
    fn pick_zone_image(&mut self) {
        let zone = with_editor(|e| e.selected().filter(|o| o.zone.is_some()).map(|o| o.id)).flatten();
        let Some(id) = zone else { return };
        self.pick_open("Choose an image for the drop zone", false, true, move |app, paths| {
            if let Some(img) = paths.first().and_then(|p| io::load_image(p).ok()) {
                with_editor(|e| e.fill_zone(id, img));
                app.bump_frame();
            }
        });
    }

    fn template_options(&self) -> combine::Options {
        let t = &self.settings.template;
        combine::Options {
            theme: combine::theme(&t.theme),
            captions: t.captions,
            numbers: t.numbers,
            footer: Some(t.footer.clone()).filter(|f| !f.trim().is_empty()),
        }
    }

    /// Changes a template choice; the next template uses it.
    pub fn template_prefs(&mut self, change: impl FnOnce(&mut crate::settings::TemplatePrefs)) {
        change(&mut self.settings.template);
        self.settings.save();
        schedule_sync();
    }

    fn template(&mut self, name: &str) {
        let name = name.to_owned();
        // Captures picked in the library go in; otherwise templates start
        // with empty drop zones and combining asks for images.
        let picked: Vec<_> = self.library_items.iter().filter(|i| self.library_selected.contains(&i.path)).filter_map(|i| io::load_image(&i.path).ok()).collect();
        if !name.starts_with("Combine") && (!picked.is_empty() || self.library_selected.is_empty()) {
            if let Some(doc) = combine::template_with(&name, &picked, &self.template_options()) {
                self.load(doc, None, None);
                with_editor(|e| e.checkpoint("Template", None));
                self.set_status(format!("{name}: drop images from the tray or Explorer onto the drop zones"));
            }
            return;
        }
        if name.starts_with("Combine") && picked.len() >= 2 {
            let arrangement = match name.as_str() {
                "Combine side by side" => combine::Arrangement::Horizontal,
                "Combine stacked" => combine::Arrangement::Vertical,
                _ => combine::Arrangement::Grid,
            };
            if let Some(doc) = combine::combine(&picked, arrangement, 24.0, Rgba::WHITE) {
                self.load(doc, None, None);
                with_editor(|e| e.checkpoint("Combine", None));
            }
            return;
        }
        let options = self.template_options();
        self.pick_open("Choose images", true, true, move |app, paths| {
            let images: Vec<_> = paths.iter().filter_map(|p| io::load_image(p).ok()).collect();
            if images.is_empty() {
                return;
            }
            let doc = match name.as_str() {
                "Combine side by side" => combine::combine(&images, combine::Arrangement::Horizontal, 24.0, Rgba::WHITE),
                "Combine stacked" => combine::combine(&images, combine::Arrangement::Vertical, 24.0, Rgba::WHITE),
                "Combine grid" => combine::combine(&images, combine::Arrangement::Grid, 24.0, Rgba::WHITE),
                other => combine::template_with(other, &images, &options),
            };
            if let Some(doc) = doc {
                app.load(doc, None, None);
                with_editor(|e| e.checkpoint("Template", None));
                app.set_status(format!("{name}: {} image(s)", images.len()));
            }
        });
    }

    pub fn open_library_folder(&mut self) {
        let _ = std::fs::create_dir_all(&self.library.root);
        winutil::open_folder(&self.library.root);
    }

    /// Ends Jiffshot: the editor's edits are saved, the capture side stops,
    /// then the application. The capture side's own Exit comes back here.
    pub fn exit(&mut self) {
        if std::mem::replace(&mut self.exiting, true) {
            return;
        }
        self.autosave();
        self.stop_video();
        jiffshot_workflow::stop_engine();
        let _ = self.scope.shutdown();
    }

    /// The editor window was closed: Jiffshot keeps running in the tray.
    pub fn hidden(&mut self) {
        self.autosave();
        self.stop_video();
    }

    /// Shows a notice from the capture side as a toast (the `toast` signal).
    pub fn toast(&mut self, title: &str, text: &str, problem: bool) {
        self.toasts += 1;
        let value = format!("{}\u{1f}{title}\u{1f}{text}\u{1f}{}", self.toasts, problem as u8);
        self.put_str("Toast", &value, |k, v| k.set_toast(v));
    }

    pub fn show_capture_window(&mut self) {
        crate::capture_window::show(&self.scope);
    }

    /// Shows Grab Text's result in its window (the `grab_text` signal).
    pub fn show_grab_text(&mut self, text: &str) {
        self.toasts += 1;
        let value = format!("{}\u{1f}{text}", self.toasts);
        self.put_str("GrabText", &value, |k, v| k.set_grab_text(v));
    }

    /// The selection overlay to show, or empty (the `overlay` signal).
    pub fn set_overlay(&mut self, json: &str) {
        self.put_str("Overlay", json, |k, v| k.set_overlay(v));
    }

    /// Screen Draw to show, or empty (the `screen_draw` signal).
    pub fn set_screen_draw(&mut self, json: &str) {
        self.put_str("ScreenDraw", json, |k, v| k.set_screen_draw(v));
    }

    /// Opens the Preferences dialog on `tab` (the `prefs` signal).
    pub fn open_prefs(&mut self, tab: &str) {
        self.toasts += 1;
        let value = format!("{}\u{1f}{}", self.toasts, crate::prefs::dialog(self, tab));
        self.put_str("Prefs", &value, |k, v| k.set_prefs(v));
    }

    /// The capture side's panels and outlines (the `hud` signal).
    pub fn set_hud(&mut self, json: &str) {
        self.put_str("Hud", json, |k, v| k.set_hud(v));
    }

    /// The Capture Widget follows the capture side's setting.
    pub fn set_widget(&mut self, on: bool) {
        self.put_flag("Widget", on, |k, v| k.set_widget(v));
    }

    /// Shows the editor window and brings it forward.
    pub fn show_editor(&mut self) {
        if let Some(window) = self.scope.main_window() {
            let _ = window.show();
        }
        winutil::bring_to_front();
        self.refresh_library();
    }

    pub fn save_quick_style(&mut self) {
        let Some((tool, style)) = with_editor(|e| (e.selected().map(ed::tool_for).unwrap_or(e.tool()), e.current_style())) else { return };
        self.settings.add_quick_style(tool.name(), style);
        self.settings.save();
        self.last_kind = Some("refresh".into());
        self.set_status(format!("Saved a quick style for {}", tool.name()));
    }

    fn publish_quick_styles(&mut self, kind: &str) {
        let _t = crate::trace::span("publish_quick_styles");
        let tool = Tool::from_name(kind).unwrap_or(Tool::Select);
        let styles = self.settings.quick_styles(tool);
        // Favorites first, each group in the list's order.
        let mut order: Vec<(usize, Style, bool)> = styles.into_iter().enumerate().map(|(i, s)| { let f = self.settings.is_favorite(tool, &s); (i, s, f) }).collect();
        order.sort_by_key(|(_, _, favorite)| !favorite);
        let Some(sink) = &self.sink else { return };
        let _ = sink.clear_quick_styles();
        for (index, style, favorite) in order {
            let _ = sink.add_quick_styles(QuickStyle { index, tool, style, favorite, sink: None });
        }
    }

    /// "up:Tool", "down:Tool", "hide:Tool", "show:Tool" or "reset"; the
    /// toolbar keeps at least one tool. `all` is every tool in the usual
    /// order, as the window lists them.
    fn toolbar(&mut self, arg: &str) {
        const ALL: &[&str] = &[
            "Select", "Arrow", "Line", "Shape", "Text", "Callout", "Step", "Pen", "Highlight", "Blur", "Magnify", "Spotlight", "Stamp", "Fill",
            "Crop", "Cut Out", "Selection", "Magic Wand", "Eraser", "Eyedropper", "Smart Move", "Text Replace",
        ];
        let mut order: Vec<String> = if self.settings.toolbar.is_empty() { ALL.iter().map(|s| s.to_string()).collect() } else { self.settings.toolbar.clone() };
        let (verb, name) = arg.split_once(':').unwrap_or((arg, ""));
        let at = order.iter().position(|t| t == name);
        match (verb, at) {
            ("reset", _) => order.clear(),
            ("up", Some(i)) if i > 0 => order.swap(i - 1, i),
            ("down", Some(i)) if i + 1 < order.len() => order.swap(i, i + 1),
            ("hide", Some(i)) if order.len() > 1 => {
                order.remove(i);
                self.set_status(format!("{name} hidden; View ▸ Show all tools brings it back"));
            }
            ("show", None) if ALL.contains(&name) => order.push(name.to_owned()),
            _ => return,
        }
        self.settings.toolbar = order;
        self.settings.save();
        let joined = self.settings.toolbar.join(",");
        self.put_str("ToolOrder", &joined, |k, v| k.set_tool_order(v));
    }

    /// Deletes, moves or (un)favorites the quick style at `index`.
    pub fn edit_quick_style(&mut self, tool: Tool, index: usize, style: &Style, what: &str) {
        match what {
            "favorite" => self.settings.toggle_favorite(tool, style),
            "delete" => self.settings.edit_styles(tool, |l| {
                if index < l.len() {
                    l.remove(index);
                }
            }),
            "earlier" if index > 0 => self.settings.edit_styles(tool, |l| {
                if index < l.len() {
                    l.swap(index - 1, index);
                }
            }),
            "later" => self.settings.edit_styles(tool, |l| {
                if index + 1 < l.len() {
                    l.swap(index, index + 1);
                }
            }),
            _ => return,
        }
        self.settings.save();
        self.publish_quick_styles(tool.name());
    }

    /// The current tool's (or selection's) quick styles from a theme, or
    /// back to the built-in ones.
    fn style_theme(&mut self, name: &str) {
        let Some(tool) = with_editor(|e| e.selected().map(ed::tool_for).unwrap_or(e.tool())) else { return };
        if name == "reset" {
            self.settings.reset_styles(tool);
            self.set_status(format!("{} quick styles reset", tool.name()));
        } else {
            let t = combine::theme(name);
            let styles = crate::settings::Settings::themed_styles(tool, &[t.accent, t.ink, t.muted, t.frame]);
            self.settings.edit_styles(tool, |l| *l = styles);
            self.set_status(format!("{} quick styles in the {} theme", tool.name(), t.name));
        }
        self.settings.save();
        self.publish_quick_styles(tool.name());
    }

    pub fn apply_quick_style(&mut self, tool: Tool, style: Style) {
        with_editor(|e| {
            let font = e.current_style().font;
            if e.selected().is_none() && e.tool() != tool {
                e.set_tool(tool);
            }
            let apply = style.clone();
            e.update_style("quickstyle", move |s| {
                *s = Style { font: font.clone(), ..apply.clone() };
            });
            e.seal();
        });
    }

    pub fn document_op(&mut self, label: &str, op: impl FnOnce(&mut Document) -> bool) {
        let done = with_editor(|e| e.edit_document(label, op)).unwrap_or(false);
        if done {
            self.zoom("fit");
        }
    }

    pub fn effect(&mut self, name: &str) {
        let style = with_editor(|e| e.current_style()).unwrap_or_default();
        if name == "Watermark" {
            return self.pick_open("Choose a watermark image", false, true, |app, paths| {
                if let Some(image) = paths.first().and_then(|p| io::load_image(p).ok()) {
                    let effect = effects::Effect::Watermark { image: Arc::new(image), opacity: 0.35 };
                    app.document_op("Watermark", |d| effects::apply(d, effect));
                    app.set_status("Watermark added");
                }
            });
        }
        if name == "Spotlight & magnify" {
            with_editor(|e| e.spotlight_magnify(1.6));
            self.bump_frame();
            return self.set_status("Spotlight & magnify: the selected area (or the middle) is enlarged; draw a rectangle first to choose it.");
        }
        if name == "Capture info" {
            let text = self.capture_info();
            let effect = effects::Effect::CaptureInfo { text };
            self.document_op(name, |d| effects::apply(d, effect));
            return;
        }
        match effects::by_name_for(name, style.stroke, style.fill) {
            Some(effect) => {
                let started = std::time::Instant::now();
                self.document_op(name, |d| effects::apply(d, effect));
                self.set_status(format!("{name} applied in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0));
            }
            None => self.set_status(format!("Unknown effect {name}")),
        }
    }

    /// When and where the capture was taken, from its sidecar file.
    fn capture_info(&self) -> String {
        let meta = self
            .source
            .as_ref()
            .and_then(|s| std::fs::read(s.with_extension("json")).ok())
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
        let mut parts = Vec::new();
        if let Some(m) = &meta {
            if let Some(t) = m["captured_at"].as_str() {
                parts.push(t.replace('T', " "));
            }
            if let Some(t) = m["window_title"].as_str() {
                parts.push(t.to_owned());
            }
            if let Some(p) = m["process"].as_str() {
                parts.push(p.to_owned());
            }
        }
        if parts.is_empty() {
            parts.push(jiffshot_capture::library::timestamp().replace('T', " "));
        }
        parts.push("Windows".to_owned());
        parts.join("  ·  ")
    }

    pub fn resize(&mut self, factor: f32) {
        self.document_op("Resize", |d| {
            let (w, h) = d.size();
            let (nw, nh) = (((w as f32 * factor).round() as u32).max(1), ((h as f32 * factor).round() as u32).max(1));
            if nw > 16384 || nh > 16384 {
                return false;
            }
            d.scale(nw, nh);
            true
        });
    }

    pub fn set_tool(&mut self, tool: Tool) {
        with_editor(|e| {
            e.select(None);
            e.set_tool(tool);
        });
        let hint = match tool {
            Tool::Crop => "Drag to crop the image.",
            Tool::CutOut => "Drag across the part to remove; the rest closes the gap.",
            Tool::Selection => "Drag to select part of the image, then move it.",
            Tool::SmartMove => "Drag around an object to lift it off the background.",
            Tool::TextReplace => "Drag around text to make it editable.",
            Tool::Fill => "Click an area to fill it.",
            Tool::Eyedropper => "Click to pick a color.",
            _ => "",
        };
        if !hint.is_empty() {
            self.set_status(hint);
        }
    }

    /// `Run` command: actions from toolbar buttons and panel lists.
    pub fn run(&mut self, action: &str) {
        let (verb, arg) = action.split_once(':').unwrap_or((action, ""));
        match verb {
            "tool" => {
                if let Some(t) = Tool::from_name(arg) {
                    self.set_tool(t);
                }
            }
            "effect" => self.effect(arg),
            "template" => self.template(arg),
            "zoom" => self.zoom(arg),
            "open" => self.open_path(Path::new(arg)),
            "library" => self.refresh_library(),
            "trim" => self.video_action(arg),
            "finish" => {
                with_editor(|e| e.finish_polygon());
            }
            "add-stamp" => self.add_stamp(),
            "drop" => self.drop_files(arg),
            "style-theme" => self.style_theme(arg),
            "toolbar" => self.toolbar(arg),
            "zone-fit" => {
                with_editor(|e| e.set_zone_fit(arg == "1"));
            }
            "zone-swap" => {
                if !with_editor(|e| e.swap_zones()).unwrap_or(false) {
                    self.set_status("Shift+click two drop zones, then Swap.");
                }
            }
            "tail" => {
                if !with_editor(|e| e.callout_tail(arg != "remove")).unwrap_or(false) {
                    self.set_status("Select a callout first.");
                }
            }
            "zone-empty" => {
                with_editor(|e| e.empty_zone());
            }
            "zone-pick" => self.pick_zone_image(),
            "share" => self.share(arg),
            "hidden" => self.hidden(),
            // The Preferences dialog: open it on a tab; apply what it sends.
            "prefs" => self.open_prefs(if arg.is_empty() { "capture" } else { arg }),
            "prefs-apply" => crate::prefs::apply(self, arg),
            // A capture panel's button ("ID:INDEX"; no index when closed), or
            // a toolbar's action ("ID:act:NAME").
            "hud" => {
                let mut parts = arg.splitn(3, ':');
                if let Some(id) = parts.next().and_then(|id| id.parse().ok()) {
                    match (parts.next(), parts.next()) {
                        (Some("act"), Some(action)) => jiffshot_workflow::panel_action(id, action),
                        (index, _) => jiffshot_workflow::answer_panel(id, index.and_then(|i| i.parse().ok())),
                    }
                }
            }
            // The Capture Widget's buttons.
            "widget" => {
                let mode = match arg {
                    "video" => jiffshot_workflow::MODE_VIDEO,
                    "scrolling" => jiffshot_workflow::MODE_SCROLLING,
                    "text" => jiffshot_workflow::MODE_TEXT,
                    _ => jiffshot_workflow::MODE_IMAGE,
                };
                jiffshot_workflow::request_capture(mode, 1);
            }
            "resize" | "trim-image" => {
                let n: Vec<u32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                match (verb, n.as_slice()) {
                    ("resize", &[w, h]) => self.resize_to(w, h),
                    ("trim-image", &[l, t, r, b]) => self.trim(l, t, r, b),
                    _ => {}
                }
            }
            // Editor Preferences.
            "theme" if matches!(arg, "Dark" | "Light" | "System") => {
                self.settings.set_theme(arg);
            }
            "export-format" => {
                self.settings.export_format = Some(arg.to_ascii_lowercase()).filter(|f| !f.is_empty());
                self.settings.save();
            }
            "ftp" => {
                self.settings.ftp = Some(arg.trim().to_owned()).filter(|u| !u.is_empty());
                self.settings.save();
            }
            "program-remove" => {
                self.settings.programs.retain(|(n, _)| n != arg);
                self.settings.save();
                self.publish_shares();
            }
            "library-folder" if !arg.is_empty() => {
                let root = PathBuf::from(arg);
                self.settings.set_library(root.clone());
                self.library = Library::new(root);
                self.show_library();
                self.set_status(format!("Library: {arg}"));
            }
            "align" => {
                with_editor(|e| e.align_selected(arg));
            }
            "deselect" => {
                with_editor(|e| e.select(None));
            }
            "export-to" => {
                let doc = with_editor(|e| e.document().clone());
                if let Some(doc) = doc {
                    match io::export(&doc, Path::new(arg)) {
                        Ok(()) => self.set_status(format!("Exported {arg}")),
                        Err(error) => self.set_status(format!("Export failed: {error}")),
                    }
                }
            }
            _ => self.set_status(format!("Unknown action {action}")),
        }
    }

    // ---- property edits from the panel ---------------------------------

    pub fn style_edit(&mut self, property: &'static str, change: impl Fn(&mut Style)) {
        with_editor(|e| e.update_style(property, change));
    }

    pub fn set_text(&mut self, value: &str) {
        with_editor(|e| e.set_selected_text(value));
    }

    pub fn set_variant(&mut self, value: &str) {
        if value.is_empty() {
            return;
        }
        with_editor(|e| e.set_variant(value));
    }

    pub fn set_amount(&mut self, value: f64) {
        with_editor(|e| {
            if e.selection().is_some() {
                e.set_selected_amount(value as f32);
            } else {
                let t = e.tool();
                e.set_tool_amount(t, value as f32);
            }
        });
    }

    pub fn set_rotation(&mut self, value: f64) {
        with_editor(|e| e.set_selected_rotation(value as f32));
    }

    pub fn document_expand(&mut self) {
        if with_editor(|e| e.expand_to_fit()).unwrap_or(false) {
            self.zoom("fit");
        } else {
            self.set_status("Every object already fits on the canvas");
        }
    }

    // ---- sharing -----------------------------------------------------------

    /// The flattened image as a temporary file named after the capture.
    fn temp_export(&self, ext: &str) -> std::result::Result<PathBuf, String> {
        let doc = with_editor(|e| e.document().clone()).ok_or("no document")?;
        let dir = std::env::temp_dir().join("Jiffshot share");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}.{ext}", self.stem()));
        io::export(&doc, &path)?;
        Ok(path)
    }

    pub fn share(&mut self, to: &str) {
        use crate::share;
        let office = |exe: &'static str| move |p: &Path| share::open_in(exe, p);
        let result: std::result::Result<String, String> = (|| match to {
            "Open with…" => share::open_with(&self.temp_export("png")?).map(|_| "Choose a program".into()),
            "Email" => share::email(&self.temp_export("png")?, &self.stem()).map(|_| "Opened a new message".into()),
            "Word" => office("winword.exe")(&self.temp_export("png")?).map(|_| "Opened in Word".into()),
            "PowerPoint" => office("powerpnt.exe")(&self.temp_export("png")?).map(|_| "Opened in PowerPoint".into()),
            "Excel" => office("excel.exe")(&self.temp_export("png")?).map(|_| "Opened in Excel".into()),
            "OneNote" => office("onenote.exe")(&self.temp_export("png")?).map(|_| "Opened in OneNote".into()),
            "Paint" => office("mspaint.exe")(&self.temp_export("png")?).map(|_| "Opened in Paint".into()),
            "Print" => share::print(&self.temp_export("png")?).map(|_| "Sent to the printer dialog".into()),
            "FTP" => match self.settings.ftp.clone() {
                Some(url) => share::ftp(&url, &self.temp_export("png")?).map(|at| format!("Uploaded to {at}")),
                None => Err("Set \"ftp\": \"ftp://user@host/folder\" in %APPDATA%\\Jiffshot\\editor.json, and the password as the \"Jiffshot FTP\" credential (cmdkey /generic:\"Jiffshot FTP\" /user:USER /pass)".into()),
            },
            "Folder…" => {
                let file = self.temp_export("png")?;
                let Some(window) = self.window() else { return Err("no window".into()) };
                let options = rustolonia::storage::FolderPickerOptions::new().title("Share to folder");
                let op = self.scope.open_folder_picker(&window, &options).map_err(|e| e.to_string())?;
                let _ = self.scope.spawn(async move {
                    if let Ok(PickerOutcome::Selected(items)) = op.await {
                        if let Some(dir) = items.first().and_then(|i| i.local_path().map(Path::to_path_buf)) {
                            let result = share::to_folder(&file, &dir);
                            post(move |app| match result {
                                Ok(p) => app.set_status(format!("Copied to {}", p.display())),
                                Err(e) => app.set_status(format!("Copy failed: {e}")),
                            });
                        }
                    }
                });
                Ok("Choose a folder".into())
            }
            "Add program…" => {
                self.pick_program();
                Ok("Choose a program".into())
            }
            name => match self.settings.programs.iter().find(|(n, _)| n == name).map(|(_, p)| p.clone()) {
                Some(exe) => share::open_in(&exe.to_string_lossy(), &self.temp_export("png")?).map(|_| format!("Opened in {name}")),
                None => Err(format!("Unknown destination {name}")),
            },
        })();
        match result {
            Ok(note) => self.set_status(note),
            Err(error) => self.set_status(format!("Share failed: {error}")),
        }
    }

    /// Asks for one or more files of the given extensions.
    pub(crate) fn pick_files(&mut self, title: &str, multiple: bool, label: &str, exts: &[&str], then: impl FnOnce(&mut App, Vec<PathBuf>) + Send + 'static) {
        let Some(window) = self.window() else { return };
        let mut filter = FileTypeFilter::new(label);
        for e in exts {
            filter = filter.with_extension(e);
        }
        let options = OpenFilePickerOptions::new().title(title).allow_multiple(multiple).start_in(WellKnownFolder::Videos).file_type(filter);
        let op = match self.scope.open_file_picker(&window, &options) {
            Ok(op) => op,
            Err(error) => return self.set_status(format!("File picker failed: {error}")),
        };
        let _ = self.scope.spawn(async move {
            if let Ok(PickerOutcome::Selected(items)) = op.await {
                let paths: Vec<PathBuf> = items.iter().filter_map(|i| i.local_path().map(Path::to_path_buf)).collect();
                post(move |app| then(app, paths));
            }
        });
    }

    fn pick_program(&mut self) {
        let Some(window) = self.window() else { return };
        let options = OpenFilePickerOptions::new().title("Add a program to share to").file_type(FileTypeFilter::new("Programs").with_extension("exe"));
        let Ok(op) = self.scope.open_file_picker(&window, &options) else { return };
        let _ = self.scope.spawn(async move {
            if let Ok(PickerOutcome::Selected(items)) = op.await {
                if let Some(exe) = items.first().and_then(|i| i.local_path().map(Path::to_path_buf)) {
                    post(move |app| {
                        let name = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                        app.settings.programs.retain(|(n, _)| *n != name);
                        app.settings.programs.push((name.clone(), exe));
                        app.settings.save();
                        app.publish_shares();
                        app.set_status(format!("{name} added to Share"));
                    });
                }
            }
        });
    }

    // ---- custom stamps ---------------------------------------------------

    fn stamps_dir() -> Option<PathBuf> {
        jiffshot_capture::paths::config_dir().map(|d| d.join("Stamps"))
    }

    /// Loads the stamps in `%APPDATA%\Jiffshot\Stamps` into the Stamp tool.
    pub fn load_stamps(&mut self) {
        let Some(dir) = Self::stamps_dir() else { return };
        let mut stamps = Vec::new();
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
            if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp") {
                continue;
            }
            if let (Ok(pm), Some(stem)) = (io::load_image(&path), path.file_stem()) {
                stamps.push((stem.to_string_lossy().into_owned(), Arc::new(pm)));
            }
        }
        stamps.sort_by(|a, b| a.0.cmp(&b.0));
        with_editor(|e| e.custom_stamps = stamps);
        self.last_kind = Some("refresh".into());
    }

    /// Copies image files into the stamps folder.
    pub fn add_stamp(&mut self) {
        self.pick_open("Add stamps", true, true, |app, paths| {
            let Some(dir) = App::stamps_dir() else { return };
            let _ = std::fs::create_dir_all(&dir);
            let mut n = 0;
            for p in &paths {
                if let Some(name) = p.file_name() {
                    if std::fs::copy(p, dir.join(name)).is_ok() {
                        n += 1;
                    }
                }
            }
            app.load_stamps();
            app.set_status(format!("Added {n} stamp(s); pick them in the Stamp tool's type list"));
        });
    }

    // ---- selection and image commands -------------------------------------

    pub fn publish_narration(&mut self) {
        let on = self.narration.is_some();
        self.put_flag("Narrating", on, |k, v| k.set_narrating(v));
    }

    /// Changes a saved video choice and applies it.
    pub fn video_prefs(&mut self, change: impl FnOnce(&mut crate::settings::VideoPrefs)) {
        change(&mut self.settings.video);
        self.settings.save();
        crate::video::apply_volume(self);
    }

    /// Dark → Light → System (follows Windows) → Dark.
    pub fn cycle_theme(&mut self) {
        let next = match self.settings.theme.as_deref().unwrap_or("Dark") {
            "Dark" => "Light",
            "Light" => "System",
            _ => "Dark",
        };
        self.settings.set_theme(next);
        self.set_status(format!("Theme: {next}"));
    }

    pub fn set_background_fill(&mut self, value: &str) {
        with_editor(|e| e.background_fill = jiffshot_doc::editor::BackgroundFill::from_name(value));
    }

    pub fn smart_redact(&mut self) {
        let Some(flat) = with_editor(|e| e.document().flatten()) else { return };
        let lines = match crate::ocr::recognize(&flat) {
            Ok(lines) => lines,
            Err(error) => return self.set_status(format!("Text recognition failed: {error}")),
        };
        let found = jiffshot_doc::redact::areas(&lines);
        if found.is_empty() {
            return self.set_status("Smart Redact found nothing sensitive");
        }
        let mut kinds: Vec<&str> = found.iter().map(|(_, k)| k.name()).collect();
        kinds.sort();
        kinds.dedup();
        let count = found.len();
        with_editor(|e| {
            let style = Style { fill: e.tool_style(Tool::Blur).fill.with_alpha(255), width: 0.0, ..Style::default() };
            let mut ids = Vec::new();
            e.edit_document("Smart Redact", |d| {
                for (area, _) in &found {
                    let rect = area.to_box().inflate(3.0);
                    ids.push(d.add(Kind::Blur { rect, mode: jiffshot_doc::object::BlurMode::Redact, amount: 10.0 }, style.clone()));
                }
                true
            });
            e.set_selection(ids);
        });
        self.set_status(format!("Smart Redact covered {count} item(s): {}. Delete a box to reveal what is under it.", kinds.join(", ")));
    }

    pub fn new_from_clipboard(&mut self) {
        use jiffshot_capture::clipboard::Pasted;
        match jiffshot_capture::clipboard::read(winutil::main_hwnd()) {
            Ok(Some(Pasted::Image { width, height, rgba })) => {
                if let Some(pm) = image::RgbaImage::from_raw(width, height, rgba).and_then(|i| io::from_rgba_image(&i)) {
                    self.load(Document::new(pm), None, None);
                    self.set_status(format!("New image from the clipboard ({width} × {height})"));
                }
            }
            Ok(Some(Pasted::Files(files))) => match files.first() {
                Some(f) => self.open_path(f),
                None => self.set_status("The clipboard has no image"),
            },
            _ => self.set_status("The clipboard has no image"),
        }
    }

    /// Prints the flattened image through the system's photo printing.
    pub fn print(&mut self) {
        let Some(doc) = with_editor(|e| e.document().clone()) else { return };
        let path = std::env::temp_dir().join(format!("{} (print).png", self.stem()));
        match io::export(&doc, &path) {
            Ok(()) if winutil::print_file(&path) => self.set_status("Sent to the printer dialog"),
            Ok(()) => self.set_status("Windows has no print handler for PNG images"),
            Err(error) => self.set_status(format!("Print failed: {error}")),
        }
    }

    /// Closes the document and shows the next capture in the library.
    pub fn close_file(&mut self) {
        self.autosave();
        let current = self.source.clone();
        let next = self.library_items.iter().map(|i| i.path.clone()).find(|p| Some(p) != current.as_ref());
        match next {
            Some(p) => self.open_path(&p),
            None => self.new_image(),
        }
    }

    /// Copies the current capture (with its edits) as a new library item.
    pub fn duplicate_file(&mut self) {
        let Some(doc) = with_editor(|e| e.document().clone()) else { return };
        let folder = self.source.as_ref().and_then(|s| s.parent().map(Path::to_path_buf)).unwrap_or_else(|| self.library.root.join("Copies"));
        let mut path = folder.join(format!("{} copy.png", self.stem()));
        let mut n = 2;
        while path.exists() {
            path = folder.join(format!("{} copy {n}.png", self.stem()));
            n += 1;
        }
        let result = std::fs::create_dir_all(&folder)
            .map_err(|e| e.to_string())
            .and_then(|_| io::write_atomic(&path, &io::encode(doc.background(), io::Format::Png, 0)?))
            .and_then(|_| io::save_document(&doc, &path.with_extension(EDITS_EXT)));
        match result {
            Ok(()) => {
                self.refresh_library();
                self.open_path(&path);
                self.set_status(format!("Duplicated as {}", path.display()));
            }
            Err(error) => self.set_status(format!("Duplicate failed: {error}")),
        }
    }

    /// Moves the current capture and its edits to the Recycle Bin.
    pub fn delete_file(&mut self) {
        let Some(src) = self.source.clone().filter(|s| s.starts_with(&self.library.root)) else {
            return self.set_status("Only library captures can be deleted here");
        };
        with_editor(|e| e.mark_saved());
        let files: Vec<PathBuf> = ["png", "mp4", EDITS_EXT, "json", "cursor.png", "cursor.json"].iter().map(|x| src.with_extension(x)).filter(|p| p.exists()).collect();
        if winutil::recycle(&files) {
            self.source = None;
            self.path = None;
            self.refresh_library();
            self.close_file();
            self.set_status(format!("Moved {} to the Recycle Bin", src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
        } else {
            self.set_status("Could not move the capture to the Recycle Bin");
        }
    }

    // ---- library ---------------------------------------------------------

    /// Reads the library from disk and shows it.
    pub fn refresh_library(&mut self) {
        self.library_entries = self.library.recent(2000).into_iter().map(library::Entry::read).collect();
        crate::tray::set_recent(self.library_entries.iter().take(10).map(|e| file_uri(&e.path)).collect());
        self.show_library();
    }

    /// Shows the library entries already read, searched, filtered and
    /// sorted.
    fn show_library(&mut self) {
        let entries = std::mem::take(&mut self.library_entries);
        self.show_entries(&entries);
        self.library_entries = entries;
    }

    fn show_entries(&mut self, entries: &[library::Entry]) {
        let view = self.settings.library_view.clone();
        // The tray leaves out what was closed from it; the library view
        // shows everything.
        let closed: std::collections::HashSet<&PathBuf> = view.closed.iter().collect();
        let visible: Vec<library::Entry> = entries.iter().filter(|e| view.expanded || !closed.contains(&e.path)).cloned().collect();
        let shown = library::select(&visible, &view.search, &view.filter, &view.sort, std::time::SystemTime::now());
        let limit = if view.expanded { 600 } else { 120 };
        self.library_selected.retain(|p| shown.iter().any(|e| &e.path == p));
        self.library_items = shown
            .into_iter()
            .take(limit)
            .map(|e| {
                let (current, selected) = (self.source.as_deref() == Some(e.path.as_path()), self.library_selected.contains(&e.path));
                library::Item::new(e, current, selected, view.thumb * 88.0 / 120.0)
            })
            .collect();
        let filters = library::filters(entries);
        if let Some(sink) = &self.sink {
            let _ = sink.clear_library();
            for item in &self.library_items {
                let _ = sink.add_library(item.clone());
            }
            if self.shown_filters != filters {
                // A new list clears the box's choice: show it again.
                self.shown.strings.remove("LibraryFilter");
                self.shown.strings.remove("LibrarySort");
                let _ = sink.clear_library_filters();
                for f in &filters {
                    let _ = sink.add_library_filters(f);
                }
                if self.shown_filters.is_empty() {
                    let _ = sink.clear_library_sorts();
                    for s in library::SORTS {
                        let _ = sink.add_library_sorts(*s);
                    }
                }
            }
        }
        self.shown_filters = filters;
        let hidden = if view.expanded { 0 } else { closed.len() };
        let mut info = format!("{} of {} captures in {}", self.library_items.len(), entries.len(), self.library.root.display());
        if hidden > 0 {
            info += &format!(" · {hidden} closed from the tray");
        }
        self.put_str("LibraryInfo", &info, |k, v| k.set_library_info(v));
        self.put_str("LibrarySearch", &view.search, |k, v| k.set_library_search(v));
        self.put_str("LibraryFilter", &view.filter, |k, v| k.set_library_filter(v));
        self.put_str("LibrarySort", &view.sort, |k, v| k.set_library_sort(v));
        self.put_num("ThumbSize", view.thumb, |k, v| k.set_thumb_size(v));
        self.put_flag("LibraryExpanded", view.expanded, |k, v| k.set_library_expanded(v));
        let count = self.library_selected.len() as f64;
        self.put_num("LibrarySelected", count, |k, v| k.set_library_selected(v as i64));
        self.mark_unsaved();
        library::make_thumbnails(self.library_items.clone());
    }

    /// Changes how the library is shown and refreshes it.
    /// Changes which parts of the window show, and remembers it.
    pub fn layout(&mut self, change: impl FnOnce(&mut crate::settings::Layout)) {
        change(&mut self.settings.layout);
        self.settings.save();
        self.publish_layout();
    }

    fn publish_layout(&mut self) {
        let l = self.settings.layout.clone();
        self.put_flag("ShowSideBar", l.side_bar, |k, v| k.set_show_side_bar(v));
        self.put_flag("ShowEffects", l.effects, |k, v| k.set_show_effects(v));
        self.put_flag("ShowTray", l.tray, |k, v| k.set_show_tray(v));
        self.put_flag("ToolLabels", l.labels, |k, v| k.set_tool_labels(v));
    }

    pub fn library_view(&mut self, change: impl FnOnce(&mut crate::settings::LibraryView)) {
        change(&mut self.settings.library_view);
        self.settings.save();
        self.show_library();
    }

    /// The captures an item action applies to: the selection when the item
    /// is part of it, else the item alone.
    fn library_targets(&self, path: &Path) -> Vec<PathBuf> {
        if self.library_selected.contains(path) {
            self.library_items.iter().filter(|i| self.library_selected.contains(&i.path)).map(|i| i.path.clone()).collect()
        } else {
            vec![path.to_path_buf()]
        }
    }

    /// Pin, close, delete, rename, tag, select or reveal library items.
    pub fn library_action(&mut self, path: &Path, what: &str, value: Option<String>) {
        let targets = self.library_targets(path);
        match what {
            "select" => {
                if !self.library_selected.remove(path) {
                    self.library_selected.insert(path.to_path_buf());
                }
                let on = self.library_selected.contains(path);
                if let Some(item) = self.library_items.iter().find(|i| i.path == path) {
                    item.set_selected(on);
                }
                let count = self.library_selected.len() as f64;
                self.put_num("LibrarySelected", count, |k, v| k.set_library_selected(v as i64));
                return;
            }
            "reveal" => return winutil::reveal(path),
            "pin" => {
                let pin = !library::Entry::read(path.to_path_buf()).pinned();
                for t in &targets {
                    self.edit_meta(t, |m| m.pinned = pin);
                }
                self.set_status(format!("{} {}", if pin { "Pinned" } else { "Unpinned" }, count_of(targets.len())));
            }
            "tags" => {
                let tags = library::parse_tags(value.as_deref().unwrap_or(""));
                for t in &targets {
                    self.edit_meta(t, |m| m.tags = tags.clone());
                }
                self.set_status(if tags.is_empty() { "Tags removed".to_owned() } else { format!("Tagged {}: {}", count_of(targets.len()), tags.join(", ")) });
            }
            "close" => {
                for t in &targets {
                    if !self.settings.library_view.closed.contains(t) {
                        self.settings.library_view.closed.push(t.clone());
                    }
                }
                self.settings.save();
                self.set_status(format!("Closed {} from the tray; they stay in the library", count_of(targets.len())));
            }
            "delete" => {
                let mut files = Vec::new();
                for t in &targets {
                    if Some(t) == self.source.as_ref() {
                        with_editor(|e| e.mark_saved());
                        self.source = None;
                        self.path = None;
                        self.close_file();
                    }
                    files.push(t.clone());
                    files.extend(jiffshot_capture::library::companions(t).into_iter().filter(|c| c.exists()));
                }
                if winutil::recycle(&files) {
                    self.set_status(format!("Moved {} to the Recycle Bin", count_of(targets.len())));
                } else {
                    self.set_status("Could not move the capture to the Recycle Bin");
                }
            }
            "rename" => {
                let name = value.unwrap_or_default();
                match jiffshot_capture::library::rename(path, &name) {
                    Ok(new) => {
                        let thumb = library::thumbnail_path(path);
                        let _ = std::fs::rename(&thumb, library::thumbnail_path(&new));
                        if self.source.as_deref() == Some(path) {
                            self.source = Some(new.clone());
                            if self.path.as_deref() == Some(path.with_extension(EDITS_EXT).as_path()) {
                                self.path = Some(new.with_extension(EDITS_EXT));
                            }
                        }
                        self.set_status(format!("Renamed to {}", new.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
                    }
                    Err(error) => self.set_status(format!("Could not rename: {error}")),
                }
            }
            _ => return,
        }
        self.refresh_library();
    }

    fn edit_meta(&mut self, path: &Path, change: impl FnOnce(&mut jiffshot_capture::library::CaptureMeta)) {
        use jiffshot_capture::library::{read_meta, write_meta, CaptureKind, CaptureMeta};
        // Recordings and imported files get a sidecar when first tagged.
        let mut meta = read_meta(path).unwrap_or_else(|| {
            let (w, h) = io::load_image(path).map(|p| (p.width(), p.height())).unwrap_or((0, 0));
            CaptureMeta {
                kind: if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4")) { CaptureKind::Video } else { CaptureKind::Region },
                rect: jiffshot_capture::Rect::new(0, 0, w as i32, h as i32),
                width: w,
                height: h,
                captured_at: String::new(),
                window_title: None,
                process: None,
                cursor_included: false,
                url: None,
                tags: Vec::new(),
                pinned: false,
            }
        });
        change(&mut meta);
        if let Err(error) = write_meta(path, &meta) {
            self.set_status(format!("Could not save the capture's details: {error}"));
        }
    }

    /// Images or videos picked in the library, combined into one.
    pub fn combine_selected(&mut self) {
        let picked: Vec<PathBuf> = self.library_items.iter().filter(|i| self.library_selected.contains(&i.path)).map(|i| i.path.clone()).collect();
        if picked.len() < 2 {
            return self.set_status("Ctrl+click two or more captures in the library first.");
        }
        let videos = picked.iter().filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4"))).count();
        if videos == picked.len() {
            return crate::video::combine_files(self, picked);
        }
        let images: Vec<_> = picked.iter().filter_map(|p| io::load_image(p).ok()).collect();
        if let Some(doc) = combine::combine(&images, combine::Arrangement::Grid, 24.0, Rgba::WHITE) {
            self.load(doc, None, None);
            with_editor(|e| e.checkpoint("Combine", None));
            self.set_status(format!("Combined {} images", images.len()));
        }
    }

    pub fn clear_library_selection(&mut self) {
        self.library_selected.clear();
        for i in &self.library_items {
            i.set_selected(false);
        }
        self.put_num("LibrarySelected", 0.0, |k, v| k.set_library_selected(v as i64));
    }

    /// Brings back everything closed from the tray.
    pub fn show_closed(&mut self) {
        let n = self.settings.library_view.closed.len();
        self.library_view(|v| v.closed.clear());
        self.set_status(format!("{} back in the tray", count_of(n)));
    }

    /// Marks the open capture's tray item while it has unsaved edits.
    fn mark_unsaved(&self) {
        let dirty = with_editor(|e| e.is_modified()).unwrap_or(false);
        for item in &self.library_items {
            item.set_unsaved(dirty && self.source.as_deref() == Some(item.path.as_path()));
        }
    }

    fn mark_current(&mut self) {
        let current = self.source.clone();
        for item in &self.library_items {
            item.set_current(current.as_deref() == Some(item.path.as_path()));
        }
    }

    /// A new capture arrived from the capture side (or another launch).
    pub fn incoming(&mut self, path: PathBuf) {
        self.show_editor();
        self.open_path(&path);
        self.refresh_library();
    }

    // ---- text recognition -------------------------------------------------

    pub fn grab_text(&mut self) {
        let Some(flat) = with_editor(|e| e.document().flatten()) else { return };
        match crate::ocr::recognize(&flat) {
            Ok(lines) if !lines.is_empty() => {
                let text = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
                let _ = jiffshot_capture::clipboard::copy_text(&text, winutil::main_hwnd());
                let preview: String = text.chars().take(80).collect();
                self.set_status(format!("Copied {} line(s) of text: {preview}", lines.len()));
            }
            Ok(_) => self.set_status("No text found"),
            Err(error) => self.set_status(format!("Text recognition failed: {error}")),
        }
    }

    pub fn simplify(&mut self) {
        let Some(flat) = with_editor(|e| e.document().flatten()) else { return };
        let lines = crate::ocr::recognize(&flat).unwrap_or_default();
        let mut bars = 0;
        self.document_op("Simplify", |d| {
            d.flatten_objects();
            bars = jiffshot_doc::smart::simplify(d, &lines, 12);
            true
        });
        self.set_status(format!("Simplified: {bars} text line(s) replaced by editable bars"));
    }

    // ---- video (see video.rs) --------------------------------------------

    pub(crate) fn stop_video(&mut self) {
        if let Some(v) = self.video.take() {
            v.stop();
        }
    }

    fn open_video(&mut self, path: &Path) {
        self.autosave();
        self.stop_video();
        match crate::video::Session::open(path) {
            Ok(mut session) => {
                let frame = session.frame_at(0.0);
                let duration = session.duration();
                let info = session.info();
                self.video = Some(session);
                crate::video::apply_volume(self);
                if let Some(pm) = frame {
                    with_editor(|e| e.load(Document::new(pm)));
                }
                self.path = None;
                self.source = Some(path.to_path_buf());
                if let Some(sink) = &self.sink {
                    let _ = sink.set_video_duration(duration);
                    let _ = sink.set_video_position(0.0);
                    let _ = sink.set_trim_start(0.0);
                    let _ = sink.set_trim_end(duration);
                    let _ = sink.set_video_info(&info);
                }
                self.zoom("fit");
                self.bump_frame();
                self.mark_current();
                self.set_status(format!("Opened video {}", path.display()));
            }
            Err(error) => self.set_status(format!("Could not open video {}: {error}", path.display())),
        }
    }

    pub fn video_action(&mut self, what: &str) {
        crate::video::action(self, what);
    }
}

/// A quick style shown in the panel; Apply puts it on the selection (or the
/// tool).
pub struct QuickStyle {
    /// Position in the tool's list.
    index: usize,
    tool: Tool,
    style: Style,
    favorite: bool,
    sink: Option<QuickStyleViewModelSink>,
}

impl QuickStyle {
    fn edit(&self, what: &'static str) {
        let (tool, index, style) = (self.tool, self.index, self.style.clone());
        post(move |app| app.edit_quick_style(tool, index, &style, what));
    }
}

impl QuickStyleViewModel for QuickStyle {
    fn attach(&mut self, sink: QuickStyleViewModelSink) -> Result<()> {
        let s = &self.style;
        sink.set_name(format!("{} {}", self.tool.name(), self.index + 1))?;
        sink.set_stroke(s.stroke.to_hex())?;
        sink.set_fill(if s.multiply { s.fill.with_alpha(255).to_hex() } else { s.fill.to_hex() })?;
        sink.set_text_color(s.text_color.to_hex())?;
        sink.set_width(s.width as f64)?;
        sink.set_is_favorite(self.favorite)?;
        self.sink = Some(sink);
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        self.sink = None;
        Ok(())
    }

    fn apply(&mut self) -> Result<()> {
        let (tool, style) = (self.tool, self.style.clone());
        // Runs after this call returns, outside the nested model's lock.
        post(move |app| app.apply_quick_style(tool, style));
        Ok(())
    }

    fn delete(&mut self) -> Result<()> {
        self.edit("delete");
        Ok(())
    }

    fn move_earlier(&mut self) -> Result<()> {
        self.edit("earlier");
        Ok(())
    }

    fn move_later(&mut self) -> Result<()> {
        self.edit("later");
        Ok(())
    }

    fn toggle_favorite(&mut self) -> Result<()> {
        self.edit("favorite");
        Ok(())
    }
}

/// Shared handle for the library item sinks, updated from the thumbnail
/// thread.
pub type ItemSink = Arc<Mutex<Option<LibraryItemViewModelSink>>>;

/// A capture taken with "pointer as an object" has the pointer saved next to
/// it (`<stem>.cursor.png` and `.cursor.json`); it opens as a movable image.
fn add_cursor_object(doc: &mut jiffshot_doc::Document, path: &Path) {
    let stem = path.with_extension("");
    let png = PathBuf::from(format!("{}.cursor.png", stem.display()));
    let Ok(json) = std::fs::read_to_string(format!("{}.cursor.json", stem.display())) else { return };
    let Ok(at) = serde_json::from_str::<serde_json::Value>(&json) else { return };
    let (Some(x), Some(y)) = (at["x"].as_f64(), at["y"].as_f64()) else { return };
    let Ok(image) = jiffshot_doc::Pixmap::load_png(&png) else { return };
    let rect = jiffshot_doc::Box2::from_xywh(x as f32, y as f32, image.width() as f32, image.height() as f32);
    let kind = jiffshot_doc::object::Kind::Image { rect, image: jiffshot_doc::object::ImageRef::new("cursor".to_owned(), image) };
    doc.add(kind, jiffshot_doc::object::Style { width: 0.0, ..Default::default() });
}

/// "1 capture", "3 captures".
fn count_of(n: usize) -> String {
    format!("{n} capture{}", if n == 1 { "" } else { "s" })
}

/// A path as the storage URI the Open recent menu shows.
fn file_uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file:///");
    for c in text.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '%' => out.push_str("%25"),
            '#' => out.push_str("%23"),
            c => out.push(c),
        }
    }
    out
}

/// Back from a `file:///` URI (or a plain path).
fn path_of_uri(uri: &str) -> PathBuf {
    let Some(rest) = uri.strip_prefix("file:///") else { return PathBuf::from(uri) };
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&rest[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    PathBuf::from(String::from_utf8_lossy(&out).replace('/', "\\"))
}

/// The image without its edges of one solid color (the top-left pixel's),
/// or `None` when there is nothing to trim.
fn trim_bounds(image: &jiffshot_doc::Pixmap) -> Option<jiffshot_doc::IRect> {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let px = image.pixels();
    let edge = px.first()?.demultiply();
    let same = |x: usize, y: usize| {
        let c = px[y * w + x].demultiply();
        let d = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 8;
        d(c.red(), edge.red()) && d(c.green(), edge.green()) && d(c.blue(), edge.blue()) && d(c.alpha(), edge.alpha())
    };
    let row = |y: usize| (0..w).all(|x| same(x, y));
    let col = |x: usize, y0: usize, y1: usize| (y0..y1).all(|y| same(x, y));
    let top = (0..h).find(|&y| !row(y))?;
    let bottom = (0..h).rev().find(|&y| !row(y))? + 1;
    let left = (0..w).find(|&x| !col(x, top, bottom))?;
    let right = (0..w).rev().find(|&x| !col(x, top, bottom))? + 1;
    let r = jiffshot_doc::IRect::new(left as i32, top as i32, (right - left) as i32, (bottom - top) as i32);
    (r != jiffshot_doc::IRect::new(0, 0, w as i32, h as i32)).then_some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_trim_finds_the_picture_inside_a_solid_border() {
        let mut pm = jiffshot_doc::Pixmap::new(40, 30).unwrap();
        pm.fill(tiny_skia::Color::WHITE);
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(200, 30, 30, 255);
        pm.fill_rect(tiny_skia::Rect::from_xywh(5.0, 7.0, 20.0, 10.0).unwrap(), &paint, tiny_skia::Transform::identity(), None);
        assert_eq!(trim_bounds(&pm), Some(jiffshot_doc::IRect::new(5, 7, 20, 10)));
        pm.fill(tiny_skia::Color::WHITE);
        assert_eq!(trim_bounds(&pm), None, "one color all over");
    }

    #[test]
    fn recent_files_round_trip_through_uris() {
        let path = Path::new(r"C:\Users\me\Pictures\Jiffshot\2026-10\a capture #1 100%.png");
        let uri = file_uri(path);
        assert_eq!(uri, "file:///C:/Users/me/Pictures/Jiffshot/2026-10/a%20capture%20%231%20100%25.png");
        assert_eq!(path_of_uri(&uri), path);
    }
}
