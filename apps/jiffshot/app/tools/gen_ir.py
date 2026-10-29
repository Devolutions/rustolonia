"""Writes view-model.ir.json, the Rustolonia contract between the Rust
editor model and the Avalonia presentation. IDs are explicit and stable:
append new entries, never renumber.

    python app/tools/gen_ir.py
"""

import json
import os

NS = "Jiffshot.Editor.Generated"


def prop(i, name, kind, writable=False, initial=None):
    p = {"id": i, "name": name, "kind": kind, "writable": writable, "nullable": False}
    if initial is not None:
        key = {"String": "initialString", "Integer": "initialInteger", "Double": "initialDouble", "Boolean": "initialBoolean"}[kind]
        p[key] = initial
    return p


def cmd(i, name, param=None):
    c = {"id": i, "name": name, "isAsync": False, "supportsProgress": False, "supportsCancellation": False}
    if param:
        c["parameterProperty"] = param
    return c


main_props = [
    prop(1, "Title", "String", initial="Jiffshot Editor"),
    prop(2, "Status", "String", initial="Ready"),
    prop(3, "Tool", "String", initial="Arrow"),
    prop(4, "CanvasApi", "Integer", initial=0),
    prop(5, "FrameId", "Integer", initial=0),
    prop(6, "StrokeColor", "String", True, "#FFE53935"),
    prop(7, "FillColor", "String", True, "#00000000"),
    prop(8, "TextColor", "String", True, "#FF000000"),
    prop(9, "StrokeWidth", "Double", True, 6.0),
    prop(10, "FontSize", "Double", True, 28.0),
    prop(11, "FontFamily", "String", True, "Segoe UI"),
    prop(12, "Bold", "Boolean", True, False),
    prop(13, "Italic", "Boolean", True, False),
    prop(14, "Shadow", "Boolean", True, False),
    prop(15, "Opacity", "Double", True, 100.0),
    prop(16, "Text", "String", True, ""),
    prop(17, "Variant", "String", True, ""),
    prop(18, "Amount", "Double", True, 0.0),
    prop(19, "Align", "String", True, "Left"),
    prop(20, "Dash", "String", True, "Solid"),
    prop(21, "StartCap", "String", True, "None"),
    prop(22, "EndCap", "String", True, "None"),
    prop(23, "Selection", "String", initial=""),
    # Semicolon-separated sections of the properties panel to show.
    prop(24, "Panel", "String", initial=""),
    prop(25, "AmountLabel", "String", initial=""),
    # Bumped when the text box should take focus.
    prop(26, "FocusText", "Integer", initial=0),
    # "in", "out", "fit" or "actual", with a counter suffix so repeats notify.
    prop(27, "ZoomRequest", "String", initial=""),
    prop(28, "Action", "String", True, ""),
    prop(29, "Mode", "String", initial="Image"),
    prop(30, "VideoPosition", "Double", True, 0.0),
    prop(31, "VideoDuration", "Double", initial=0.0),
    prop(32, "VideoInfo", "String", initial=""),
    prop(33, "TrimStart", "Double", initial=0.0),
    prop(34, "TrimEnd", "Double", initial=0.0),
    prop(35, "Playing", "Boolean", initial=False),
    prop(36, "DocumentInfo", "String", initial=""),
    prop(37, "Rotation", "Double", True, 0.0),
    prop(38, "LibraryInfo", "String", initial=""),
    prop(39, "BackgroundFill", "String", True, "Auto-fill"),
    # Further selected objects' boxes, "x,y,w,h;..." for the canvas overlay.
    prop(40, "SelectionCount", "Integer", initial=0),
    prop(41, "HeadSize", "Double", True, 1.0),
    prop(42, "ShadowAngle", "Double", True, 45.0),
    prop(43, "ShadowDistance", "Double", True, 5.0),
    prop(44, "ShadowBlur", "Double", True, 5.0),
    prop(45, "ShadowColor", "String", True, "#73000000"),
    prop(46, "OutlineColor", "String", True, "#00000000"),
    prop(47, "OutlineWidth", "Double", True, 0.0),
    prop(48, "Underline", "Boolean", True, False),
    prop(49, "Strike", "Boolean", True, False),
    prop(50, "VAlign", "String", True, "Top"),
    prop(51, "Padding", "Double", True, -1.0),
    prop(52, "Fill2", "String", True, "#00000000"),
    prop(53, "GradientRadial", "Boolean", True, False),
    prop(54, "GradientAngle", "Double", True, 90.0),
    prop(55, "CanvasColor", "String", True, "#FFFFFFFF"),
    prop(56, "Theme", "String", False, "Dark"),
    prop(57, "GifPreset", "String", True, "Screen recording"),
    prop(58, "GifLoop", "Boolean", True, True),
    prop(59, "GifFade", "Boolean", True, False),
    prop(60, "GifDither", "Boolean", True, False),
    prop(61, "VideoVolume", "Double", True, 1.0),
    prop(62, "Muted", "Boolean", False, False),
    prop(63, "LibrarySearch", "String", True, ""),
    prop(64, "LibraryFilter", "String", True, "All"),
    prop(65, "LibrarySort", "String", True, "Newest"),
    prop(66, "ThumbSize", "Double", True, 120.0),
    prop(67, "LibraryExpanded", "Boolean", True, False),
    prop(68, "LibrarySelected", "Integer", False, 0),
    prop(69, "TemplateTheme", "String", True, "Light"),
    prop(70, "TemplateCaptions", "Boolean", True, False),
    prop(71, "TemplateNumbers", "Boolean", True, False),
    prop(72, "TemplateFooter", "String", True, ""),
    prop(73, "ZoneFit", "Boolean", False, False),
    prop(74, "ToolOrder", "String", False, ""),
    prop(75, "Narrating", "Boolean", False, False),
    prop(76, "ShowSideBar", "Boolean", True, True),
    prop(77, "ShowEffects", "Boolean", True, False),
    prop(78, "ShowTray", "Boolean", True, True),
    prop(79, "ToolLabels", "Boolean", True, True),
    # A notice to show as a toast: "serial\x1ftitle\x1ftext\x1f1 when a problem".
    prop(80, "Toast", "String", False, ""),
    # Grab Text's result to show: "serial\x1ftext".
    prop(81, "GrabText", "String", False, ""),
    # The Capture Widget is on (the capture side's setting).
    prop(82, "Widget", "Boolean", False, False),
    # The capture panels and outlines on screen, as JSON (see the capture
    # side's hud.rs); a panel's buttons answer with "hud:ID:INDEX".
    prop(83, "Hud", "String", False, ""),
    # The capture side's function table for the overlay and Screen Draw
    # windows (jiffshot_workflow::ui_api_address).
    prop(84, "UiApi", "Integer", initial=0),
    # The selection overlay to show (JSON: session, desktop, monitors), or
    # empty to take it down.
    prop(85, "Overlay", "String", False, ""),
    # Screen Draw to show (JSON: session, area), or empty.
    prop(86, "ScreenDraw", "String", False, ""),
    # The Preferences dialog to open: "serial\x1fJSON" (tabs of rows, see
    # src/prefs.rs); it answers with "prefs-apply:{...}".
    prop(87, "Prefs", "String", False, ""),
]

main_collections = [
    {"id": 1, "name": "Variants", "elementKind": "String", "recursive": False},
    {"id": 2, "name": "Fonts", "elementKind": "String", "recursive": False},
    {"id": 3, "name": "Library", "elementKind": "Model", "elementModelName": "LibraryItemViewModel", "recursive": False},
    {"id": 4, "name": "QuickStyles", "elementKind": "Model", "elementModelName": "QuickStyleViewModel", "recursive": False},
    {"id": 5, "name": "Effects", "elementKind": "String", "recursive": False},
    {"id": 6, "name": "Templates", "elementKind": "String", "recursive": False},
    {"id": 7, "name": "Shares", "elementKind": "String", "recursive": False},
    {"id": 8, "name": "LibraryFilters", "elementKind": "String", "recursive": False},
    {"id": 9, "name": "LibrarySorts", "elementKind": "String", "recursive": False},
    {"id": 10, "name": "TemplateThemes", "elementKind": "String", "recursive": False},
    {"id": 11, "name": "RecentFiles", "elementKind": "String", "recursive": False},
]

tools = ["Select", "Arrow", "Line", "Shape", "Text", "Callout", "Step", "Pen", "Highlight", "Blur", "Magnify", "Spotlight",
         "Stamp", "Fill", "Crop", "CutOut", "Selection", "Eyedropper", "SmartMove", "TextReplace"]
tool_headers = {"CutOut": "Cut _Out", "SmartMove": "Smart _Move", "TextReplace": "Text _Replace", "Selection": "Se_lection",
                "Eyedropper": "_Eyedropper", "MagicWand": "Magic _Wand", "Eraser": "Era_ser", "Select": "_Select / move", "Spotlight": "Spotlig_ht", "Stamp": "Stam_p", "Step": "S_tep"}

names = [
    "NewImage", "Open", "Save", "SaveAs", "Export", "CopyImage", "Paste", "NewCapture", "CombineImages", "OpenLibraryFolder", "Exit",
    "Undo", "Redo", "Copy", "Cut", "Delete", "Duplicate", "BringToFront", "BringForward", "SendBackward", "SendToBack", "SaveQuickStyle",
    "RotateLeft", "RotateRight", "FlipHorizontal", "FlipVertical", "Resize50", "Resize200", "Flatten",
    "ZoomIn", "ZoomOut", "ZoomFit", "ZoomActual",
    "GrabText", "Simplify",
    "PlayPause", "SetTrimStart", "SetTrimEnd", "ExportVideo", "ExportGif", "FrameToImage", "RecordVideo",
    "CanvasChanged",
    # Appended (IDs are positional): batch 1 of the clone plan.
    "SelectAll", "DeselectAll", "InvertSelection", "Group", "Ungroup", "FlattenSelected", "SmartRedact",
    "NewFromClipboard", "Print", "CloseFile", "DuplicateFile", "DeleteFile",
    "ToolMagicWand", "ToolEraser",
    "ExpandCanvas",
    "CycleTheme",
    "PrevFrame", "NextFrame", "CutSelection", "SilentCopy", "CombineVideos", "VideoFromImages", "ToggleMute",
    "ToggleLibraryView", "CombineSelected", "ClearLibrarySelection", "ShowClosed",
    "ResetToolbar",
    "NarrateImages", "NarrateNext", "NarrateFinish",
    "ToggleSideBar", "ToggleEffects", "ToggleTray", "ToggleToolLabels", "OpenCaptureWindow",
    "OpenRecentFile", "AutoTrim", "ToolCropMenu",
] + ["Tool" + t for t in tools]
commands = [cmd(i + 1, n) for i, n in enumerate(names)]
commands.append(cmd(len(commands) + 1, "Run", "Action"))

next_menu_id = [0]


def mid():
    next_menu_id[0] += 1
    return next_menu_id[0]


def item(command, header, gesture=None):
    i = {"id": mid(), "name": command, "kind": "Command", "header": header, "command": command, "items": []}
    if gesture:
        i["gesture"] = gesture
    return i


def toggle(name, header, prop, gesture=None):
    t = {"id": mid(), "name": name, "kind": "Toggle", "header": header, "toggleProperty": prop, "items": []}
    if gesture:
        t["gesture"] = gesture
    return t


def recent(name, header):
    return {"id": mid(), "name": name, "kind": "RecentFiles", "header": header, "items": []}


def sep(name):
    return {"id": mid(), "name": name, "kind": "Separator", "items": []}


def sub(name, header, items):
    s = {"id": mid(), "name": name, "kind": "Submenu", "header": header, "items": []}
    s["items"] = items()
    return s


menus = [{
    "id": 1, "name": "Main", "kind": "Application", "items": [
        sub("File", "_File", lambda: [
            item("NewCapture", "New _capture", "Ctrl+Shift+N"),
            item("OpenCaptureWindow", "Capture _window…"),
            item("RecordVideo", "_Record video"),
            item("NewImage", "_New image", "Ctrl+N"),
            item("Open", "_Open…", "Ctrl+O"),
            recent("OpenRecent", "Open _recent"),
            item("CombineImages", "Co_mbine images…"),
            sep("FileSep1"),
            item("Save", "_Save", "Ctrl+S"),
            item("SaveAs", "Save _as…", "Ctrl+Shift+S"),
            item("Export", "_Export image…", "Ctrl+E"),
            item("NewFromClipboard", "New from clip_board", "Ctrl+Shift+V"),
            item("CopyImage", "Copy _all", "Ctrl+Shift+C"),
            item("Print", "_Print…", "Ctrl+P"),
            item("DuplicateFile", "D_uplicate file", "Ctrl+Shift+D"),
            item("CloseFile", "_Close", "Ctrl+W"),
            item("DeleteFile", "Delete _file", "Ctrl+Delete"),
            sep("FileSep2"),
            item("OpenLibraryFolder", "Open _library folder"),
            item("Exit", "E_xit", "Ctrl+Q"),
        ]),
        sub("Edit", "_Edit", lambda: [
            item("Undo", "_Undo", "Ctrl+Z"),
            item("Redo", "_Redo", "Ctrl+Y"),
            sep("EditSep1"),
            item("Cut", "Cu_t", "Ctrl+X"),
            item("Copy", "_Copy", "Ctrl+C"),
            item("Paste", "_Paste", "Ctrl+V"),
            item("Duplicate", "_Duplicate"),
            item("Delete", "De_lete", "Delete"),
            sep("EditSep2"),
            item("SelectAll", "Select _all", "Ctrl+A"),
            item("DeselectAll", "Dese_lect all", "Ctrl+D"),
            item("InvertSelection", "_Invert selection", "Ctrl+Shift+I"),
            item("Group", "_Group", "Ctrl+Shift+G"),
            item("Ungroup", "U_ngroup", "Ctrl+Shift+U"),
            sep("EditSep4"),
            item("BringToFront", "Bring to _front", "Ctrl+Shift+OemCloseBrackets"),
            item("BringForward", "Bring _forward", "Ctrl+Shift+F"),
            item("SendBackward", "Send _backward", "Ctrl+Shift+B"),
            item("SendToBack", "Send to bac_k", "Ctrl+Shift+OemOpenBrackets"),
            sep("EditSep3"),
            item("SaveQuickStyle", "Save as _quick style"),
        ]),
        sub("Image", "_Image", lambda: [
            item("ToolCropMenu", "_Crop"),
            item("AutoTrim", "_Auto trim"),
            sep("ImageSep0"),
            item("RotateLeft", "Rotate _left"),
            item("RotateRight", "Rotate _right"),
            item("FlipHorizontal", "Flip _horizontal"),
            item("FlipVertical", "Flip _vertical"),
            sep("ImageSep1"),
            item("Resize50", "Resize to _50%"),
            item("Resize200", "Resize to _200%"),
            item("ExpandCanvas", "E_xpand canvas to fit objects"),
            item("FlattenSelected", "_Flatten selected", "Ctrl+T"),
            item("Flatten", "Flatten a_ll", "Ctrl+Shift+T"),
            item("SmartRedact", "Smart _redact"),
            sep("ImageSep2"),
            item("GrabText", "_Grab text"),
            item("Simplify", "_Simplify"),
        ]),
        sub("Tools", "_Tools", lambda: [item("Tool" + t, tool_headers.get(t, "_" + t), f"Ctrl+D{i + 1}" if i < 9 else None) for i, t in enumerate(tools + ["MagicWand", "Eraser"])]),
        sub("View", "_View", lambda: [
            item("ZoomIn", "Zoom _in", "Ctrl+OemPlus"),
            item("ZoomOut", "Zoom _out", "Ctrl+OemMinus"),
            item("ZoomFit", "_Fit", "Ctrl+Alt+D0"),
            item("ZoomActual", "_Actual size", "Ctrl+D0"),
            sep("ViewSep1"),
            item("ToggleLibraryView", "_Library view", "Ctrl+L"),
            item("ShowClosed", "Show captures _closed from the tray"),
            sep("ViewSep2"),
            toggle("SideBarToggle", "Side _bar", "ShowSideBar"),
            toggle("EffectsToggle", "_Effects panel", "ShowEffects"),
            toggle("TrayToggle", "Recent captures _tray", "ShowTray"),
            toggle("LabelsToggle", "Toolbar labe_ls", "ToolLabels"),
            item("ResetToolbar", "Show _all tools in their usual order"),
            item("CycleTheme", "_Theme: dark / light / system"),
        ]),
        sub("Video", "Vi_deo", lambda: [
            item("PlayPause", "_Play / pause"),
            item("SetTrimStart", "Set trim _start"),
            item("SetTrimEnd", "Set trim _end"),
            sep("VideoSep1"),
            item("PrevFrame", "P_revious frame"),
            item("NextFrame", "_Next frame"),
            item("ToggleMute", "_Mute"),
            sep("VideoSep2"),
            item("ExportVideo", "Save _selection as MP4…"),
            item("CutSelection", "_Cut selection…"),
            item("SilentCopy", "Save a si_lent copy…"),
            item("ExportGif", "Export _GIF…"),
            item("CombineVideos", "C_ombine videos…"),
            item("VideoFromImages", "Video from _images…"),
            item("NarrateImages", "Narrated video from images…"),
            item("FrameToImage", "Edit _frame as image"),
        ]),
    ],
}]

library_item = {
    "id": 2, "name": "LibraryItemViewModel", "managedNamespace": NS,
    "properties": [
        prop(1, "Path", "String", initial=""),
        prop(2, "Label", "String", initial=""),
        prop(3, "Thumbnail", "String", initial=""),
        prop(4, "IsCurrent", "Boolean", initial=False),
        prop(5, "Kind", "String", initial="Image"),
        prop(6, "IsPinned", "Boolean", initial=False),
        prop(7, "Tags", "String", initial=""),
        prop(8, "Info", "String", initial=""),
        prop(9, "IsSelected", "Boolean", initial=False),
        prop(10, "NewName", "String", True, ""),
        prop(11, "TagText", "String", True, ""),
        prop(12, "Unsaved", "Boolean", initial=False),
        prop(13, "ThumbHeight", "Double", initial=88.0),
    ],
    "collections": [], "maps": [],
    "commands": [cmd(1, "Open"), cmd(2, "TogglePin"), cmd(3, "Close"), cmd(4, "Delete"), cmd(5, "Rename", "NewName"),
                 cmd(6, "SetTags", "TagText"), cmd(7, "ToggleSelect"), cmd(8, "ShowInFolder")],
    "menus": [],
}

quick_style = {
    "id": 3, "name": "QuickStyleViewModel", "managedNamespace": NS,
    "properties": [
        prop(1, "Name", "String", initial=""),
        prop(2, "Stroke", "String", initial="#00000000"),
        prop(3, "Fill", "String", initial="#00000000"),
        prop(4, "TextColor", "String", initial="#00000000"),
        prop(5, "Width", "Double", initial=0.0),
        prop(6, "IsFavorite", "Boolean", initial=False),
    ],
    "collections": [], "maps": [],
    "commands": [cmd(1, "Apply"), cmd(2, "Delete"), cmd(3, "MoveEarlier"), cmd(4, "MoveLater"), cmd(5, "ToggleFavorite")],
    "menus": [],
}

# The notification-area icon: its menu runs the capture side's commands and
# shows its switches; left-click captures.
tray_commands = [
    "Capture", "CaptureDelayed", "CaptureScreen", "RepeatLast", "CaptureWindow", "ShowEditor",
    "Scrolling", "ScrollingHorizontal", "ScrollingBoth", "Panoramic", "Freehand", "MultipleAreas", "FixedSize",
    "MenuCapture", "GrabText", "StepCapture", "FromClipboard", "FromFile", "RecordVideo", "OpenLibraryFolder", "Exit",
    "Preferences", "OpenRecentCapture",
]


def tray_item(command, header, enabled=None):
    i = {"id": mid(), "name": "Tray" + command, "kind": "Command", "header": header, "command": command, "items": []}
    if enabled:
        i["isEnabledProperty"] = enabled
    return i


tray_model = {
    "id": 4, "name": "TrayViewModel", "managedNamespace": NS,
    "properties": [
        prop(1, "CopyToClipboard", "Boolean", True, True),
        prop(2, "OpenInEditor", "Boolean", True, True),
        prop(3, "IncludeCursor", "Boolean", True, False),
        prop(4, "ShowWidget", "Boolean", True, False),
        prop(5, "RunAtStartup", "Boolean", True, False),
        prop(6, "TimeLapse", "Boolean", True, False),
        prop(7, "Schedule", "Boolean", True, False),
        prop(8, "CanRepeat", "Boolean", False, False),
    ],
    # The library's newest captures, as file URIs, for Recent captures.
    "collections": [{"id": 1, "name": "RecentCaptures", "elementKind": "String", "recursive": False}],
    "maps": [],
    "commands": [cmd(i + 1, n) for i, n in enumerate(tray_commands)],
    "recentFiles": {"id": 1, "collection": "RecentCaptures", "activateCommand": "OpenRecentCapture", "capacity": 10},
    "menus": [{
        "id": 1, "name": "Tray", "kind": "Tray", "items": [
            tray_item("Capture", "_Capture"),
            tray_item("CaptureDelayed", "Capture after the _delay"),
            tray_item("CaptureScreen", "Capture the _screen under the pointer"),
            tray_item("RepeatLast", "_Repeat the last area", enabled="CanRepeat"),
            sep("TraySep1"),
            tray_item("CaptureWindow", "Capture _window…"),
            tray_item("ShowEditor", "Open Jiffshot _Editor"),
            recent("TrayRecent", "Recent ca_ptures"),
            sep("TraySep2"),
            sub("TrayModes", "More capture _modes", lambda: [
                tray_item("Scrolling", "_Scrolling"),
                tray_item("ScrollingHorizontal", "Scrolling, _horizontal"),
                tray_item("ScrollingBoth", "Scrolling, _both directions"),
                tray_item("Panoramic", "_Panoramic"),
                tray_item("Freehand", "_Freehand"),
                tray_item("MultipleAreas", "_Multiple areas"),
                tray_item("FixedSize", "Fixed si_ze"),
                tray_item("MenuCapture", "M_enu (3 second countdown)"),
                tray_item("GrabText", "_Grab text"),
                tray_item("StepCapture", "S_tep Capture"),
                sep("TraySep3"),
                tray_item("FromClipboard", "From the _clipboard"),
                tray_item("FromFile", "From a _file…"),
            ]),
            tray_item("RecordVideo", "Record _video"),
            sep("TraySep4"),
            toggle("TrayTimeLapse", "_Time-lapse", "TimeLapse"),
            toggle("TraySchedule", "Scheduled capture (_daily)", "Schedule"),
            sep("TraySep5"),
            toggle("TrayClipboard", "Copy to clip_board", "CopyToClipboard"),
            toggle("TrayEditor", "Open in the edit_or", "OpenInEditor"),
            toggle("TrayCursor", "Include the c_ursor", "IncludeCursor"),
            sep("TraySep6"),
            toggle("TrayWidget", "Show the Capture Wid_get", "ShowWidget"),
            toggle("TrayStartup", "Run when Windows _starts", "RunAtStartup"),
            tray_item("OpenLibraryFolder", "Open the _library folder"),
            tray_item("Preferences", "_Preferences…"),
            sep("TraySep7"),
            tray_item("Exit", "E_xit Jiffshot"),
        ],
    }],
}

# The Capture window. Its layout is data (see crates/jiffshot-workflow
# src/panel.rs): the view, menus and option panels arrive as JSON, and the
# window sends back actions ("tab:1", "pick:sel:window", "apply:delay:[...]").
capture_model = {
    "id": 5, "name": "CaptureViewModel", "managedNamespace": NS,
    "properties": [
        prop(1, "View", "String", False, "{}"),
        # "serial\x1fJSON" so asking for the same menu again still notifies.
        prop(2, "Menu", "String", False, ""),
        prop(3, "Panel", "String", False, ""),
        prop(4, "Problem", "String", False, ""),
        prop(5, "Action", "String", True, ""),
    ],
    "collections": [], "maps": [],
    "commands": [cmd(1, "Run", "Action")],
    "menus": [],
}

ir = {
    "version": 5,
    "enums": [],
    "models": [
        {"id": 1, "name": "MainViewModel", "managedNamespace": NS, "properties": main_props, "collections": main_collections,
         "maps": [], "commands": commands, "menus": menus,
         "recentFiles": {"id": 1, "collection": "RecentFiles", "activateCommand": "OpenRecentFile", "capacity": 10}},
        library_item,
        quick_style,
        tray_model,
        capture_model,
    ],
    "trays": [{"id": 1, "name": "Tray", "model": "TrayViewModel", "menu": "Tray", "toolTip": "Jiffshot",
               "icon": "avares://Consumer.Presentation/Assets/jiffshot.ico", "activateCommand": "Capture"}],
    "views": [{"id": 1, "name": "MainWindow", "model": "MainViewModel", "managedTypeName": "Jiffshot.Editor.Views.MainWindow",
               "dynamicBindings": False, "converters": []},
              {"id": 2, "name": "CaptureWindow", "model": "CaptureViewModel", "managedTypeName": "Jiffshot.Editor.Views.CaptureWindow",
               "dynamicBindings": False, "converters": []}],
    "converters": [],
}

out = os.path.join(os.path.dirname(__file__), "..", "view-model.ir.json")
with open(out, "w", encoding="utf-8", newline="\n") as f:
    json.dump(ir, f, indent=2, ensure_ascii=False)
    f.write("\n")
print("wrote", os.path.normpath(out), len(commands), "commands")
