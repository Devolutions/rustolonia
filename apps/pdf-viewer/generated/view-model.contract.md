# Generated Rust view-model contract

Schema version: `5`

## Model `MainViewModel` (`1`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Title` | `String` | Rust to managed |
| Property | 2 | `Status` | `String` | Rust to managed |
| Property | 3 | `DocumentName` | `String` | Rust to managed |
| Property | 4 | `PageImage` | `String` | Rust to managed |
| Property | 5 | `PageText` | `String` | Rust to managed |
| Property | 6 | `PageLabel` | `String` | Rust to managed |
| Property | 7 | `CanGoPrevious` | `Boolean` | Rust to managed |
| Property | 8 | `CanGoNext` | `Boolean` | Rust to managed |
| Property | 9 | `IsLoading` | `Boolean` | Rust to managed |
| Property | 10 | `SearchText` | `String` | Rust and managed |
| Property | 11 | `SearchStatus` | `String` | Rust to managed |
| Property | 12 | `SearchMatchLabel` | `String` | Rust to managed |
| Property | 13 | `CanGoPreviousMatch` | `Boolean` | Rust to managed |
| Property | 14 | `CanGoNextMatch` | `Boolean` | Rust to managed |
| Property | 15 | `OutlineVisible` | `Boolean` | Rust and managed |
| Property | 16 | `HasOutline` | `Boolean` | Rust to managed |
| Property | 17 | `OutlineStatus` | `String` | Rust to managed |
| Collection | 1 | `RecentFiles` | `String` | Rust to managed |
| Collection | 2 | `Outline` | Model `OutlineItemViewModel` (tree root) | Rust to managed |
| Async command | 1 | `OpenFile` | None | Managed to Rust |
| Command | 2 | `PreviousPage` | None | Managed to Rust |
| Command | 3 | `NextPage` | None | Managed to Rust |
| Command | 4 | `OpenRecentFile` | None | Managed to Rust |
| Command | 5 | `Exit` | None | Managed to Rust |
| Async command | 6 | `Search` | None | Managed to Rust |
| Command | 7 | `PreviousMatch` | None | Managed to Rust |
| Command | 8 | `NextMatch` | None | Managed to Rust |

### Tree `Outline`

Node model `OutlineItemViewModel`, children `Children`, header `Title`, has-children `HasChildren`.

### Recent files `RecentFiles`

Storage URIs published into collection `RecentFiles`, capacity 5, activated by `OpenRecentFileCommand` with the chosen URI as its command parameter.

### Application menu `Main` (`1`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `File` | Submenu | _File | - | - | - |
| 2 |     `OpenFile` | Command | _Open PDF... | `OpenFileCommand` | `Ctrl+O` | - |
| 3 |     `Recent` | RecentFiles | Recent _files | - | - | recent files |
| 4 |     `FileSeparator` | Separator | - | - | - | - |
| 5 |     `Exit` | Command | E_xit | `ExitCommand` | `Ctrl+Q` | - |
| 6 | `Page` | Submenu | _Page | - | - | - |
| 7 |     `PreviousPage` | Command | _Previous page | `PreviousPageCommand` | `Alt+Left` | - |
| 8 |     `NextPage` | Command | _Next page | `NextPageCommand` | `Alt+Right` | - |
| 9 | `View` | Submenu | _View | - | - | - |
| 10 |     `Bookmarks` | Toggle | _Bookmarks | - | `Ctrl+B` | `OutlineVisible` |

## Model `OutlineItemViewModel` (`2`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Title` | `String` | Rust to managed |
| Property | 2 | `PageLabel` | `String` | Rust to managed |
| Property | 3 | `HasChildren` | `Boolean` | Rust to managed |
| Collection | 1 | `Children` | Model `OutlineItemViewModel` (recursive children) | Rust to managed |
| Command | 1 | `GoTo` | None | Managed to Rust |

## Views

| ID | Name | Model | Managed type | Binding path |
| ---: | --- | --- | --- | --- |
| 1 | `MainWindow` | `MainViewModel` | `PdfViewer.Presentation.Views.MainWindow` | Generated CLR properties |
