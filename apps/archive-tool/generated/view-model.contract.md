# Generated Rust view-model contract

Schema version: `5`

## Model `MainViewModel` (`1`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Title` | `String` | Rust to managed |
| Property | 2 | `Status` | `String` | Rust to managed |
| Property | 3 | `ArchiveName` | `String` | Rust to managed |
| Property | 4 | `ArchiveKind` | `String` | Rust to managed |
| Property | 5 | `EntryCountLabel` | `String` | Rust to managed |
| Property | 6 | `FilterText` | `String` | Rust and managed |
| Property | 7 | `SelectedIndex` | `Integer` | Rust and managed |
| Property | 8 | `SelectedKey` | `String` | Rust and managed |
| Property | 9 | `SortDirection` | `String` | Rust to managed |
| Property | 10 | `IsLoading` | `Boolean` | Rust to managed |
| Property | 11 | `CanExtractSelected` | `Boolean` | Rust to managed |
| Property | 12 | `CanExtractAll` | `Boolean` | Rust to managed |
| Property | 13 | `SelectedCountLabel` | `String` | Rust to managed |
| Property | 14 | `CurrentPath` | `String` | Rust to managed |
| Property | 15 | `CanGoUp` | `Boolean` | Rust to managed |
| Property | 16 | `TotalsLabel` | `String` | Rust to managed |
| Property | 17 | `OpenAfterExtract` | `Boolean` | Rust and managed |
| Property | 18 | `CanTest` | `Boolean` | Rust to managed |
| Property | 19 | `LeftPath` | `String` | Rust to managed |
| Property | 20 | `RightPath` | `String` | Rust to managed |
| Property | 21 | `LeftKind` | `String` | Rust to managed |
| Property | 22 | `RightKind` | `String` | Rust to managed |
| Property | 23 | `LeftCanGoUp` | `Boolean` | Rust to managed |
| Property | 24 | `RightCanGoUp` | `Boolean` | Rust to managed |
| Property | 25 | `LeftActive` | `Boolean` | Rust to managed |
| Property | 26 | `RightActive` | `Boolean` | Rust to managed |
| Property | 27 | `LeftSelectedIndex` | `Integer` | Rust and managed |
| Property | 28 | `LeftSelectedKey` | `String` | Rust and managed |
| Property | 29 | `RightSelectedIndex` | `Integer` | Rust and managed |
| Property | 30 | `RightSelectedKey` | `String` | Rust and managed |
| Property | 31 | `CanDelete` | `Boolean` | Rust to managed |
| Property | 32 | `CanCopyToOther` | `Boolean` | Rust to managed |
| Collection | 1 | `RecentFiles` | `String` | Rust to managed |
| Collection | 2 | `LeftEntries` | Model `EntryRowViewModel` | Rust to managed |
| Collection | 3 | `RightEntries` | Model `EntryRowViewModel` | Rust to managed |
| Async command | 1 | `OpenFile` | None | Managed to Rust |
| Async command | 2 | `ExtractSelected` | None | Managed to Rust |
| Async command | 3 | `ExtractAll` | None | Managed to Rust |
| Command | 4 | `SelectAll` | None | Managed to Rust |
| Command | 5 | `ClearSelection` | None | Managed to Rust |
| Command | 6 | `SortEntries` | None | Managed to Rust |
| Command | 7 | `OpenRecentFile` | None | Managed to Rust |
| Command | 8 | `Exit` | None | Managed to Rust |
| Command | 9 | `GoUp` | None | Managed to Rust |
| Async command | 10 | `OpenItem` | None | Managed to Rust |
| Async command | 11 | `ExtractHere` | None | Managed to Rust |
| Async command | 12 | `TestArchive` | None | Managed to Rust |
| Async command | 13 | `NewArchive` | None | Managed to Rust |
| Command | 14 | `InvertSelection` | None | Managed to Rust |
| Async command | 15 | `CopyPath` | None | Managed to Rust |
| Async command | 16 | `OpenFolder` | None | Managed to Rust |
| Command | 17 | `ActivateLeft` | None | Managed to Rust |
| Command | 18 | `ActivateRight` | None | Managed to Rust |
| Async command | 19 | `DeleteSelected` | None | Managed to Rust |
| Async command | 20 | `CopyToOther` | None | Managed to Rust |
| Command | 21 | `OpenComputer` | None | Managed to Rust |
| Command | 22 | `OpenHome` | None | Managed to Rust |

### Recent files `RecentFiles`

Storage URIs published into collection `RecentFiles`, capacity 5, activated by `OpenRecentFileCommand` with the chosen URI as its command parameter.

### Application menu `Main` (`1`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `File` | Submenu | _File | - | - | - |
| 2 |     `OpenFile` | Command | _Open archive... | `OpenFileCommand` | `Ctrl+O` | - |
| 23 |     `OpenFolder` | Command | Open _folder... | `OpenFolderCommand` | `Ctrl+Shift+O` | - |
| 3 |     `NewArchive` | Command | _New archive... | `NewArchiveCommand` | `Ctrl+N` | - |
| 4 |     `Recent` | RecentFiles | Recent _files | - | - | recent files |
| 5 |     `FileSeparator` | Separator | - | - | - | - |
| 6 |     `ExtractSelected` | Command | _Extract selected... | `ExtractSelectedCommand` | `Ctrl+E` | - |
| 7 |     `ExtractHere` | Command | Extract _here | `ExtractHereCommand` | `Ctrl+Shift+H` | - |
| 8 |     `ExtractAll` | Command | Extract _all... | `ExtractAllCommand` | `Ctrl+Shift+E` | - |
| 9 |     `TestArchive` | Command | _Test archive | `TestArchiveCommand` | `Ctrl+T` | - |
| 10 |     `FileSeparator2` | Separator | - | - | - | - |
| 11 |     `Exit` | Command | E_xit | `ExitCommand` | `Ctrl+Q` | - |
| 12 | `Edit` | Submenu | _Edit | - | - | - |
| 13 |     `OpenItem` | Command | O_pen | `OpenItemCommand` | `Enter` | - |
| 14 |     `GoUp` | Command | Go _up | `GoUpCommand` | `Alt+Up` | - |
| 15 |     `EditSeparator` | Separator | - | - | - | - |
| 16 |     `SelectAll` | Command | Select _all | `SelectAllCommand` | `Ctrl+A` | - |
| 17 |     `InvertSelection` | Command | _Invert selection | `InvertSelectionCommand` | - | - |
| 18 |     `ClearSelection` | Command | _Clear selection | `ClearSelectionCommand` | - | - |
| 19 |     `CopyPath` | Command | Copy _path | `CopyPathCommand` | `Ctrl+C` | - |
| 24 |     `CopyToOther` | Command | Copy to other pane | `CopyToOtherCommand` | `F5` | - |
| 25 |     `DeleteSelected` | Command | _Delete | `DeleteSelectedCommand` | `Delete` | - |
| 20 | `View` | Submenu | _View | - | - | - |
| 21 |     `OpenAfterExtract` | Toggle | Open folder after e_xtract | - | - | `OpenAfterExtract` |
| 26 |     `OpenComputer` | Command | _Computer | `OpenComputerCommand` | - | - |
| 27 |     `OpenHome` | Command | _Home | `OpenHomeCommand` | - | - |

### Context menu `LeftEntries` (`2`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `OpenRow` | Command | Open | `OpenItemCommand` | - | - |
| 2 | `ExtractRow` | Command | Extract... | `ExtractSelectedCommand` | - | - |
| 3 | `ExtractHereRow` | Command | Extract here | `ExtractHereCommand` | - | - |
| 4 | `ContextSeparator` | Separator | - | - | - | - |
| 5 | `CopyRow` | Command | Copy path | `CopyPathCommand` | - | - |

### Context menu `RightEntries` (`3`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `OpenRow` | Command | Open | `OpenItemCommand` | - | - |
| 2 | `ExtractRow` | Command | Extract... | `ExtractSelectedCommand` | - | - |
| 3 | `CopyToOtherRow` | Command | Copy to other pane | `CopyToOtherCommand` | - | - |
| 4 | `ContextSeparator` | Separator | - | - | - | - |
| 5 | `CopyRow` | Command | Copy path | `CopyPathCommand` | - | - |

### Table `LeftEntries`

| ID | Name | Header | Row path | Width | Resize | Sort | Alignment |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | `Name` | Name | `Name` | * | Yes | Yes | Left |
| 2 | `Kind` | Kind | `Kind` | 80 | Yes | Yes | Left |
| 3 | `Size` | Size | `SizeLabel` | 90 | Yes | Yes | Right |
| 4 | `Packed` | Packed | `PackedLabel` | 90 | Yes | Yes | Right |
| 5 | `Ratio` | Ratio | `RatioLabel` | 70 | Yes | Yes | Right |
| 6 | `Modified` | Modified | `Modified` | 140 | Yes | Yes | Left |
| 7 | `Crc` | CRC | `CrcLabel` | 90 | Yes | Yes | Left |
Selection: index `LeftSelectedIndex`, key `LeftSelectedKey`, row key `Key`.
Sort: `SortEntries` command, initial column `Name`, direction property `SortDirection`.

### Table `RightEntries`

| ID | Name | Header | Row path | Width | Resize | Sort | Alignment |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | `Name` | Name | `Name` | * | Yes | Yes | Left |
| 2 | `Kind` | Kind | `Kind` | 80 | Yes | Yes | Left |
| 3 | `Size` | Size | `SizeLabel` | 90 | Yes | Yes | Right |
| 4 | `Packed` | Packed | `PackedLabel` | 90 | Yes | Yes | Right |
| 5 | `Modified` | Modified | `Modified` | 140 | Yes | Yes | Left |
Selection: index `RightSelectedIndex`, key `RightSelectedKey`, row key `Key`.
Sort: `SortEntries` command, initial column `Name`, direction property `SortDirection`.

## Model `EntryRowViewModel` (`2`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Key` | `String` | Rust to managed |
| Property | 2 | `Path` | `String` | Rust to managed |
| Property | 3 | `Name` | `String` | Rust to managed |
| Property | 4 | `Kind` | `String` | Rust to managed |
| Property | 5 | `SizeLabel` | `String` | Rust to managed |
| Property | 6 | `PackedLabel` | `String` | Rust to managed |
| Property | 7 | `Modified` | `String` | Rust to managed |
| Property | 8 | `IsSelected` | `Boolean` | Rust and managed |
| Property | 9 | `CrcLabel` | `String` | Rust to managed |
| Property | 10 | `RatioLabel` | `String` | Rust to managed |
| Command | 1 | `Open` | None | Managed to Rust |

## Views

| ID | Name | Model | Managed type | Binding path |
| ---: | --- | --- | --- | --- |
| 1 | `MainWindow` | `MainViewModel` | `ArchiveTool.Presentation.Views.MainWindow` | Generated CLR properties |
