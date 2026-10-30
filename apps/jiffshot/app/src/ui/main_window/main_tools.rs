//! The tool palette across the top (the tools that fit, in the user's
//! order, then More with the rest; right-click a tool to move or hide it)
//! and the Effects panel.

use super::main_common::{act, action, chip, note, run, section, solid, with_context_menu, Ctx, Tone, ACCENT, TRANSPARENT, WHITE};
use crate::vm::MainViewModelSink;
use rustolonia::{
    AnyEl, AppScope, Border, Button, Dock, DockPanel, Effect, El, FontWeight, HorizontalAlignment, MenuFlyout, MenuItem, MenuItemToggleType, Orientation, Panel, ScrollBarVisibility, ScrollViewer, Separator, Signal, StackPanel, TextBlock,
    VerticalAlignment, WrapPanel,
};

/// A palette tool: its view-model name, icon glyph and caption.
pub struct ToolDef {
    pub name: &'static str,
    glyph: &'static str,
    caption: &'static str,
}

const fn tool(name: &'static str, glyph: &'static str, caption: &'static str) -> ToolDef {
    ToolDef { name, glyph, caption }
}

/// Every tool, in the usual order.
pub const TOOLS: [ToolDef; 22] = [
    tool("Select", "\u{E7C9}", "Select"),
    tool("Arrow", "\u{E72A}", "Arrow"),
    tool("Line", "\u{E738}", "Line"),
    tool("Shape", "\u{E739}", "Shape"),
    tool("Text", "\u{E8D2}", "Text"),
    tool("Callout", "\u{E8BD}", "Callout"),
    tool("Step", "①", "Step"),
    tool("Pen", "\u{E70F}", "Pen"),
    tool("Highlight", "\u{E7E6}", "Highlight"),
    tool("Blur", "▦", "Blur"),
    tool("Magnify", "\u{E8A3}", "Magnify"),
    tool("Spotlight", "\u{E706}", "Spotlight"),
    tool("Stamp", "\u{E734}", "Stamp"),
    tool("Fill", "\u{E790}", "Fill"),
    tool("Crop", "\u{E7A8}", "Crop"),
    tool("Cut Out", "\u{E8C6}", "Cut Out"),
    tool("Selection", "\u{E8B3}", "Selection"),
    tool("Magic Wand", "✨", "Magic Wand"),
    tool("Eraser", "\u{E75C}", "Eraser"),
    tool("Eyedropper", "⌖", "Pick color"),
    tool("Smart Move", "✥", "Smart Move"),
    tool("Text Replace", "\u{E8AC}", "Text Replace"),
];

/// The icon font of the palette's glyphs.
const ICON_FONT: &str = "Segoe Fluent Icons, Segoe MDL2 Assets, Segoe UI Symbol";

fn index_of(name: &str) -> Option<usize> {
    TOOLS.iter().position(|t| t.name == name)
}

/// The tools in the user's order (all of them when none is set), as
/// indices into [`TOOLS`].
fn ordered(order: &str) -> Vec<usize> {
    let picked: Vec<usize> = order.split(',').filter(|s| !s.is_empty()).filter_map(index_of).collect();
    if picked.is_empty() && order.split(',').all(|s| s.is_empty()) {
        (0..TOOLS.len()).collect()
    } else {
        picked
    }
}

/// How many tools fit beside the More button in `width`.
fn capacity(width: f64, labels: bool) -> usize {
    let per = if labels { 64.0 } else { 42.0 };
    if width.is_finite() && width > per {
        ((width - per) / per) as usize
    } else {
        0
    }
}

/// The tools shown in the bar and those under More. `extra` (a tool picked
/// from More or chosen elsewhere) takes the last place that fits.
fn layout(order: &str, capacity: usize, extra: Option<usize>) -> (Vec<usize>, Vec<usize>) {
    let shown = ordered(order);
    let hidden = (0..TOOLS.len()).filter(|i| !shown.contains(i));
    let mut fits: Vec<usize> = shown.iter().copied().take(capacity).collect();
    let mut more: Vec<usize> = shown.iter().copied().skip(capacity).chain(hidden).collect();
    if let Some(extra) = extra.filter(|e| capacity > 0 && !fits.contains(e)) {
        if let Some(at) = more.iter().position(|&i| i == extra) {
            more.remove(at);
            if fits.len() == capacity {
                if let Some(last) = fits.pop() {
                    more.insert(0, last);
                }
            }
            fits.push(extra);
        }
    }
    (fits, more)
}

/// One place in the bar.
#[derive(Clone, PartialEq)]
enum Slot {
    Tool(usize),
    More(Vec<usize>),
}

impl Slot {
    fn key(&self) -> Option<usize> {
        match self {
            Slot::Tool(i) => Some(*i),
            Slot::More(_) => None,
        }
    }
}

/// The tool palette: fills the space between the toolbar's other buttons.
pub fn tool_bar(ctx: &Ctx) -> El<Panel> {
    let sink = ctx.sink;
    let width = Signal::new(0.0_f64);
    // A tool picked from More, shown in place of the last tool that fits.
    let extra: Signal<Option<usize>> = Signal::new(None);
    let slots = move || {
        let labels = sink.tool_labels.get();
        // Other signals are read before `with`, which must not nest reads.
        let (cap, extra) = (capacity(width.get(), labels), extra.get());
        let (fits, more) = sink.tool_order.with(|order| layout(order, cap, extra));
        let mut slots: Vec<Slot> = fits.into_iter().map(Slot::Tool).collect();
        slots.push(Slot::More(more));
        slots
    };
    let theme = ctx.theme;
    let scope = ctx.scope.clone();
    let bar = StackPanel::el()
        .orientation(Orientation::Horizontal)
        .horizontal_alignment(HorizontalAlignment::Left)
        .vertical_alignment(VerticalAlignment::Center)
        .children_for(slots, Slot::key, move |slot: Signal<Slot>| match slot.get_untracked() {
            Slot::Tool(i) => tool_button(sink, theme, i),
            Slot::More(_) => more_button(scope.clone(), sink, theme, slot, extra),
        });
    Panel::el()
        .background(solid(TRANSPARENT))
        .on_size_changed(move |args| width.set_if_changed(args.new_size.width))
        .child(bar)
        .with_op(move |_, _| {
            // A tool chosen from a menu or a shortcut shows in the bar.
            Effect::new(move || {
                let tool = sink.tool.get();
                let Some(index) = index_of(&tool) else { return Ok(()) };
                let cap = capacity(width.get(), sink.tool_labels.get());
                let current = extra.get_untracked();
                let (fits, _) = sink.tool_order.with(|order| layout(order, cap, current));
                if cap > 0 && !fits.contains(&index) {
                    extra.set(Some(index));
                }
                Ok(())
            })?;
            Ok(())
        })
}

/// A palette button's glyph above its caption (hidden without labels).
fn tool_content(sink: MainViewModelSink, theme: super::main_common::Theme, glyph: &str, caption: &str, active: impl Fn() -> bool + Clone + 'static) -> El<StackPanel> {
    let glyph_active = active.clone();
    StackPanel::el()
        .spacing(2.0)
        .child(
            TextBlock::el()
                .text(glyph.to_owned())
                .font_family(ICON_FONT)
                .font_size(18.0)
                .horizontal_alignment(HorizontalAlignment::Center)
                .foreground(move || if glyph_active() { solid(WHITE) } else { theme.get(Tone::Text) }),
        )
        .child(
            TextBlock::el()
                .text(caption.to_owned())
                .font_size(10.0)
                .horizontal_alignment(HorizontalAlignment::Center)
                .text_trimming("CharacterEllipsis")
                .visible(sink.tool_labels)
                .foreground(move || if active() { solid(WHITE) } else { theme.get(Tone::Caption) }),
        )
}

/// A palette-sized button (`Button.tool`, `compact` without labels).
fn palette_button(sink: MainViewModelSink) -> El<Button> {
    Button::el()
        .width(move || if sink.tool_labels.get() { 60.0 } else { 38.0 })
        .height(move || if sink.tool_labels.get() { 48.0 } else { 34.0 })
        .padding(0.0)
        .margin(2.0)
        .corner_radius(6.0)
        .horizontal_content_alignment(HorizontalAlignment::Center)
        .vertical_content_alignment(VerticalAlignment::Center)
}

/// "Icon and text" or "Icon only", checked when it is the current choice.
fn labels_item(sink: MainViewModelSink, labels: bool) -> AnyEl {
    MenuItem::el()
        .header(if labels { "Icon and text" } else { "Icon only" })
        .toggle_type(MenuItemToggleType::Radio)
        .checked(move || sink.tool_labels.get() == labels)
        .command(act(move || sink.tool_labels.set_if_changed(labels)))
        .into()
}

fn tool_button(sink: MainViewModelSink, theme: super::main_common::Theme, index: usize) -> AnyEl {
    let def = &TOOLS[index];
    let name = def.name;
    let active = move || sink.tool.with(|t| t == name);
    let button = palette_button(sink)
        .tool_tip_tip(def.caption)
        .background(move || if active() { solid(ACCENT) } else { solid(TRANSPARENT) })
        .content(tool_content(sink, theme, def.glyph, def.caption, active))
        .command(action(format!("tool:{name}")));
    with_context_menu(
        button,
        vec![
            MenuItem::el().header("Move left").command(action(format!("toolbar:up:{name}"))).into(),
            MenuItem::el().header("Move right").command(action(format!("toolbar:down:{name}"))).into(),
            MenuItem::el().header("Move to More").command(action(format!("toolbar:hide:{name}"))).into(),
            Separator::el().into(),
            labels_item(sink, true),
            labels_item(sink, false),
            Separator::el().into(),
            MenuItem::el().header("Show all tools").command(action("toolbar:reset")).into(),
        ],
    )
    .into()
}

fn more_button(scope: AppScope, sink: MainViewModelSink, theme: super::main_common::Theme, slot: Signal<Slot>, extra: Signal<Option<usize>>) -> AnyEl {
    palette_button(sink)
        .tool_tip_tip("More tools")
        .background(solid(TRANSPARENT))
        .content(tool_content(sink, theme, "⋯", "More", || false))
        .with_op(move |_, button| {
            let target = button.clone();
            button.clone().on_click(&scope.clone(), move |_| {
                let Slot::More(more) = slot.get_untracked() else { return };
                if let Err(error) = more_menu(sink, &more, extra).build(&scope).and_then(|flyout| flyout.show_at_with_control(&target)) {
                    eprintln!("could not show More tools: {error}");
                }
            })?;
            Ok(())
        })
        .into()
}

/// More's menu: the tools that do not fit, then the label choices.
fn more_menu(sink: MainViewModelSink, more: &[usize], extra: Signal<Option<usize>>) -> El<MenuFlyout> {
    let mut items: Vec<AnyEl> = more
        .iter()
        .map(|&i| {
            let name = TOOLS[i].name;
            MenuItem::el()
                .header(TOOLS[i].caption)
                .toggle_type(MenuItemToggleType::Radio)
                .checked(sink.tool.get_untracked() == name)
                .command(act(move || {
                    extra.set(Some(i));
                    run(format!("tool:{name}"));
                }))
                .into()
        })
        .collect();
    if !items.is_empty() {
        items.push(Separator::el().into());
    }
    items.push(labels_item(sink, true));
    items.push(labels_item(sink, false));
    items.push(MenuItem::el().header("Show all tools in the usual order").command(action("toolbar:reset")).into());
    items.push(MenuItem::el().header("Right-click a tool to move it or put it in More").enabled(false).into());
    MenuFlyout::el().items_iter(items)
}

// ---- effects ----------------------------------------------------------------

/// The Effects panel's groups; effects not listed go under More.
const EFFECT_GROUPS: [(&str, &[&str]); 9] = [
    ("Border", &["Border"]),
    ("Edges", &["Torn edge", "Torn bottom", "Sawtooth edge", "Wave edge", "Fade edges"]),
    ("Shadow", &["Shadow"]),
    ("Perspective", &["Perspective", "Tilt", "Page curl", "Reflection"]),
    ("Corners", &["Rounded corners"]),
    ("Filters", &["Sharpen", "Blur image", "Monochrome", "Grayscale", "Sepia", "Invert", "Vignette"]),
    ("Color adjustment", &["Brighten", "Darken", "More contrast", "Less contrast", "Saturate", "Desaturate", "Hue shift", "Gamma up", "Gamma down"]),
    ("Color replacement", &["Replace color"]),
    ("Size", &["Scale to 1920"]),
];

/// The effects the model offers, in their groups (those in no group under
/// More), leaving out empty groups.
pub fn grouped_effects(names: &[String]) -> Vec<(String, Vec<String>)> {
    let listed = |name: &str| EFFECT_GROUPS.iter().any(|(_, effects)| effects.contains(&name));
    EFFECT_GROUPS
        .iter()
        .map(|(group, effects)| (group.to_string(), effects.iter().filter(|e| names.iter().any(|n| n == *e)).map(|e| e.to_string()).collect::<Vec<_>>()))
        .chain(std::iter::once(("More".to_owned(), names.iter().filter(|n| !listed(n)).cloned().collect())))
        .filter(|(_, effects)| !effects.is_empty())
        .collect()
}

/// The Effects panel: each group's effects as buttons.
pub fn effects_panel(ctx: &Ctx) -> El<Border> {
    let (sink, theme) = (ctx.sink, ctx.theme);
    let groups = StackPanel::el().margin((14.0, 0.0, 14.0, 16.0)).children_for(
        move || sink.effects.with(|names| grouped_effects(names)),
        |(group, _): &(String, Vec<String>)| group.clone(),
        move |group: Signal<(String, Vec<String>)>| {
            StackPanel::el()
                .child(section(theme, &group.get_untracked().0.to_uppercase()))
                .child(WrapPanel::el().children_for(move || group.get().1, String::clone, |effect: Signal<String>| {
                    let name = effect.get_untracked();
                    chip(&name).command(action(format!("effect:{name}")))
                }))
        },
    );
    Border::el()
        .dock_panel_dock(Dock::Right)
        .width(230.0)
        .background(theme.brush(Tone::Panel))
        .border_brush(theme.brush(Tone::Border))
        .border_thickness((1.0, 0.0, 0.0, 0.0))
        .visible(sink.show_effects)
        .child(
            DockPanel::el()
                .child(
                    DockPanel::el()
                        .dock_panel_dock(Dock::Top)
                        .margin((14.0, 10.0, 8.0, 0.0))
                        .child(super::main_common::bar("✕").dock_panel_dock(Dock::Right).tool_tip_tip("Hide the Effects panel").command(act(move || sink.show_effects.set(false))))
                        .child(TextBlock::el().text("Effects").font_size(16.0).font_weight(FontWeight::DemiBold).vertical_alignment(VerticalAlignment::Center)),
                )
                .child(note(theme, "Applies to the whole image. Undo takes one back off.").dock_panel_dock(Dock::Top).margin((14.0, 4.0, 14.0, 0.0)))
                .child(ScrollViewer::el().vertical_scroll_bar_visibility(ScrollBarVisibility::Auto).content(groups)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_puts_the_rest_under_more() {
        let (fits, more) = layout("", 3, None);
        assert_eq!(fits, [0, 1, 2]);
        assert_eq!(more.len(), TOOLS.len() - 3);
    }

    #[test]
    fn an_extra_tool_takes_the_last_place() {
        let (fits, more) = layout("", 3, Some(10));
        assert_eq!(fits, [0, 1, 10]);
        assert_eq!(more[0], 2);
        assert!(!more.contains(&10));
    }

    #[test]
    fn hidden_tools_go_under_more() {
        let (fits, more) = layout("Arrow,Text", 5, None);
        assert_eq!(fits, [1, 4]);
        assert!(more.contains(&0));
    }

    #[test]
    fn effects_outside_the_groups_go_under_more() {
        let names = vec!["Border".to_owned(), "Sepia".to_owned(), "Sparkle".to_owned()];
        let groups = grouped_effects(&names);
        assert_eq!(groups.iter().map(|(g, _)| g.as_str()).collect::<Vec<_>>(), ["Border", "Filters", "More"]);
    }
}
