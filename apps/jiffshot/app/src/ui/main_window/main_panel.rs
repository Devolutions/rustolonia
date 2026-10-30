//! The properties side bar: quick styles, then the sections the model's
//! `panel` lists for the current tool or selection, then Arrange (with a
//! selection) or Canvas color (without).

use super::main_common::{action, action_box, chip, choice_box, color_brush, note, run, section, solid, value_label, with_context_menu, Ctx, Theme, Tone, ORANGE};
use crate::vm::{MainViewModelSink, QuickStyleViewModel};
use rustolonia::{
    AnyEl, Border, Button, CheckBox, Dock, DockPanel, El, FontStyle, FontWeight, Grid, HorizontalAlignment, MenuItem, Orientation, Panel, Rectangle, ScrollBarVisibility, ScrollViewer, Separator, Signal, Slider, StackPanel, TextBlock, TextBox,
    TextWrapping, ToggleButton, VerticalAlignment, WrapPanel,
};

/// The swatch colors.
const PALETTE: [&str; 12] = ["#FFE53935", "#FFFF9800", "#FFFFEB3B", "#FF43A047", "#FF1E88E5", "#FF1E6FD9", "#FF8E24AA", "#FFFFFFFF", "#FFBDBDBD", "#FF757575", "#FF22262E", "#FF000000"];

/// The "none" swatch.
const NONE: &str = "#00000000";

const CAPS: [&str; 7] = ["None", "Arrow", "Open arrow", "Dot", "Square", "Diamond", "Taper"];

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Whether the model's `panel` lists `name`.
fn shows(sink: MainViewModelSink, name: &'static str) -> impl Fn() -> bool + Clone + 'static {
    move || sink.panel.with(|p| p.split(';').any(|s| s == name))
}

/// A section shown while the panel lists `name`.
fn part(sink: MainViewModelSink, name: &'static str) -> El<StackPanel> {
    StackPanel::el().visible(shows(sink, name))
}

/// Color swatches that set `value`, the first one "none" when `with_none`.
fn swatches(value: Signal<String>, with_none: bool) -> El<WrapPanel> {
    let colors = with_none.then_some(NONE).into_iter().chain(PALETTE);
    WrapPanel::el().children_iter(colors.map(|hex| {
        let button = Button::el()
            .width(22.0)
            .height(22.0)
            .min_width(22.0)
            .padding(0.0)
            .margin((0.0, 0.0, 4.0, 4.0))
            .corner_radius(4.0)
            .border_thickness(1.0)
            .border_brush(solid(0x66FFFFFF))
            .background(color_brush(hex))
            .command(super::main_common::act(move || value.set_if_changed(hex.to_owned())));
        let button = if hex == NONE {
            button.content(TextBlock::el().text("∅").font_size(12.0).horizontal_alignment(HorizontalAlignment::Center)).tool_tip_tip("None")
        } else {
            button
        };
        AnyEl::from(button)
    }))
}

/// A slider with its value on the right.
fn slider_row(value: Signal<f64>, min: f64, max: f64) -> El<DockPanel> {
    DockPanel::el().child(value_label(value)).child(Slider::el().minimum(min).maximum(max).value(value.two_way()))
}

/// A color as `#AARRGGBB` text.
fn hex_box(value: Signal<String>) -> El<TextBox> {
    TextBox::el().text(value.two_way()).font_family("Consolas")
}

/// The side bar.
pub fn side_bar(ctx: &Ctx) -> El<Border> {
    let (s, theme) = (ctx.sink, ctx.theme);
    let content = StackPanel::el()
        .margin((14.0, 8.0, 14.0, 16.0))
        .child(TextBlock::el().text(move || if s.selection.with(String::is_empty) { s.tool.get() } else { s.selection.get() }).font_size(16.0).font_weight(FontWeight::DemiBold).margin((0.0, 4.0, 0.0, 2.0)))
        .child(quick_styles(s, theme))
        .child(variant_section(s, theme))
        .child(
            part(s, "tails")
                .child(section(theme, "TAILS"))
                .child(WrapPanel::el().child(chip("Add tail").tool_tip_tip("Point the callout at one more place").command(action("tail:add"))).child(chip("Remove tail").command(action("tail:remove")))),
        )
        .child(zone_section(s, theme))
        .child(part(s, "backfill").child(section(theme, "BACKGROUND FILL")).child(choice_box(theme, || strings(&["Auto-fill", "Transparent", "Color"]), s.background_fill).horizontal_alignment(HorizontalAlignment::Stretch).tool_tip_tip("What fills the area a lifted part leaves behind")))
        .child(
            part(s, "text").child(section(theme, "TEXT")).child(
                TextBox::el()
                    .text(s.text.two_way())
                    .accepts_return(true)
                    .text_wrapping(TextWrapping::Wrap)
                    .min_height(64.0)
                    .max_height(220.0)
                    .placeholder_text("Type the text…")
                    .node_ref(ctx.text_editor),
            ),
        )
        .child(font_section(s, theme))
        .child(text_options(s, theme))
        .child(part(s, "textcolor").child(section(theme, "TEXT COLOR")).child(swatches(s.text_color, false)).child(hex_box(s.text_color)))
        .child(part(s, "stroke").child(section(theme, "OUTLINE COLOR")).child(swatches(s.stroke_color, false)).child(hex_box(s.stroke_color)))
        .child(part(s, "fill").child(section(theme, "FILL COLOR")).child(swatches(s.fill_color, true)).child(hex_box(s.fill_color)))
        .child(
            part(s, "gradient").child(section(theme, "GRADIENT TO (∅ = SOLID)")).child(swatches(s.fill2, true)).child(
                DockPanel::el()
                    .child(CheckBox::el().content("Radial").checked(s.gradient_radial.two_way()).dock_panel_dock(Dock::Right))
                    .child(Slider::el().minimum(0.0).maximum(360.0).value(s.gradient_angle.two_way()).tool_tip_tip("Angle of a linear gradient")),
            ),
        )
        .child(part(s, "width").child(section(theme, "WIDTH")).child(slider_row(s.stroke_width, 0.0, 60.0)))
        .child(
            part(s, "caps")
                .child(section(theme, "ENDS"))
                .child(
                    Grid::el()
                        .column_definitions("*,8,*")
                        .child(choice_box(theme, || strings(&CAPS), s.start_cap).grid_column(0).horizontal_alignment(HorizontalAlignment::Stretch))
                        .child(choice_box(theme, || strings(&CAPS), s.end_cap).grid_column(2).horizontal_alignment(HorizontalAlignment::Stretch)),
                )
                .child(section(theme, "HEAD SIZE"))
                .child(Slider::el().minimum(0.3).maximum(4.0).value(s.head_size.two_way())),
        )
        .child(part(s, "dash").child(section(theme, "LINE STYLE")).child(choice_box(theme, || strings(&["Solid", "Dash", "Dot"]), s.dash).horizontal_alignment(HorizontalAlignment::Stretch)))
        .child(amount_section(s, theme))
        .child(part(s, "opacity").child(section(theme, "OPACITY")).child(slider_row(s.opacity, 5.0, 100.0)))
        .child(part(s, "rotation").child(section(theme, "ROTATION")).child(slider_row(s.rotation, 0.0, 360.0)))
        .child(shadow_section(s, theme))
        .child(arrange_section(s, theme))
        .child(
            StackPanel::el()
                .visible(move || s.selection.with(String::is_empty))
                .child(section(theme, "CANVAS COLOR"))
                .child(swatches(s.canvas_color, true))
                .child(chip("Expand canvas to fit objects").command(super::main_common::command(s.expand_canvas_enabled, |m| m.expand_canvas()))),
        );
    Border::el()
        .dock_panel_dock(Dock::Right)
        .width(300.0)
        .background(theme.brush(Tone::Panel))
        .border_brush(theme.brush(Tone::Border))
        .border_thickness((1.0, 0.0, 0.0, 0.0))
        .visible(s.show_side_bar)
        .child(ScrollViewer::el().vertical_scroll_bar_visibility(ScrollBarVisibility::Auto).content(content))
}

/// Quick styles: one-click styles for the current tool.
fn quick_styles(s: MainViewModelSink, theme: Theme) -> El<StackPanel> {
    let keys = move || s.quick_styles.with(|list| list.iter().map(|q| q.key).collect::<Vec<u64>>());
    StackPanel::el()
        .visible(move || s.quick_styles.with(|l| !l.is_empty()) || s.panel.with(|p| !p.is_empty()))
        .child(section(theme, "QUICK STYLES"))
        .child(WrapPanel::el().children_for(keys, |k: &u64| *k, move |key: Signal<u64>| quick_style(s, theme, key.get_untracked())))
        .child(
            WrapPanel::el()
                .child(chip("Save current style").command(super::main_common::command(s.save_quick_style_enabled, |m| m.save_quick_style())))
                .child(
                    action_box(theme, move || s.template_themes.get(), |name| run(format!("style-theme:{name}")))
                        .placeholder_text("Theme…")
                        .min_width(90.0)
                        .font_size(11.0)
                        .tool_tip_tip("Recolor this tool's quick styles with a theme")
                        .margin((0.0, 0.0, 6.0, 6.0)),
                )
                .child(chip("Reset").tool_tip_tip("Back to the built-in quick styles").command(action("style-theme:reset"))),
        )
        .child(note(theme, "Right-click a style to favorite, move or delete it.").font_size(10.5))
}

/// Runs `f` on the quick style `key`, when it is still listed.
fn on_quick_style(sink: MainViewModelSink, key: u64, f: fn(&mut dyn QuickStyleViewModel) -> rustolonia::Result<()>) {
    let model = sink.quick_styles.with_untracked(|list| list.iter().find(|q| q.key == key).map(|q| q.model.clone()));
    if let Some(model) = model {
        model.run(f);
    }
}

fn quick_style(s: MainViewModelSink, theme: Theme, key: u64) -> AnyEl {
    let Some(fields) = s.quick_styles.with_untracked(|list| list.iter().find(|q| q.key == key).map(|q| q.fields)) else {
        return Panel::el().into();
    };
    let on = move |f: fn(&mut dyn QuickStyleViewModel) -> rustolonia::Result<()>| super::main_common::act(move || on_quick_style(s, key, f));
    let button = Button::el()
        .width(44.0)
        .height(36.0)
        .margin((0.0, 0.0, 6.0, 6.0))
        .padding(0.0)
        .corner_radius(6.0)
        .tool_tip_tip(move || fields.with(|f| f.name.clone()))
        .background(theme.brush(Tone::Well))
        .command(on(|m| m.apply()))
        .content(
            Panel::el()
                .child(
                    Border::el()
                        .width(34.0)
                        .height(26.0)
                        .corner_radius(4.0)
                        .border_thickness(3.0)
                        .background(move || fields.with(|f| color_brush(&f.fill)))
                        .border_brush(move || fields.with(|f| color_brush(&f.stroke)))
                        .child(
                            TextBlock::el()
                                .text("Aa")
                                .font_size(11.0)
                                .font_weight(FontWeight::Bold)
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .vertical_alignment(VerticalAlignment::Center)
                                .foreground(move || fields.with(|f| color_brush(&f.text_color))),
                        ),
                )
                .child(
                    TextBlock::el()
                        .text("★")
                        .font_size(10.0)
                        .foreground(solid(ORANGE))
                        .horizontal_alignment(HorizontalAlignment::Right)
                        .vertical_alignment(VerticalAlignment::Top)
                        .margin((0.0, -2.0, -2.0, 0.0))
                        .visible(move || fields.with(|f| f.is_favorite)),
                ),
        );
    with_context_menu(
        button,
        vec![
            MenuItem::el().header("Favorite").command(on(|m| m.toggle_favorite())).into(),
            MenuItem::el().header("Move earlier").command(on(|m| m.move_earlier())).into(),
            MenuItem::el().header("Move later").command(on(|m| m.move_later())).into(),
            Separator::el().into(),
            MenuItem::el().header("Delete").command(on(|m| m.delete())).into(),
        ],
    )
    .into()
}

fn variant_section(s: MainViewModelSink, theme: Theme) -> El<StackPanel> {
    part(s, "variant")
        .child(section(theme, "TYPE"))
        .child(choice_box(theme, move || s.variants.get(), s.variant).horizontal_alignment(HorizontalAlignment::Stretch))
        .child(chip("Add stamps from files…").margin((0.0, 6.0, 0.0, 0.0)).visible(move || s.tool.with(|t| t == "Stamp") && s.selection.with(String::is_empty)).command(action("add-stamp")))
}

fn zone_section(s: MainViewModelSink, theme: Theme) -> El<StackPanel> {
    let zone_toggle = |label: &str, fit: bool| {
        super::main_common::one_way_toggle(move || s.zone_fit.get() == fit, move || run(format!("zone-fit:{}", u8::from(fit))))
            .content(label.to_owned())
            .padding((8.0, 3.0))
            .margin((0.0, 0.0, 4.0, 4.0))
            .font_size(11.5)
    };
    part(s, "zone")
        .child(section(theme, "DROP ZONE"))
        .child(note(theme, "Drop an image here from the tray or Explorer. Drag inside a selected zone to move its picture.").margin((0.0, 0.0, 0.0, 6.0)))
        .child(
            WrapPanel::el()
                .child(zone_toggle("Fill", false).tool_tip_tip("Fill the zone, cropping the picture"))
                .child(zone_toggle("Fit", true).tool_tip_tip("Show the whole picture"))
                .child(chip("Choose image…").command(action("zone-pick")))
                .child(chip("Swap").tool_tip_tip("Swap the pictures of two zones picked with Shift+click").command(action("zone-swap")))
                .child(chip("Empty").command(action("zone-empty"))),
        )
}

fn font_section(s: MainViewModelSink, theme: Theme) -> El<StackPanel> {
    let align = |label: &str, value: &'static str| chip(label).command(super::main_common::act(move || s.align.set_if_changed(value.to_owned())));
    part(s, "font")
        .child(section(theme, "FONT"))
        .child(choice_box(theme, move || s.fonts.get(), s.font_family).horizontal_alignment(HorizontalAlignment::Stretch))
        .child(slider_row(s.font_size, 6.0, 160.0).margin((0.0, 6.0, 0.0, 0.0)))
        .child(
            StackPanel::el()
                .orientation(Orientation::Horizontal)
                .spacing(4.0)
                .child(ToggleButton::el().content("B").font_weight(FontWeight::Bold).width(34.0).checked(s.bold.two_way()))
                .child(ToggleButton::el().content("I").font_style(FontStyle::Italic).width(34.0).checked(s.italic.two_way()))
                .child(Rectangle::el().width(8.0))
                .child(align("⯇ Left", "Left"))
                .child(align("Center", "Center"))
                .child(align("Right ⯈", "Right")),
        )
}

fn text_options(s: MainViewModelSink, theme: Theme) -> El<StackPanel> {
    part(s, "textopts")
        .child(
            StackPanel::el()
                .orientation(Orientation::Horizontal)
                .spacing(4.0)
                .margin((0.0, 6.0, 0.0, 0.0))
                .child(ToggleButton::el().content("U").width(34.0).checked(s.underline.two_way()).tool_tip_tip("Underline"))
                .child(ToggleButton::el().content("S").width(34.0).checked(s.strike.two_way()).tool_tip_tip("Strikethrough"))
                .child(choice_box(theme, || strings(&["Top", "Middle", "Bottom"]), s.v_align).width(110.0).tool_tip_tip("Vertical alignment")),
        )
        .child(section(theme, "LETTER OUTLINE"))
        .child(swatches(s.outline_color, true))
        .child(slider_row(s.outline_width, 0.0, 12.0))
        .child(section(theme, "PADDING (−1 = AUTO)"))
        .child(slider_row(s.padding, -1.0, 60.0))
}

/// The Amount slider's range for the current tool's amount.
fn amount_range(label: &str) -> (f64, f64, f64) {
    match label {
        "Tolerance" => (0.0, 100.0, 1.0),
        "Zoom" => (1.0, 6.0, 0.1),
        "Dim" => (0.0, 1.0, 0.05),
        "Smoothing" => (0.0, 10.0, 1.0),
        _ => (1.0, 60.0, 1.0),
    }
}

fn amount_section(s: MainViewModelSink, theme: Theme) -> El<StackPanel> {
    let range = move || s.amount_label.with(|l| amount_range(l));
    part(s, "amount")
        .child(
            TextBlock::el()
                .text(move || s.amount_label.with(|l| if l.is_empty() { "AMOUNT".to_owned() } else { l.to_uppercase() }))
                .font_size(11.0)
                .font_weight(FontWeight::DemiBold)
                .foreground(theme.brush(Tone::Muted))
                .margin((0.0, 10.0, 0.0, 4.0)),
        )
        .child(
            DockPanel::el().child(value_label(s.amount)).child(
                Slider::el()
                    .minimum(move || range().0)
                    .maximum(move || range().1)
                    .small_change(move || range().2)
                    .value(s.amount.two_way()),
            ),
        )
}

fn shadow_section(s: MainViewModelSink, theme: Theme) -> El<StackPanel> {
    let on = shows(s, "shadow");
    part(s, "shadow").child(CheckBox::el().content("Drop shadow").checked(s.shadow.two_way()).margin((0.0, 10.0, 0.0, 0.0))).child(
        StackPanel::el()
            .visible(move || on() && s.shadow.get())
            .child(section(theme, "SHADOW DIRECTION"))
            .child(Slider::el().minimum(0.0).maximum(360.0).value(s.shadow_angle.two_way()))
            .child(section(theme, "DISTANCE"))
            .child(Slider::el().minimum(0.0).maximum(40.0).value(s.shadow_distance.two_way()))
            .child(section(theme, "SOFTNESS"))
            .child(Slider::el().minimum(0.0).maximum(30.0).value(s.shadow_blur.two_way()))
            .child(hex_box(s.shadow_color).tool_tip_tip("Shadow color, #AARRGGBB")),
    )
}

fn arrange_section(s: MainViewModelSink, theme: Theme) -> El<StackPanel> {
    use super::main_common::command;
    let align = |label: &str, tip: &str, how: &str| chip(label).tool_tip_tip(tip.to_owned()).command(action(format!("align:{how}")));
    StackPanel::el()
        .visible(move || s.selection.with(|t| !t.is_empty()))
        .child(section(theme, "ARRANGE"))
        .child(
            WrapPanel::el()
                .child(chip("Front").command(command(s.bring_to_front_enabled, |m| m.bring_to_front())))
                .child(chip("Forward").command(command(s.bring_forward_enabled, |m| m.bring_forward())))
                .child(chip("Backward").command(command(s.send_backward_enabled, |m| m.send_backward())))
                .child(chip("Back").command(command(s.send_to_back_enabled, |m| m.send_to_back())))
                .child(chip("Duplicate").command(command(s.duplicate_enabled, |m| m.duplicate())))
                .child(chip("Delete").command(command(s.delete_enabled, |m| m.delete())))
                .child(chip("Group").command(command(s.group_enabled, |m| m.group())))
                .child(chip("Ungroup").command(command(s.ungroup_enabled, |m| m.ungroup())))
                .child(chip("Flatten").command(command(s.flatten_selected_enabled, |m| m.flatten_selected()))),
        )
        .child(
            WrapPanel::el()
                .child(align("⇤", "Align left", "left"))
                .child(align("↔", "Align centers", "center"))
                .child(align("⇥", "Align right", "right"))
                .child(align("⤒", "Align top", "top"))
                .child(align("↕", "Align middles", "middle"))
                .child(align("⤓", "Align bottom", "bottom")),
        )
}
