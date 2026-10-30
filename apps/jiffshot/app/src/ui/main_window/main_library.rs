//! The library: the Recent Captures tray along the bottom (one scrolling
//! row) and, with Library view on, the whole library filling the window
//! (wrapping). Click a capture to open it, Ctrl+click to pick it.

use super::main_common::{act, choice_box, command, solid, with_context_menu, Ctx, Theme, Tone, BLUE, ORANGE, TRANSPARENT, WHITE};
use crate::vm::{LibraryItemFields, LibraryItemViewModel, MainViewModelSink};
use rustolonia::{
    AnyEl, Border, Button, Dock, DockPanel, El, Ellipse, FontWeight, HorizontalAlignment, Image, MenuItem, Orientation, Panel, Result, ScrollBarVisibility, ScrollViewer, Separator, Signal, Slider, StackPanel, Stretch, TextBlock, TextBox,
    ToggleButton, VerticalAlignment, WrapPanel,
};

/// `KeyModifiers.Control`.
const CONTROL: i32 = 2;

/// The library panel, docked along the bottom.
pub fn library_panel(ctx: &Ctx) -> El<Border> {
    let (s, theme, window_height) = (ctx.sink, ctx.theme, ctx.window_height);
    let expanded = s.library_expanded;
    let small = |button: El<Button>| button.padding((8.0, 2.0)).font_size(11.0);
    let header = DockPanel::el()
        .dock_panel_dock(Dock::Top)
        .margin((10.0, 6.0, 10.0, 0.0))
        .child(
            StackPanel::el()
                .dock_panel_dock(Dock::Right)
                .orientation(Orientation::Horizontal)
                .spacing(6.0)
                .child(
                    TextBox::el()
                        .width(170.0)
                        .placeholder_text("Search name, app, site, tag")
                        .text(s.library_search.two_way())
                        .font_size(11.0)
                        .padding((6.0, 3.0))
                        .min_height(0.0)
                        .height(26.0)
                        .accepts_return(false)
                        .vertical_content_alignment(VerticalAlignment::Center),
                )
                .child(choice_box(theme, move || s.library_filters.get(), s.library_filter).min_width(120.0).font_size(11.0).tool_tip_tip("Show only"))
                .child(choice_box(theme, move || s.library_sorts.get(), s.library_sort).min_width(90.0).font_size(11.0).tool_tip_tip("Sort by"))
                .child(Slider::el().width(80.0).minimum(60.0).maximum(240.0).value(s.thumb_size.two_way()).vertical_alignment(VerticalAlignment::Center).tool_tip_tip("Thumbnail size"))
                .child(small(Button::el().content("Combine")).command(command(s.combine_selected_enabled, |m| m.combine_selected())).tool_tip_tip("Combine the captures picked with Ctrl+click"))
                .child(ToggleButton::el().content("Library view").checked(expanded.two_way()).padding((8.0, 2.0)).font_size(11.0).tool_tip_tip("Show the whole library (Ctrl+L)"))
                .child(small(Button::el().content("Open folder")).command(command(s.open_library_folder_enabled, |m| m.open_library_folder()))),
        )
        .child(TextBlock::el().text("Library").font_weight(FontWeight::DemiBold).vertical_alignment(VerticalAlignment::Center))
        .child(TextBlock::el().text(s.library_info).margin((10.0, 0.0)).foreground(theme.brush(Tone::Muted)).font_size(11.0).vertical_alignment(VerticalAlignment::Center).text_trimming("CharacterEllipsis"));
    let keys = move || s.library.with(|list| list.iter().map(|item| item.key).collect::<Vec<u64>>());
    let tiles = WrapPanel::el()
        .orientation(Orientation::Horizontal)
        .margin((10.0, 6.0))
        .item_spacing(8.0)
        .line_spacing(8.0)
        .children_for(keys, |k: &u64| *k, move |key: Signal<u64>| tile(s, theme, key.get_untracked()));
    Border::el()
        .dock_panel_dock(Dock::Bottom)
        .background(theme.brush(Tone::Tray))
        .border_brush(theme.brush(Tone::Border))
        .border_thickness((0.0, 1.0, 0.0, 0.0))
        .visible(move || s.show_tray.get() || expanded.get())
        // The library view takes the canvas's place; the tray is one row.
        .height(move || if expanded.get() { (window_height.get() - 130.0).max(300.0) } else { 76.0 + s.thumb_size.get() * 88.0 / 120.0 })
        .child(
            DockPanel::el().child(header).child(
                ScrollViewer::el()
                    .horizontal_scroll_bar_visibility(move || if expanded.get() { ScrollBarVisibility::Disabled } else { ScrollBarVisibility::Auto })
                    .vertical_scroll_bar_visibility(move || if expanded.get() { ScrollBarVisibility::Auto } else { ScrollBarVisibility::Disabled })
                    .content(tiles),
            ),
        )
}

/// Runs `f` on the library item `key`, when it is still listed.
fn on_item(sink: MainViewModelSink, key: u64, f: impl FnOnce(&mut dyn LibraryItemViewModel) -> Result<()>) {
    let model = sink.library.with_untracked(|list| list.iter().find(|item| item.key == key).map(|item| item.model.clone()));
    if let Some(model) = model {
        model.run(f);
    }
}

/// A capture's thumbnail, name and tags.
fn tile(s: MainViewModelSink, theme: Theme, key: u64) -> AnyEl {
    let Some(fields) = s.library.with_untracked(|list| list.iter().find(|item| item.key == key).map(|item| item.fields)) else {
        return Panel::el().into();
    };
    let on = move |f: fn(&mut dyn LibraryItemViewModel) -> Result<()>| act(move || on_item(s, key, f));
    let get = move |f: fn(&LibraryItemFields) -> bool| move || fields.with(f);
    let height = move || fields.with(|f| f.thumb_height);
    let thumbnail = Image::el().stretch(Stretch::Uniform).height(height).with_op(move |_, image| {
        let image = image.clone();
        rustolonia::Effect::new(move || {
            // Thumbnails that are not there yet show nothing (ThumbnailConverter).
            let path = fields.with(|f| f.thumbnail.clone());
            if !path.is_empty() && std::path::Path::new(&path).is_file() {
                image.set_source(&path)?;
            }
            Ok(())
        })?;
        Ok(())
    });
    let picture = Panel::el()
        .height(height)
        .min_width(height)
        .child(thumbnail)
        .child(TextBlock::el().text("▶").font_size(26.0).foreground(solid(WHITE)).horizontal_alignment(HorizontalAlignment::Center).vertical_alignment(VerticalAlignment::Center).visible(move || fields.with(|f| f.kind == "Video")))
        .child(TextBlock::el().text("📌").font_size(13.0).horizontal_alignment(HorizontalAlignment::Left).vertical_alignment(VerticalAlignment::Top).visible(get(|f| f.is_pinned)).tool_tip_tip("Pinned"))
        .child(Ellipse::el().width(10.0).height(10.0).fill(solid(ORANGE)).horizontal_alignment(HorizontalAlignment::Right).vertical_alignment(VerticalAlignment::Top).margin(2.0).visible(get(|f| f.unsaved)).tool_tip_tip("Unsaved edits"))
        .child(
            TextBlock::el()
                .text("✓")
                .font_size(16.0)
                .font_weight(FontWeight::Bold)
                .foreground(solid(WHITE))
                .background(solid(BLUE))
                .padding((4.0, 0.0))
                .horizontal_alignment(HorizontalAlignment::Right)
                .vertical_alignment(VerticalAlignment::Bottom)
                .visible(get(|f| f.is_selected)),
        );
    let content = StackPanel::el()
        .spacing(3.0)
        .child(picture)
        .child(TextBlock::el().text(move || fields.with(|f| f.label.clone())).font_size(10.5).horizontal_alignment(HorizontalAlignment::Center).foreground(theme.brush(Tone::Caption)).max_width(move || height() * 2.0).text_trimming("CharacterEllipsis"))
        .child(
            TextBlock::el()
                .text(move || fields.with(|f| f.tags.clone()))
                .font_size(9.5)
                .horizontal_alignment(HorizontalAlignment::Center)
                .foreground(theme.brush(Tone::Muted))
                .text_trimming("CharacterEllipsis")
                .max_width(move || height() * 2.0)
                .visible(move || fields.with(|f| !f.tags.is_empty())),
        );
    let card = Border::el()
        .background(theme.brush(Tone::Chip))
        .border_thickness(2.0)
        .border_brush(move || fields.with(|f| solid(if f.is_selected { ORANGE } else if f.is_current { BLUE } else { TRANSPARENT })))
        .corner_radius(6.0)
        .padding(4.0)
        .tool_tip_tip(move || fields.with(|f| f.info.clone()))
        .on_tapped(move |args| {
            if args.key_modifiers & CONTROL != 0 {
                on_item(s, key, |m| m.toggle_select());
            } else {
                on_item(s, key, |m| m.open());
            }
        })
        .child(content);
    with_context_menu(
        card,
        vec![
            MenuItem::el().header("Pin / unpin").command(on(|m| m.toggle_pin())).into(),
            MenuItem::el().header("Rename").item(edit_row(s, key, "", move || fields.with(|f| f.new_name.clone()), "Rename", |m, text| m.rename(text))).into(),
            MenuItem::el().header("Tags").item(edit_row(s, key, "tag, another tag", move || fields.with(|f| f.tag_text.clone()), "Set", |m, text| m.set_tags(text))).into(),
            MenuItem::el().header("Pick (Ctrl+click)").command(on(|m| m.toggle_select())).into(),
            MenuItem::el().header("Show in folder").command(on(|m| m.show_in_folder())).into(),
            Separator::el().into(),
            MenuItem::el().header("Close from tray").command(on(|m| m.close())).into(),
            MenuItem::el().header("Delete").command(on(|m| m.delete())).into(),
        ],
    )
    .into()
}

/// A submenu row with a text box and a button that sends its text.
fn edit_row(s: MainViewModelSink, key: u64, placeholder: &str, text: impl FnMut() -> String + 'static, label: &str, send: fn(&mut dyn LibraryItemViewModel, String) -> Result<()>) -> El<MenuItem> {
    let field = rustolonia::NodeRef::<TextBox>::new();
    let row = StackPanel::el()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .child(TextBox::el().width(200.0).placeholder_text(placeholder.to_owned()).text(text).node_ref(field))
        .child(Button::el().content(label.to_owned()).command(act(move || {
            let text = field.get().and_then(|f| f.get_text().ok().flatten()).unwrap_or_default();
            on_item(s, key, |m| send(m, text));
        })));
    MenuItem::el().header(row).stays_open_on_click(true)
}
