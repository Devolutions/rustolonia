# Avalonia for Rust

This experimental workspace provides an idiomatic Rust API over an Avalonia
.NET 10 NativeAOT shared library. Both the managed nano-COM wrappers and Rust
bindings are generated from `projection.ir.json`; examples are consumers, not
binding specifications.

The native ownership contract and the MicroCom-versus-handle-table decision
are documented in [OWNERSHIP.md](OWNERSHIP.md).
Geometry value types (`Thickness`, `CornerRadius`, `Size`, `Point`, `Rect`,
`Color`) cross the ABI as blittable structs, an `IBrush` crosses as a read-only
solid-colour `IAvnBrush`, and the projected layout members (`Margin`, `Padding`,
alignments, min/max sizes, `IsVisible`, `Opacity`, `Name`, `CanResize`,
`WindowState`), chrome members (`Background`, `BorderBrush`,
`BorderThickness`, `CornerRadius`, `Foreground`, `FontSize`, `FontWeight`,
`TextAlignment`) and control members (`ClickMode`, `IsDefault`, `IsCancel`,
`IsPressed`, `IsThreeState`, content alignments, `SelectionMode`, `SelectAll`,
`UnselectAll`, `IsDropDownOpen`, `IsEditable`, `MaxDropDownHeight`) are
documented in [MARSHALLING.md](MARSHALLING.md). `Grid.ColumnDefinitions` and
`Grid.RowDefinitions` cross as the same comma-separated length list AXAML uses —
`grid.set_column_definitions("*,Auto,120")?` — which is normalising rather than
byte-preserving and carries only each track's length; see
[Grid track definitions](MARSHALLING.md#grid-track-definitions).
`Image`, `TabControl`/`TabItem`, `TreeView`/`TreeViewItem` and the `ToolTip`
attached properties are projected too. `Image.Source` crosses as the **source
string** the host resolves into a bitmap — `image.set_source("assets/logo.png")?`
takes a file path, a `file://` URI, or an `avares://`/`resm:` URI — and reads
back the string the ABI set, or `null` for an image the ABI never set; see
[Image sources](MARSHALLING.md#image-sources). `ToolTip::set_tip` carries text
only; a control-valued tip reads back as `null`. See
[ToolTip](MARSHALLING.md#tooltip) and [Tabs and trees](MARSHALLING.md#tabs-and-trees).
`PathIcon`, `TableView` and `TableViewColumn` are projected too.
`CommandBar`, `PipsPager` and `ThemeVariantScope` are projected too.
`Popup`, `TrayIcon`, `WindowNotificationManager`, `NotificationCard` and
`RefreshContainer` are projected too.
Shapes (`Rectangle`, `Ellipse`, `Line`, `Path`, `Polygon`, `Polyline`, `Arc`,
`Sector`) are projected too. Fill/Stroke are brushes; `Path.Data` is path markup.
`LinearGradientBrush` and `RadialGradientBrush` are projected as additive,
directly-constructible safe types built on the `IAvnControlFactory` creators
`create_linear_gradient_brush`/`create_radial_gradient_brush`; a `Paint` enum
spans solid and gradient shapes. Existing `Brush`-typed properties
(`Background`, `BorderBrush`, `Foreground`) are unchanged and stay solid-only;
see [Gradient brushes](MARSHALLING.md#gradient-brushes).
`Carousel`, `TransitioningContentControl`, `Label`, `Separator`, `GroupBox`,
`UserControl` and `LayoutTransformControl` are projected too.
`Calendar` and `CalendarDatePicker` are projected too. Calendar days cross as
`yyyy-MM-dd`. `NumericUpDown`, `AutoCompleteBox`, `MaskedTextBox`,
`SelectableTextBlock` and `ButtonSpinner` are projected too. Decimals cross as
invariant strings.
`RepeatButton`, `DropDownButton`, `SplitButton`, `ToggleSplitButton`,
`HyperlinkButton`, `ContextMenu` and `MenuFlyout` are projected too.
`HyperlinkButton.NavigateUri` is a URI string; `SplitButton.Flyout` is not.
`WrapPanel`, `UniformGrid`, `RelativePanel`, `Viewbox`, `FlexPanel` and
`GridSplitter` (plus its `Thumb` base) are projected too. RelativePanel's
`Align*WithPanel` bools cross as attached properties; object-valued `Above`/`LeftOf`
and Flex's attached `Order`/`Grow`/`Shrink` stay gaps.
`Flyout`, the imperative `Menu`/`MenuItem` pair, `SplitView`, `DatePicker` and
`TimePicker` are projected too. A flyout is an `AvaloniaObject` rather than a
`Control`, and it reaches a control through `flyout.show_at_with_control(&button)?`
rather than through an attached property — a COM-valued attached property has no
ABI shape yet. `SelectedDate` and `SelectedTime` cross as **ISO-8601 strings**;
`MenuItem.Command`, `HotKey` and `InputGesture` do not cross at all. See
[Flyouts](MARSHALLING.md#flyouts), [Menus as controls](MARSHALLING.md#menus-as-controls),
[SplitView](MARSHALLING.md#splitview) and [Dates and times](MARSHALLING.md#dates-and-times).
The executor-neutral completion ABI and clipboard integration are documented
in [ASYNC.md](ASYNC.md).
Desktop file integration (pickers, drag/drop, "open with", file associations)
is documented in [DESKTOP_FILES.md](DESKTOP_FILES.md).
Menus, keyboard accelerators, recent files and clipboard commands are
documented in [MENUS.md](MENUS.md).
The Rust-state/managed-presentation application model is documented in
[VIEW_MODELS.md](VIEW_MODELS.md).
Platform host selection and the Windows/Linux/macOS validation are documented in
[PLATFORMS.md](PLATFORMS.md).
The application template, one-command build, native host discovery,
packaging/checksums/signing, and SBOM scope are documented in
[PRODUCTIZATION.md](PRODUCTIZATION.md).
Release compatibility and versioning rules are documented in
[COMPATIBILITY.md](COMPATIBILITY.md).

## Prerequisites

- Windows 10 or later, Linux with X11, or macOS
- PowerShell 7 (`pwsh`)
- .NET SDK 10
- A current stable Rust toolchain

## Build and test

From the repository root, with PowerShell 7:

```powershell
pwsh ./rust/build.ps1
```

The script publishes the NativeAOT host for this OS, points
`AVN_HOST_NATIVE_LIB` at it, and runs the complete Rust workspace tests.
Use `-Architecture arm64` on a matching ARM64 runner.

On Linux, initialize DBus sources first:

```powershell
git submodule update --init external/Avalonia.DBus
pwsh ./rust/build.ps1
```

To run an example:

```powershell
$env:AVN_HOST_NATIVE_LIB = (Resolve-Path `
  .\host\bin\Release\net10.0\win-x64\publish\Avalonia.Host.dll)
cargo run --manifest-path .\rust\Cargo.toml -p avalonia --example hello_world
```

The examples progress from a basic window (`hello_world`) through a port of
`AppWithoutLifetime` and the Button, Slider, TextBox, ToggleSwitch, ComboBox,
ListBox, RadioButton, and Expander portion of `WinUIEmbedSample`
(`control_basics`), including its typed keyboard, hover, and asynchronous
clipboard interactions.
`text_test_input` ports the editable text, live preview, and new-window
interaction from the top of `TextTestApp`. `progress_bar` ports the interactive
core of ControlCatalog's `ProgressBarPage`, and `scroll_viewer` ports its basic
`ScrollViewerPage` controls and command surface.

`rust_vm_axaml` demonstrates the alternate application model: presentation is
precompiled managed AXAML while state, edits, collection mutations, commands,
and asynchronous work are owned by Rust. `rust_dynamic_vm_axaml` uses the same
Rust model through generated runtime metadata and the AOT-safe `RustBinding`
markup extension. Both also demonstrate stage 29 desktop file integration:
Rust-owned Open files / Open folder / Save-export commands, a drop panel, and
the startup "open with" activation list. Launch either example with file paths
to populate that list. `rust_vm_axaml` additionally demonstrates stage 31's
declared command surface: a File menu (Open, a recent-files submenu, Exit), an
Edit menu wired to the clipboard (copy, cut, paste, clear), a View menu with a
checkable item and a radio group, a context menu on the CMTrace table, a
`Ctrl+O` accelerator and a standalone `Ctrl+R` shortcut that is on no menu at
all. See [MENUS.md](MENUS.md). The dynamic adapter also implements `IReflectableType` for
JIT reflection bindings; NativeAOT applications use `RustBinding` because
Avalonia's general reflection binding requires dynamic code. Both examples
also register a Rust-authored `IValueConverter` (`CountToLabel`, formatting
`Count` as text) through the same generated `ValueConverters` trait; see
[VIEW_MODELS.md](VIEW_MODELS.md#rust-value-converters) for the transport and
lifetime rules. Both examples also exercise nested view models, nullable
values, enums, ordered collection edits, command `CanExecute` state, and
validation-error projection through a second, independently versioned sink
interface (`IAvnRustVmSink2`); see
[VIEW_MODELS.md](VIEW_MODELS.md#real-application-data-model-support).

AXAML is compiled during the managed build for NativeAOT applications.
`AvaloniaRuntimeXamlLoader` uses `System.Reflection.Emit`; preserving Avalonia
metadata with trimming or runtime directives does not make runtime AXAML
available when NativeAOT disables dynamic code. A future JIT-only development
host may offer runtime AXAML without changing the release path.

## Regenerate bindings

The one-command `regenerate-and-build.ps1`
(see [PRODUCTIZATION.md](PRODUCTIZATION.md#one-command-developer-workflow))
run every step below plus the managed and Rust builds; the commands here
are what it runs, spelled out for anyone changing the pipeline itself:

```powershell
dotnet run --project .\projection\Avalonia.Projection.Tool `
  -- .\rust\projection.ir.json `
  .\host\Generated\ObjectModel `
  .\rust\avalonia-sys\include\avalonia-rust-abi.h

Push-Location .\rust
cargo run -p avalonia-bindgen -- `
  .\projection.ir.json `
  .\avalonia-sys\src\generated.rs `
  .\avalonia\src\generated.rs
cargo fmt --all
Pop-Location

dotnet run --project .\projection\Avalonia.ViewModelProjection.Tool -- `
  .\rust\avalonia-sample\view-model.ir.json `
  .\samples\RustViewModelSample.Managed\Generated `
  .\samples\RustViewModelSample.Managed\Generated `
  .\rust\avalonia-sample\src\generated_view_models.rs `
  .\rust\avalonia-sample\view-model.contract.md `
  --external-rust
```

Public control coverage is declared in
`projection\Avalonia.Projection.Ir\AvaloniaProjectionProfiles.cs`. Unsupported
members remain visible in `projection.ir.gaps.txt`. New samples must widen
that shared policy and the generators when blocked; sample-specific host
bindings are not accepted.

The view-model command generates typed Rust traits and sinks, managed binding
adapters, the host's view registry, Rust-authored `IValueConverter` traits and
managed converter instances, and the readable `view-model.contract.md` report
from one versioned schema. Application names are confined to generated files
and the managed presentation project; the interop transport and handwritten
Rust runtime remain model-independent. The schema now also declares
schema-wide enums and nested-model/nullable/collection-element-kind
properties (see [VIEW_MODELS.md](VIEW_MODELS.md#real-application-data-model-support));
these ride the existing versioned `IAvnRustVmSink`/`IAvnRustVmSink2`
transport rather than a per-application ABI. Stage 27 additionally introduces
the independently versioned `IAvnRustVmSink3` immutable batch capability and its
`IAvnRustVmUpdateBatch2` ownership-commit companion. Worker and high-volume
publishers must use generated named batch builders: submission is one
nonblocking dispatcher post, stale/equal generations are rejected
deterministically, and completion is asynchronous. Both the generated and
reflectable adapters apply a batch through one shared staged transactional
engine, so a batch either applies whole or leaves state and notifications
untouched.
Stage 28 adds optional schema-v3 `TableView` metadata for model collections.
It emits typed table descriptors and named sort-command APIs, while compiled
AXAML owns the actual cell bindings. The CMTrace sample atomically publishes
100,000 nested rows: `TableView` virtualizes viewport controls, although the
stage-28 snapshot ABI still creates managed row adapters for every row. Stage
30's range-backed windows bound the data objects too. Batch publication raises a
table collection Reset before its associated selection properties, preserving
the Rust-owned row-key selection across a sort snapshot.
Stage 30 adds richer application data shapes on schema v4 through one more
separately versioned sink capability (`IAvnRustVmSink4`) plus Rust-implemented
`IAvnRustRangeSource`/`IAvnRustViewModel2`: observable keyed maps, hierarchical
`TreeDataTemplate` models, range-backed windows, typed structured command
results, and async progress with cancellation. A windowed collection reports
the full Rust dataset size while keeping live element objects bounded by
`pageSize * maxLivePages`, so 100,000 rows realize fewer than 1,000 adapters.
See [VIEW_MODELS.md](VIEW_MODELS.md).

Stage 29 adds platform-neutral desktop file integration through a third,separately versioned application capability (`IAvnApplication3`): multi-select
open, folder and save/export pickers, incoming file drag-and-drop, and
startup/"open with" activation. It reuses `TopLevel.StorageProvider`, the
`DragDrop` routed events and the desktop lifetime rather than any platform
dialog, keeps user cancellation distinct from failure, and never asks Rust to
negotiate a drag effect inside the platform drag loop. See
[DESKTOP_FILES.md](DESKTOP_FILES.md).

Stage 31 adds a declared command surface on schema v5: application menus,
context menus, keyboard accelerators, a recent-file list and clipboard
operations. Menus are presentation, so they add **no view-model ABI** -- the
generated factories build real `NativeMenu`/`ContextMenu`/`KeyBinding` objects
bound to the already generated command and property surface, with no reflection
anywhere. Recent files ride the published string-collection transport because an
entry is a stage 29 storage URI. Only the clipboard needs the host, through a
fourth separately versioned capability (`IAvnApplication4`) that adds clearing,
multi-format writes and reading file entries back as the same immutable storage
snapshots; the frozen text methods on `IAvnApplication` are unchanged. See
[MENUS.md](MENUS.md).

## Start a new application

Copy the [`templates/avalonia-app`](templates/avalonia-app) scaffold to
bootstrap a new Rust application against the same generated API these
examples use:

```powershell
.\rust\new-app.ps1 -Name my_app -Destination .\my_app
```

```bash
pwsh ./rust/new-app.ps1 -Name my_app -Destination ~/src/my_app
```

The scaffold is an external consumer with managed AXAML and view-model IR, not
an in-repository sample. Pin Rustolonia (Rust crates and host) and initialize its
separate Avalonia producer submodule recursively, then apply the carried patches.
From the Rustolonia repository root, build the new app in one step:

```powershell
pwsh .\rust\build-app.ps1 -ProducerRoot .\avalonia-src `
  -Manifest .\my_app\avalonia-app.json -UpdateLockFile
```

The manifest selects the managed presentation project, IR, Cargo package and
normal binary, RID, configuration, and adjacent output directory. The tool
checks source pins, patches and tool prerequisites, generates consumer
adapters/registry/Rust API, then builds Cargo with `--locked`,
builds AXAML, publishes a NativeAOT host with those exact external inputs, and
creates a checksummed CycloneDX bundle. See [PRODUCTIZATION.md](PRODUCTIZATION.md).

Commit the generated `global.json` and resolved `Cargo.lock`, then omit
`-UpdateLockFile` for normal builds. Formatting handwritten Rust is a separate,
explicit `cargo fmt` operation. Scaffold roots are relative (when on the same
volume) and the default RID matches the current OS and architecture.

`Avalonia.Host`, `Avalonia.Rust`, `Avalonia.Rust.Interop`, and the projection
tool/generator projects are currently non-packable, and the `rust/*` crates
are source-only (`publish = false`). None of this stage's new artifacts are
published as NuGet packages; see
[PRODUCTIZATION.md#sbom-eu-cra-scope](PRODUCTIZATION.md#sbom-eu-cra-scope)
for why that keeps them outside the repository's CycloneDX SBOM generation,
and what would need to change first if that ever changes.
