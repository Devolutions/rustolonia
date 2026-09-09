# Generated Rust view-model contract

Schema version: `5`

## Model `MainViewModel` (`1`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Title` | `String` | Rust to managed |
| Property | 2 | `Status` | `String` | Rust to managed |
| Property | 3 | `FileCountLabel` | `String` | Rust to managed |
| Property | 4 | `ExpectedHash` | `String` | Rust and managed |
| Property | 5 | `CompareStatus` | `String` | Rust to managed |
| Property | 6 | `IncludeMd5` | `Boolean` | Rust and managed |
| Property | 7 | `IncludeSha1` | `Boolean` | Rust and managed |
| Property | 8 | `IncludeSha256` | `Boolean` | Rust and managed |
| Property | 9 | `IncludeSha512` | `Boolean` | Rust and managed |
| Property | 10 | `IncludeBlake3` | `Boolean` | Rust and managed |
| Property | 11 | `IsBusy` | `Boolean` | Rust to managed |
| Property | 12 | `CanClear` | `Boolean` | Rust to managed |
| Property | 13 | `CanCopyReport` | `Boolean` | Rust to managed |
| Property | 14 | `PrimaryDigest` | `String` | Rust to managed |
| Property | 15 | `EmptyVisible` | `Boolean` | Rust to managed |
| Collection | 1 | `RecentFiles` | `String` | Rust to managed |
| Collection | 2 | `Files` | Model `FileRowViewModel` | Rust to managed |
| Async command | 1 | `OpenFiles` | None | Managed to Rust |
| Command | 2 | `ClearFiles` | None | Managed to Rust |
| Async command | 3 | `CopyReport` | None | Managed to Rust |
| Command | 4 | `OpenRecentFile` | None | Managed to Rust |
| Command | 5 | `Exit` | None | Managed to Rust |

### Recent files `RecentFiles`

Storage URIs published into collection `RecentFiles`, capacity 8, activated by `OpenRecentFileCommand` with the chosen URI as its command parameter.

### Application menu `Main` (`1`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `File` | Submenu | _File | - | - | - |
| 2 |     `OpenFiles` | Command | _Open files... | `OpenFilesCommand` | `Ctrl+O` | - |
| 3 |     `Recent` | RecentFiles | Recent _files | - | - | recent files |
| 4 |     `CopyReport` | Command | _Copy report | `CopyReportCommand` | `Ctrl+Shift+C` | - |
| 5 |     `ClearFiles` | Command | C_lear list | `ClearFilesCommand` | `Ctrl+Shift+N` | - |
| 6 |     `FileSeparator` | Separator | - | - | - | - |
| 7 |     `Exit` | Command | E_xit | `ExitCommand` | `Ctrl+Q` | - |
| 8 | `Hash` | Submenu | _Hash | - | - | - |
| 9 |     `IncludeMd5` | Toggle | _MD5 | - | - | `IncludeMd5` |
| 10 |     `IncludeSha1` | Toggle | SHA-_1 | - | - | `IncludeSha1` |
| 11 |     `IncludeSha256` | Toggle | SHA-_256 | - | - | `IncludeSha256` |
| 12 |     `IncludeSha512` | Toggle | SHA-_512 | - | - | `IncludeSha512` |
| 13 |     `IncludeBlake3` | Toggle | _BLAKE3 | - | - | `IncludeBlake3` |

## Model `FileRowViewModel` (`2`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Name` | `String` | Rust to managed |
| Property | 2 | `FilePath` | `String` | Rust to managed |
| Property | 3 | `SizeLabel` | `String` | Rust to managed |
| Property | 4 | `Status` | `String` | Rust to managed |
| Property | 5 | `MatchLabel` | `String` | Rust to managed |
| Property | 6 | `Md5` | `String` | Rust to managed |
| Property | 7 | `Sha1` | `String` | Rust to managed |
| Property | 8 | `Sha256` | `String` | Rust to managed |
| Property | 9 | `Sha512` | `String` | Rust to managed |
| Property | 10 | `Blake3` | `String` | Rust to managed |
| Property | 11 | `ShowMd5` | `Boolean` | Rust to managed |
| Property | 12 | `ShowSha1` | `Boolean` | Rust to managed |
| Property | 13 | `ShowSha256` | `Boolean` | Rust to managed |
| Property | 14 | `ShowSha512` | `Boolean` | Rust to managed |
| Property | 15 | `ShowBlake3` | `Boolean` | Rust to managed |
| Property | 16 | `CanCopySha256` | `Boolean` | Rust to managed |
| Async command | 1 | `CopySha256` | None | Managed to Rust |
| Async command | 2 | `CopyRow` | None | Managed to Rust |
| Command | 3 | `Remove` | None | Managed to Rust |

## Views

| ID | Name | Model | Managed type | Binding path |
| ---: | --- | --- | --- | --- |
| 1 | `MainWindow` | `MainViewModel` | `HashCalculator.Presentation.Views.MainWindow` | Generated CLR properties |
