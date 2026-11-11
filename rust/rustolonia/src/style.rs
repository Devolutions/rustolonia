//! Pure-Rust declarative styling (`StyleSheet`, `Style`), [`BoxShadow`] /
//! [`BoxShadows`], and platform-aware [`Shortcut`] key gestures.

use crate::runtime::AsControl;
use crate::view::{El, IntoCommandProp, IntoValue};
use crate::{
    App, AppScope, Color, CornerRadius, FontWeight, Key, KeyModifiers, LinearGradientBrush, Paint,
    RadialGradientBrush, RelativePoint, RelativeScalar, Result, Thickness, Variant, Window,
};
use std::fmt;
use std::time::Duration;

/// Single drop or inset shadow on a `Border`, mirroring `Avalonia.Media.BoxShadow`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur: f64,
    pub spread: f64,
    pub color: Color,
    pub inset: bool,
}

impl BoxShadow {
    pub const fn new(offset_x: f64, offset_y: f64, blur: f64, color: Color) -> Self {
        Self {
            offset_x,
            offset_y,
            blur,
            spread: 0.0,
            color,
            inset: false,
        }
    }

    pub const fn spread(mut self, spread: f64) -> Self {
        self.spread = spread;
        self
    }

    pub const fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }
}

impl fmt::Display for BoxShadow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.inset {
            f.write_str("inset ")?;
        }
        write!(
            f,
            "{} {} {} {} #{:02X}{:02X}{:02X}{:02X}",
            self.offset_x,
            self.offset_y,
            self.blur,
            self.spread,
            self.color.a,
            self.color.r,
            self.color.g,
            self.color.b
        )
    }
}

/// Collection of [`BoxShadow`] layers for `Border::box_shadow` and [`Style::box_shadow`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoxShadows(pub Vec<BoxShadow>);

impl BoxShadows {
    pub const fn none() -> Self {
        Self(Vec::new())
    }
}

impl From<BoxShadow> for BoxShadows {
    fn from(shadow: BoxShadow) -> Self {
        Self(vec![shadow])
    }
}

impl<const N: usize> From<[BoxShadow; N]> for BoxShadows {
    fn from(shadows: [BoxShadow; N]) -> Self {
        Self(shadows.into())
    }
}

impl fmt::Display for BoxShadows {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("none");
        }
        for (i, shadow) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{shadow}")?;
        }
        Ok(())
    }
}

impl IntoValue<String> for BoxShadow {
    fn into_value(self) -> String {
        self.to_string()
    }
}

impl IntoValue<String> for BoxShadows {
    fn into_value(self) -> String {
        self.to_string()
    }
}

/// Platform-aware keyboard shortcut (`Key` + `KeyModifiers`), convertible to
/// Avalonia `KeyGesture` strings (`Cmd+O` on Apple platforms, `Ctrl+O` elsewhere).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub modifiers: KeyModifiers,
    pub primary: bool,
    pub key: Key,
}

impl Shortcut {
    pub const fn key(key: Key) -> Self {
        Self {
            modifiers: KeyModifiers::NONE,
            primary: false,
            key,
        }
    }

    pub const fn new(modifiers: KeyModifiers, key: Key) -> Self {
        Self {
            modifiers,
            primary: false,
            key,
        }
    }

    /// `Cmd+<key>` on macOS/iOS and `Ctrl+<key>` on Windows/Linux/Browser.
    pub const fn primary(key: Key) -> Self {
        Self {
            modifiers: KeyModifiers::NONE,
            primary: true,
            key,
        }
    }

    /// `Cmd+Shift+<key>` on macOS/iOS and `Ctrl+Shift+<key>` elsewhere.
    pub const fn primary_shift(key: Key) -> Self {
        Self {
            modifiers: KeyModifiers::SHIFT,
            primary: true,
            key,
        }
    }

    pub fn to_gesture_string(self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        let is_apple = cfg!(any(target_os = "macos", target_os = "ios"));
        if self.primary {
            parts.push(if is_apple { "Cmd" } else { "Ctrl" }.to_string());
        }
        if self.modifiers.contains(KeyModifiers::CONTROL) && !(self.primary && !is_apple) {
            parts.push("Ctrl".to_string());
        }
        if self.modifiers.contains(KeyModifiers::META) && !(self.primary && is_apple) {
            parts.push("Cmd".to_string());
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt".to_string());
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift".to_string());
        }
        parts.push(format!("{:?}", self.key));
        f.write_str(&parts.join("+"))
    }
}

impl From<Shortcut> for String {
    fn from(s: Shortcut) -> Self {
        s.to_string()
    }
}

impl IntoValue<String> for Shortcut {
    fn into_value(self) -> String {
        self.to_string()
    }
}

impl IntoValue<Option<String>> for Shortcut {
    fn into_value(self) -> Option<String> {
        Some(self.to_string())
    }
}

#[derive(Clone, Debug)]
enum StyleSetter {
    Paint(&'static str, Option<Paint>),
    Thickness(&'static str, Thickness),
    CornerRadius(CornerRadius),
    Float(&'static str, f64),
    Int(&'static str, i32),
    Text(&'static str, String),
}

#[derive(Clone, Debug)]
struct StyleTransition {
    prop: &'static str,
    duration_ms: i64,
}

/// A single declarative style rule targeting a control type, `.class` list, and
/// `:pseudo` state (`:pointerover`, `:pressed`, `:disabled`, `:focus`, `:checked`, `:selected`).
#[derive(Clone, Debug, Default)]
pub struct Style {
    target: Option<String>,
    classes: Vec<String>,
    pseudos: Vec<String>,
    setters: Vec<StyleSetter>,
    transitions: Vec<StyleTransition>,
}

impl Style {
    /// Parses a concise CSS/Avalonia selector such as `"Button.primary:pointerover"`,
    /// `".card"`, or `"Border.elevated:focus"`.
    pub fn select(selector: impl AsRef<str>) -> Self {
        let raw = selector.as_ref().trim();
        let mut style = Self::default();
        if raw.is_empty() {
            return style;
        }
        // Split `:pseudo` segments after classes/target
        let mut i = 0;
        let bytes = raw.as_bytes();
        while i < bytes.len() {
            match bytes[i] {
                b'.' => {
                    i += 1;
                    let start = i;
                    while i < bytes.len()
                        && bytes[i] != b'.'
                        && bytes[i] != b':'
                        && !bytes[i].is_ascii_whitespace()
                    {
                        i += 1;
                    }
                    if i > start {
                        style.classes.push(raw[start..i].to_string());
                    }
                }
                b':' => {
                    let start = i;
                    i += 1;
                    while i < bytes.len()
                        && bytes[i] != b'.'
                        && bytes[i] != b':'
                        && !bytes[i].is_ascii_whitespace()
                    {
                        i += 1;
                    }
                    if i > start + 1 {
                        style.pseudos.push(raw[start..i].to_string());
                    }
                }
                ch if ch.is_ascii_whitespace() => {
                    i += 1;
                }
                _ => {
                    let start = i;
                    while i < bytes.len()
                        && bytes[i] != b'.'
                        && bytes[i] != b':'
                        && !bytes[i].is_ascii_whitespace()
                    {
                        i += 1;
                    }
                    if style.target.is_none() && i > start {
                        let t = &raw[start..i];
                        if t != "*" {
                            style.target = Some(t.to_string());
                        }
                    }
                }
            }
        }
        style
    }

    pub fn control(target_type: impl Into<String>) -> Self {
        Self {
            target: Some(target_type.into()),
            ..Self::default()
        }
    }

    pub fn class(mut self, name: impl Into<String>) -> Self {
        self.classes.push(name.into());
        self
    }

    pub fn pseudo(mut self, pseudo: impl Into<String>) -> Self {
        let p = pseudo.into();
        self.pseudos.push(if p.starts_with(':') {
            p
        } else {
            format!(":{p}")
        });
        self
    }

    pub fn hover(self) -> Self {
        self.pseudo(":pointerover")
    }

    pub fn pressed(self) -> Self {
        self.pseudo(":pressed")
    }

    pub fn disabled(self) -> Self {
        self.pseudo(":disabled")
    }

    pub fn focused(self) -> Self {
        self.pseudo(":focus")
    }

    pub fn checked(self) -> Self {
        self.pseudo(":checked")
    }

    pub fn selected(self) -> Self {
        self.pseudo(":selected")
    }

    pub fn background(mut self, paint: impl IntoValue<Option<Paint>>) -> Self {
        self.setters
            .push(StyleSetter::Paint("background", paint.into_value()));
        self
    }

    pub fn foreground(mut self, paint: impl IntoValue<Option<Paint>>) -> Self {
        self.setters
            .push(StyleSetter::Paint("foreground", paint.into_value()));
        self
    }

    pub fn border_brush(mut self, paint: impl IntoValue<Option<Paint>>) -> Self {
        self.setters
            .push(StyleSetter::Paint("borderBrush", paint.into_value()));
        self
    }

    pub fn border_thickness(mut self, thickness: impl IntoValue<Thickness>) -> Self {
        self.setters.push(StyleSetter::Thickness(
            "borderThickness",
            thickness.into_value(),
        ));
        self
    }

    pub fn corner_radius(mut self, radius: impl IntoValue<CornerRadius>) -> Self {
        self.setters
            .push(StyleSetter::CornerRadius(radius.into_value()));
        self
    }

    pub fn padding(mut self, padding: impl IntoValue<Thickness>) -> Self {
        self.setters
            .push(StyleSetter::Thickness("padding", padding.into_value()));
        self
    }

    pub fn margin(mut self, margin: impl IntoValue<Thickness>) -> Self {
        self.setters
            .push(StyleSetter::Thickness("margin", margin.into_value()));
        self
    }

    pub fn opacity(mut self, opacity: f64) -> Self {
        self.setters.push(StyleSetter::Float("opacity", opacity));
        self
    }

    pub fn font_size(mut self, size: f64) -> Self {
        self.setters.push(StyleSetter::Float("fontSize", size));
        self
    }

    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.setters
            .push(StyleSetter::Int("fontWeight", weight as i32));
        self
    }

    pub fn box_shadow(mut self, shadow: impl Into<BoxShadows>) -> Self {
        self.setters
            .push(StyleSetter::Text("boxShadow", shadow.into().to_string()));
        self
    }

    pub fn width(mut self, width: f64) -> Self {
        self.setters.push(StyleSetter::Float("width", width));
        self
    }

    pub fn height(mut self, height: f64) -> Self {
        self.setters.push(StyleSetter::Float("height", height));
        self
    }

    pub fn min_width(mut self, min_width: f64) -> Self {
        self.setters.push(StyleSetter::Float("minWidth", min_width));
        self
    }

    pub fn min_height(mut self, min_height: f64) -> Self {
        self.setters
            .push(StyleSetter::Float("minHeight", min_height));
        self
    }

    pub fn max_width(mut self, max_width: f64) -> Self {
        self.setters.push(StyleSetter::Float("maxWidth", max_width));
        self
    }

    pub fn max_height(mut self, max_height: f64) -> Self {
        self.setters
            .push(StyleSetter::Float("maxHeight", max_height));
        self
    }

    pub fn transition_background(self, duration: Duration) -> Self {
        self.transition("background", duration)
    }

    pub fn transition_foreground(self, duration: Duration) -> Self {
        self.transition("foreground", duration)
    }

    pub fn transition_border_brush(self, duration: Duration) -> Self {
        self.transition("borderBrush", duration)
    }

    pub fn transition_opacity(self, duration: Duration) -> Self {
        self.transition("opacity", duration)
    }

    pub fn transition_corner_radius(self, duration: Duration) -> Self {
        self.transition("cornerRadius", duration)
    }

    pub fn transition_box_shadow(self, duration: Duration) -> Self {
        self.transition("boxShadow", duration)
    }

    pub fn transition_padding(self, duration: Duration) -> Self {
        self.transition("padding", duration)
    }

    pub fn transition_margin(self, duration: Duration) -> Self {
        self.transition("margin", duration)
    }

    fn transition(mut self, prop: &'static str, duration: Duration) -> Self {
        let duration_ms = duration.as_millis().clamp(0, i64::MAX as u128) as i64;
        self.transitions.push(StyleTransition { prop, duration_ms });
        self
    }

    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        if let Some(target) = &self.target {
            out.push_str("\"target\":");
            push_json_str(out, target);
            first = false;
        }
        if !self.classes.is_empty() {
            if !first {
                out.push(',');
            }
            out.push_str("\"classes\":[");
            for (i, cls) in self.classes.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                push_json_str(out, cls);
            }
            out.push(']');
            first = false;
        }
        if !self.pseudos.is_empty() {
            if !first {
                out.push(',');
            }
            out.push_str("\"pseudos\":[");
            for (i, p) in self.pseudos.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                push_json_str(out, p);
            }
            out.push(']');
            first = false;
        }
        if !first {
            out.push(',');
        }
        out.push_str("\"setters\":[");
        for (i, setter) in self.setters.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_setter_json(out, setter);
        }
        out.push(']');
        if !self.transitions.is_empty() {
            out.push_str(",\"transitions\":[");
            for (i, tr) in self.transitions.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&format!(
                    "{{\"prop\":\"{}\",\"durationMs\":{}}}",
                    tr.prop, tr.duration_ms
                ));
            }
            out.push(']');
        }
        out.push('}');
    }
}

/// Ordered collection of [`Style`] rules applicable at the application or control level.
#[derive(Clone, Debug, Default)]
pub struct StyleSheet {
    rules: Vec<Style>,
}

impl StyleSheet {
    pub const fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn rule(mut self, rule: Style) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"rules\":[");
        for (i, rule) in self.rules.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            rule.write_json(&mut out);
        }
        out.push_str("]}");
        out
    }
}

impl From<Style> for StyleSheet {
    fn from(rule: Style) -> Self {
        Self { rules: vec![rule] }
    }
}

impl<const N: usize> From<[Style; N]> for StyleSheet {
    fn from(rules: [Style; N]) -> Self {
        Self {
            rules: rules.into(),
        }
    }
}

impl From<Vec<Style>> for StyleSheet {
    fn from(rules: Vec<Style>) -> Self {
        Self { rules }
    }
}

impl FromIterator<Style> for StyleSheet {
    fn from_iter<T: IntoIterator<Item = Style>>(iter: T) -> Self {
        Self {
            rules: iter.into_iter().collect(),
        }
    }
}

fn push_json_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn pack_argb(c: Color) -> u32 {
    (u32::from(c.a) << 24) | (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b)
}

fn write_rel_point(out: &mut String, pt: RelativePoint) {
    out.push_str(&format!("[{},{},{}]", pt.x, pt.y, pt.unit as i32));
}

fn write_rel_scalar(out: &mut String, sc: RelativeScalar) {
    out.push_str(&format!("[{},{}]", sc.scalar, sc.unit as i32));
}

fn write_paint_json(out: &mut String, paint: &Option<Paint>) {
    match paint {
        None => out.push_str("null"),
        Some(Paint::Solid(brush)) => {
            out.push_str(&format!(
                "{{\"kind\":\"solid\",\"argb\":{},\"opacity\":{}}}",
                pack_argb(brush.color),
                brush.opacity
            ));
        }
        Some(Paint::LinearGradient(LinearGradientBrush {
            opacity,
            spread_method,
            stops,
            start_point,
            end_point,
        })) => {
            out.push_str(&format!(
                "{{\"kind\":\"linear\",\"opacity\":{},\"spread\":{},\"start\":",
                opacity, *spread_method as i32
            ));
            write_rel_point(out, *start_point);
            out.push_str(",\"end\":");
            write_rel_point(out, *end_point);
            out.push_str(",\"stops\":[");
            for (i, stop) in stops.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&format!("[{},{}]", stop.offset, pack_argb(stop.color)));
            }
            out.push_str("]}");
        }
        Some(Paint::RadialGradient(RadialGradientBrush {
            opacity,
            spread_method,
            stops,
            center,
            gradient_origin,
            radius_x,
            radius_y,
        })) => {
            out.push_str(&format!(
                "{{\"kind\":\"radial\",\"opacity\":{},\"spread\":{},\"center\":",
                opacity, *spread_method as i32
            ));
            write_rel_point(out, *center);
            out.push_str(",\"origin\":");
            write_rel_point(out, *gradient_origin);
            out.push_str(",\"radiusX\":");
            write_rel_scalar(out, *radius_x);
            out.push_str(",\"radiusY\":");
            write_rel_scalar(out, *radius_y);
            out.push_str(",\"stops\":[");
            for (i, stop) in stops.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&format!("[{},{}]", stop.offset, pack_argb(stop.color)));
            }
            out.push_str("]}");
        }
    }
}

fn write_setter_json(out: &mut String, setter: &StyleSetter) {
    match setter {
        StyleSetter::Paint(prop, paint) => {
            out.push_str(&format!("{{\"prop\":\"{prop}\",\"value\":"));
            write_paint_json(out, paint);
            out.push('}');
        }
        StyleSetter::Thickness(prop, t) => {
            out.push_str(&format!(
                "{{\"prop\":\"{prop}\",\"value\":[{},{},{},{}]}}",
                t.left, t.top, t.right, t.bottom
            ));
        }
        StyleSetter::CornerRadius(cr) => {
            out.push_str(&format!(
                "{{\"prop\":\"cornerRadius\",\"value\":[{},{},{},{}]}}",
                cr.top_left, cr.top_right, cr.bottom_right, cr.bottom_left
            ));
        }
        StyleSetter::Float(prop, v) => {
            out.push_str(&format!("{{\"prop\":\"{prop}\",\"value\":{v}}}"));
        }
        StyleSetter::Int(prop, v) => {
            out.push_str(&format!("{{\"prop\":\"{prop}\",\"value\":{v}}}"));
        }
        StyleSetter::Text(prop, s) => {
            out.push_str(&format!("{{\"prop\":\"{prop}\",\"value\":"));
            push_json_str(out, s);
            out.push('}');
        }
    }
}

impl AppScope {
    /// Applies a declarative [`StyleSheet`] to `Application.Current.Styles` for
    /// the duration of the current view or application scope.
    pub fn apply_styles(&self, styles: impl Into<StyleSheet>) -> Result<i64> {
        let sheet = styles.into();
        let utf16: Vec<u16> = sheet.to_json().encode_utf16().chain(Some(0)).collect();
        let handle = self.application().apply_styles(None, &utf16)?;
        let app = self.application().clone();
        self.retain_subscription(crate::EventSubscription::new(move || {
            app.remove_styles(handle)
        }));
        Ok(handle)
    }

    /// Applies a declarative [`StyleSheet`] scoped to `control`'s subtree.
    pub fn apply_control_styles(
        &self,
        control: &impl AsControl,
        styles: impl Into<StyleSheet>,
    ) -> Result<i64> {
        let sheet = styles.into();
        let raw_ctrl = control.as_control()?;
        let utf16: Vec<u16> = sheet.to_json().encode_utf16().chain(Some(0)).collect();
        let handle = self.application().apply_styles(Some(&raw_ctrl), &utf16)?;
        let app = self.application().clone();
        self.retain_subscription(crate::EventSubscription::new(move || {
            app.remove_styles(handle)
        }));
        Ok(handle)
    }

    /// Registers a keyboard shortcut on `window` invoking `command` with `parameter`.
    pub fn bind_window_shortcut(
        &self,
        window: &Window,
        shortcut: impl Into<String>,
        command: impl IntoCommandProp,
        parameter: impl Into<Variant>,
    ) -> Result<i64> {
        let Some(cmd_ptr) = command.into_command_prop() else {
            return Err(crate::Error::InvalidValue(
                "window shortcut requires a non-null Command".into(),
            ));
        };
        let gesture: Vec<u16> = shortcut.into().encode_utf16().chain(Some(0)).collect();
        let abi_param = parameter.into().to_abi()?;
        let id = self.application().add_window_key_binding(
            &window.raw,
            &gesture,
            &cmd_ptr,
            *abi_param,
        )?;
        let app = self.application().clone();
        let win_raw = window.raw.clone();
        self.retain_subscription(crate::EventSubscription::new(move || {
            app.remove_window_key_binding(&win_raw, id)
        }));
        Ok(id)
    }
}

impl<T: AsControl + 'static> El<T> {
    /// Attaches a scoped [`StyleSheet`] to this element's subtree, automatically
    /// removing it if the element's reactive scope is disposed.
    pub fn styles(self, styles: impl Into<StyleSheet> + 'static) -> Self {
        let sheet = styles.into();
        self.with_op(move |scope, control| {
            scope.apply_control_styles(control, sheet)?;
            Ok(())
        })
    }
}

impl El<Window> {
    /// Binds a [`Shortcut`] (or gesture string) on this window to `command`.
    pub fn shortcut(
        self,
        shortcut: impl Into<String> + 'static,
        command: impl IntoCommandProp + 'static,
    ) -> Self {
        self.shortcut_with_param(shortcut, command, Variant::None)
    }

    /// Binds a [`Shortcut`] (or gesture string) on this window to `command` with `parameter`.
    pub fn shortcut_with_param(
        self,
        shortcut: impl Into<String> + 'static,
        command: impl IntoCommandProp + 'static,
        parameter: impl Into<Variant> + 'static,
    ) -> Self {
        self.with_op(move |scope, window| {
            scope.bind_window_shortcut(window, shortcut, command, parameter)?;
            Ok(())
        })
    }
}

impl App {
    /// Runs the application with `styles` pre-installed on `Application.Current.Styles`
    /// before mounting `callback`.
    pub fn run_with_styles(
        self,
        styles: impl Into<StyleSheet> + Send + 'static,
        callback: impl FnOnce(&AppScope) -> Result<()> + Send + 'static,
    ) -> Result<()> {
        let sheet = styles.into();
        self.run(move |scope| {
            scope.apply_styles(sheet)?;
            callback(scope)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_parses_type_classes_and_pseudos() {
        let s = Style::select("Button.primary.compact:pointerover:focus")
            .background(Color::rgb(0x25, 0x63, 0xEB))
            .corner_radius(8.0)
            .box_shadow(BoxShadow::new(0.0, 4.0, 12.0, Color::rgba(0, 0, 0, 0x40)))
            .transition_opacity(Duration::from_millis(120));
        let json = StyleSheet::from(s).to_json();
        assert!(json.contains("\"target\":\"Button\""));
        assert!(json.contains("\"classes\":[\"primary\",\"compact\"]"));
        assert!(json.contains("\"pseudos\":[\":pointerover\",\":focus\"]"));
        assert!(json.contains("\"prop\":\"background\""));
        assert!(json.contains("\"prop\":\"boxShadow\""));
        assert!(json.contains("\"durationMs\":120"));
    }
}
