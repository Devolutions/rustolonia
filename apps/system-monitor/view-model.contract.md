# Generated Rust view-model contract

Schema version: `5`

## Model `MainViewModel` (`1`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `SearchText` | `String` | Rust and managed |
| Property | 2 | `IsFrozen` | `Boolean` | Rust and managed |
| Property | 3 | `IsDarkTheme` | `Boolean` | Rust and managed |
| Property | 4 | `CpuPercent` | `Double` | Rust to managed |
| Property | 5 | `MemoryPercent` | `Double` | Rust to managed |
| Property | 6 | `MemoryUsed` | `Integer` | Rust to managed |
| Property | 7 | `MemoryTotal` | `Integer` | Rust to managed |
| Property | 8 | `ProcessCount` | `Integer` | Rust to managed |
| Property | 9 | `Status` | `String` | Rust to managed |
| Property | 10 | `SelectedPid` | `Integer` | Rust and managed |
| Property | 11 | `SelectedIndex` | `Integer` | Rust and managed |
| Property | 12 | `SelectedKey` | `String` | Rust and managed |
| Property | 13 | `SortDirection` | `String` | Rust to managed |
| Property | 14 | `ErrorMessage` | `String`, nullable | Rust to managed |
| Property | 15 | `CpuSummary` | `String` | Rust to managed |
| Property | 16 | `MemorySummary` | `String` | Rust to managed |
| Property | 17 | `NetworkSummary` | `String` | Rust to managed |
| Property | 18 | `StorageSummary` | `String` | Rust to managed |
| Property | 19 | `StoragePercent` | `Double` | Rust to managed |
| Property | 20 | `SystemSummary` | `String` | Rust to managed |
| Property | 21 | `FreezeActionLabel` | `String` | Rust to managed |
| Property | 22 | `ThemeActionLabel` | `String` | Rust to managed |
| Property | 23 | `ShowKillConfirm` | `Boolean` | Rust to managed |
| Property | 24 | `KillConfirmMessage` | `String` | Rust to managed |
| Property | 25 | `IsKilling` | `Boolean` | Rust to managed |
| Property | 26 | `ShowDetails` | `Boolean` | Rust to managed |
| Property | 27 | `DetailTitle` | `String` | Rust to managed |
| Property | 28 | `DetailBody` | `String` | Rust to managed |
| Property | 29 | `ShowParentEnabled` | `Boolean` | Rust to managed |
| Property | 30 | `CpuFilterEnabled` | `Boolean` | Rust and managed |
| Property | 31 | `CpuFilterOperator` | `String` | Rust and managed |
| Property | 32 | `CpuFilterValue` | `Integer` | Rust and managed |
| Property | 33 | `RamFilterEnabled` | `Boolean` | Rust and managed |
| Property | 34 | `RamFilterOperator` | `String` | Rust and managed |
| Property | 35 | `RamFilterValue` | `Integer` | Rust and managed |
| Property | 36 | `RuntimeFilterEnabled` | `Boolean` | Rust and managed |
| Property | 37 | `RuntimeFilterOperator` | `String` | Rust and managed |
| Property | 38 | `RuntimeFilterValue` | `Integer` | Rust and managed |
| Property | 39 | `StatusFilter` | `String` | Rust and managed |
| Property | 40 | `ProcessCountLabel` | `String` | Rust to managed |
| Property | 41 | `MemoryFree` | `Integer` | Rust to managed |
| Property | 42 | `NetworkRx` | `Integer` | Rust to managed |
| Property | 43 | `NetworkTx` | `Integer` | Rust to managed |
| Property | 44 | `StorageUsed` | `Integer` | Rust to managed |
| Property | 45 | `StorageTotal` | `Integer` | Rust to managed |
| Property | 46 | `StorageFree` | `Integer` | Rust to managed |
| Property | 47 | `RefreshRateMs` | `Integer` | Rust and managed |
| Property | 48 | `RefreshRateLabel` | `String` | Rust to managed |
| Property | 49 | `MemoryUsedLabel` | `String` | Rust to managed |
| Property | 50 | `MemoryTotalLabel` | `String` | Rust to managed |
| Property | 51 | `MemoryFreeLabel` | `String` | Rust to managed |
| Property | 52 | `NetworkRxLabel` | `String` | Rust to managed |
| Property | 53 | `NetworkTxLabel` | `String` | Rust to managed |
| Property | 54 | `StorageUsedLabel` | `String` | Rust to managed |
| Property | 55 | `StorageTotalLabel` | `String` | Rust to managed |
| Property | 56 | `StorageFreeLabel` | `String` | Rust to managed |
| Property | 57 | `UptimeLabel` | `String` | Rust to managed |
| Property | 58 | `LoadOneLabel` | `String` | Rust to managed |
| Property | 59 | `LoadFiveLabel` | `String` | Rust to managed |
| Property | 60 | `LoadFifteenLabel` | `String` | Rust to managed |
| Property | 61 | `ShowFilters` | `Boolean` | Rust and managed |
| Property | 62 | `ShowColumns` | `Boolean` | Rust and managed |
| Property | 63 | `ShowSearchHelp` | `Boolean` | Rust and managed |
| Property | 64 | `ShowPid` | `Boolean` | Rust and managed |
| Property | 65 | `ShowStatus` | `Boolean` | Rust and managed |
| Property | 66 | `ShowUser` | `Boolean` | Rust and managed |
| Property | 67 | `ShowCpu` | `Boolean` | Rust and managed |
| Property | 68 | `ShowRam` | `Boolean` | Rust and managed |
| Property | 69 | `ShowVirt` | `Boolean` | Rust and managed |
| Property | 70 | `ShowDisk` | `Boolean` | Rust and managed |
| Property | 71 | `ShowRunTime` | `Boolean` | Rust and managed |
| Property | 72 | `ShowCommand` | `Boolean` | Rust and managed |
| Property | 73 | `ShowPpid` | `Boolean` | Rust and managed |
| Property | 74 | `ShowRoot` | `Boolean` | Rust and managed |
| Property | 75 | `ShowEnviron` | `Boolean` | Rust and managed |
| Property | 76 | `ShowSession` | `Boolean` | Rust and managed |
| Property | 77 | `ShowStartTime` | `Boolean` | Rust and managed |
| Property | 78 | `DetailName` | `String` | Rust to managed |
| Property | 79 | `DetailPidLabel` | `String` | Rust to managed |
| Property | 80 | `DetailPpidLabel` | `String` | Rust to managed |
| Property | 81 | `DetailUser` | `String` | Rust to managed |
| Property | 82 | `DetailStatus` | `String` | Rust to managed |
| Property | 83 | `DetailSessionLabel` | `String` | Rust to managed |
| Property | 84 | `DetailCpuLabel` | `String` | Rust to managed |
| Property | 85 | `DetailMemoryLabel` | `String` | Rust to managed |
| Property | 86 | `DetailVirtLabel` | `String` | Rust to managed |
| Property | 87 | `DetailDiskLabel` | `String` | Rust to managed |
| Property | 88 | `DetailCommand` | `String` | Rust to managed |
| Property | 89 | `DetailRoot` | `String` | Rust to managed |
| Property | 90 | `DetailChildren` | `String` | Rust to managed |
| Property | 91 | `CpuPercentLabel` | `String` | Rust to managed |
| Property | 92 | `MemoryPercentLabel` | `String` | Rust to managed |
| Property | 93 | `StoragePercentLabel` | `String` | Rust to managed |
| Collection | 1 | `Processes` | Model `ProcessRowViewModel` | Rust to managed |
| Collection | 2 | `CpuCores` | Model `CpuCoreViewModel` | Rust to managed |
| Command | 1 | `Refresh` | None | Managed to Rust |
| Async command | 2 | `Kill` | `SelectedKey` | Managed to Rust |
| Command | 3 | `Pin` | `SelectedKey` | Managed to Rust |
| Command | 4 | `ShowDetails` | `SelectedKey` | Managed to Rust |
| Command | 5 | `ToggleTheme` | None | Managed to Rust |
| Command | 6 | `ToggleFreeze` | None | Managed to Rust |
| Command | 7 | `SortProcesses` | None | Managed to Rust |
| Command | 8 | `ConfirmKill` | None | Managed to Rust |
| Command | 9 | `CancelKill` | None | Managed to Rust |
| Command | 10 | `CloseDetails` | None | Managed to Rust |
| Command | 11 | `ShowParentDetails` | None | Managed to Rust |
| Async command | 12 | `CopySelectedRow` | None | Managed to Rust |
| Command | 13 | `ExitApplication` | None | Managed to Rust |
| Command | 14 | `ClearSearch` | None | Managed to Rust |
| Command | 15 | `ToggleCpuOperator` | None | Managed to Rust |
| Command | 16 | `ToggleRamOperator` | None | Managed to Rust |
| Command | 17 | `ToggleRuntimeOperator` | None | Managed to Rust |
| Command | 18 | `ToggleFilters` | None | Managed to Rust |
| Command | 19 | `ToggleColumns` | None | Managed to Rust |
| Command | 20 | `ToggleSearchHelp` | None | Managed to Rust |
| Command | 21 | `CycleRefreshRate` | None | Managed to Rust |
| Command | 22 | `CloseOverlays` | None | Managed to Rust |

### Application menu `Main` (`1`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `File` | Submenu | _File | - | - | - |
| 2 |     `Refresh` | Command | _Refresh | `RefreshCommand` | `F5` | - |
| 3 |     `Freeze` | Toggle | F_reeze | - | `Ctrl+F` | `IsFrozen` |
| 4 |     `FileSeparator` | Separator | - | - | - | - |
| 5 |     `Exit` | Command | E_xit | `ExitApplicationCommand` | `Ctrl+Q` | - |
| 6 | `View` | Submenu | _View | - | - | - |
| 7 |     `DarkTheme` | Toggle | _Dark theme | - | `Ctrl+T` | `IsDarkTheme` |
| 14 |     `Filters` | Toggle | _Filters | - | - | `ShowFilters` |
| 15 |     `Columns` | Toggle | _Columns | - | - | `ShowColumns` |
| 16 |     `SearchHelp` | Command | Search _help | `ToggleSearchHelpCommand` | - | - |
| 8 | `Process` | Submenu | _Process | - | - | - |
| 9 |     `PinSelected` | Command | _Pin / unpin | `PinCommand` | `Ctrl+P` | - |
| 10 |     `Details` | Command | Show _details | `ShowDetailsCommand` | `Ctrl+I` | - |
| 11 |     `EndProcess` | Command | _End process | `KillCommand` | `Delete` | - |
| 12 |     `ProcessSeparator` | Separator | - | - | - | - |
| 13 |     `Copy` | Command | _Copy row | `CopySelectedRowCommand` | `Ctrl+C` | - |

### Context menu `Processes` (`2`)

| ID | Item | Kind | Header | Command | Gesture | Bound member |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `PinRow` | Command | Pin / unpin | `PinCommand` | - | - |
| 2 | `DetailsRow` | Command | Show details | `ShowDetailsCommand` | - | - |
| 3 | `KillRow` | Command | End process | `KillCommand` | - | - |
| 4 | `ContextSeparator` | Separator | - | - | - | - |
| 5 | `CopyRow` | Command | Copy row | `CopySelectedRowCommand` | - | - |

### Table `Processes`

| ID | Name | Header | Row path | Width | Resize | Sort | Alignment |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | `Name` | Process Name | `Name` | * | Yes | Yes | Left |
| 2 | `Pid` | PID | `Pid` | 80 | Yes | Yes | Right |
| 3 | `Status` | Status | `Status` | 100 | Yes | Yes | Left |
| 4 | `User` | User | `User` | 120 | Yes | Yes | Left |
| 5 | `CpuUsage` | CPU % | `CpuUsage` | 90 | Yes | Yes | Right |
| 6 | `MemoryUsage` | RAM | `MemoryUsage` | 100 | Yes | Yes | Right |
| 7 | `VirtualMemory` | VIRT | `VirtualMemory` | 100 | Yes | Yes | Right |
| 8 | `DiskIo` | Disk I/O (R/W) | `DiskIo` | 140 | Yes | Yes | Right |
| 9 | `RunTime` | Run Time | `RunTimeLabel` | 120 | Yes | Yes | Left |
| 10 | `Command` | Command | `Command` | 240 | Yes | Yes | Left |
| 11 | `Ppid` | Parent PID | `Ppid` | 90 | Yes | Yes | Right |
| 12 | `Root` | Root | `Root` | 160 | Yes | Yes | Left |
| 13 | `Environ` | Environment Variables | `Environ` | 200 | Yes | No | Left |
| 14 | `SessionId` | Session ID | `SessionId` | 100 | Yes | Yes | Right |
| 15 | `StartTime` | Start Time | `StartTimeLabel` | 160 | Yes | Yes | Left |
Selection: index `SelectedIndex`, key `SelectedKey`, row key `Key`.
Sort: `SortProcesses` command, initial column `CpuUsage`, direction property `SortDirection`.

## Model `ProcessRowViewModel` (`2`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Name` | `String` | Rust to managed |
| Property | 2 | `Pid` | `Integer` | Rust to managed |
| Property | 3 | `Ppid` | `Integer` | Rust to managed |
| Property | 4 | `Status` | `String` | Rust to managed |
| Property | 5 | `User` | `String` | Rust to managed |
| Property | 6 | `CpuUsage` | `Double` | Rust to managed |
| Property | 7 | `MemoryUsage` | `Integer` | Rust to managed |
| Property | 8 | `VirtualMemory` | `Integer` | Rust to managed |
| Property | 9 | `DiskRead` | `Integer` | Rust to managed |
| Property | 10 | `DiskWrite` | `Integer` | Rust to managed |
| Property | 11 | `Command` | `String` | Rust to managed |
| Property | 12 | `Root` | `String` | Rust to managed |
| Property | 13 | `SessionId` | `Integer` | Rust to managed |
| Property | 14 | `StartTime` | `Integer` | Rust to managed |
| Property | 15 | `RunTime` | `Integer` | Rust to managed |
| Property | 16 | `IsPinned` | `Boolean` | Rust to managed |
| Property | 17 | `Key` | `String` | Rust to managed |
| Property | 18 | `Environ` | `String` | Rust to managed |
| Property | 19 | `DiskIo` | `String` | Rust to managed |
| Property | 20 | `RunTimeLabel` | `String` | Rust to managed |
| Property | 21 | `MemoryLabel` | `String` | Rust to managed |
| Property | 22 | `VirtualMemoryLabel` | `String` | Rust to managed |
| Property | 23 | `CpuLabel` | `String` | Rust to managed |
| Property | 24 | `PinLabel` | `String` | Rust to managed |
| Property | 25 | `IsHighUsage` | `Boolean` | Rust to managed |
| Property | 26 | `StartTimeLabel` | `String` | Rust to managed |
| Property | 27 | `SessionLabel` | `String` | Rust to managed |
| Property | 28 | `IconPng` | `String` | Rust to managed |

Display projection (`ToString()`): `Name`.

## Model `CpuCoreViewModel` (`3`)

| Kind | ID | Name | Type | Direction |
| --- | ---: | --- | --- | --- |
| Property | 1 | `Label` | `String` | Rust to managed |
| Property | 2 | `Usage` | `Double` | Rust to managed |

Display projection (`ToString()`): `Label`.

## Views

| ID | Name | Model | Managed type | Binding path |
| ---: | --- | --- | --- | --- |
| 1 | `MainWindow` | `MainViewModel` | `NeoHtop.Presentation.Views.MainWindow` | Generated CLR properties |
