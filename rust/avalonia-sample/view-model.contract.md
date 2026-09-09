# Generated Rust view-model contract

Schema version: `5`

## Enums

| ID | Name | Members |
| ---: | --- | --- |
| 1 | `Priority` | `Low` = 0, `Normal` = 1, `High` = 2 |

## Model `SampleViewModel` (`1`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Name` | `String` | Rust and managed |
| Property | 2 | `Count` | `Integer` | Rust to managed |
| Property | 3 | `NewItem` | `String` | Rust and managed |
| Property | 4 | `Status` | `String` | Rust to managed |
| Property | 5 | `Nickname` | `String`, nullable | Rust and managed |
| Property | 6 | `Priority` | Enum `Priority` | Rust and managed |
| Property | 7 | `Address` | Model `AddressViewModel`, nullable | Rust to managed |
| Property | 8 | `NewTaskTitle` | `String` | Rust and managed |
| Property | 9 | `SelectedTraceIndex` | `Integer` | Rust and managed |
| Property | 10 | `SelectedTraceKey` | `String` | Rust and managed |
| Property | 11 | `TraceSortDirection` | `String` | Rust to managed |
| Property | 12 | `FileStatus` | `String` | Rust to managed |
| Property | 13 | `DropStatus` | `String` | Rust to managed |
| Property | 14 | `ActivationStatus` | `String` | Rust to managed |
| Property | 15 | `LogWindowStatus` | `String` | Rust to managed |
| Property | 16 | `ShowTraceDetails` | `Boolean` | Rust and managed |
| Property | 17 | `ClipboardStatus` | `String` | Rust to managed |
| Collection | 1 | `Items` | `String` | Rust to managed |
| Collection | 2 | `Tasks` | Model `TaskItemViewModel` | Rust to managed |
| Collection | 3 | `TraceRows` | Model `TraceRowViewModel` | Rust to managed |
| Collection | 4 | `SelectedFiles` | `String` | Rust to managed |
| Collection | 5 | `LogWindow` | Model `TraceRowViewModel` (windowed: page 64, 8 live pages) | Rust to managed |
| Collection | 6 | `LogTree` | Model `LogNodeViewModel` (tree root) | Rust to managed |
| Collection | 7 | `RecentFiles` | `String` | Rust to managed |
| Collection | 8 | `CoreLoads` | `Double` | Rust to managed |
| Collection | 9 | `CoreTicks` | `Integer` | Rust to managed |
| Map | 1 | `SeverityCounts` | `String` to `Integer` | Rust to managed |
| Map | 2 | `SourceDetails` | `String` to Model `TraceEventViewModel` | Rust to managed |
| Command | 1 | `Increment` | None | Managed to Rust |
| Command | 2 | `Add` | `NewItem` | Managed to Rust |
| Async command | 3 | `Save` (result `SaveReportViewModel`, progress, cancellable) | None | Managed to Rust |
| Command | 4 | `ClearNickname` | None | Managed to Rust |
| Command | 5 | `ToggleAddress` | None | Managed to Rust |
| Command | 6 | `AddTask` | `NewTaskTitle` | Managed to Rust |
| Command | 7 | `RemoveFirstTask` | None | Managed to Rust |
| Command | 8 | `ShuffleTasks` | None | Managed to Rust |
| Command | 9 | `ClearTasks` | None | Managed to Rust |
| Command | 10 | `SortTraceRows` | None | Managed to Rust |
| Async command | 11 | `OpenFiles` | None | Managed to Rust |
| Async command | 12 | `OpenFolder` | None | Managed to Rust |
| Async command | 13 | `SaveExport` | None | Managed to Rust |
| Command | 14 | `RefreshLogWindow` | None | Managed to Rust |
| Command | 15 | `OpenRecentFile` | None | Managed to Rust |
| Async command | 16 | `CopySelectedRow` | None | Managed to Rust |
| Async command | 17 | `CutSelectedRow` | None | Managed to Rust |
| Async command | 18 | `PasteFromClipboard` | None | Managed to Rust |
| Async command | 19 | `ClearClipboard` | None | Managed to Rust |
| Command | 20 | `ExitApplication` | None | Managed to Rust |

### Tree `LogTree`

Node model `LogNodeViewModel`, children `Children`, header `Label`, has-children `HasChildren`.

### Recent files `RecentFiles`

Storage URIs published into collection `RecentFiles`, capacity 8, activated by `OpenRecentFileCommand` with the chosen URI as its command parameter.

### Application menu `Main` (`1`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `File` | Submenu | _File | - | - | - |
| 2 |     `Open` | Command | _Open files... | `OpenFilesCommand` | `Ctrl+O` | - |
| 3 |     `OpenFolder` | Command | Open f_older... | `OpenFolderCommand` | - | - |
| 4 |     `Recent` | RecentFiles | Recent _files | - | - | recent files |
| 5 |     `FileSeparator` | Separator | - | - | - | - |
| 6 |     `Exit` | Command | E_xit | `ExitApplicationCommand` | `Ctrl+Q` | - |
| 7 | `Edit` | Submenu | _Edit | - | - | - |
| 8 |     `Copy` | Command | _Copy row | `CopySelectedRowCommand` | `Ctrl+C` | - |
| 9 |     `Cut` | Command | Cu_t row | `CutSelectedRowCommand` | `Ctrl+X` | - |
| 10 |     `Paste` | Command | _Paste | `PasteFromClipboardCommand` | `Ctrl+V` | - |
| 11 |     `EditSeparator` | Separator | - | - | - | - |
| 12 |     `ClearClipboard` | Command | C_lear clipboard | `ClearClipboardCommand` | - | - |
| 13 | `View` | Submenu | _View | - | - | - |
| 14 |     `ShowDetails` | Toggle | Show trace _details | - | - | `ShowTraceDetails` |
| 15 |     `ViewSeparator` | Separator | - | - | - | - |
| 16 |     `PriorityLow` | Radio | Priority: low | - | - | `Priority` = `Low` |
| 17 |     `PriorityNormal` | Radio | Priority: normal | - | - | `Priority` = `Normal` |
| 18 |     `PriorityHigh` | Radio | Priority: high | - | - | `Priority` = `High` |

### Context menu `TraceRows` (`2`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `CopyRow` | Command | Copy row | `CopySelectedRowCommand` | `Ctrl+C` | - |
| 2 | `Separator` | Separator | - | - | - | - |
| 3 | `ShowDetails` | Toggle | Show trace details | - | - | `ShowTraceDetails` |
| 4 | `Recent` | RecentFiles | Recent files | - | - | recent files |

### Accelerators menu `Shortcuts` (`3`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `RefreshLogWindow` | Command | Refresh log window | `RefreshLogWindowCommand` | `Ctrl+R` | - |

### Table `TraceRows`

| ID | Name | Header | Row path | Width | Resize | Sort | Alignment |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | `Timestamp` | Timestamp | `Timestamp` | 150 | Yes | Yes | Left |
| 2 | `Severity` | Severity | `Severity` | 90 | Yes | Yes | Center |
| 3 | `Source` | Source | `Event.Source` | 120 | Yes | Yes | Left |
| 4 | `Message` | Message | `Message` | * | Yes | No | Left |
Selection: index `SelectedTraceIndex`, key `SelectedTraceKey`, row key `Event.Id`.
Sort: `SortTraceRows` command, initial column `Timestamp`, direction property `TraceSortDirection`.

## Model `AddressViewModel` (`2`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Street` | `String` | Rust and managed |
| Property | 2 | `City` | `String` | Rust and managed |

## Model `TaskItemViewModel` (`3`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Title` | `String` | Rust to managed |
| Property | 2 | `Done` | `Boolean` | Rust and managed |

## Model `TraceRowViewModel` (`4`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Timestamp` | `String` | Rust to managed |
| Property | 2 | `Severity` | `String` | Rust to managed |
| Property | 3 | `Message` | `String` | Rust to managed |
| Property | 4 | `Event` | Model `TraceEventViewModel`, nullable | Rust to managed |

Display projection (`ToString()`): `Message`.

## Model `TraceEventViewModel` (`5`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Id` | `String` | Rust to managed |
| Property | 2 | `Source` | `String` | Rust to managed |

## Model `LogNodeViewModel` (`6`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Label` | `String` | Rust to managed |
| Property | 2 | `Detail` | `String` | Rust to managed |
| Property | 3 | `HasChildren` | `Boolean` | Rust to managed |
| Collection | 1 | `Children` | Model `LogNodeViewModel` (recursive children) | Rust to managed |

## Model `SaveReportViewModel` (`7`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Destination` | `String` | Rust to managed |
| Property | 2 | `Bytes` | `Integer` | Rust to managed |
| Property | 3 | `Succeeded` | `Boolean` | Rust to managed |

## Value converters

| ID | Name | Value | Parameter | Result | ConvertBack | Used by |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `CountToLabel` | `Integer` | None | `String` | No | `RustVmWindow`, `RustDynamicVmWindow` |

## Views

| ID | Name | Model | Managed type | Binding path |
| ---: | --- | --- | --- | --- |
| 1 | `RustVmWindow` | `SampleViewModel` | `Avalonia.Rust.Sample.Views.RustVmWindow` | Generated CLR properties |
| 2 | `RustDynamicVmWindow` | `SampleViewModel` | `Avalonia.Rust.Sample.Views.RustDynamicVmWindow` | Dynamic Rust metadata |
