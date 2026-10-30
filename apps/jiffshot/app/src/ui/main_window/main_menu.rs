//! The editor's menu bar (the view-model schema's menu tree plus what
//! `MainWindow.BuildMenus` added: Trim, Resize image, Canvas color and
//! Effects in Image, a Share menu, View ▸ Pixel grid, Edit ▸ Preferences and
//! Help) and the toolbar's Create ▾ and Share ▾ menus.
//!
//! Menu items run commands, so their shortcuts work while the menus are
//! closed (`hot_key`) and they are disabled with the model's
//! `<command>_enabled` values.

use crate::ui::dialogs;
use super::main_common::{act, action, command, file_name, reactive_items, run, Call, Ctx};
use super::main_tools::grouped_effects;
use crate::vm::MainViewModelSink;
use rustolonia::{AnyEl, AppScope, Command, El, Menu, MenuFlyout, MenuItem, MenuItemToggleType, Result, Separator, Signal, TextBox, Variant, Window};

/// Shown in Open recent while the list is empty.
const NO_RECENT_FILES: &str = "(no recent files)";

/// At most this many recent files are listed.
const RECENT_FILES_SHOWN: usize = 10;

/// Image ▸ Canvas color.
const CANVAS_COLORS: [(&str, &str); 12] = [
    ("Transparent", "#00000000"),
    ("White", "#FFFFFFFF"),
    ("Light gray", "#FFBDBDBD"),
    ("Gray", "#FF757575"),
    ("Charcoal", "#FF22262E"),
    ("Black", "#FF000000"),
    ("Red", "#FFE53935"),
    ("Orange", "#FFFF9800"),
    ("Yellow", "#FFFFEB3B"),
    ("Green", "#FF43A047"),
    ("Blue", "#FF1E88E5"),
    ("Purple", "#FF8E24AA"),
];

/// A menu header. Headers are text blocks, which do not underline access
/// keys, so the schema's `_` markers are dropped.
fn header(text: &str) -> String {
    text.replace('_', "")
}

/// A menu item that runs `command`, with an optional shortcut.
fn item(text: &str, command: Command, gesture: Option<&str>) -> El<MenuItem> {
    let el = MenuItem::el().header(header(text)).command(command);
    match gesture {
        Some(gesture) => el.input_gesture(gesture.to_owned()).hot_key(gesture.to_owned()),
        None => el,
    }
}

/// A menu item for a view-model command.
fn cmd(text: &str, enabled: Signal<bool>, f: Call, gesture: Option<&str>) -> AnyEl {
    item(text, command(enabled, f), gesture).into()
}

/// A check item that flips `value` (a writable property).
fn toggle(text: &str, value: Signal<bool>) -> AnyEl {
    MenuItem::el()
        .header(header(text))
        .toggle_type(MenuItemToggleType::CheckBox)
        .checked(value)
        .command(act(move || value.set(!value.get_untracked())))
        .into()
}

/// A submenu.
fn submenu(text: &str, items: Vec<AnyEl>) -> El<MenuItem> {
    MenuItem::el().header(header(text)).items_iter(items)
}

fn sep() -> AnyEl {
    Separator::el().into()
}

/// The menu bar.
pub fn menu_bar(ctx: &Ctx) -> El<Menu> {
    Menu::el().item(file_menu(ctx)).item(edit_menu(ctx)).item(image_menu(ctx)).item(share_menu(ctx)).item(tools_menu(ctx)).item(view_menu(ctx)).item(video_menu(ctx)).item(help_menu(ctx))
}

fn file_menu(ctx: &Ctx) -> El<MenuItem> {
    let s = ctx.sink;
    submenu(
        "_File",
        vec![
            cmd("New _capture", s.new_capture_enabled, |m| m.new_capture(), Some("Ctrl+Shift+N")),
            cmd("Capture _window…", s.open_capture_window_enabled, |m| m.open_capture_window(), None),
            cmd("_Record video", s.record_video_enabled, |m| m.record_video(), None),
            cmd("_New image", s.new_image_enabled, |m| m.new_image(), Some("Ctrl+N")),
            cmd("_Open…", s.open_enabled, |m| m.open(), Some("Ctrl+O")),
            open_recent(s).into(),
            cmd("Co_mbine images…", s.combine_images_enabled, |m| m.combine_images(), None),
            sep(),
            cmd("_Save", s.save_enabled, |m| m.save(), Some("Ctrl+S")),
            cmd("Save _as…", s.save_as_enabled, |m| m.save_as(), Some("Ctrl+Shift+S")),
            cmd("_Export image…", s.export_enabled, |m| m.export(), Some("Ctrl+E")),
            cmd("New from clip_board", s.new_from_clipboard_enabled, |m| m.new_from_clipboard(), Some("Ctrl+Shift+V")),
            cmd("Copy _all", s.copy_image_enabled, |m| m.copy_image(), Some("Ctrl+Shift+C")),
            cmd("_Print…", s.print_enabled, |m| m.print(), Some("Ctrl+P")),
            cmd("D_uplicate file", s.duplicate_file_enabled, |m| m.duplicate_file(), Some("Ctrl+Shift+D")),
            cmd("_Close", s.close_file_enabled, |m| m.close_file(), Some("Ctrl+W")),
            cmd("Delete _file", s.delete_file_enabled, |m| m.delete_file(), Some("Ctrl+Delete")),
            sep(),
            cmd("Open _library folder", s.open_library_folder_enabled, |m| m.open_library_folder(), None),
            cmd("E_xit", s.exit_enabled, |m| m.exit(), Some("Ctrl+Q")),
        ],
    )
}

/// File ▸ Open recent: the recent files by name, newest first.
fn open_recent(sink: MainViewModelSink) -> El<MenuItem> {
    let enabled = sink.open_recent_file_enabled;
    let el = MenuItem::el().header(header("Open _recent"));
    reactive_items(el, MenuItem::items, move || sink.recent_files.get(), move |files: &Vec<String>| {
        if files.is_empty() {
            return vec![MenuItem::el().header(NO_RECENT_FILES).enabled(false).into()];
        }
        files
            .iter()
            .take(RECENT_FILES_SHOWN)
            .map(|uri| {
                let name = file_name(uri);
                let uri = uri.clone();
                let open = move |_: Variant| {
                    let uri = uri.clone();
                    crate::ui::main().model.run(move |m| m.open_recent_file(uri));
                    Ok(())
                };
                let command = Command::with_can_execute(enabled, open.clone()).unwrap_or_else(|_| Command::new(open));
                MenuItem::el().header(name).command(command).into()
            })
            .collect()
    })
}

fn edit_menu(ctx: &Ctx) -> El<MenuItem> {
    let s = ctx.sink;
    submenu(
        "_Edit",
        vec![
            cmd("_Undo", s.undo_enabled, |m| m.undo(), Some("Ctrl+Z")),
            cmd("_Redo", s.redo_enabled, |m| m.redo(), Some("Ctrl+Y")),
            sep(),
            cmd("Cu_t", s.cut_enabled, |m| m.cut(), Some("Ctrl+X")),
            cmd("_Copy", s.copy_enabled, |m| m.copy(), Some("Ctrl+C")),
            cmd("_Paste", s.paste_enabled, |m| m.paste(), Some("Ctrl+V")),
            cmd("_Duplicate", s.duplicate_enabled, |m| m.duplicate(), None),
            cmd("De_lete", s.delete_enabled, |m| m.delete(), Some("Delete")),
            sep(),
            cmd("Select _all", s.select_all_enabled, |m| m.select_all(), Some("Ctrl+A")),
            cmd("Dese_lect all", s.deselect_all_enabled, |m| m.deselect_all(), Some("Ctrl+D")),
            cmd("_Invert selection", s.invert_selection_enabled, |m| m.invert_selection(), Some("Ctrl+Shift+I")),
            cmd("_Group", s.group_enabled, |m| m.group(), Some("Ctrl+Shift+G")),
            cmd("U_ngroup", s.ungroup_enabled, |m| m.ungroup(), Some("Ctrl+Shift+U")),
            sep(),
            cmd("Bring to _front", s.bring_to_front_enabled, |m| m.bring_to_front(), Some("Ctrl+Shift+OemCloseBrackets")),
            cmd("Bring _forward", s.bring_forward_enabled, |m| m.bring_forward(), Some("Ctrl+Shift+F")),
            cmd("Send _backward", s.send_backward_enabled, |m| m.send_backward(), Some("Ctrl+Shift+B")),
            cmd("Send to bac_k", s.send_to_back_enabled, |m| m.send_to_back(), Some("Ctrl+Shift+OemOpenBrackets")),
            sep(),
            cmd("Save as _quick style", s.save_quick_style_enabled, |m| m.save_quick_style(), None),
            sep(),
            item("_Preferences…", action("prefs:editor"), Some("Ctrl+OemComma")).into(),
        ],
    )
}

fn image_menu(ctx: &Ctx) -> El<MenuItem> {
    let s = ctx.sink;
    let scope = ctx.scope.clone();
    let trim_scope = scope.clone();
    let colors: Vec<AnyEl> = CANVAS_COLORS
        .iter()
        .map(|&(name, hex)| item(name, act(move || s.canvas_color.set(hex.to_owned())), None).into())
        .collect();
    let effects = reactive_items(MenuItem::el().header(header("_Effects")), MenuItem::items, move || s.effects.get(), |names: &Vec<String>| {
        grouped_effects(names)
            .into_iter()
            .map(|(group, effects)| match effects.as_slice() {
                [one] => item(one, action(format!("effect:{one}")), None).into(),
                _ => submenu(&group, effects.iter().map(|name| item(name, action(format!("effect:{name}")), None).into()).collect()).into(),
            })
            .collect()
    });
    submenu(
        "_Image",
        vec![
            cmd("_Crop", s.tool_crop_menu_enabled, |m| m.tool_crop_menu(), None),
            cmd("_Auto trim", s.auto_trim_enabled, |m| m.auto_trim(), None),
            item("_Trim…", act(move || ask_trim(&trim_scope, s)), None).into(),
            item("Resize _image…", act(move || ask_resize(&scope, s)), Some("Ctrl+Shift+R")).into(),
            sep(),
            cmd("Rotate _left", s.rotate_left_enabled, |m| m.rotate_left(), None),
            cmd("Rotate _right", s.rotate_right_enabled, |m| m.rotate_right(), None),
            cmd("Flip _horizontal", s.flip_horizontal_enabled, |m| m.flip_horizontal(), None),
            cmd("Flip _vertical", s.flip_vertical_enabled, |m| m.flip_vertical(), None),
            sep(),
            cmd("Resize to _50%", s.resize50_enabled, |m| m.resize50(), None),
            cmd("Resize to _200%", s.resize200_enabled, |m| m.resize200(), None),
            cmd("E_xpand canvas to fit objects", s.expand_canvas_enabled, |m| m.expand_canvas(), None),
            cmd("_Flatten selected", s.flatten_selected_enabled, |m| m.flatten_selected(), Some("Ctrl+T")),
            cmd("Flatten a_ll", s.flatten_enabled, |m| m.flatten(), Some("Ctrl+Shift+T")),
            cmd("Smart _redact", s.smart_redact_enabled, |m| m.smart_redact(), None),
            sep(),
            cmd("_Grab text", s.grab_text_enabled, |m| m.grab_text(), None),
            cmd("_Simplify", s.simplify_enabled, |m| m.simplify(), None),
            sep(),
            submenu("Canvas c_olor", colors).into(),
            effects.into(),
        ],
    )
}

/// The Share menu: the share destinations, then Copy all, Export and Print.
fn share_menu(ctx: &Ctx) -> El<MenuItem> {
    let s = ctx.sink;
    reactive_items(MenuItem::el().header(header("_Share")), MenuItem::items, move || s.shares.get(), move |shares: &Vec<String>| {
        let mut items = share_items(shares);
        items.push(sep());
        items.push(cmd("Copy all", s.copy_image_enabled, |m| m.copy_image(), None));
        items.push(cmd("Export…", s.export_enabled, |m| m.export(), None));
        items.push(cmd("Print…", s.print_enabled, |m| m.print(), None));
        items
    })
}

fn share_items(shares: &[String]) -> Vec<AnyEl> {
    shares.iter().map(|name| item(name, action(format!("share:{name}")), None).into()).collect()
}

/// Tools: every tool, the first nine on Ctrl+1 … Ctrl+9.
fn tools_menu(ctx: &Ctx) -> El<MenuItem> {
    let s = ctx.sink;
    let tools: [(&str, Signal<bool>, Call); 22] = [
        ("_Select / move", s.tool_select_enabled, |m| m.tool_select()),
        ("_Arrow", s.tool_arrow_enabled, |m| m.tool_arrow()),
        ("_Line", s.tool_line_enabled, |m| m.tool_line()),
        ("_Shape", s.tool_shape_enabled, |m| m.tool_shape()),
        ("_Text", s.tool_text_enabled, |m| m.tool_text()),
        ("_Callout", s.tool_callout_enabled, |m| m.tool_callout()),
        ("S_tep", s.tool_step_enabled, |m| m.tool_step()),
        ("_Pen", s.tool_pen_enabled, |m| m.tool_pen()),
        ("_Highlight", s.tool_highlight_enabled, |m| m.tool_highlight()),
        ("_Blur", s.tool_blur_enabled, |m| m.tool_blur()),
        ("_Magnify", s.tool_magnify_enabled, |m| m.tool_magnify()),
        ("Spotlig_ht", s.tool_spotlight_enabled, |m| m.tool_spotlight()),
        ("Stam_p", s.tool_stamp_enabled, |m| m.tool_stamp()),
        ("_Fill", s.tool_fill_enabled, |m| m.tool_fill()),
        ("_Crop", s.tool_crop_enabled, |m| m.tool_crop()),
        ("Cut _Out", s.tool_cut_out_enabled, |m| m.tool_cut_out()),
        ("Se_lection", s.tool_selection_enabled, |m| m.tool_selection()),
        ("_Eyedropper", s.tool_eyedropper_enabled, |m| m.tool_eyedropper()),
        ("Smart _Move", s.tool_smart_move_enabled, |m| m.tool_smart_move()),
        ("Text _Replace", s.tool_text_replace_enabled, |m| m.tool_text_replace()),
        ("Magic _Wand", s.tool_magic_wand_enabled, |m| m.tool_magic_wand()),
        ("Era_ser", s.tool_eraser_enabled, |m| m.tool_eraser()),
    ];
    const GESTURES: [&str; 9] = ["Ctrl+D1", "Ctrl+D2", "Ctrl+D3", "Ctrl+D4", "Ctrl+D5", "Ctrl+D6", "Ctrl+D7", "Ctrl+D8", "Ctrl+D9"];
    let items = tools.into_iter().enumerate().map(|(i, (text, enabled, f))| cmd(text, enabled, f, GESTURES.get(i).copied())).collect();
    submenu("_Tools", items)
}

fn view_menu(ctx: &Ctx) -> El<MenuItem> {
    let s = ctx.sink;
    let grid = Signal::new(ctx.canvas.show_pixel_grid());
    let canvas = ctx.canvas_handle();
    let pixel_grid = MenuItem::el()
        .header(header("Pixel _grid"))
        .toggle_type(MenuItemToggleType::CheckBox)
        .checked(grid)
        .input_gesture("Ctrl+G")
        .hot_key("Ctrl+G")
        .command(act(move || {
            let canvas = canvas.get();
            canvas.set_show_pixel_grid(!canvas.show_pixel_grid());
            grid.set(canvas.show_pixel_grid());
        }));
    submenu(
        "_View",
        vec![
            cmd("Zoom _in", s.zoom_in_enabled, |m| m.zoom_in(), Some("Ctrl+OemPlus")),
            cmd("Zoom _out", s.zoom_out_enabled, |m| m.zoom_out(), Some("Ctrl+OemMinus")),
            cmd("_Fit", s.zoom_fit_enabled, |m| m.zoom_fit(), Some("Ctrl+Alt+D0")),
            cmd("_Actual size", s.zoom_actual_enabled, |m| m.zoom_actual(), Some("Ctrl+D0")),
            pixel_grid.into(),
            sep(),
            cmd("_Library view", s.toggle_library_view_enabled, |m| m.toggle_library_view(), Some("Ctrl+L")),
            cmd("Show captures _closed from the tray", s.show_closed_enabled, |m| m.show_closed(), None),
            sep(),
            toggle("Side _bar", s.show_side_bar),
            toggle("_Effects panel", s.show_effects),
            toggle("Recent captures _tray", s.show_tray),
            toggle("Toolbar labe_ls", s.tool_labels),
            cmd("Show _all tools in their usual order", s.reset_toolbar_enabled, |m| m.reset_toolbar(), None),
            cmd("_Theme: dark / light / system", s.cycle_theme_enabled, |m| m.cycle_theme(), None),
        ],
    )
}

fn video_menu(ctx: &Ctx) -> El<MenuItem> {
    let s = ctx.sink;
    submenu(
        "Vi_deo",
        vec![
            cmd("_Play / pause", s.play_pause_enabled, |m| m.play_pause(), None),
            cmd("Set trim _start", s.set_trim_start_enabled, |m| m.set_trim_start(), None),
            cmd("Set trim _end", s.set_trim_end_enabled, |m| m.set_trim_end(), None),
            sep(),
            cmd("P_revious frame", s.prev_frame_enabled, |m| m.prev_frame(), None),
            cmd("_Next frame", s.next_frame_enabled, |m| m.next_frame(), None),
            cmd("_Mute", s.toggle_mute_enabled, |m| m.toggle_mute(), None),
            sep(),
            cmd("Save _selection as MP4…", s.export_video_enabled, |m| m.export_video(), None),
            cmd("_Cut selection…", s.cut_selection_enabled, |m| m.cut_selection(), None),
            cmd("Save a si_lent copy…", s.silent_copy_enabled, |m| m.silent_copy(), None),
            cmd("Export _GIF…", s.export_gif_enabled, |m| m.export_gif(), None),
            cmd("C_ombine videos…", s.combine_videos_enabled, |m| m.combine_videos(), None),
            cmd("Video from _images…", s.video_from_images_enabled, |m| m.video_from_images(), None),
            cmd("Narrated video from images…", s.narrate_images_enabled, |m| m.narrate_images(), None),
            cmd("Edit _frame as image", s.frame_to_image_enabled, |m| m.frame_to_image(), None),
        ],
    )
}

fn help_menu(ctx: &Ctx) -> El<MenuItem> {
    let (shortcuts, about) = (ctx.scope.clone(), ctx.scope.clone());
    submenu(
        "_Help",
        vec![
            item("_Keyboard shortcuts", act(move || with_owner(&shortcuts, dialogs::keyboard_shortcuts)), Some("F1")).into(),
            sep(),
            item("_About Jiffshot", act(move || with_owner(&about, dialogs::about)), None).into(),
        ],
    )
}

/// Shows a dialog over the editor window.
fn with_owner(scope: &AppScope, show: fn(&AppScope, &Window) -> Result<()>) {
    if let Some(owner) = scope.main_window() {
        if let Err(error) = show(scope, &owner) {
            eprintln!("could not show the dialog: {error}");
        }
    }
}

/// The image's size, from the document line ("1600 × 1000 px · …").
pub fn document_size(info: &str) -> Option<(u32, u32)> {
    let (w, rest) = info.split_once(" × ")?;
    let h = rest.split(|c: char| !c.is_ascii_digit()).next()?;
    Some((w.trim().rsplit(' ').next()?.parse().ok()?, h.parse().ok()?))
}

/// Image ▸ Resize image.
fn ask_resize(scope: &AppScope, sink: MainViewModelSink) {
    let (Some(owner), Some((w, h))) = (scope.main_window(), document_size(&sink.document_info.get_untracked())) else { return };
    if let Err(error) = dialogs::resize_image(scope, &owner, w, h, |w, h| run(format!("resize:{w},{h}"))) {
        eprintln!("could not show Resize image: {error}");
    }
}

/// Image ▸ Trim.
fn ask_trim(scope: &AppScope, sink: MainViewModelSink) {
    let (Some(owner), Some((w, h))) = (scope.main_window(), document_size(&sink.document_info.get_untracked())) else { return };
    if let Err(error) = dialogs::trim(scope, &owner, w, h, |[l, t, r, b]| run(format!("trim-image:{l},{t},{r},{b}"))) {
        eprintln!("could not show Trim: {error}");
    }
}

// ---- toolbar menus ----------------------------------------------------------

/// Create ▾: an image from a template (with its options), combining
/// captures, and videos made from images.
pub fn create_flyout(ctx: &Ctx) -> El<MenuFlyout> {
    let s = ctx.sink;
    let templates = reactive_items(MenuItem::el().header("Image from template"), MenuItem::items, move || s.templates.get(), |names: &Vec<String>| {
        names.iter().map(|name| item(name, action(format!("template:{name}")), None).into()).collect()
    });
    let themes = reactive_items(MenuItem::el().header("Template theme"), MenuItem::items, move || s.template_themes.get(), move |names: &Vec<String>| {
        names
            .iter()
            .map(|name| {
                let (shown, picked) = (name.clone(), name.clone());
                MenuItem::el()
                    .header(name.clone())
                    .toggle_type(MenuItemToggleType::Radio)
                    .checked(move || s.template_theme.get() == shown)
                    .command(act(move || s.template_theme.set_if_changed(picked.clone())))
                    .into()
            })
            .collect()
    });
    let footer = TextBox::el()
        .width(220.0)
        .placeholder_text("Footer text (optional)")
        .text(move || s.template_footer.get())
        .with_op(move |scope, text_box| {
            let reader = text_box.clone();
            text_box.clone().on_lost_focus(scope, move |_| {
                if let Ok(text) = reader.get_text() {
                    s.template_footer.set_if_changed(text.unwrap_or_default());
                }
            })?;
            Ok(())
        });
    MenuFlyout::el()
        .item(templates)
        .item(themes)
        .item(toggle("Captions under images", s.template_captions))
        .item(toggle("Number images", s.template_numbers))
        .item(submenu("Footer", vec![MenuItem::el().header(footer).stays_open_on_click(true).into()]))
        .item(sep())
        .item(cmd("Combine images…", s.combine_images_enabled, |m| m.combine_images(), None))
        .item(cmd("Combine the picked captures", s.combine_selected_enabled, |m| m.combine_selected(), None))
        .item(sep())
        .item(cmd("Video from images…", s.video_from_images_enabled, |m| m.video_from_images(), None))
        .item(cmd("Narrated video from images…", s.narrate_images_enabled, |m| m.narrate_images(), None))
        .item(cmd("Combine videos…", s.combine_videos_enabled, |m| m.combine_videos(), None))
}

/// Share ▾: the share destinations, Export and Print.
pub fn share_flyout(ctx: &Ctx) -> El<MenuFlyout> {
    let s = ctx.sink;
    reactive_items(MenuFlyout::el(), MenuFlyout::items, move || s.shares.get(), move |shares: &Vec<String>| {
        let mut items = share_items(shares);
        items.push(sep());
        items.push(cmd("Export…", s.export_enabled, |m| m.export(), None));
        items.push(cmd("Print…", s.print_enabled, |m| m.print(), None));
        items
    })
}

#[cfg(test)]
mod tests {
    use super::document_size;

    #[test]
    fn the_document_size_comes_from_its_line() {
        assert_eq!(document_size("1600 × 1000 px · 3 objects"), Some((1600, 1000)));
        assert_eq!(document_size(""), None);
    }
}
