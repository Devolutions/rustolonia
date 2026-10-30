//! Interactive editing: tools, selection, handles, undo/redo, and the
//! display raster kept up to date with partial re-renders.
//!
//! The UI forwards pointer events in document pixels; every method that
//! changes something records the changed area, and [`Editor::take_damage`]
//! re-renders just that area into [`Editor::display`].

use crate::color::Rgba;
use crate::document::Document;
use crate::geom::{Box2, IRect, Pt};
use crate::object::*;
use crate::raster;
use crate::stamps::Stamp;
use crate::text::Align;
use std::collections::HashMap;
use tiny_skia::Pixmap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    Select,
    Arrow,
    Line,
    Shape,
    Text,
    Callout,
    Step,
    Pen,
    Highlight,
    Blur,
    Magnify,
    Spotlight,
    Stamp,
    Fill,
    Crop,
    CutOut,
    Selection,
    MagicWand,
    Eraser,
    Eyedropper,
    SmartMove,
    TextReplace,
}

pub const TOOLS: &[Tool] = &[
    Tool::Select,
    Tool::Arrow,
    Tool::Line,
    Tool::Shape,
    Tool::Text,
    Tool::Callout,
    Tool::Step,
    Tool::Pen,
    Tool::Highlight,
    Tool::Blur,
    Tool::Magnify,
    Tool::Spotlight,
    Tool::Stamp,
    Tool::Fill,
    Tool::Crop,
    Tool::CutOut,
    Tool::Selection,
    Tool::MagicWand,
    Tool::Eraser,
    Tool::Eyedropper,
    Tool::SmartMove,
    Tool::TextReplace,
];

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Arrow => "Arrow",
            Tool::Line => "Line",
            Tool::Shape => "Shape",
            Tool::Text => "Text",
            Tool::Callout => "Callout",
            Tool::Step => "Step",
            Tool::Pen => "Pen",
            Tool::Highlight => "Highlight",
            Tool::Blur => "Blur",
            Tool::Magnify => "Magnify",
            Tool::Spotlight => "Spotlight",
            Tool::Stamp => "Stamp",
            Tool::Fill => "Fill",
            Tool::Crop => "Crop",
            Tool::CutOut => "Cut Out",
            Tool::Selection => "Selection",
            Tool::MagicWand => "Magic Wand",
            Tool::Eraser => "Eraser",
            Tool::Eyedropper => "Eyedropper",
            Tool::SmartMove => "Smart Move",
            Tool::TextReplace => "Text Replace",
        }
    }

    pub fn from_name(name: &str) -> Option<Tool> {
        let key: String = name.chars().filter(|c| !c.is_whitespace()).collect();
        TOOLS.iter().copied().find(|t| t.name().replace(' ', "").eq_ignore_ascii_case(&key))
    }

    /// Choices for the tool's variant, first is the default.
    pub fn variants(self) -> &'static [&'static str] {
        match self {
            Tool::Shape => &["Rectangle", "Rounded", "Ellipse", "Triangle", "Polygon"],
            Tool::Callout => &["Rounded", "Rectangle", "Ellipse", "Arrow"],
            Tool::Highlight => &["Rectangle", "Rounded", "Ellipse", "Freehand"],
            Tool::Pen => &["Pen", "Marker"],
            Tool::Blur => &["Blur", "Pixelate", "Redact"],
            Tool::Magnify => &["Circle", "Rectangle", "Circle with view", "Rectangle with view"],
            Tool::Spotlight => &["Rectangle", "Rounded", "Ellipse"],
            Tool::Stamp => &["Check", "Cross", "Star", "Heart", "Warning", "Info", "Question", "Exclamation", "Plus", "Minus", "Pointer", "Thumbs up"],
            Tool::CutOut => &["Horizontal", "Vertical", "Horizontal sawtooth", "Vertical sawtooth", "Horizontal wave", "Vertical wave"],
            Tool::Fill => &["Area", "Global"],
            Tool::MagicWand => &["Area", "Global"],
            Tool::Selection => &["Rectangle", "Ellipse", "Freehand"],
            Tool::Eraser => &["Round", "Square"],
            Tool::Step => &["Numbers", "Letters", "Lowercase", "Roman", "Numbers square", "Letters square"],
            _ => &[],
        }
    }

    /// Tools that draw new annotation objects (and so also pick existing
    /// ones up when clicked).
    fn creates_objects(self) -> bool {
        !matches!(
            self,
            Tool::Select
                | Tool::Fill
                | Tool::Crop
                | Tool::CutOut
                | Tool::Selection
                | Tool::MagicWand
                | Tool::Eraser
                | Tool::Eyedropper
                | Tool::SmartMove
                | Tool::TextReplace
        )
    }

    fn uses_marquee(self) -> bool {
        matches!(self, Tool::Crop | Tool::CutOut | Tool::Selection | Tool::SmartMove)
    }

    /// Tools that lift part of the image and so use a background fill.
    pub fn lifts(self) -> bool {
        matches!(self, Tool::Selection | Tool::MagicWand | Tool::SmartMove)
    }
}

pub fn default_style(tool: Tool) -> Style {
    let base = Style::default();
    match tool {
        Tool::Arrow => Style { end_cap: Cap::Arrow, width: 8.0, ..base },
        Tool::Line => Style { width: 6.0, ..base },
        Tool::Shape => Style { width: 6.0, ..base },
        Tool::Text => Style { width: 0.0, text_color: Rgba::RED, font_size: 32.0, bold: true, ..base },
        Tool::Callout => Style {
            width: 3.0,
            stroke: Rgba::rgb(0x1E, 0x6F, 0xD9),
            fill: Rgba::rgb(0x1E, 0x6F, 0xD9),
            text_color: Rgba::WHITE,
            font_size: 24.0,
            bold: true,
            align: Align::Center,
            ..base
        },
        Tool::Step => Style { width: 0.0, fill: Rgba::RED, text_color: Rgba::WHITE, font_size: 22.0, ..base },
        Tool::Pen => Style { width: 5.0, ..base },
        Tool::Highlight => Style { fill: Rgba::YELLOW.with_alpha(180), stroke: Rgba::YELLOW.with_alpha(180), width: 24.0, multiply: true, ..base },
        Tool::Blur => Style { fill: Rgba::BLACK, width: 0.0, ..base },
        Tool::Magnify => Style { width: 4.0, stroke: Rgba::rgb(0x30, 0x30, 0x30), shadow: true, ..base },
        Tool::Spotlight => Style { width: 0.0, ..base },
        Tool::Fill => Style { fill: Rgba::YELLOW, ..base },
        Tool::Eraser => Style { width: 24.0, ..base },
        Tool::Selection | Tool::MagicWand | Tool::SmartMove => Style { fill: Rgba::WHITE, ..base },
        _ => base,
    }
}

/// Editor events for the UI, OR-ed together until taken.
pub const EVENT_SELECTION: u32 = 1;
pub const EVENT_DOCUMENT: u32 = 2;
pub const EVENT_FOCUS_TEXT: u32 = 4;
pub const EVENT_RESIZED: u32 = 8;
pub const EVENT_TOOL: u32 = 16;
pub const EVENT_COLOR_PICKED: u32 = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Pointer shapes the UI should show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum Cursor {
    Default = 0,
    Move = 1,
    ResizeNwse = 2,
    ResizeNesw = 3,
    ResizeNs = 4,
    ResizeEw = 5,
    Cross = 6,
    Point = 7,
    Text = 8,
}

#[derive(Clone, Debug)]
enum Drag {
    Create { id: u64, start: Pt },
    Move { id: u64, last: Pt, moved: bool },
    /// Moving every selected object together.
    MoveMany { last: Pt, moved: bool },
    /// Drawing a box to select the objects it touches.
    Select { start: Pt, add: bool },
    Handle { id: u64, handle: Handle },
    Marquee { start: Pt },
    /// A freehand selection outline.
    Lasso { points: Vec<Pt> },
    Erase { last: Pt },
    /// Dragging the canvas's right, bottom or corner edge.
    Canvas { handle: Handle },
    /// Moving the image inside a selected drop zone.
    Pan { id: u64, last: Pt, moved: bool },
}

/// What fills the hole an object leaves when it is lifted off the image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackgroundFill {
    /// Rebuilt from the surroundings.
    #[default]
    Auto,
    Transparent,
    /// The tool's fill color.
    Color,
}

impl BackgroundFill {
    pub const NAMES: &'static [&'static str] = &["Auto-fill", "Transparent", "Color"];

    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    pub fn from_name(n: &str) -> BackgroundFill {
        match n {
            "Transparent" => BackgroundFill::Transparent,
            "Color" => BackgroundFill::Color,
            _ => BackgroundFill::Auto,
        }
    }
}

/// Text recognition, which the document crate does not do itself (the
/// editor plugs in the platform OCR engine).
pub trait Analyzer: Send {
    /// Lines of text found in `area` of `image`.
    fn text_in(&mut self, image: &Pixmap, area: IRect) -> Vec<crate::smart::TextLine>;
}

#[derive(Clone, Debug)]
struct Snapshot {
    doc: Document,
    label: String,
    key: Option<String>,
}

pub struct Editor {
    doc: Document,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    tool: Tool,
    styles: HashMap<Tool, Style>,
    variants: HashMap<Tool, String>,
    amounts: HashMap<Tool, f32>,
    selection: Option<u64>,
    /// A polygon being drawn corner by corner.
    building: Option<u64>,
    /// Further selected objects (Shift-click, Select all, box select);
    /// `selection` is the one the properties panel shows.
    others: Vec<u64>,
    drag: Option<Drag>,
    marquee: Option<Box2>,
    display: Pixmap,
    dirty: Option<IRect>,
    events: u32,
    modified: bool,
    picked: Option<Rgba>,
    pub analyzer: Option<Box<dyn Analyzer>>,
    pub background_fill: BackgroundFill,
    /// Color of canvas areas added by resizing or expanding the canvas.
    pub canvas_color: Rgba,
    /// Stamps the user added (name, image), offered after the built-in ones.
    pub custom_stamps: Vec<(String, std::sync::Arc<Pixmap>)>,
}

const HISTORY_LIMIT: usize = 100;
const MIN_DRAG: f32 = 4.0;

impl Editor {
    pub fn new(doc: Document) -> Editor {
        let display = Pixmap::new(doc.width(), doc.height()).expect("non-zero document");
        let mut e = Editor {
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
            tool: Tool::Arrow,
            styles: HashMap::new(),
            variants: HashMap::new(),
            amounts: HashMap::new(),
            selection: None,
            building: None,
            others: Vec::new(),
            drag: None,
            marquee: None,
            display,
            dirty: None,
            events: 0,
            modified: false,
            picked: None,
            analyzer: None,
            background_fill: BackgroundFill::Auto,
            custom_stamps: Vec::new(),
            canvas_color: Rgba::WHITE,
        };
        e.refresh_all();
        e
    }

    pub fn document(&self) -> &Document {
        &self.doc
    }

    pub fn display(&self) -> &Pixmap {
        &self.display
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    /// Replaces the document (open, new capture); clears history.
    pub fn load(&mut self, doc: Document) {
        self.doc = doc;
        self.undo.clear();
        self.redo.clear();
        self.selection = None;
        self.drag = None;
        self.marquee = None;
        self.modified = false;
        self.refresh_all();
        self.events |= EVENT_SELECTION | EVENT_DOCUMENT;
    }

    pub fn take_events(&mut self) -> u32 {
        std::mem::take(&mut self.events)
    }

    // ---- tools and styles ------------------------------------------------

    pub fn tool(&self) -> Tool {
        self.tool
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.finish_polygon();
        if self.tool != tool {
            self.tool = tool;
            self.set_marquee(None);
            self.events |= EVENT_TOOL;
        }
    }

    pub fn tool_style(&self, tool: Tool) -> Style {
        self.styles.get(&tool).cloned().unwrap_or_else(|| default_style(tool))
    }

    pub fn set_tool_style(&mut self, tool: Tool, style: Style) {
        self.styles.insert(tool, style);
    }

    /// The variants offered for `tool`, including custom stamps.
    pub fn variants_for(&self, tool: Tool) -> Vec<String> {
        let mut v: Vec<String> = tool.variants().iter().map(|s| s.to_string()).collect();
        if tool == Tool::Stamp {
            v.extend(self.custom_stamps.iter().map(|(n, _)| n.clone()));
        }
        v
    }

    pub fn variant(&self, tool: Tool) -> String {
        self.variants.get(&tool).cloned().unwrap_or_else(|| tool.variants().first().unwrap_or(&"").to_string())
    }

    /// Sets the current tool's variant, and the selected object's when it
    /// was made by that tool.
    pub fn set_variant(&mut self, value: &str) {
        let tool = self.tool;
        self.variants.insert(tool, value.to_owned());
        if let Some(id) = self.selection {
            let v = value.to_owned();
            self.edit(id, "Change type", Some(format!("variant{id}")), move |o| apply_variant(o, &v));
        }
        let variant = self.variant(tool);
        if tool == Tool::Pen || tool == Tool::Highlight {
            let marker = variant == "Marker" || variant == "Freehand";
            let mut style = self.tool_style(tool);
            if tool == Tool::Pen {
                style.multiply = marker;
                if marker && style.width < 12.0 {
                    style.width = 20.0;
                    style.stroke = Rgba::YELLOW.with_alpha(180);
                }
            }
            self.styles.insert(tool, style);
        }
    }

    /// The blur intensity, magnifier zoom or spotlight dimming new objects
    /// of `tool` get.
    pub fn tool_amount(&self, tool: Tool) -> f32 {
        let default = match tool {
            Tool::Blur => if self.variant(Tool::Blur) == "Pixelate" { 12.0 } else { 10.0 },
            Tool::Magnify => 2.0,
            Tool::Spotlight => 0.6,
            Tool::MagicWand => 25.0,
            Tool::Fill => 10.0,
            Tool::Pen => 3.0,
            _ => 0.0,
        };
        self.amounts.get(&tool).copied().unwrap_or(default)
    }

    pub fn set_tool_amount(&mut self, tool: Tool, value: f32) {
        self.amounts.insert(tool, value);
    }

    /// Style shown in the properties panel: the selection's, or the tool's.
    pub fn current_style(&self) -> Style {
        self.selected().map(|o| o.style.clone()).unwrap_or_else(|| self.tool_style(self.tool))
    }

    /// Edits the selection's style (undoably, coalescing edits of the same
    /// property) and the current tool's style for new objects.
    pub fn update_style(&mut self, property: &str, change: impl Fn(&mut Style)) {
        for id in self.others.clone() {
            self.edit(id, "Change style", Some(format!("{property}*")), |o| {
                o.walk_mut(&mut |o| change(&mut o.style));
                o.fit_text();
            });
        }
        if let Some(id) = self.selection {
            self.edit(id, "Change style", Some(format!("{property}{id}")), |o| {
                change(&mut o.style);
                o.fit_text();
            });
            if self.selected().map(|o| tool_for(o)) != Some(self.tool) {
                return;
            }
        }
        let mut style = self.tool_style(self.tool);
        change(&mut style);
        self.styles.insert(self.tool, style);
    }

    // ---- selection ---------------------------------------------------------

    pub fn selection(&self) -> Option<u64> {
        self.selection
    }

    pub fn selected(&self) -> Option<&Object> {
        self.selection.and_then(|id| self.doc.get(id))
    }

    /// Every selected object, the panel's one first.
    pub fn selected_ids(&self) -> Vec<u64> {
        self.selection.into_iter().chain(self.others.iter().copied()).collect()
    }

    /// Adds `id` to the selection, or takes it out (Shift-click).
    pub fn toggle_selected(&mut self, id: u64) {
        let mut ids = self.selected_ids();
        if let Some(i) = ids.iter().position(|x| *x == id) {
            ids.remove(i);
        } else {
            ids.insert(0, id);
        }
        self.set_selection(ids);
    }

    /// Replaces the selection; the first id is the panel's.
    pub fn set_selection(&mut self, ids: Vec<u64>) {
        let mut ids: Vec<u64> = ids.into_iter().filter(|id| self.doc.get(*id).is_some()).collect();
        ids.dedup();
        let first = if ids.is_empty() { None } else { Some(ids.remove(0)) };
        self.select(first);
        self.others = ids;
        self.events |= EVENT_SELECTION;
    }

    pub fn select_all(&mut self) {
        let ids = self.doc.objects.iter().rev().map(|o| o.id).collect();
        self.set_selection(ids);
    }

    pub fn invert_selection(&mut self) {
        let chosen = self.selected_ids();
        let ids = self.doc.objects.iter().rev().map(|o| o.id).filter(|id| !chosen.contains(id)).collect();
        self.set_selection(ids);
    }

    /// Boxes of the selected objects other than the panel's one.
    pub fn other_boxes(&self) -> Vec<Box2> {
        self.others.iter().filter_map(|id| self.doc.get(*id)).filter_map(|o| o.rect()).collect()
    }

    /// Joins the selected objects into one group (keeping their order).
    pub fn group_selected(&mut self) -> Option<u64> {
        let ids = self.selected_ids();
        if ids.len() < 2 {
            return None;
        }
        self.checkpoint("Group", None);
        let at = ids.iter().filter_map(|id| self.doc.index_of(*id)).max()?;
        let mut members: Vec<Object> = Vec::new();
        let mut kept = Vec::new();
        for (i, o) in std::mem::take(&mut self.doc.objects).into_iter().enumerate() {
            if ids.contains(&o.id) {
                members.push(o);
            } else {
                kept.push((i, o));
            }
        }
        let insert_at = kept.iter().filter(|(i, _)| *i < at).count();
        self.doc.objects = kept.into_iter().map(|(_, o)| o).collect();
        let id = self.doc.allocate_id();
        let group = Object::new(id, Kind::Group { objects: members }, Style { width: 0.0, ..Style::default() });
        let b = group.bounds();
        self.doc.objects.insert(insert_at, group);
        self.invalidate_box(b);
        self.others.clear();
        self.selection = Some(id);
        self.events |= EVENT_SELECTION;
        Some(id)
    }

    /// Splits selected groups back into their objects.
    pub fn ungroup_selected(&mut self) -> bool {
        let groups: Vec<u64> = self.selected_ids().into_iter().filter(|id| matches!(self.doc.get(*id).map(|o| &o.kind), Some(Kind::Group { .. }))).collect();
        if groups.is_empty() {
            return false;
        }
        self.checkpoint("Ungroup", None);
        let mut freed = Vec::new();
        for id in groups {
            let Some(i) = self.doc.index_of(id) else { continue };
            let group = self.doc.objects.remove(i);
            if let Kind::Group { objects } = group.kind {
                for (k, mut o) in objects.into_iter().enumerate() {
                    o.id = self.doc.allocate_id();
                    freed.push(o.id);
                    self.doc.objects.insert(i + k, o);
                }
            }
        }
        self.set_selection(freed);
        true
    }

    /// Draws the selected objects into the image and removes them.
    pub fn flatten_selected(&mut self) -> bool {
        let ids = self.selected_ids();
        if ids.is_empty() {
            return false;
        }
        self.edit_document("Flatten", |d| {
            let mut keep = Vec::new();
            let mut flat = Document::new(d.background().clone());
            for o in std::mem::take(&mut d.objects) {
                if ids.contains(&o.id) {
                    flat.objects.push(o);
                } else {
                    keep.push(o);
                }
            }
            let image = flat.flatten();
            d.set_background(image);
            d.objects = keep;
            true
        });
        self.selection = None;
        self.others.clear();
        true
    }

    /// Lines up the selected objects' edges or centers: `how` is one of
    /// left, center, right, top, middle, bottom.
    pub fn align_selected(&mut self, how: &str) {
        let ids = self.selected_ids();
        let boxes: Vec<(u64, Box2)> = ids.iter().filter_map(|id| Some((*id, self.doc.get(*id)?.rect()?))).collect();
        if boxes.len() < 2 {
            return;
        }
        let all = boxes.iter().map(|(_, b)| *b).reduce(|a, b| a.union(&b)).unwrap();
        self.checkpoint("Align", None);
        for (id, b) in boxes {
            let (dx, dy) = match how {
                "left" => (all.x0 - b.x0, 0.0),
                "right" => (all.x1 - b.x1, 0.0),
                "center" => (all.center().x - b.center().x, 0.0),
                "top" => (0.0, all.y0 - b.y0),
                "bottom" => (0.0, all.y1 - b.y1),
                _ => (0.0, all.center().y - b.center().y),
            };
            if let Some(o) = self.doc.get_mut(id) {
                let before = o.bounds();
                o.translate(dx, dy);
                let after = o.bounds();
                self.invalidate_box(before.union(&after));
            }
        }
    }

    /// Selects just `id` (or nothing).
    pub fn select(&mut self, id: Option<u64>) {
        if !self.others.is_empty() {
            self.others.clear();
            self.events |= EVENT_SELECTION;
        }
        if self.selection == id {
            return;
        }
        if let Some(prev) = self.selection {
            // Text boxes left empty disappear, as nothing would show them.
            if self.doc.get(prev).and_then(|o| o.text()).is_some_and(|t| t.trim().is_empty())
                && !matches!(self.doc.get(prev).map(|o| &o.kind), Some(Kind::Step { .. }))
            {
                if let Some(o) = self.doc.remove(prev) {
                    self.invalidate_box(o.bounds());
                }
            }
        }
        self.selection = id;
        self.events |= EVENT_SELECTION;
    }

    /// Handles of the selection, for the UI to draw.
    pub fn selection_handles(&self) -> Vec<(Handle, Pt)> {
        self.selected().map(|o| o.handles()).unwrap_or_default()
    }

    pub fn selection_box(&self) -> Option<Box2> {
        let o = self.selected()?;
        match &o.kind {
            Kind::Spotlight { rect, .. } => Some(*rect),
            Kind::Callout { rect, .. } => Some(*rect),
            _ => o.rect(),
        }
    }

    pub fn marquee(&self) -> Option<Box2> {
        self.marquee
    }

    fn set_marquee(&mut self, m: Option<Box2>) {
        if self.marquee != m {
            self.marquee = m;
            self.events |= EVENT_SELECTION;
        }
    }

    pub fn picked_color(&mut self) -> Option<Rgba> {
        self.picked.take()
    }

    // ---- history -------------------------------------------------------

    /// Records the state before an edit. Consecutive edits with the same
    /// `key` share one undo step.
    pub fn checkpoint(&mut self, label: &str, key: Option<String>) {
        self.modified = true;
        self.events |= EVENT_DOCUMENT;
        if key.is_some() && self.undo.last().is_some_and(|s| s.key == key) {
            return;
        }
        self.undo.push(Snapshot { doc: self.doc.clone(), label: label.to_owned(), key });
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Ends coalescing, so the next edit starts a new undo step.
    pub fn seal(&mut self) {
        if let Some(s) = self.undo.last_mut() {
            s.key = None;
        }
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|s| s.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }

    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop() else { return false };
        let current = std::mem::replace(&mut self.doc, s.doc);
        self.redo.push(Snapshot { doc: current, label: s.label, key: None });
        self.after_history();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(s) = self.redo.pop() else { return false };
        let current = std::mem::replace(&mut self.doc, s.doc);
        self.undo.push(Snapshot { doc: current, label: s.label, key: None });
        self.after_history();
        true
    }

    fn after_history(&mut self) {
        self.drag = None;
        if self.selection.is_some_and(|id| self.doc.get(id).is_none()) {
            self.selection = None;
        }
        self.modified = true;
        self.refresh_all();
        self.events |= EVENT_SELECTION | EVENT_DOCUMENT;
    }

    // ---- damage ----------------------------------------------------------

    fn invalidate(&mut self, r: IRect) {
        self.dirty = Some(self.dirty.map_or(r, |d| d.union(&r)));
    }

    fn invalidate_box(&mut self, b: Box2) {
        self.invalidate(b.to_irect());
    }

    /// Re-renders everything, reallocating the display if the canvas size
    /// changed.
    pub fn refresh_all(&mut self) {
        if (self.display.width(), self.display.height()) != self.doc.size() {
            self.display = Pixmap::new(self.doc.width(), self.doc.height()).expect("non-zero document");
            self.events |= EVENT_RESIZED;
        }
        self.invalidate(self.doc.bounds());
    }

    /// Brings the display up to date; returns the area that changed.
    pub fn take_damage(&mut self) -> Option<IRect> {
        let area = self.dirty.take()?;
        self.doc.render_into(&mut self.display, area, true)
    }

    /// Runs `change` on object `id` with an undo step, invalidating its old
    /// and new extent.
    pub fn edit(&mut self, id: u64, label: &str, key: Option<String>, change: impl FnOnce(&mut Object)) {
        let Some(before) = self.doc.get(id).map(|o| o.bounds()) else { return };
        self.checkpoint(label, key);
        let o = self.doc.get_mut(id).unwrap();
        change(o);
        let after = o.bounds();
        self.invalidate_box(before.union(&after));
    }

    /// Changes the document as a whole (one undo step, full redraw).
    pub fn edit_document(&mut self, label: &str, change: impl FnOnce(&mut Document) -> bool) -> bool {
        let saved = self.doc.clone();
        if !change(&mut self.doc) {
            return false;
        }
        self.undo.push(Snapshot { doc: saved, label: label.to_owned(), key: None });
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.modified = true;
        if self.selection.is_some_and(|id| self.doc.get(id).is_none()) {
            self.selection = None;
        }
        self.events |= EVENT_DOCUMENT | EVENT_SELECTION;
        self.refresh_all();
        true
    }

    // ---- object commands -------------------------------------------------

    pub fn add_object(&mut self, label: &str, kind: Kind, style: Style) -> u64 {
        self.checkpoint(label, None);
        let id = self.doc.add(kind, style);
        let b = self.doc.get(id).unwrap().bounds();
        self.invalidate_box(b);
        self.select(Some(id));
        id
    }

    pub fn delete_selected(&mut self) -> bool {
        let ids = self.selected_ids();
        if ids.is_empty() {
            return false;
        }
        self.checkpoint("Delete", None);
        for id in ids {
            if let Some(o) = self.doc.remove(id) {
                self.invalidate_box(o.bounds());
            }
        }
        self.selection = None;
        self.others.clear();
        self.events |= EVENT_SELECTION;
        true
    }

    pub fn duplicate_selected(&mut self) -> Option<u64> {
        if !self.others.is_empty() {
            let copies: Vec<Object> = self.selected_ids().iter().filter_map(|id| self.doc.get(*id).cloned()).collect();
            self.checkpoint("Duplicate", None);
            let mut ids = Vec::new();
            for mut c in copies {
                c.translate(20.0, 20.0);
                let id = self.doc.insert(c);
                let b = self.doc.get(id).unwrap().bounds();
                self.invalidate_box(b);
                ids.push(id);
            }
            self.set_selection(ids.clone());
            return ids.first().copied();
        }
        let mut copy = self.selected()?.clone();
        copy.translate(20.0, 20.0);
        if let Kind::Step { label, .. } = &mut copy.kind {
            *label = self.doc.next_step_label();
        }
        self.checkpoint("Duplicate", None);
        let id = self.doc.insert(copy);
        let b = self.doc.get(id).unwrap().bounds();
        self.invalidate_box(b);
        self.select(Some(id));
        Some(id)
    }

    /// Inserts a copied object (for paste), offset so it is visible.
    pub fn paste_object(&mut self, mut object: Object) -> u64 {
        object.translate(16.0, 16.0);
        self.checkpoint("Paste", None);
        let id = self.doc.insert(object);
        let b = self.doc.get(id).unwrap().bounds();
        self.invalidate_box(b);
        self.select(Some(id));
        id
    }

    /// Spotlight and magnify: the selected object's area (or the middle of
    /// the image) enlarged `zoom` times in place, framed, with the rest
    /// dimmed; the area object itself is removed. All of it stays
    /// editable.
    pub fn spotlight_magnify(&mut self, zoom: f32) {
        let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
        let picked = self.selected().map(|o| (o.id, o.rect().unwrap_or_else(|| o.bounds())));
        let area = match picked {
            Some((_, r)) => r,
            None => Box2::around(Pt::new(w / 2.0, h / 2.0), w / 6.0, h / 6.0),
        };
        let area = Box2::new(area.x0.max(0.0), area.y0.max(0.0), area.x1.min(w), area.y1.min(h));
        if area.is_degenerate(4.0) {
            return;
        }
        let picked_id = picked.map(|(id, _)| id);
        self.checkpoint("Spotlight & magnify", None);
        if let Some(id) = picked_id {
            self.doc.remove(id);
        }
        let flat = self.doc.flatten();
        let Some(piece) = raster::extract(&flat, area.to_irect()) else { return };
        let c = area.center();
        let (bw, bh) = ((area.width() * zoom).min(w * 0.9), (area.height() * zoom).min(h * 0.9));
        let big = Box2::around(Pt::new(c.x.clamp(bw / 2.0, w - bw / 2.0), c.y.clamp(bh / 2.0, h - bh / 2.0)), bw / 2.0, bh / 2.0);
        let dim = Style { width: 0.0, ..Style::default() };
        self.doc.add(Kind::Spotlight { rect: big, ellipse: false, dim: 0.55, blur: 4.0, rounded: false }, dim);
        let frame = Style { width: 4.0, stroke: Rgba::WHITE, shadow: true, ..Style::default() };
        let id = self.doc.add(Kind::Image { rect: big, image: ImageRef::new(crate::object::fresh_key("magnified"), piece) }, frame);
        self.invalidate(IRect::of_size(self.doc.width(), self.doc.height()));
        self.seal();
        self.select(Some(id));
    }

    /// Adds a tail to the selected callout (pointing beside the first),
    /// or removes its last extra one.
    pub fn callout_tail(&mut self, add: bool) -> bool {
        let Some(id) = self.selection.filter(|id| self.doc.get(*id).is_some_and(|o| matches!(o.kind, Kind::Callout { .. }))) else { return false };
        let label = if add { "Add tail" } else { "Remove tail" };
        self.edit(id, label, None, move |o| {
            if let Kind::Callout { rect, tail, more_tails, .. } = &mut o.kind {
                if add && more_tails.len() < 6 {
                    let c = rect.center();
                    let n = more_tails.len() as f32 + 1.0;
                    // Fan out around the first tail.
                    let angle = 0.6 * n * if more_tails.len() % 2 == 0 { 1.0 } else { -1.0 };
                    let d = tail.sub(c);
                    let (sin, cos) = angle.sin_cos();
                    more_tails.push(Pt::new(c.x + d.x * cos - d.y * sin, c.y + d.x * sin + d.y * cos));
                } else if !add {
                    more_tails.pop();
                }
            }
        });
        true
    }

    /// The drop zone at `p`, if any (topmost first).
    pub fn zone_at(&self, p: Pt) -> Option<u64> {
        self.doc.objects.iter().rev().find(|o| o.zone.is_some() && o.bounds().contains(p)).map(|o| o.id)
    }

    /// Puts `image` into drop zone `id`, centered.
    pub fn fill_zone(&mut self, id: u64, image: Pixmap) {
        let key = crate::object::fresh_key("zone");
        self.edit(id, "Fill drop zone", None, move |o| {
            if let Kind::Image { image: r, .. } = &mut o.kind {
                *r = ImageRef::new(key, image);
            }
            if let Some(z) = o.zone.as_mut() {
                z.pan = Pt::default();
            }
        });
        self.select(Some(id));
    }

    /// An image dropped at `p`: into the drop zone there, else as a new
    /// image object centered on `p`.
    pub fn drop_image(&mut self, p: Pt, image: Pixmap) -> u64 {
        if let Some(id) = self.zone_at(p) {
            self.fill_zone(id, image);
            return id;
        }
        let (dw, dh) = (self.doc.width() as f32, self.doc.height() as f32);
        let (w, h) = (image.width() as f32, image.height() as f32);
        let scale = (dw * 0.6 / w).min(dh * 0.6 / h).min(1.0);
        let rect = Box2::around(p, w * scale / 2.0, h * scale / 2.0);
        let style = Style { width: 0.0, ..Style::default() };
        self.add_object("Drop image", Kind::Image { rect, image: ImageRef::new(crate::object::fresh_key("image"), image) }, style)
    }

    /// Exchanges the pictures of the two selected drop zones.
    pub fn swap_zones(&mut self) -> bool {
        let ids = self.selected_ids();
        let zones: Vec<u64> = ids.into_iter().filter(|id| self.doc.get(*id).is_some_and(|o| o.zone.is_some())).collect();
        let [a, b] = zones[..] else { return false };
        let take = |d: &Document, id| match &d.get(id).unwrap().kind {
            Kind::Image { image, .. } => image.clone(),
            _ => unreachable!(),
        };
        let (ia, ib) = (take(&self.doc, a), take(&self.doc, b));
        self.checkpoint("Swap images", None);
        for (id, img) in [(a, ib), (b, ia)] {
            let o = self.doc.get_mut(id).unwrap();
            if let Kind::Image { image, .. } = &mut o.kind {
                *image = img;
            }
            if let Some(z) = o.zone.as_mut() {
                z.pan = Pt::default();
            }
            let bounds = o.bounds();
            self.invalidate_box(bounds);
        }
        self.seal();
        true
    }

    /// Fill (crop to the zone) or fit (show all) for the selected zones.
    pub fn set_zone_fit(&mut self, fit: bool) {
        for id in self.selected_ids() {
            if self.doc.get(id).is_some_and(|o| o.zone.is_some()) {
                self.edit(id, "Fill or fit", Some(format!("fit{id}")), move |o| {
                    if let Some(z) = o.zone.as_mut() {
                        z.fit = fit;
                        z.pan = Pt::default();
                    }
                });
            }
        }
    }

    /// Takes the picture out of the selected drop zone.
    pub fn empty_zone(&mut self) {
        let Some(id) = self.selection.filter(|id| self.doc.get(*id).is_some_and(|o| o.zone.is_some())) else { return };
        self.edit(id, "Empty drop zone", None, |o| {
            if let Kind::Image { image, .. } = &mut o.kind {
                *image = ImageRef { key: String::new(), pixmap: None };
            }
        });
    }

    /// Inserts a raster as an image object, centered and scaled to fit.
    pub fn paste_image(&mut self, image: Pixmap) -> u64 {
        if let Some(id) = self.selection.filter(|id| self.doc.get(*id).is_some_and(|o| o.zone.is_some())) {
            self.fill_zone(id, image);
            return id;
        }
        let (dw, dh) = (self.doc.width() as f32, self.doc.height() as f32);
        let (w, h) = (image.width() as f32, image.height() as f32);
        let scale = (dw * 0.9 / w).min(dh * 0.9 / h).min(1.0);
        let rect = Box2::around(Pt::new(dw / 2.0, dh / 2.0), w * scale / 2.0, h * scale / 2.0);
        let key = crate::object::fresh_key("image");
        let style = Style { width: 0.0, ..Style::default() };
        self.add_object("Paste image", Kind::Image { rect, image: ImageRef::new(key, image) }, style)
    }

    pub fn nudge(&mut self, dx: f32, dy: f32) {
        for id in self.selected_ids() {
            self.edit(id, "Move", Some("nudge".to_owned()), |o| o.translate(dx, dy));
        }
    }

    /// Moves the selection in z-order: +1/-1 one step, +2/-2 to the
    /// top/bottom.
    pub fn arrange(&mut self, how: i32) {
        let Some(id) = self.selection else { return };
        let Some(i) = self.doc.index_of(id) else { return };
        let n = self.doc.objects.len();
        let j = match how {
            2 => n - 1,
            1 => (i + 1).min(n - 1),
            -1 => i.saturating_sub(1),
            _ => 0,
        };
        if i == j {
            return;
        }
        self.checkpoint("Arrange", None);
        let o = self.doc.objects.remove(i);
        let b = o.bounds();
        self.doc.objects.insert(j, o);
        self.invalidate_box(b);
    }

    pub fn set_selected_text(&mut self, text: &str) {
        let Some(id) = self.selection else { return };
        if self.selected().and_then(|o| o.text()) == Some(text) {
            return;
        }
        let owned = text.to_owned();
        self.edit(id, "Edit text", Some(format!("text{id}")), move |o| o.set_text(&owned));
    }

    /// Sets a numeric parameter of the selection (blur amount, zoom, dim).
    pub fn set_selected_amount(&mut self, value: f32) {
        let Some(id) = self.selection else { return };
        self.edit(id, "Change amount", Some(format!("amount{id}")), |o| match &mut o.kind {
            Kind::Blur { amount, .. } => *amount = value.clamp(1.0, 100.0),
            Kind::Magnify { zoom, .. } => *zoom = value.clamp(1.0, 8.0),
            Kind::Spotlight { dim, .. } => *dim = value.clamp(0.0, 1.0),
            _ => {}
        });
    }

    pub fn set_selected_rotation(&mut self, degrees: f32) {
        let Some(id) = self.selection else { return };
        if !self.selected().is_some_and(|o| o.rotatable()) {
            return;
        }
        self.edit(id, "Rotate", Some(format!("rotate{id}")), |o| o.rotation = degrees.rem_euclid(360.0));
    }

    /// Font size edits also size a selected step badge.
    pub fn resize_selected_step(&mut self, font_size: f32) {
        let Some(id) = self.selection else { return };
        if matches!(self.selected().map(|o| &o.kind), Some(Kind::Step { .. })) {
            self.edit(id, "Change style", Some(format!("size{id}")), |o| {
                if let Kind::Step { radius, .. } = &mut o.kind {
                    *radius = font_size * 0.85;
                }
            });
        }
    }

    pub fn has_damage(&self) -> bool {
        self.dirty.is_some()
    }

    pub fn selected_amount(&self) -> Option<f32> {
        match self.selected()?.kind {
            Kind::Blur { amount, .. } => Some(amount),
            Kind::Magnify { zoom, .. } => Some(zoom),
            Kind::Spotlight { dim, .. } => Some(dim),
            _ => None,
        }
    }

    // ---- pointer -----------------------------------------------------------

    /// Handles at the canvas's right, bottom and bottom-right edges, shown
    /// when nothing is selected.
    pub fn canvas_handles(&self) -> Vec<(Handle, Pt)> {
        if self.selection.is_some() || !matches!(self.tool, Tool::Select | Tool::Arrow | Tool::Shape | Tool::Line) {
            return Vec::new();
        }
        let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
        vec![(H_R, Pt::new(w, h / 2.0)), (H_B, Pt::new(w / 2.0, h)), (H_BR, Pt::new(w, h))]
    }

    /// Grows the canvas to include every object (in the canvas color).
    pub fn expand_to_fit(&mut self) -> bool {
        let Some(all) = self.doc.objects.iter().map(|o| o.bounds()).reduce(|a, b| a.union(&b)) else { return false };
        let canvas = self.doc.bounds().to_box();
        let b = all.inflate(-2.0).union(&canvas);
        if b == canvas {
            return false;
        }
        let r = b.to_irect();
        let color = self.canvas_color;
        self.edit_document("Expand canvas", |d| {
            d.resize_canvas(r.w as u32, r.h as u32, -r.x, -r.y, color);
            true
        })
    }

    pub fn hover(&self, p: Pt, tolerance: f32) -> Cursor {
        for (h, q) in self.canvas_handles() {
            if q.dist(p) <= tolerance * 1.5 {
                return handle_cursor(h);
            }
        }
        if self.tool.uses_marquee() || matches!(self.tool, Tool::Fill | Tool::Eyedropper | Tool::TextReplace | Tool::MagicWand | Tool::Eraser) {
            return Cursor::Cross;
        }
        if let Some(o) = self.selected() {
            for (h, q) in o.handles() {
                if q.dist(p) <= tolerance * 1.5 {
                    return handle_cursor(h);
                }
            }
        }
        if self.doc.hit(p, tolerance).is_some() && self.tool != Tool::Pen {
            return Cursor::Move;
        }
        match self.tool {
            Tool::Select => Cursor::Default,
            Tool::Text => Cursor::Text,
            _ => Cursor::Cross,
        }
    }

    pub fn pointer_down(&mut self, p: Pt, m: Modifiers, tolerance: f32) {
        self.drag = None;
        if let Some((h, _)) = self.canvas_handles().into_iter().find(|(_, q)| q.dist(p) <= tolerance * 1.5) {
            self.drag = Some(Drag::Canvas { handle: h });
            return;
        }
        if let Some(id) = self.building {
            let first = match self.doc.get(id).map(|o| &o.kind) {
                Some(Kind::Polygon { points }) => points.first().copied(),
                _ => None,
            };
            match first {
                Some(f) if f.dist(p) <= tolerance * 2.0 => self.finish_polygon(),
                Some(_) => self.edit(id, "Add Shape", Some(format!("poly{id}")), move |o| {
                    if let Kind::Polygon { points } = &mut o.kind {
                        if let Some(last) = points.last_mut() {
                            *last = p;
                        }
                        points.push(p);
                    }
                }),
                None => self.building = None,
            }
            return;
        }
        if self.tool == Tool::Selection && self.variant(Tool::Selection) == "Freehand" {
            self.select(None);
            self.drag = Some(Drag::Lasso { points: vec![p] });
            self.set_marquee(Some(Box2::from_points(p, p)));
            return;
        }
        if self.tool == Tool::MagicWand {
            self.select(None);
            return self.magic_wand(p);
        }
        if self.tool == Tool::Eraser {
            self.select(None);
            self.checkpoint("Erase", None);
            self.erase_segment(p, p);
            self.drag = Some(Drag::Erase { last: p });
            return;
        }
        if self.tool.uses_marquee() {
            self.select(None);
            self.drag = Some(Drag::Marquee { start: p });
            self.set_marquee(Some(Box2::from_points(p, p)));
            return;
        }
        match self.tool {
            Tool::Fill => return self.fill_at(p),
            Tool::Eyedropper => {
                self.picked = Some(raster::pixel(&self.doc.flatten(), p.x as i32, p.y as i32));
                self.events |= EVENT_COLOR_PICKED;
                return;
            }
            Tool::TextReplace => {
                self.select(None);
                self.drag = Some(Drag::Marquee { start: p });
                self.set_marquee(Some(Box2::from_points(p, p)));
                return;
            }
            _ => {}
        }
        if let Some(o) = self.selected() {
            let id = o.id;
            if let Some((h, _)) = o.handles().into_iter().find(|(_, q)| q.dist(p) <= tolerance * 1.5) {
                self.checkpoint("Resize", None);
                self.drag = Some(Drag::Handle { id, handle: h });
                return;
            }
        }
        let picks_up = self.tool != Tool::Pen && !(self.tool == Tool::Highlight && self.variant(Tool::Highlight) == "Freehand");
        if picks_up || self.tool == Tool::Select {
            if let Some(id) = self.doc.hit(p, tolerance) {
                if m.shift {
                    self.toggle_selected(id);
                    return;
                }
                if self.others.contains(&id) || (!self.others.is_empty() && self.selection == Some(id)) {
                    self.drag = Some(Drag::MoveMany { last: p, moved: false });
                    return;
                }
                // A drag inside a selected, filled drop zone moves its picture.
                let pans = self.selection == Some(id) && self.doc.get(id).is_some_and(|o| o.zone.is_some() && matches!(&o.kind, Kind::Image { image, .. } if image.pixmap.is_some()));
                self.select(Some(id));
                self.others.clear();
                self.drag = Some(if pans { Drag::Pan { id, last: p, moved: false } } else { Drag::Move { id, last: p, moved: false } });
                return;
            }
        }
        if self.tool == Tool::Select {
            if !m.shift {
                self.set_selection(Vec::new());
            }
            self.drag = Some(Drag::Select { start: p, add: m.shift });
            self.set_marquee(Some(Box2::from_points(p, p)));
            return;
        }
        if !self.tool.creates_objects() {
            self.select(None);
            return;
        }
        self.others.clear();
        self.select(None);
        let (kind, style) = self.new_object(p);
        let label = format!("Add {}", self.tool.name());
        let step = matches!(kind, Kind::Step { .. }) || self.tool == Tool::Stamp;
        let polygon = matches!(kind, Kind::Polygon { .. });
        let id = self.add_object(&label, kind, style);
        if polygon {
            self.building = Some(id);
            self.drag = None;
            return;
        }
        self.drag = Some(if step { Drag::Move { id, last: p, moved: true } } else { Drag::Create { id, start: p } });
    }

    /// Closes the polygon being built (double-click, Enter, or a click on
    /// its first corner).
    pub fn finish_polygon(&mut self) {
        let Some(id) = self.building.take() else { return };
        let too_small = match self.doc.get_mut(id).map(|o| &mut o.kind) {
            Some(Kind::Polygon { points }) => {
                // Drop the floating corner that follows the pointer.
                points.pop();
                points.dedup_by(|a, b| a.dist(*b) < 1.0);
                points.len() < 3
            }
            _ => true,
        };
        if too_small {
            if let Some(o) = self.doc.remove(id) {
                self.invalidate_box(o.bounds());
            }
            self.selection = None;
        } else if let Some(o) = self.doc.get(id) {
            let b = o.bounds();
            self.invalidate_box(b.inflate(4.0));
        }
        self.seal();
        self.events |= EVENT_SELECTION;
    }

    pub fn pointer_move(&mut self, p: Pt, m: Modifiers) {
        if let Some(id) = self.building {
            if let Some(o) = self.doc.get_mut(id) {
                let before = o.bounds();
                if let Kind::Polygon { points } = &mut o.kind {
                    let anchor = points[points.len().saturating_sub(2)];
                    if let Some(last) = points.last_mut() {
                        *last = if m.shift { snap45(anchor, p) } else { p };
                    }
                }
                let after = o.bounds();
                self.invalidate_box(before.union(&after));
            }
            return;
        }
        let Some(drag) = self.drag.clone() else { return };
        match drag {
            Drag::Create { id, start } => {
                let Some(o) = self.doc.get(id) else { return };
                let before = o.bounds();
                let pen = matches!(o.kind, Kind::Pen { .. });
                let o = self.doc.get_mut(id).unwrap();
                let damage = if let Kind::Pen { points } = &mut o.kind {
                    let last = *points.last().unwrap();
                    if last.dist(p) < 0.75 {
                        return;
                    }
                    let tail: Vec<Pt> = points.iter().rev().take(2).copied().chain([p]).collect();
                    points.push(p);
                    Box2::bounding(tail).unwrap().inflate(o.style.width / 2.0 + 3.0)
                } else {
                    shape_drag(o, start, p, m.shift);
                    before.union(&o.bounds())
                };
                let _ = pen;
                self.invalidate_box(damage);
            }
            Drag::Move { id, last, moved } => {
                if !moved && last.dist(p) < 1.0 {
                    return;
                }
                if !moved {
                    self.checkpoint("Move", None);
                }
                let (dx, dy) = (p.x - last.x, p.y - last.y);
                let Some(o) = self.doc.get_mut(id) else { return };
                let before = o.bounds();
                o.translate(dx, dy);
                let after = o.bounds();
                self.invalidate_box(before.union(&after));
                self.drag = Some(Drag::Move { id, last: p, moved: true });
            }
            Drag::Pan { id, last, moved } => {
                if !moved && last.dist(p) < 1.0 {
                    return;
                }
                if !moved {
                    self.checkpoint("Pan image", None);
                }
                let Some(o) = self.doc.get_mut(id) else { return };
                if let (Some(z), Kind::Image { rect, image }) = (o.zone.as_mut(), &o.kind) {
                    if let Some(src) = &image.pixmap {
                        z.pan = Pt::new(z.pan.x + p.x - last.x, z.pan.y + p.y - last.y);
                        // Keep the stored pan within what shows.
                        z.pan = z.placement(rect, src.width() as f32, src.height() as f32).1;
                    }
                }
                let b = o.bounds();
                self.invalidate_box(b);
                self.drag = Some(Drag::Pan { id, last: p, moved: true });
            }
            Drag::Handle { id, handle } => {
                let Some(o) = self.doc.get_mut(id) else { return };
                let before = o.bounds();
                o.move_handle(handle, p, m.shift);
                let after = o.bounds();
                self.invalidate_box(before.union(&after));
                self.events |= EVENT_SELECTION;
            }
            Drag::Marquee { start } => {
                let b = self.marquee_box(start, p, m.shift);
                self.set_marquee(Some(b));
            }
            Drag::Select { start, .. } => self.set_marquee(Some(Box2::from_points(start, p))),
            Drag::Lasso { mut points } => {
                if points.last().is_some_and(|q| q.dist(p) >= 2.0) {
                    points.push(p);
                }
                self.set_marquee(Box2::bounding(points.iter().copied()));
                self.drag = Some(Drag::Lasso { points });
            }
            Drag::Erase { last } => {
                self.erase_segment(last, p);
                self.drag = Some(Drag::Erase { last: p });
            }
            Drag::Canvas { handle } => {
                let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
                let (nw, nh) = match handle {
                    H_R => (p.x, h),
                    H_B => (w, p.y),
                    _ => (p.x, p.y),
                };
                self.set_marquee(Some(Box2::new(0.0, 0.0, nw.max(8.0), nh.max(8.0))));
            }
            Drag::MoveMany { last, moved } => {
                if !moved && last.dist(p) < 1.0 {
                    return;
                }
                if !moved {
                    self.checkpoint("Move", None);
                }
                let (dx, dy) = (p.x - last.x, p.y - last.y);
                for id in self.selected_ids() {
                    if let Some(o) = self.doc.get_mut(id) {
                        let before = o.bounds();
                        o.translate(dx, dy);
                        let after = o.bounds();
                        self.invalidate_box(before.union(&after));
                    }
                }
                self.drag = Some(Drag::MoveMany { last: p, moved: true });
                self.events |= EVENT_SELECTION;
            }
        }
    }

    pub fn pointer_up(&mut self, p: Pt, m: Modifiers) {
        self.pointer_move(p, m);
        let Some(drag) = self.drag.take() else { return };
        match drag {
            Drag::Create { id, start } => {
                let small = start.dist(p) < MIN_DRAG;
                let Some(o) = self.doc.get(id) else { return };
                let text = matches!(o.kind, Kind::Text { .. } | Kind::Callout { .. });
                if small && !matches!(o.kind, Kind::Pen { .. }) {
                    if text || matches!(o.kind, Kind::Magnify { .. }) {
                        let before = o.bounds();
                        let o = self.doc.get_mut(id).unwrap();
                        default_size(o, start);
                        let after = o.bounds();
                        self.invalidate_box(before.union(&after));
                    } else {
                        // A click with a drawing tool: nothing to keep.
                        self.selection = None;
                        if let Some(o) = self.doc.remove(id) {
                            self.invalidate_box(o.bounds());
                        }
                        if let Some(s) = self.undo.pop() {
                            self.doc = s.doc;
                        }
                        self.events |= EVENT_SELECTION;
                        return;
                    }
                }
                if text {
                    self.events |= EVENT_FOCUS_TEXT;
                }
                let strength = self.tool_amount(Tool::Pen);
                if let Some(o) = self.doc.get_mut(id) {
                    let before = o.bounds();
                    if let Kind::Pen { points } = &mut o.kind {
                        if strength > 0.0 && points.len() > 3 {
                            *points = smooth_stroke(points, strength);
                            self.invalidate_box(before);
                        }
                    }
                }
                if self.tool == Tool::Magnify && self.variant(Tool::Magnify).ends_with("view") {
                    let v = self.variant(Tool::Magnify);
                    self.edit(id, "Add Magnify", Some(format!("view{id}")), move |o| apply_variant(o, &v));
                }
                self.events |= EVENT_SELECTION;
            }
            Drag::Lasso { points } => {
                self.set_marquee(None);
                if points.len() >= 3 {
                    self.lift_polygon(&points);
                }
            }
            Drag::Erase { .. } => {}
            Drag::Canvas { .. } => {
                if let Some(m) = self.marquee {
                    self.set_marquee(None);
                    let (nw, nh) = (m.x1.round() as u32, m.y1.round() as u32);
                    let color = self.canvas_color;
                    if (nw, nh) != self.doc.size() {
                        self.edit_document("Resize canvas", |d| {
                            d.resize_canvas(nw.max(8), nh.max(8), 0, 0, color);
                            true
                        });
                    }
                }
            }
            Drag::Select { start, add } => {
                let b = Box2::from_points(start, p);
                self.set_marquee(None);
                let mut ids: Vec<u64> = if add { self.selected_ids() } else { Vec::new() };
                ids.extend(self.doc.objects.iter().rev().filter(|o| o.rect().is_some_and(|r| r.intersects(&b))).map(|o| o.id));
                if !b.is_degenerate(MIN_DRAG) {
                    self.set_selection(ids);
                }
            }
            Drag::Marquee { start } => {
                let b = self.marquee_box(start, p, m.shift);
                self.set_marquee(None);
                if b.is_degenerate(MIN_DRAG) {
                    return;
                }
                let (x0, y0) = (b.x0.round() as i32, b.y0.round() as i32);
                let r = IRect::new(x0, y0, b.x1.round() as i32 - x0, b.y1.round() as i32 - y0);
                match self.tool {
                    Tool::Crop => {
                        self.edit_document("Crop", |d| d.crop(r));
                    }
                    Tool::CutOut => {
                        let v = self.variant(Tool::CutOut);
                        let vertical = v.starts_with("Vertical");
                        let edge = if v.ends_with("sawtooth") {
                            crate::effects::Edge::Sawtooth
                        } else if v.ends_with("wave") {
                            crate::effects::Edge::Wave
                        } else {
                            crate::effects::Edge::Straight
                        };
                        self.edit_document("Cut out", |d| crate::effects::cut_out_with(d, r, vertical, edge));
                    }
                    Tool::Selection if self.variant(Tool::Selection) == "Ellipse" => self.lift_ellipse(r),
                    Tool::Selection => self.lift_selection(r, false),
                    Tool::SmartMove => self.lift_selection(r, true),
                    Tool::TextReplace => self.replace_text_in(r),
                    _ => {}
                }
            }
            _ => self.events |= EVENT_SELECTION,
        }
    }

    fn marquee_box(&self, start: Pt, p: Pt, square: bool) -> Box2 {
        let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
        let p = Pt::new(p.x.clamp(0.0, w), p.y.clamp(0.0, h));
        let b = Box2::from_points(start, p);
        if self.tool == Tool::CutOut {
            return if self.variant(Tool::CutOut).starts_with("Vertical") { Box2::new(b.x0, 0.0, b.x1, h) } else { Box2::new(0.0, b.y0, w, b.y1) };
        }
        if square {
            let s = b.width().max(b.height());
            return Box2::from_points(start, Pt::new(start.x + s * (p.x - start.x).signum(), start.y + s * (p.y - start.y).signum()));
        }
        b
    }

    fn new_object(&mut self, p: Pt) -> (Kind, Style) {
        let tool = self.tool;
        let mut style = self.tool_style(tool);
        let variant = self.variant(tool);
        let at = Box2::from_points(p, p);
        let kind = match tool {
            Tool::Arrow | Tool::Line => Kind::Line { a: p, b: p, control: None },
            Tool::Shape if variant == "Polygon" => Kind::Polygon { points: vec![p, p] },
            Tool::Shape => Kind::Shape { rect: at, shape: shape_of(&variant) },
            Tool::Text => Kind::Text { rect: at, text: String::new() },
            Tool::Callout => Kind::Callout { rect: at, tail: p, text: String::new(), shape: shape_of(&variant), arrow: variant == "Arrow", more_tails: Vec::new() },
            Tool::Step => {
                let format = variant.split(' ').next().unwrap_or("Numbers");
                let shape = if variant.ends_with("square") { ShapeKind::Rounded } else { ShapeKind::Ellipse };
                Kind::Step { center: p, radius: style.font_size * 0.85, label: self.doc.next_step_label_as(format), shape }
            }
            Tool::Pen => Kind::Pen { points: vec![p] },
            Tool::Highlight if variant == "Freehand" => {
                style.stroke = style.fill;
                Kind::Pen { points: vec![p] }
            }
            Tool::Highlight => Kind::Highlight { rect: at, shape: shape_of(&variant) },
            Tool::Blur => Kind::Blur {
                rect: at,
                mode: match variant.as_str() {
                    "Pixelate" => BlurMode::Pixelate,
                    "Redact" => BlurMode::Redact,
                    _ => BlurMode::Blur,
                },
                amount: self.tool_amount(Tool::Blur),
            },
            Tool::Magnify => Kind::Magnify { rect: at, zoom: self.tool_amount(Tool::Magnify), ellipse: !variant.starts_with("Rectangle"), offset: Pt::default() },
            Tool::Spotlight => Kind::Spotlight { rect: at, ellipse: variant == "Ellipse", dim: self.tool_amount(Tool::Spotlight), blur: 0.0, rounded: variant == "Rounded" },
            Tool::Stamp if self.custom_stamps.iter().any(|(n, _)| *n == variant) => {
                let (name, image) = self.custom_stamps.iter().find(|(n, _)| *n == variant).unwrap();
                let scale = (96.0 / image.width().max(image.height()) as f32).min(1.0);
                let (w, h) = (image.width() as f32 * scale, image.height() as f32 * scale);
                style.width = 0.0;
                Kind::Image { rect: Box2::around(p, w / 2.0, h / 2.0), image: ImageRef { key: format!("stamp-{name}"), pixmap: Some(image.clone()) } }
            }
            Tool::Stamp => {
                let size = 72.0;
                Kind::Stamp { rect: Box2::around(p, size / 2.0, size / 2.0), stamp: Stamp::from_name(&variant).unwrap_or_default() }
            }
            _ => Kind::Shape { rect: at, shape: ShapeKind::Rectangle },
        };
        (kind, style)
    }

    fn fill_at(&mut self, p: Pt) {
        let color = self.tool_style(Tool::Fill).fill;
        let tolerance = (self.tool_amount(Tool::Fill).clamp(0.0, 100.0) * 2.55) as u8;
        let global = self.variant(Tool::Fill) == "Global";
        self.edit_document("Fill", |d| {
            if global {
                raster::replace_color(d.background_mut(), p.x as i32, p.y as i32, color, tolerance)
            } else {
                raster::flood_fill(d.background_mut(), p.x as i32, p.y as i32, color, tolerance).is_some()
            }
        });
    }

    /// Lifts the pixels in `r` off the background into a movable image.
    /// Smart Move cuts out just the object and fills the hole from its
    /// surroundings; plain Selection lifts the rectangle and leaves the
    /// canvas color behind.
    fn lift_selection(&mut self, r: IRect, smart: bool) {
        let Some(r) = r.intersect(&self.doc.bounds()) else { return };
        let lifted = if smart { crate::smart::lift_mask(self.doc.background(), r) } else { None };
        let (area, mask) = lifted.unwrap_or_else(|| (r, vec![true; (r.w * r.h) as usize]));
        self.lift_mask(area, &mask, if smart { "Smart Move" } else { "Move selection" });
    }

    fn lift_ellipse(&mut self, r: IRect) {
        let Some(r) = r.intersect(&self.doc.bounds()) else { return };
        let (cx, cy, rx, ry) = (r.w as f32 / 2.0, r.h as f32 / 2.0, r.w as f32 / 2.0, r.h as f32 / 2.0);
        let mask = (0..r.h)
            .flat_map(|y| (0..r.w).map(move |x| ((x as f32 + 0.5 - cx) / rx).powi(2) + ((y as f32 + 0.5 - cy) / ry).powi(2) <= 1.0))
            .collect::<Vec<_>>();
        self.lift_mask(r, &mask, "Move selection");
    }

    fn lift_polygon(&mut self, points: &[Pt]) {
        let Some(b) = Box2::bounding(points.iter().copied()) else { return };
        let Some(r) = b.to_irect().intersect(&self.doc.bounds()) else { return };
        // Even-odd point-in-polygon per pixel center.
        let inside = |x: f32, y: f32| {
            let mut c = false;
            let n = points.len();
            for i in 0..n {
                let (a, b) = (points[i], points[(i + n - 1) % n]);
                if (a.y > y) != (b.y > y) && x < (b.x - a.x) * (y - a.y) / (b.y - a.y) + a.x {
                    c = !c;
                }
            }
            c
        };
        let mask: Vec<bool> = (0..r.h).flat_map(|y| (0..r.w).map(move |x| (x, y))).map(|(x, y)| inside((r.x + x) as f32 + 0.5, (r.y + y) as f32 + 0.5)).collect();
        self.lift_mask(r, &mask, "Move selection");
    }

    /// Selects the area of similar color around `p` and lifts it.
    fn magic_wand(&mut self, p: Pt) {
        let tolerance = (self.tool_amount(Tool::MagicWand).clamp(0.0, 100.0) * 2.55) as u8;
        let found = if self.variant(Tool::MagicWand) == "Global" {
            raster::color_mask(self.doc.background(), p.x as i32, p.y as i32, tolerance)
        } else {
            raster::flood_mask(self.doc.background(), p.x as i32, p.y as i32, tolerance)
        };
        let Some((area, mask)) = found else { return };
        self.lift_mask(area, &mask, "Magic Wand");
    }

    /// Clears the image along a stroke from `a` to `b`.
    fn erase_segment(&mut self, a: Pt, b: Pt) {
        let style = self.tool_style(Tool::Eraser);
        let square = self.variant(Tool::Eraser) == "Square";
        let w = style.width.max(1.0);
        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(a.x, a.y);
        pb.line_to(b.x + 0.01, b.y);
        let Some(path) = pb.finish() else { return };
        let mut paint = tiny_skia::Paint::default();
        paint.anti_alias = true;
        paint.blend_mode = tiny_skia::BlendMode::Clear;
        let cap = if square { tiny_skia::LineCap::Square } else { tiny_skia::LineCap::Round };
        let stroke = tiny_skia::Stroke { width: w, line_cap: cap, ..Default::default() };
        self.doc.background_mut().stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
        self.invalidate_box(Box2::from_points(a, b).inflate(w));
    }

    /// Lifts the masked pixels of `area` into an image object and fills the
    /// hole as `background_fill` says.
    fn lift_mask(&mut self, area: IRect, mask: &[bool], label: &str) {
        let bg = self.doc.background();
        let (aw, ah) = (area.w as usize, area.h as usize);
        let (mut x0, mut y0, mut x1, mut y1) = (aw, ah, 0, 0);
        for y in 0..ah {
            for x in 0..aw {
                if mask[y * aw + x] {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                }
            }
        }
        if x1 < x0 {
            return;
        }
        let at = IRect::new(area.x + x0 as i32, area.y + y0 as i32, (x1 - x0 + 1) as i32, (y1 - y0 + 1) as i32);
        let Some(mut piece) = tiny_skia::Pixmap::new(at.w as u32, at.h as u32) else { return };
        for y in 0..at.h {
            for x in 0..at.w {
                if mask[(y as usize + y0) * aw + x as usize + x0] {
                    raster::set_pixel(&mut piece, x, y, raster::pixel(bg, at.x + x, at.y + y));
                }
            }
        }
        let mut filled = bg.clone();
        match self.background_fill {
            BackgroundFill::Auto => {
                // Pad the mask with a ring of known pixels to fill from.
                let padded = area.inflate(2).intersect(&self.doc.bounds()).unwrap_or(area);
                let (pw, ph) = (padded.w as usize, padded.h as usize);
                let mut ring = vec![false; pw * ph];
                for y in 0..ah {
                    for x in 0..aw {
                        let (px, py) = (area.x - padded.x + x as i32, area.y - padded.y + y as i32);
                        if px >= 0 && py >= 0 && (px as usize) < pw && (py as usize) < ph {
                            ring[py as usize * pw + px as usize] = mask[y * aw + x];
                        }
                    }
                }
                crate::smart::fill_mask(&mut filled, padded, &ring)
            }
            BackgroundFill::Transparent | BackgroundFill::Color => {
                let color = if self.background_fill == BackgroundFill::Color { self.tool_style(self.tool).fill } else { Rgba::TRANSPARENT };
                for y in 0..ah {
                    for x in 0..aw {
                        if mask[y * aw + x] {
                            raster::set_pixel(&mut filled, area.x + x as i32, area.y + y as i32, color);
                        }
                    }
                }
            }
        }
        let rect = at.to_box();
        let key = format!("lift-{}-{}", self.doc.objects.len() + 1, at.x ^ (at.y << 12));
        let mut new_id = 0;
        self.edit_document(label, |d| {
            d.set_background(filled);
            new_id = d.add(Kind::Image { rect, image: ImageRef::new(key, piece) }, Style { width: 0.0, ..Style::default() });
            true
        });
        self.selection = Some(new_id);
        self.set_tool(Tool::Select);
        self.events |= EVENT_SELECTION;
    }

    fn replace_text_in(&mut self, r: IRect) {
        let bg = self.doc.background_arc();
        let found = match self.analyzer.as_mut() {
            Some(a) => a.text_in(&bg, r),
            None => return,
        };
        if found.is_empty() {
            return;
        }
        let mut first = None;
        self.edit_document("Text Replace", |d| {
            for line in found {
                first.get_or_insert(crate::smart::replace_line(d, &line));
            }
            true
        });
        self.selection = first;
        self.events |= EVENT_SELECTION | EVENT_FOCUS_TEXT;
    }
}

fn handle_cursor(h: Handle) -> Cursor {
    match h {
        H_TL | H_BR => Cursor::ResizeNwse,
        H_TR | H_BL => Cursor::ResizeNesw,
        H_T | H_B => Cursor::ResizeNs,
        H_L | H_R => Cursor::ResizeEw,
        _ => Cursor::Point,
    }
}

fn shape_of(variant: &str) -> ShapeKind {
    match variant {
        "Rounded" => ShapeKind::Rounded,
        "Ellipse" | "Circle" => ShapeKind::Ellipse,
        "Triangle" => ShapeKind::Triangle,
        _ => ShapeKind::Rectangle,
    }
}

/// The tool that makes objects like `o`.
pub fn tool_for(o: &Object) -> Tool {
    match &o.kind {
        Kind::Line { .. } if o.style.end_cap == Cap::Arrow || o.style.start_cap == Cap::Arrow => Tool::Arrow,
        Kind::Line { .. } => Tool::Line,
        Kind::Shape { .. } => Tool::Shape,
        Kind::Text { .. } => Tool::Text,
        Kind::Callout { .. } => Tool::Callout,
        Kind::Step { .. } => Tool::Step,
        Kind::Pen { .. } => Tool::Pen,
        Kind::Highlight { .. } => Tool::Highlight,
        Kind::Blur { .. } => Tool::Blur,
        Kind::Magnify { .. } => Tool::Magnify,
        Kind::Spotlight { .. } => Tool::Spotlight,
        Kind::Stamp { .. } => Tool::Stamp,
        Kind::Image { .. } | Kind::Group { .. } => Tool::Select,
        Kind::Polygon { .. } => Tool::Shape,
    }
}

/// The variant name describing `o`, if its kind has variants.
pub fn variant_of(o: &Object) -> Option<String> {
    let name = match &o.kind {
        Kind::Callout { arrow: true, .. } => "Arrow",
        Kind::Shape { shape, .. } | Kind::Callout { shape, .. } => match shape {
            ShapeKind::Rectangle => "Rectangle",
            ShapeKind::Rounded => "Rounded",
            ShapeKind::Ellipse => "Ellipse",
            ShapeKind::Triangle => "Triangle",
        },
        Kind::Blur { mode, .. } => match mode {
            BlurMode::Blur => "Blur",
            BlurMode::Pixelate => "Pixelate",
            BlurMode::Redact => "Redact",
        },
        Kind::Magnify { ellipse, offset, .. } => match (*ellipse, offset.len() > 0.5) {
            (true, false) => "Circle",
            (false, false) => "Rectangle",
            (true, true) => "Circle with view",
            (false, true) => "Rectangle with view",
        },
        Kind::Spotlight { ellipse, rounded, .. } => {
            if *ellipse {
                "Ellipse"
            } else if *rounded {
                "Rounded"
            } else {
                "Rectangle"
            }
        }
        Kind::Highlight { shape, .. } => match shape {
            ShapeKind::Rounded => "Rounded",
            ShapeKind::Ellipse => "Ellipse",
            _ => "Rectangle",
        },
        Kind::Polygon { .. } => "Polygon",
        Kind::Stamp { stamp, .. } => stamp.name(),
        Kind::Pen { .. } => if o.style.multiply { "Marker" } else { "Pen" },
        _ => return None,
    };
    Some(name.to_owned())
}

fn apply_variant(o: &mut Object, v: &str) {
    match &mut o.kind {
        Kind::Callout { shape, arrow, .. } => {
            *arrow = v == "Arrow";
            *shape = shape_of(v);
        }
        Kind::Shape { shape, .. } => *shape = shape_of(v),
        Kind::Blur { mode, .. } => {
            *mode = match v {
                "Pixelate" => BlurMode::Pixelate,
                "Redact" => BlurMode::Redact,
                _ => BlurMode::Blur,
            }
        }
        Kind::Magnify { rect, ellipse, offset, .. } => {
            *ellipse = !v.starts_with("Rectangle");
            if v.ends_with("view") && offset.len() < 0.5 {
                *offset = Pt::new(rect.width() * 1.3, -rect.height() * 0.4);
            } else if !v.ends_with("view") {
                *offset = Pt::default();
            }
        }
        Kind::Spotlight { ellipse, rounded, .. } => {
            *ellipse = v == "Ellipse";
            *rounded = v == "Rounded";
        }
        Kind::Highlight { shape, .. } => *shape = shape_of(v),
        Kind::Step { shape, .. } => *shape = if v.ends_with("square") { ShapeKind::Rounded } else { ShapeKind::Ellipse },
        Kind::Stamp { stamp, .. } => *stamp = Stamp::from_name(v).unwrap_or(*stamp),
        Kind::Pen { .. } => o.style.multiply = v == "Marker",
        _ => {}
    }
}

/// Updates an object being drawn from `start` to `p`.
fn shape_drag(o: &mut Object, start: Pt, p: Pt, shift: bool) {
    let b = if shift {
        let s = (p.x - start.x).abs().max((p.y - start.y).abs());
        Box2::from_points(start, Pt::new(start.x + s * (p.x - start.x).signum(), start.y + s * (p.y - start.y).signum()))
    } else {
        Box2::from_points(start, p)
    };
    match &mut o.kind {
        Kind::Line { b: end, .. } => *end = if shift { snap45(start, p) } else { p },
        Kind::Callout { rect, tail, .. } => {
            *rect = b;
            *tail = Pt::new(b.x0 + b.width() * 0.25, b.y1 + b.height().max(30.0) * 0.6);
        }
        Kind::Stamp { rect, .. } => {
            let s = (p.x - start.x).abs().max((p.y - start.y).abs()).max(16.0);
            *rect = Box2::around(start, s, s);
        }
        _ => {
            if let Some(r) = rect_mut_any(o) {
                *r = b;
            }
            o.fit_text();
        }
    }
}

fn rect_mut_any(o: &mut Object) -> Option<&mut Box2> {
    match &mut o.kind {
        Kind::Shape { rect, .. }
        | Kind::Text { rect, .. }
        | Kind::Highlight { rect, .. }
        | Kind::Blur { rect, .. }
        | Kind::Magnify { rect, .. }
        | Kind::Spotlight { rect, .. }
        | Kind::Image { rect, .. } => Some(rect),
        _ => None,
    }
}

fn default_size(o: &mut Object, at: Pt) {
    let fs = o.style.font_size;
    match &mut o.kind {
        Kind::Text { rect, .. } => *rect = Box2::from_xywh(at.x, at.y, (fs * 10.0).max(160.0), fs * 1.8),
        Kind::Callout { rect, tail, .. } => {
            *rect = Box2::from_xywh(at.x, at.y, (fs * 9.0).max(180.0), fs * 3.4);
            *tail = Pt::new(at.x + rect.width() * 0.25, rect.y1 + fs * 2.0);
        }
        Kind::Magnify { rect, .. } => *rect = Box2::around(at, 90.0, 90.0),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> Editor {
        let mut e = Editor::new(Document::blank(400, 300, Rgba::WHITE));
        e.take_damage();
        e
    }

    const NONE: Modifiers = Modifiers { shift: false, ctrl: false, alt: false };

    fn drag(e: &mut Editor, a: (f32, f32), b: (f32, f32)) {
        e.pointer_down(Pt::new(a.0, a.1), NONE, 4.0);
        e.pointer_move(Pt::new((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), NONE);
        e.pointer_up(Pt::new(b.0, b.1), NONE);
    }

    #[test]
    fn draw_select_move_undo_redo() {
        let mut e = editor();
        e.set_tool(Tool::Arrow);
        drag(&mut e, (50.0, 50.0), (200.0, 50.0));
        assert_eq!(e.document().objects.len(), 1);
        let id = e.selection().unwrap();
        let damage = e.take_damage().unwrap();
        assert!(damage.w < 220 && damage.h < 80, "only the arrow was redrawn: {damage:?}");
        // Click on the arrow with the same tool picks it up and moves it.
        drag(&mut e, (90.0, 50.0), (90.0, 150.0));
        assert_eq!(e.document().objects.len(), 1);
        let Kind::Line { a, .. } = e.document().get(id).unwrap().kind else { panic!() };
        assert_eq!(a, Pt::new(50.0, 150.0));
        assert!(e.undo());
        let Kind::Line { a, .. } = e.document().get(id).unwrap().kind else { panic!() };
        assert_eq!(a, Pt::new(50.0, 50.0));
        assert!(e.undo());
        assert!(e.document().objects.is_empty());
        assert!(e.redo() && e.redo());
        assert_eq!(e.document().objects.len(), 1);
        // A plain click with a drawing tool adds nothing.
        e.select(None);
        e.pointer_down(Pt::new(300.0, 250.0), NONE, 4.0);
        e.pointer_up(Pt::new(300.0, 250.0), NONE);
        assert_eq!(e.document().objects.len(), 1);
    }

    #[test]
    fn text_steps_styles_and_crop() {
        let mut e = editor();
        e.set_tool(Tool::Text);
        e.pointer_down(Pt::new(10.0, 10.0), NONE, 4.0);
        e.pointer_up(Pt::new(10.0, 10.0), NONE);
        assert!(e.take_events() & EVENT_FOCUS_TEXT != 0);
        e.set_selected_text("Hello");
        e.set_selected_text("Hello there");
        assert_eq!(e.selected().unwrap().text(), Some("Hello there"));
        e.update_style("size", |s| s.font_size = 40.0);
        e.update_style("size", |s| s.font_size = 44.0);
        assert_eq!(e.tool_style(Tool::Text).font_size, 44.0);
        assert_eq!(e.undo.len(), 3, "add, text and size edits coalesce");
        // Leaving an empty text box removes it.
        e.set_tool(Tool::Text);
        e.select(None);
        e.pointer_down(Pt::new(200.0, 200.0), NONE, 4.0);
        e.pointer_up(Pt::new(200.0, 200.0), NONE);
        assert_eq!(e.document().objects.len(), 2);
        e.select(None);
        assert_eq!(e.document().objects.len(), 1);

        e.set_tool(Tool::Step);
        for x in [100.0, 150.0, 200.0] {
            e.select(None);
            e.pointer_down(Pt::new(x, 250.0), NONE, 4.0);
            e.pointer_up(Pt::new(x, 250.0), NONE);
        }
        let labels: Vec<_> = e.document().objects.iter().filter_map(|o| o.text().filter(|_| matches!(o.kind, Kind::Step { .. }))).collect();
        assert_eq!(labels, ["1", "2", "3"]);

        e.set_tool(Tool::Crop);
        drag(&mut e, (5.0, 5.0), (305.0, 205.0));
        assert_eq!(e.document().size(), (300, 200));
        assert!(e.take_events() & EVENT_RESIZED != 0);
        assert!(e.undo());
        assert_eq!(e.document().size(), (400, 300));
    }

    #[test]
    fn handles_resize_and_fill() {
        let mut e = editor();
        e.set_tool(Tool::Shape);
        drag(&mut e, (10.0, 10.0), (110.0, 60.0));
        let id = e.selection().unwrap();
        drag(&mut e, (110.0, 60.0), (210.0, 110.0));
        assert_eq!(e.document().get(id).unwrap().rect(), Some(Box2::new(10.0, 10.0, 210.0, 110.0)));
        e.set_tool(Tool::Fill);
        e.pointer_down(Pt::new(300.0, 250.0), NONE, 4.0);
        e.pointer_up(Pt::new(300.0, 250.0), NONE);
        assert_eq!(raster::pixel(e.document().background(), 399, 299), Rgba::YELLOW);
    }
}

/// Evens out a freehand stroke: a few passes of neighbor averaging
/// (`strength` 1 to 10), ends kept, then points closer than a pixel
/// dropped.
pub fn smooth_stroke(points: &[Pt], strength: f32) -> Vec<Pt> {
    let passes = strength.round().clamp(1.0, 10.0) as usize;
    let mut cur = points.to_vec();
    for _ in 0..passes {
        let mut next = cur.clone();
        for i in 1..cur.len() - 1 {
            let (a, b, c) = (cur[i - 1], cur[i], cur[i + 1]);
            next[i] = Pt::new(a.x * 0.25 + b.x * 0.5 + c.x * 0.25, a.y * 0.25 + b.y * 0.5 + c.y * 0.25);
        }
        cur = next;
    }
    let mut out = vec![cur[0]];
    for &p in &cur[1..] {
        if out.last().unwrap().dist(p) >= 1.0 {
            out.push(p);
        }
    }
    if out.last() != cur.last() {
        out.push(*cur.last().unwrap());
    }
    out
}
