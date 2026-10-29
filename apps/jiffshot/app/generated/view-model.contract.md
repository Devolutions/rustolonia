# Generated Rust view-model contract

Schema version: `5`

## Model `MainViewModel` (`1`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Title` | `String` | Rust to managed |
| Property | 2 | `Status` | `String` | Rust to managed |
| Property | 3 | `Tool` | `String` | Rust to managed |
| Property | 4 | `CanvasApi` | `Integer` | Rust to managed |
| Property | 5 | `FrameId` | `Integer` | Rust to managed |
| Property | 6 | `StrokeColor` | `String` | Rust and managed |
| Property | 7 | `FillColor` | `String` | Rust and managed |
| Property | 8 | `TextColor` | `String` | Rust and managed |
| Property | 9 | `StrokeWidth` | `Double` | Rust and managed |
| Property | 10 | `FontSize` | `Double` | Rust and managed |
| Property | 11 | `FontFamily` | `String` | Rust and managed |
| Property | 12 | `Bold` | `Boolean` | Rust and managed |
| Property | 13 | `Italic` | `Boolean` | Rust and managed |
| Property | 14 | `Shadow` | `Boolean` | Rust and managed |
| Property | 15 | `Opacity` | `Double` | Rust and managed |
| Property | 16 | `Text` | `String` | Rust and managed |
| Property | 17 | `Variant` | `String` | Rust and managed |
| Property | 18 | `Amount` | `Double` | Rust and managed |
| Property | 19 | `Align` | `String` | Rust and managed |
| Property | 20 | `Dash` | `String` | Rust and managed |
| Property | 21 | `StartCap` | `String` | Rust and managed |
| Property | 22 | `EndCap` | `String` | Rust and managed |
| Property | 23 | `Selection` | `String` | Rust to managed |
| Property | 24 | `Panel` | `String` | Rust to managed |
| Property | 25 | `AmountLabel` | `String` | Rust to managed |
| Property | 26 | `FocusText` | `Integer` | Rust to managed |
| Property | 27 | `ZoomRequest` | `String` | Rust to managed |
| Property | 28 | `Action` | `String` | Rust and managed |
| Property | 29 | `Mode` | `String` | Rust to managed |
| Property | 30 | `VideoPosition` | `Double` | Rust and managed |
| Property | 31 | `VideoDuration` | `Double` | Rust to managed |
| Property | 32 | `VideoInfo` | `String` | Rust to managed |
| Property | 33 | `TrimStart` | `Double` | Rust to managed |
| Property | 34 | `TrimEnd` | `Double` | Rust to managed |
| Property | 35 | `Playing` | `Boolean` | Rust to managed |
| Property | 36 | `DocumentInfo` | `String` | Rust to managed |
| Property | 37 | `Rotation` | `Double` | Rust and managed |
| Property | 38 | `LibraryInfo` | `String` | Rust to managed |
| Property | 39 | `BackgroundFill` | `String` | Rust and managed |
| Property | 40 | `SelectionCount` | `Integer` | Rust to managed |
| Property | 41 | `HeadSize` | `Double` | Rust and managed |
| Property | 42 | `ShadowAngle` | `Double` | Rust and managed |
| Property | 43 | `ShadowDistance` | `Double` | Rust and managed |
| Property | 44 | `ShadowBlur` | `Double` | Rust and managed |
| Property | 45 | `ShadowColor` | `String` | Rust and managed |
| Property | 46 | `OutlineColor` | `String` | Rust and managed |
| Property | 47 | `OutlineWidth` | `Double` | Rust and managed |
| Property | 48 | `Underline` | `Boolean` | Rust and managed |
| Property | 49 | `Strike` | `Boolean` | Rust and managed |
| Property | 50 | `VAlign` | `String` | Rust and managed |
| Property | 51 | `Padding` | `Double` | Rust and managed |
| Property | 52 | `Fill2` | `String` | Rust and managed |
| Property | 53 | `GradientRadial` | `Boolean` | Rust and managed |
| Property | 54 | `GradientAngle` | `Double` | Rust and managed |
| Property | 55 | `CanvasColor` | `String` | Rust and managed |
| Property | 56 | `Theme` | `String` | Rust to managed |
| Property | 57 | `GifPreset` | `String` | Rust and managed |
| Property | 58 | `GifLoop` | `Boolean` | Rust and managed |
| Property | 59 | `GifFade` | `Boolean` | Rust and managed |
| Property | 60 | `GifDither` | `Boolean` | Rust and managed |
| Property | 61 | `VideoVolume` | `Double` | Rust and managed |
| Property | 62 | `Muted` | `Boolean` | Rust to managed |
| Property | 63 | `LibrarySearch` | `String` | Rust and managed |
| Property | 64 | `LibraryFilter` | `String` | Rust and managed |
| Property | 65 | `LibrarySort` | `String` | Rust and managed |
| Property | 66 | `ThumbSize` | `Double` | Rust and managed |
| Property | 67 | `LibraryExpanded` | `Boolean` | Rust and managed |
| Property | 68 | `LibrarySelected` | `Integer` | Rust to managed |
| Property | 69 | `TemplateTheme` | `String` | Rust and managed |
| Property | 70 | `TemplateCaptions` | `Boolean` | Rust and managed |
| Property | 71 | `TemplateNumbers` | `Boolean` | Rust and managed |
| Property | 72 | `TemplateFooter` | `String` | Rust and managed |
| Property | 73 | `ZoneFit` | `Boolean` | Rust to managed |
| Property | 74 | `ToolOrder` | `String` | Rust to managed |
| Property | 75 | `Narrating` | `Boolean` | Rust to managed |
| Property | 76 | `ShowSideBar` | `Boolean` | Rust and managed |
| Property | 77 | `ShowEffects` | `Boolean` | Rust and managed |
| Property | 78 | `ShowTray` | `Boolean` | Rust and managed |
| Property | 79 | `ToolLabels` | `Boolean` | Rust and managed |
| Property | 80 | `Toast` | `String` | Rust to managed |
| Property | 81 | `GrabText` | `String` | Rust to managed |
| Property | 82 | `Widget` | `Boolean` | Rust to managed |
| Property | 83 | `Hud` | `String` | Rust to managed |
| Property | 84 | `UiApi` | `Integer` | Rust to managed |
| Property | 85 | `Overlay` | `String` | Rust to managed |
| Property | 86 | `ScreenDraw` | `String` | Rust to managed |
| Property | 87 | `Prefs` | `String` | Rust to managed |
| Collection | 1 | `Variants` | `String` | Rust to managed |
| Collection | 2 | `Fonts` | `String` | Rust to managed |
| Collection | 3 | `Library` | Model `LibraryItemViewModel` | Rust to managed |
| Collection | 4 | `QuickStyles` | Model `QuickStyleViewModel` | Rust to managed |
| Collection | 5 | `Effects` | `String` | Rust to managed |
| Collection | 6 | `Templates` | `String` | Rust to managed |
| Collection | 7 | `Shares` | `String` | Rust to managed |
| Collection | 8 | `LibraryFilters` | `String` | Rust to managed |
| Collection | 9 | `LibrarySorts` | `String` | Rust to managed |
| Collection | 10 | `TemplateThemes` | `String` | Rust to managed |
| Collection | 11 | `RecentFiles` | `String` | Rust to managed |
| Command | 1 | `NewImage` | None | Managed to Rust |
| Command | 2 | `Open` | None | Managed to Rust |
| Command | 3 | `Save` | None | Managed to Rust |
| Command | 4 | `SaveAs` | None | Managed to Rust |
| Command | 5 | `Export` | None | Managed to Rust |
| Command | 6 | `CopyImage` | None | Managed to Rust |
| Command | 7 | `Paste` | None | Managed to Rust |
| Command | 8 | `NewCapture` | None | Managed to Rust |
| Command | 9 | `CombineImages` | None | Managed to Rust |
| Command | 10 | `OpenLibraryFolder` | None | Managed to Rust |
| Command | 11 | `Exit` | None | Managed to Rust |
| Command | 12 | `Undo` | None | Managed to Rust |
| Command | 13 | `Redo` | None | Managed to Rust |
| Command | 14 | `Copy` | None | Managed to Rust |
| Command | 15 | `Cut` | None | Managed to Rust |
| Command | 16 | `Delete` | None | Managed to Rust |
| Command | 17 | `Duplicate` | None | Managed to Rust |
| Command | 18 | `BringToFront` | None | Managed to Rust |
| Command | 19 | `BringForward` | None | Managed to Rust |
| Command | 20 | `SendBackward` | None | Managed to Rust |
| Command | 21 | `SendToBack` | None | Managed to Rust |
| Command | 22 | `SaveQuickStyle` | None | Managed to Rust |
| Command | 23 | `RotateLeft` | None | Managed to Rust |
| Command | 24 | `RotateRight` | None | Managed to Rust |
| Command | 25 | `FlipHorizontal` | None | Managed to Rust |
| Command | 26 | `FlipVertical` | None | Managed to Rust |
| Command | 27 | `Resize50` | None | Managed to Rust |
| Command | 28 | `Resize200` | None | Managed to Rust |
| Command | 29 | `Flatten` | None | Managed to Rust |
| Command | 30 | `ZoomIn` | None | Managed to Rust |
| Command | 31 | `ZoomOut` | None | Managed to Rust |
| Command | 32 | `ZoomFit` | None | Managed to Rust |
| Command | 33 | `ZoomActual` | None | Managed to Rust |
| Command | 34 | `GrabText` | None | Managed to Rust |
| Command | 35 | `Simplify` | None | Managed to Rust |
| Command | 36 | `PlayPause` | None | Managed to Rust |
| Command | 37 | `SetTrimStart` | None | Managed to Rust |
| Command | 38 | `SetTrimEnd` | None | Managed to Rust |
| Command | 39 | `ExportVideo` | None | Managed to Rust |
| Command | 40 | `ExportGif` | None | Managed to Rust |
| Command | 41 | `FrameToImage` | None | Managed to Rust |
| Command | 42 | `RecordVideo` | None | Managed to Rust |
| Command | 43 | `CanvasChanged` | None | Managed to Rust |
| Command | 44 | `SelectAll` | None | Managed to Rust |
| Command | 45 | `DeselectAll` | None | Managed to Rust |
| Command | 46 | `InvertSelection` | None | Managed to Rust |
| Command | 47 | `Group` | None | Managed to Rust |
| Command | 48 | `Ungroup` | None | Managed to Rust |
| Command | 49 | `FlattenSelected` | None | Managed to Rust |
| Command | 50 | `SmartRedact` | None | Managed to Rust |
| Command | 51 | `NewFromClipboard` | None | Managed to Rust |
| Command | 52 | `Print` | None | Managed to Rust |
| Command | 53 | `CloseFile` | None | Managed to Rust |
| Command | 54 | `DuplicateFile` | None | Managed to Rust |
| Command | 55 | `DeleteFile` | None | Managed to Rust |
| Command | 56 | `ToolMagicWand` | None | Managed to Rust |
| Command | 57 | `ToolEraser` | None | Managed to Rust |
| Command | 58 | `ExpandCanvas` | None | Managed to Rust |
| Command | 59 | `CycleTheme` | None | Managed to Rust |
| Command | 60 | `PrevFrame` | None | Managed to Rust |
| Command | 61 | `NextFrame` | None | Managed to Rust |
| Command | 62 | `CutSelection` | None | Managed to Rust |
| Command | 63 | `SilentCopy` | None | Managed to Rust |
| Command | 64 | `CombineVideos` | None | Managed to Rust |
| Command | 65 | `VideoFromImages` | None | Managed to Rust |
| Command | 66 | `ToggleMute` | None | Managed to Rust |
| Command | 67 | `ToggleLibraryView` | None | Managed to Rust |
| Command | 68 | `CombineSelected` | None | Managed to Rust |
| Command | 69 | `ClearLibrarySelection` | None | Managed to Rust |
| Command | 70 | `ShowClosed` | None | Managed to Rust |
| Command | 71 | `ResetToolbar` | None | Managed to Rust |
| Command | 72 | `NarrateImages` | None | Managed to Rust |
| Command | 73 | `NarrateNext` | None | Managed to Rust |
| Command | 74 | `NarrateFinish` | None | Managed to Rust |
| Command | 75 | `ToggleSideBar` | None | Managed to Rust |
| Command | 76 | `ToggleEffects` | None | Managed to Rust |
| Command | 77 | `ToggleTray` | None | Managed to Rust |
| Command | 78 | `ToggleToolLabels` | None | Managed to Rust |
| Command | 79 | `OpenCaptureWindow` | None | Managed to Rust |
| Command | 80 | `OpenRecentFile` | None | Managed to Rust |
| Command | 81 | `AutoTrim` | None | Managed to Rust |
| Command | 82 | `ToolCropMenu` | None | Managed to Rust |
| Command | 83 | `ToolSelect` | None | Managed to Rust |
| Command | 84 | `ToolArrow` | None | Managed to Rust |
| Command | 85 | `ToolLine` | None | Managed to Rust |
| Command | 86 | `ToolShape` | None | Managed to Rust |
| Command | 87 | `ToolText` | None | Managed to Rust |
| Command | 88 | `ToolCallout` | None | Managed to Rust |
| Command | 89 | `ToolStep` | None | Managed to Rust |
| Command | 90 | `ToolPen` | None | Managed to Rust |
| Command | 91 | `ToolHighlight` | None | Managed to Rust |
| Command | 92 | `ToolBlur` | None | Managed to Rust |
| Command | 93 | `ToolMagnify` | None | Managed to Rust |
| Command | 94 | `ToolSpotlight` | None | Managed to Rust |
| Command | 95 | `ToolStamp` | None | Managed to Rust |
| Command | 96 | `ToolFill` | None | Managed to Rust |
| Command | 97 | `ToolCrop` | None | Managed to Rust |
| Command | 98 | `ToolCutOut` | None | Managed to Rust |
| Command | 99 | `ToolSelection` | None | Managed to Rust |
| Command | 100 | `ToolEyedropper` | None | Managed to Rust |
| Command | 101 | `ToolSmartMove` | None | Managed to Rust |
| Command | 102 | `ToolTextReplace` | None | Managed to Rust |
| Command | 103 | `Run` | `Action` | Managed to Rust |

### Recent files `RecentFiles`

Storage URIs published into collection `RecentFiles`, capacity 10, activated by `OpenRecentFileCommand` with the chosen URI as its command parameter.

### Application menu `Main` (`1`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `File` | Submenu | _File | - | - | - |
| 2 |     `NewCapture` | Command | New _capture | `NewCaptureCommand` | `Ctrl+Shift+N` | - |
| 3 |     `OpenCaptureWindow` | Command | Capture _window… | `OpenCaptureWindowCommand` | - | - |
| 4 |     `RecordVideo` | Command | _Record video | `RecordVideoCommand` | - | - |
| 5 |     `NewImage` | Command | _New image | `NewImageCommand` | `Ctrl+N` | - |
| 6 |     `Open` | Command | _Open… | `OpenCommand` | `Ctrl+O` | - |
| 7 |     `OpenRecent` | RecentFiles | Open _recent | - | - | recent files |
| 8 |     `CombineImages` | Command | Co_mbine images… | `CombineImagesCommand` | - | - |
| 9 |     `FileSep1` | Separator | - | - | - | - |
| 10 |     `Save` | Command | _Save | `SaveCommand` | `Ctrl+S` | - |
| 11 |     `SaveAs` | Command | Save _as… | `SaveAsCommand` | `Ctrl+Shift+S` | - |
| 12 |     `Export` | Command | _Export image… | `ExportCommand` | `Ctrl+E` | - |
| 13 |     `NewFromClipboard` | Command | New from clip_board | `NewFromClipboardCommand` | `Ctrl+Shift+V` | - |
| 14 |     `CopyImage` | Command | Copy _all | `CopyImageCommand` | `Ctrl+Shift+C` | - |
| 15 |     `Print` | Command | _Print… | `PrintCommand` | `Ctrl+P` | - |
| 16 |     `DuplicateFile` | Command | D_uplicate file | `DuplicateFileCommand` | `Ctrl+Shift+D` | - |
| 17 |     `CloseFile` | Command | _Close | `CloseFileCommand` | `Ctrl+W` | - |
| 18 |     `DeleteFile` | Command | Delete _file | `DeleteFileCommand` | `Ctrl+Delete` | - |
| 19 |     `FileSep2` | Separator | - | - | - | - |
| 20 |     `OpenLibraryFolder` | Command | Open _library folder | `OpenLibraryFolderCommand` | - | - |
| 21 |     `Exit` | Command | E_xit | `ExitCommand` | `Ctrl+Q` | - |
| 22 | `Edit` | Submenu | _Edit | - | - | - |
| 23 |     `Undo` | Command | _Undo | `UndoCommand` | `Ctrl+Z` | - |
| 24 |     `Redo` | Command | _Redo | `RedoCommand` | `Ctrl+Y` | - |
| 25 |     `EditSep1` | Separator | - | - | - | - |
| 26 |     `Cut` | Command | Cu_t | `CutCommand` | `Ctrl+X` | - |
| 27 |     `Copy` | Command | _Copy | `CopyCommand` | `Ctrl+C` | - |
| 28 |     `Paste` | Command | _Paste | `PasteCommand` | `Ctrl+V` | - |
| 29 |     `Duplicate` | Command | _Duplicate | `DuplicateCommand` | - | - |
| 30 |     `Delete` | Command | De_lete | `DeleteCommand` | `Delete` | - |
| 31 |     `EditSep2` | Separator | - | - | - | - |
| 32 |     `SelectAll` | Command | Select _all | `SelectAllCommand` | `Ctrl+A` | - |
| 33 |     `DeselectAll` | Command | Dese_lect all | `DeselectAllCommand` | `Ctrl+D` | - |
| 34 |     `InvertSelection` | Command | _Invert selection | `InvertSelectionCommand` | `Ctrl+Shift+I` | - |
| 35 |     `Group` | Command | _Group | `GroupCommand` | `Ctrl+Shift+G` | - |
| 36 |     `Ungroup` | Command | U_ngroup | `UngroupCommand` | `Ctrl+Shift+U` | - |
| 37 |     `EditSep4` | Separator | - | - | - | - |
| 38 |     `BringToFront` | Command | Bring to _front | `BringToFrontCommand` | `Ctrl+Shift+OemCloseBrackets` | - |
| 39 |     `BringForward` | Command | Bring _forward | `BringForwardCommand` | `Ctrl+Shift+F` | - |
| 40 |     `SendBackward` | Command | Send _backward | `SendBackwardCommand` | `Ctrl+Shift+B` | - |
| 41 |     `SendToBack` | Command | Send to bac_k | `SendToBackCommand` | `Ctrl+Shift+OemOpenBrackets` | - |
| 42 |     `EditSep3` | Separator | - | - | - | - |
| 43 |     `SaveQuickStyle` | Command | Save as _quick style | `SaveQuickStyleCommand` | - | - |
| 44 | `Image` | Submenu | _Image | - | - | - |
| 45 |     `ToolCropMenu` | Command | _Crop | `ToolCropMenuCommand` | - | - |
| 46 |     `AutoTrim` | Command | _Auto trim | `AutoTrimCommand` | - | - |
| 47 |     `ImageSep0` | Separator | - | - | - | - |
| 48 |     `RotateLeft` | Command | Rotate _left | `RotateLeftCommand` | - | - |
| 49 |     `RotateRight` | Command | Rotate _right | `RotateRightCommand` | - | - |
| 50 |     `FlipHorizontal` | Command | Flip _horizontal | `FlipHorizontalCommand` | - | - |
| 51 |     `FlipVertical` | Command | Flip _vertical | `FlipVerticalCommand` | - | - |
| 52 |     `ImageSep1` | Separator | - | - | - | - |
| 53 |     `Resize50` | Command | Resize to _50% | `Resize50Command` | - | - |
| 54 |     `Resize200` | Command | Resize to _200% | `Resize200Command` | - | - |
| 55 |     `ExpandCanvas` | Command | E_xpand canvas to fit objects | `ExpandCanvasCommand` | - | - |
| 56 |     `FlattenSelected` | Command | _Flatten selected | `FlattenSelectedCommand` | `Ctrl+T` | - |
| 57 |     `Flatten` | Command | Flatten a_ll | `FlattenCommand` | `Ctrl+Shift+T` | - |
| 58 |     `SmartRedact` | Command | Smart _redact | `SmartRedactCommand` | - | - |
| 59 |     `ImageSep2` | Separator | - | - | - | - |
| 60 |     `GrabText` | Command | _Grab text | `GrabTextCommand` | - | - |
| 61 |     `Simplify` | Command | _Simplify | `SimplifyCommand` | - | - |
| 62 | `Tools` | Submenu | _Tools | - | - | - |
| 63 |     `ToolSelect` | Command | _Select / move | `ToolSelectCommand` | `Ctrl+D1` | - |
| 64 |     `ToolArrow` | Command | _Arrow | `ToolArrowCommand` | `Ctrl+D2` | - |
| 65 |     `ToolLine` | Command | _Line | `ToolLineCommand` | `Ctrl+D3` | - |
| 66 |     `ToolShape` | Command | _Shape | `ToolShapeCommand` | `Ctrl+D4` | - |
| 67 |     `ToolText` | Command | _Text | `ToolTextCommand` | `Ctrl+D5` | - |
| 68 |     `ToolCallout` | Command | _Callout | `ToolCalloutCommand` | `Ctrl+D6` | - |
| 69 |     `ToolStep` | Command | S_tep | `ToolStepCommand` | `Ctrl+D7` | - |
| 70 |     `ToolPen` | Command | _Pen | `ToolPenCommand` | `Ctrl+D8` | - |
| 71 |     `ToolHighlight` | Command | _Highlight | `ToolHighlightCommand` | `Ctrl+D9` | - |
| 72 |     `ToolBlur` | Command | _Blur | `ToolBlurCommand` | - | - |
| 73 |     `ToolMagnify` | Command | _Magnify | `ToolMagnifyCommand` | - | - |
| 74 |     `ToolSpotlight` | Command | Spotlig_ht | `ToolSpotlightCommand` | - | - |
| 75 |     `ToolStamp` | Command | Stam_p | `ToolStampCommand` | - | - |
| 76 |     `ToolFill` | Command | _Fill | `ToolFillCommand` | - | - |
| 77 |     `ToolCrop` | Command | _Crop | `ToolCropCommand` | - | - |
| 78 |     `ToolCutOut` | Command | Cut _Out | `ToolCutOutCommand` | - | - |
| 79 |     `ToolSelection` | Command | Se_lection | `ToolSelectionCommand` | - | - |
| 80 |     `ToolEyedropper` | Command | _Eyedropper | `ToolEyedropperCommand` | - | - |
| 81 |     `ToolSmartMove` | Command | Smart _Move | `ToolSmartMoveCommand` | - | - |
| 82 |     `ToolTextReplace` | Command | Text _Replace | `ToolTextReplaceCommand` | - | - |
| 83 |     `ToolMagicWand` | Command | Magic _Wand | `ToolMagicWandCommand` | - | - |
| 84 |     `ToolEraser` | Command | Era_ser | `ToolEraserCommand` | - | - |
| 85 | `View` | Submenu | _View | - | - | - |
| 86 |     `ZoomIn` | Command | Zoom _in | `ZoomInCommand` | `Ctrl+OemPlus` | - |
| 87 |     `ZoomOut` | Command | Zoom _out | `ZoomOutCommand` | `Ctrl+OemMinus` | - |
| 88 |     `ZoomFit` | Command | _Fit | `ZoomFitCommand` | `Ctrl+Alt+D0` | - |
| 89 |     `ZoomActual` | Command | _Actual size | `ZoomActualCommand` | `Ctrl+D0` | - |
| 90 |     `ViewSep1` | Separator | - | - | - | - |
| 91 |     `ToggleLibraryView` | Command | _Library view | `ToggleLibraryViewCommand` | `Ctrl+L` | - |
| 92 |     `ShowClosed` | Command | Show captures _closed from the tray | `ShowClosedCommand` | - | - |
| 93 |     `ViewSep2` | Separator | - | - | - | - |
| 94 |     `SideBarToggle` | Toggle | Side _bar | - | - | `ShowSideBar` |
| 95 |     `EffectsToggle` | Toggle | _Effects panel | - | - | `ShowEffects` |
| 96 |     `TrayToggle` | Toggle | Recent captures _tray | - | - | `ShowTray` |
| 97 |     `LabelsToggle` | Toggle | Toolbar labe_ls | - | - | `ToolLabels` |
| 98 |     `ResetToolbar` | Command | Show _all tools in their usual order | `ResetToolbarCommand` | - | - |
| 99 |     `CycleTheme` | Command | _Theme: dark / light / system | `CycleThemeCommand` | - | - |
| 100 | `Video` | Submenu | Vi_deo | - | - | - |
| 101 |     `PlayPause` | Command | _Play / pause | `PlayPauseCommand` | - | - |
| 102 |     `SetTrimStart` | Command | Set trim _start | `SetTrimStartCommand` | - | - |
| 103 |     `SetTrimEnd` | Command | Set trim _end | `SetTrimEndCommand` | - | - |
| 104 |     `VideoSep1` | Separator | - | - | - | - |
| 105 |     `PrevFrame` | Command | P_revious frame | `PrevFrameCommand` | - | - |
| 106 |     `NextFrame` | Command | _Next frame | `NextFrameCommand` | - | - |
| 107 |     `ToggleMute` | Command | _Mute | `ToggleMuteCommand` | - | - |
| 108 |     `VideoSep2` | Separator | - | - | - | - |
| 109 |     `ExportVideo` | Command | Save _selection as MP4… | `ExportVideoCommand` | - | - |
| 110 |     `CutSelection` | Command | _Cut selection… | `CutSelectionCommand` | - | - |
| 111 |     `SilentCopy` | Command | Save a si_lent copy… | `SilentCopyCommand` | - | - |
| 112 |     `ExportGif` | Command | Export _GIF… | `ExportGifCommand` | - | - |
| 113 |     `CombineVideos` | Command | C_ombine videos… | `CombineVideosCommand` | - | - |
| 114 |     `VideoFromImages` | Command | Video from _images… | `VideoFromImagesCommand` | - | - |
| 115 |     `NarrateImages` | Command | Narrated video from images… | `NarrateImagesCommand` | - | - |
| 116 |     `FrameToImage` | Command | Edit _frame as image | `FrameToImageCommand` | - | - |

## Model `LibraryItemViewModel` (`2`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Path` | `String` | Rust to managed |
| Property | 2 | `Label` | `String` | Rust to managed |
| Property | 3 | `Thumbnail` | `String` | Rust to managed |
| Property | 4 | `IsCurrent` | `Boolean` | Rust to managed |
| Property | 5 | `Kind` | `String` | Rust to managed |
| Property | 6 | `IsPinned` | `Boolean` | Rust to managed |
| Property | 7 | `Tags` | `String` | Rust to managed |
| Property | 8 | `Info` | `String` | Rust to managed |
| Property | 9 | `IsSelected` | `Boolean` | Rust to managed |
| Property | 10 | `NewName` | `String` | Rust and managed |
| Property | 11 | `TagText` | `String` | Rust and managed |
| Property | 12 | `Unsaved` | `Boolean` | Rust to managed |
| Property | 13 | `ThumbHeight` | `Double` | Rust to managed |
| Command | 1 | `Open` | None | Managed to Rust |
| Command | 2 | `TogglePin` | None | Managed to Rust |
| Command | 3 | `Close` | None | Managed to Rust |
| Command | 4 | `Delete` | None | Managed to Rust |
| Command | 5 | `Rename` | `NewName` | Managed to Rust |
| Command | 6 | `SetTags` | `TagText` | Managed to Rust |
| Command | 7 | `ToggleSelect` | None | Managed to Rust |
| Command | 8 | `ShowInFolder` | None | Managed to Rust |

## Model `QuickStyleViewModel` (`3`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Name` | `String` | Rust to managed |
| Property | 2 | `Stroke` | `String` | Rust to managed |
| Property | 3 | `Fill` | `String` | Rust to managed |
| Property | 4 | `TextColor` | `String` | Rust to managed |
| Property | 5 | `Width` | `Double` | Rust to managed |
| Property | 6 | `IsFavorite` | `Boolean` | Rust to managed |
| Command | 1 | `Apply` | None | Managed to Rust |
| Command | 2 | `Delete` | None | Managed to Rust |
| Command | 3 | `MoveEarlier` | None | Managed to Rust |
| Command | 4 | `MoveLater` | None | Managed to Rust |
| Command | 5 | `ToggleFavorite` | None | Managed to Rust |

## Model `TrayViewModel` (`4`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `CopyToClipboard` | `Boolean` | Rust and managed |
| Property | 2 | `OpenInEditor` | `Boolean` | Rust and managed |
| Property | 3 | `IncludeCursor` | `Boolean` | Rust and managed |
| Property | 4 | `ShowWidget` | `Boolean` | Rust and managed |
| Property | 5 | `RunAtStartup` | `Boolean` | Rust and managed |
| Property | 6 | `TimeLapse` | `Boolean` | Rust and managed |
| Property | 7 | `Schedule` | `Boolean` | Rust and managed |
| Property | 8 | `CanRepeat` | `Boolean` | Rust to managed |
| Collection | 1 | `RecentCaptures` | `String` | Rust to managed |
| Command | 1 | `Capture` | None | Managed to Rust |
| Command | 2 | `CaptureDelayed` | None | Managed to Rust |
| Command | 3 | `CaptureScreen` | None | Managed to Rust |
| Command | 4 | `RepeatLast` | None | Managed to Rust |
| Command | 5 | `CaptureWindow` | None | Managed to Rust |
| Command | 6 | `ShowEditor` | None | Managed to Rust |
| Command | 7 | `Scrolling` | None | Managed to Rust |
| Command | 8 | `ScrollingHorizontal` | None | Managed to Rust |
| Command | 9 | `ScrollingBoth` | None | Managed to Rust |
| Command | 10 | `Panoramic` | None | Managed to Rust |
| Command | 11 | `Freehand` | None | Managed to Rust |
| Command | 12 | `MultipleAreas` | None | Managed to Rust |
| Command | 13 | `FixedSize` | None | Managed to Rust |
| Command | 14 | `MenuCapture` | None | Managed to Rust |
| Command | 15 | `GrabText` | None | Managed to Rust |
| Command | 16 | `StepCapture` | None | Managed to Rust |
| Command | 17 | `FromClipboard` | None | Managed to Rust |
| Command | 18 | `FromFile` | None | Managed to Rust |
| Command | 19 | `RecordVideo` | None | Managed to Rust |
| Command | 20 | `OpenLibraryFolder` | None | Managed to Rust |
| Command | 21 | `Exit` | None | Managed to Rust |
| Command | 22 | `Preferences` | None | Managed to Rust |
| Command | 23 | `OpenRecentCapture` | None | Managed to Rust |

### Recent files `RecentCaptures`

Storage URIs published into collection `RecentCaptures`, capacity 10, activated by `OpenRecentCaptureCommand` with the chosen URI as its command parameter.

### Tray menu `Tray` (`1`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 117 | `TrayCapture` | Command | _Capture | `CaptureCommand` | - | - |
| 118 | `TrayCaptureDelayed` | Command | Capture after the _delay | `CaptureDelayedCommand` | - | - |
| 119 | `TrayCaptureScreen` | Command | Capture the _screen under the pointer | `CaptureScreenCommand` | - | - |
| 120 | `TrayRepeatLast` | Command | _Repeat the last area | `RepeatLastCommand` | - | enabled by `CanRepeat` |
| 121 | `TraySep1` | Separator | - | - | - | - |
| 122 | `TrayCaptureWindow` | Command | Capture _window… | `CaptureWindowCommand` | - | - |
| 123 | `TrayShowEditor` | Command | Open Jiffshot _Editor | `ShowEditorCommand` | - | - |
| 124 | `TrayRecent` | RecentFiles | Recent ca_ptures | - | - | recent files |
| 125 | `TraySep2` | Separator | - | - | - | - |
| 126 | `TrayModes` | Submenu | More capture _modes | - | - | - |
| 127 |     `TrayScrolling` | Command | _Scrolling | `ScrollingCommand` | - | - |
| 128 |     `TrayScrollingHorizontal` | Command | Scrolling, _horizontal | `ScrollingHorizontalCommand` | - | - |
| 129 |     `TrayScrollingBoth` | Command | Scrolling, _both directions | `ScrollingBothCommand` | - | - |
| 130 |     `TrayPanoramic` | Command | _Panoramic | `PanoramicCommand` | - | - |
| 131 |     `TrayFreehand` | Command | _Freehand | `FreehandCommand` | - | - |
| 132 |     `TrayMultipleAreas` | Command | _Multiple areas | `MultipleAreasCommand` | - | - |
| 133 |     `TrayFixedSize` | Command | Fixed si_ze | `FixedSizeCommand` | - | - |
| 134 |     `TrayMenuCapture` | Command | M_enu (3 second countdown) | `MenuCaptureCommand` | - | - |
| 135 |     `TrayGrabText` | Command | _Grab text | `GrabTextCommand` | - | - |
| 136 |     `TrayStepCapture` | Command | S_tep Capture | `StepCaptureCommand` | - | - |
| 137 |     `TraySep3` | Separator | - | - | - | - |
| 138 |     `TrayFromClipboard` | Command | From the _clipboard | `FromClipboardCommand` | - | - |
| 139 |     `TrayFromFile` | Command | From a _file… | `FromFileCommand` | - | - |
| 140 | `TrayRecordVideo` | Command | Record _video | `RecordVideoCommand` | - | - |
| 141 | `TraySep4` | Separator | - | - | - | - |
| 142 | `TrayTimeLapse` | Toggle | _Time-lapse | - | - | `TimeLapse` |
| 143 | `TraySchedule` | Toggle | Scheduled capture (_daily) | - | - | `Schedule` |
| 144 | `TraySep5` | Separator | - | - | - | - |
| 145 | `TrayClipboard` | Toggle | Copy to clip_board | - | - | `CopyToClipboard` |
| 146 | `TrayEditor` | Toggle | Open in the edit_or | - | - | `OpenInEditor` |
| 147 | `TrayCursor` | Toggle | Include the c_ursor | - | - | `IncludeCursor` |
| 148 | `TraySep6` | Separator | - | - | - | - |
| 149 | `TrayWidget` | Toggle | Show the Capture Wid_get | - | - | `ShowWidget` |
| 150 | `TrayStartup` | Toggle | Run when Windows _starts | - | - | `RunAtStartup` |
| 151 | `TrayOpenLibraryFolder` | Command | Open the _library folder | `OpenLibraryFolderCommand` | - | - |
| 152 | `TrayPreferences` | Command | _Preferences… | `PreferencesCommand` | - | - |
| 153 | `TraySep7` | Separator | - | - | - | - |
| 154 | `TrayExit` | Command | E_xit Jiffshot | `ExitCommand` | - | - |

## Model `CaptureViewModel` (`5`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `View` | `String` | Rust to managed |
| Property | 2 | `Menu` | `String` | Rust to managed |
| Property | 3 | `Panel` | `String` | Rust to managed |
| Property | 4 | `Problem` | `String` | Rust to managed |
| Property | 5 | `Action` | `String` | Rust and managed |
| Command | 1 | `Run` | `Action` | Managed to Rust |

## Views

| ID | Name | Model | Managed type | Binding path |
| ---: | --- | --- | --- | --- |
| 1 | `MainWindow` | `MainViewModel` | `Jiffshot.Editor.Views.MainWindow` | Generated CLR properties |
| 2 | `CaptureWindow` | `CaptureViewModel` | `Jiffshot.Editor.Views.CaptureWindow` | Generated CLR properties |
