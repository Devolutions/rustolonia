//! Editor settings: the `editor` section of Jiffshot's settings file (see
//! `jiffshot_capture::config`). The library folder and the color theme are
//! shared with the capture side and kept at the top of that file.

use serde::{Deserialize, Serialize};
use jiffshot_doc::editor::{default_style, Tool};
use jiffshot_doc::object::{Cap, Dash, Style};
use jiffshot_doc::Rgba;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The shared library folder, as loaded (changed through
    /// `config::set_library`).
    #[serde(skip)]
    pub library: Option<PathBuf>,
    /// Quick styles saved by the user, by tool name, newest last.
    pub quick_styles: BTreeMap<String, Vec<Style>>,
    /// A tool's whole list of quick styles, once it was reordered,
    /// trimmed or themed (built-in ones included).
    pub style_lists: BTreeMap<String, Vec<Style>>,
    /// Favorite quick styles, by tool name; shown first.
    pub favorite_styles: BTreeMap<String, Vec<Style>>,
    /// Share destination `ftp://user@host/folder`; the password is the
    /// "Jiffshot FTP" generic credential in Windows Credential Manager.
    pub ftp: Option<String>,
    /// Programs added as share destinations (name, executable).
    pub programs: Vec<(String, PathBuf)>,
    /// The shared color theme, "Dark", "Light" or "System", as loaded
    /// (changed through `config::set_theme`).
    #[serde(skip)]
    pub theme: Option<String>,
    pub video: VideoPrefs,
    pub library_view: LibraryView,
    pub layout: Layout,
    /// The extension Export used last.
    pub export_format: Option<String>,
    pub template: TemplatePrefs,
    /// The tools shown in the toolbar, in order (all, in the usual order,
    /// when empty).
    pub toolbar: Vec<String>,
    /// Files opened lately, newest first.
    pub recent: Vec<PathBuf>,
}

/// Which parts of the window show, kept between sessions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    /// Quick Styles and tool properties, on the right.
    pub side_bar: bool,
    /// The Effects panel, beside the side bar.
    pub effects: bool,
    /// The Recent Captures tray along the bottom.
    pub tray: bool,
    /// Tool names under their icons in the toolbar.
    pub labels: bool,
}

impl Default for Layout {
    fn default() -> Layout {
        Layout { side_bar: true, effects: false, tray: true, labels: true }
    }
}

/// Template choices, kept between sessions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TemplatePrefs {
    pub theme: String,
    pub captions: bool,
    pub numbers: bool,
    pub footer: String,
}

/// How the library is shown, kept between sessions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryView {
    pub search: String,
    pub filter: String,
    pub sort: String,
    /// Thumbnail height in the tray, pixels.
    pub thumb: f64,
    pub expanded: bool,
    /// Captures closed from the tray (still in the library).
    pub closed: Vec<PathBuf>,
}

impl Default for LibraryView {
    fn default() -> LibraryView {
        LibraryView { search: String::new(), filter: "All".into(), sort: "Newest".into(), thumb: 120.0, expanded: false, closed: Vec::new() }
    }
}

/// GIF export and playback choices, kept between sessions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoPrefs {
    /// "Screen recording", "High motion", "Small file" or "Custom".
    pub gif_preset: String,
    /// Used by the Custom preset.
    pub gif_fps: f64,
    pub gif_width: u32,
    pub gif_loop: bool,
    pub gif_fade: bool,
    pub gif_dither: bool,
    /// Playback volume, 0 to 1.
    pub volume: f64,
    pub muted: bool,
    /// Seconds each picture shows in a video made from images.
    pub slide_seconds: f64,
}

impl Default for VideoPrefs {
    fn default() -> VideoPrefs {
        VideoPrefs {
            gif_preset: "Screen recording".into(),
            gif_fps: 12.0,
            gif_width: 800,
            gif_loop: true,
            gif_fade: false,
            gif_dither: false,
            volume: 1.0,
            muted: false,
            slide_seconds: 3.0,
        }
    }
}

pub const GIF_PRESETS: &[&str] = &["Screen recording", "High motion", "Small file", "Custom"];

impl VideoPrefs {
    /// Frames a second and largest width.
    pub fn gif(&self) -> (f64, u32) {
        match self.gif_preset.as_str() {
            "High motion" => (20.0, 640),
            "Small file" => (8.0, 480),
            "Custom" => (self.gif_fps.clamp(1.0, 50.0), self.gif_width.clamp(64, 3840)),
            _ => (10.0, 800),
        }
    }
}

fn built_in(tool: Tool) -> Vec<Style> {
    let base = default_style(tool);
    let blue = Rgba::rgb(0x1E, 0x6F, 0xD9);
    let green = Rgba::rgb(0x2E, 0x9E, 0x4F);
    let ink = Rgba::rgb(0x22, 0x26, 0x2E);
    let with = |stroke: Rgba, fill: Rgba, text: Rgba, width: f32| Style { stroke, fill, text_color: text, width, ..base.clone() };
    match tool {
        Tool::Arrow | Tool::Line => vec![
            with(Rgba::RED, Rgba::TRANSPARENT, Rgba::BLACK, 8.0),
            with(blue, Rgba::TRANSPARENT, Rgba::BLACK, 8.0),
            Style { shadow: true, ..with(Rgba::YELLOW, Rgba::TRANSPARENT, Rgba::BLACK, 10.0) },
            Style { dash: Dash::Dash, end_cap: if tool == Tool::Arrow { Cap::Arrow } else { Cap::None }, ..with(ink, Rgba::TRANSPARENT, Rgba::BLACK, 4.0) },
            Style { start_cap: Cap::Arrow, ..with(green, Rgba::TRANSPARENT, Rgba::BLACK, 6.0) },
        ],
        Tool::Shape => vec![
            with(Rgba::RED, Rgba::TRANSPARENT, Rgba::BLACK, 6.0),
            with(blue, Rgba::TRANSPARENT, Rgba::BLACK, 6.0),
            Style { shadow: true, ..with(Rgba::WHITE, Rgba::TRANSPARENT, Rgba::BLACK, 6.0) },
            with(Rgba::TRANSPARENT, Rgba::YELLOW.with_alpha(110), Rgba::BLACK, 0.0),
            with(ink, ink, Rgba::BLACK, 0.0),
        ],
        Tool::Text => vec![
            with(Rgba::TRANSPARENT, Rgba::TRANSPARENT, Rgba::RED, 0.0),
            with(Rgba::TRANSPARENT, Rgba::TRANSPARENT, ink, 0.0),
            with(Rgba::TRANSPARENT, ink, Rgba::WHITE, 0.0),
            with(Rgba::TRANSPARENT, Rgba::YELLOW, ink, 0.0),
            Style { shadow: true, ..with(Rgba::TRANSPARENT, Rgba::TRANSPARENT, Rgba::WHITE, 0.0) },
        ],
        Tool::Callout => vec![
            with(blue, blue, Rgba::WHITE, 3.0),
            with(Rgba::RED, Rgba::RED, Rgba::WHITE, 3.0),
            with(ink, Rgba::YELLOW, ink, 3.0),
            with(ink, Rgba::WHITE, ink, 3.0),
        ],
        Tool::Step => vec![
            with(Rgba::TRANSPARENT, Rgba::RED, Rgba::WHITE, 0.0),
            with(Rgba::TRANSPARENT, blue, Rgba::WHITE, 0.0),
            with(Rgba::TRANSPARENT, ink, Rgba::WHITE, 0.0),
            with(Rgba::RED, Rgba::WHITE, Rgba::RED, 3.0),
        ],
        Tool::Pen => vec![
            with(Rgba::RED, Rgba::TRANSPARENT, Rgba::BLACK, 5.0),
            with(blue, Rgba::TRANSPARENT, Rgba::BLACK, 5.0),
            with(ink, Rgba::TRANSPARENT, Rgba::BLACK, 3.0),
            Style { multiply: true, ..with(Rgba::YELLOW.with_alpha(180), Rgba::TRANSPARENT, Rgba::BLACK, 20.0) },
        ],
        Tool::Highlight => vec![
            Style { fill: Rgba::YELLOW.with_alpha(180), stroke: Rgba::YELLOW.with_alpha(180), ..base.clone() },
            Style { fill: Rgba::rgb(0x8B, 0xF0, 0x8B).with_alpha(180), stroke: Rgba::rgb(0x8B, 0xF0, 0x8B).with_alpha(180), ..base.clone() },
            Style { fill: Rgba::rgb(0xFF, 0x9E, 0xD6).with_alpha(180), stroke: Rgba::rgb(0xFF, 0x9E, 0xD6).with_alpha(180), ..base.clone() },
            Style { fill: Rgba::rgb(0x8E, 0xD1, 0xFF).with_alpha(180), stroke: Rgba::rgb(0x8E, 0xD1, 0xFF).with_alpha(180), ..base.clone() },
        ],
        Tool::Magnify => vec![
            with(Rgba::rgb(0x30, 0x30, 0x30), Rgba::TRANSPARENT, Rgba::BLACK, 4.0),
            with(Rgba::RED, Rgba::TRANSPARENT, Rgba::BLACK, 5.0),
            with(Rgba::WHITE, Rgba::TRANSPARENT, Rgba::BLACK, 6.0),
        ],
        _ => Vec::new(),
    }
}

impl Settings {
    pub fn load() -> Settings {
        let mut s: Settings = serde_json::from_value(jiffshot_capture::config::section("editor")).unwrap_or_default();
        s.library = jiffshot_capture::config::library();
        s.theme = jiffshot_capture::config::theme();
        s
    }

    /// Saves the editor's own section (not the shared library or theme).
    pub fn save(&self) {
        if let Ok(value) = serde_json::to_value(self) {
            jiffshot_capture::config::set_section("editor", value);
        }
    }

    /// Changes the shared color theme.
    pub fn set_theme(&mut self, theme: &str) {
        self.theme = Some(theme.to_owned());
        jiffshot_capture::config::set_theme(theme);
    }

    /// Changes the shared library folder.
    pub fn set_library(&mut self, folder: PathBuf) {
        jiffshot_capture::config::set_library(Some(&folder));
        self.library = Some(folder);
    }

    /// The tool's quick styles in order: an edited list, or the built-in
    /// styles followed by the user's own.
    pub fn quick_styles(&self, tool: Tool) -> Vec<Style> {
        if let Some(list) = self.style_lists.get(tool.name()) {
            return list.clone();
        }
        let mut all = built_in(tool);
        all.extend(self.quick_styles.get(tool.name()).cloned().unwrap_or_default());
        all
    }

    /// Changes a tool's list of quick styles (keeping it from then on).
    pub fn edit_styles(&mut self, tool: Tool, change: impl FnOnce(&mut Vec<Style>)) {
        let mut list = self.quick_styles(tool);
        change(&mut list);
        self.style_lists.insert(tool.name().to_owned(), list);
    }

    pub fn is_favorite(&self, tool: Tool, style: &Style) -> bool {
        self.favorite_styles.get(tool.name()).is_some_and(|f| f.contains(style))
    }

    pub fn toggle_favorite(&mut self, tool: Tool, style: &Style) {
        let list = self.favorite_styles.entry(tool.name().to_owned()).or_default();
        match list.iter().position(|s| s == style) {
            Some(i) => {
                list.remove(i);
            }
            None => list.push(style.clone()),
        }
    }

    /// Back to the built-in styles (saved ones are forgotten).
    pub fn reset_styles(&mut self, tool: Tool) {
        self.style_lists.remove(tool.name());
        self.quick_styles.remove(tool.name());
    }

    /// The built-in styles recolored with a palette (stroke, fill and text
    /// colors that are not white or clear take the palette in turn).
    pub fn themed_styles(tool: Tool, palette: &[Rgba]) -> Vec<Style> {
        let recolor = |c: Rgba, i: usize| {
            if !c.is_visible() || (c.r > 240 && c.g > 240 && c.b > 240) {
                c
            } else {
                palette[i % palette.len()].with_alpha(c.a)
            }
        };
        built_in(tool)
            .into_iter()
            .enumerate()
            .map(|(i, s)| Style { stroke: recolor(s.stroke, i), fill: recolor(s.fill, i), text_color: recolor(s.text_color, i + 1), ..s })
            .collect()
    }

    pub fn add_quick_style(&mut self, tool: &str, style: Style) {
        if let Some(list) = self.style_lists.get_mut(tool) {
            list.retain(|s| *s != style);
            list.push(style);
            return;
        }
        let list = self.quick_styles.entry(tool.to_owned()).or_default();
        list.retain(|s| *s != style);
        list.push(style);
        if list.len() > 12 {
            list.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_styles_reorder_delete_favorite_theme_and_reset() {
        let mut s = Settings::default();
        let built = s.quick_styles(Tool::Arrow);
        assert!(built.len() >= 3);
        s.edit_styles(Tool::Arrow, |l| l.swap(0, 1));
        assert_eq!(s.quick_styles(Tool::Arrow)[0], built[1]);
        s.edit_styles(Tool::Arrow, |l| {
            l.remove(0);
        });
        assert_eq!(s.quick_styles(Tool::Arrow).len(), built.len() - 1);
        s.add_quick_style("Arrow", Style { width: 33.0, ..built[0].clone() });
        assert_eq!(s.quick_styles(Tool::Arrow).last().unwrap().width, 33.0, "saved styles join an edited list");
        s.toggle_favorite(Tool::Arrow, &built[2]);
        assert!(s.is_favorite(Tool::Arrow, &built[2]));
        s.toggle_favorite(Tool::Arrow, &built[2]);
        assert!(!s.is_favorite(Tool::Arrow, &built[2]));
        let palette = [Rgba::rgb(1, 2, 3), Rgba::rgb(4, 5, 6)];
        let themed = Settings::themed_styles(Tool::Arrow, &palette);
        assert_eq!(themed[0].stroke, palette[0]);
        assert_eq!(themed[1].stroke, palette[1]);
        s.reset_styles(Tool::Arrow);
        assert_eq!(s.quick_styles(Tool::Arrow), built);
    }
}
