# Rust host compatibility policy

`Avalonia.Host`, the generated managed projection, and all crates in this
workspace are released in semver lockstep from one source revision.
`rustolonia`, `rustolonia-sys` and `rustolonia-bindgen` are published to
crates.io with the same version as the `rustolonia-host-<version>-<rid>.tar.gz`
assets of the GitHub release `v<version>` (see
[Crates.io and prebuilt hosts](#cratesio-and-prebuilt-hosts)); consumers must
not mix a host or generated bindings from one release with crates from another
release.

[`rust/release-manifest.json`](release-manifest.json) is the single
machine-readable record tying that lockstep release together: the Rustolonia
version, the exact pinned producer commit, the additive producer patch set
(by file and content hash), and every independently-versioned schema this
repository publishes (`projection.ir.json`, the view-model IR, and the
external consumer manifest). A test
(`ReleaseManifestTests`) fails closed if the manifest drifts from any of
those actual sources of truth, so bumping one schema or patch without
updating the manifest is caught rather than silently accepted.

## Generated IR

`projection.ir.json` (framework object model) and
`rust/avalonia-sample/view-model.ir.json` (sample/application schema) are
versioned schemas. A generator change must regenerate all checked-in managed,
Rust, and contract outputs in the same change. Sample-only Rust API that used
to be reexported from `rustolonia` now lives in `avalonia-sample`. Additive, optional schema fields require a schema
version bump and readers that reject unsupported future versions clearly.
Removing or changing the meaning, type, ordering, or requiredness of an
existing field is breaking and requires a coordinated major release.
Version 3 adds optional table metadata only. It reuses the existing
collection/property/command transport: compiled AXAML owns cell bindings and
the generated table descriptor contains no runtime reflection path.
Version 4 adds the stage 30 richer data shapes as optional members only:
`maps` on a model, `window`/`tree`/`recursive` on a collection, and
`resultModelName`/`supportsProgress`/`supportsCancellation` on a command. A
schema that declares any of them while claiming version 3 or lower is rejected
with an explicit upgrade message rather than being silently downgraded.
Version 5 adds the stage 31 command surfaces as optional members only: `menus`,
`recentFiles` and `displayPath` on a model. A schema that declares any of them
while claiming version 4 or lower is rejected the same way. These are
presentation and Rust-owned state only: menus bind the already generated command
and property surface and recent files ride the published string-collection
transport, so version 5 introduces no view-model ABI.

`projection.ir.json` version 12 adds the optional `brushInterfaceName`,
`brushInterfaceIid` and `brushAbiVersion` members (the version was removed in
version 18), present only when a projected
member marshals as `Brush`. `MarshallingKind.Brush` was appended after the
geometry kinds rather than grouped with the interface kinds, so every previously
published ordinal is unmoved.

Version 14 adds optional `commandInterfaceName`, `commandInterfaceIid` and
`commandAbiVersion` (the version was removed in version 18), present only when
a member marshals as `Command`.
`MarshallingKind.Command` was appended after CharUtf16, so every previously
published ordinal is unmoved.

Version 15 adds the optional `commandHandlerInterfaceName` and
`commandHandlerInterfaceIid` pair, present whenever `commandInterfaceName` is.
It publishes the IID of `IAvnCommandCanExecuteChangedHandler` so the Rust
bindgen can emit the handler interface and its CCW constructor without
hardcoding the identity. Both fields are additive; a reader that ignores them
sees the same vtables it always did.

Version 17 adds six optional root-level members, present whenever any member
marshals as `MarshallingKind.Brush`: `gradientBrushInterfaceName`/`Iid`,
`linearGradientBrushInterfaceName`/`Iid` and
`radialGradientBrushInterfaceName`/`Iid`. They are additive to
`BrushInterfaceName`/`BrushInterfaceIid` — the solid-brush interface, IID and
vtable are unchanged — and no `MarshallingKind` ordinal moves. A reader that
ignores the six new fields sees the same IR it always did; only a generator
that projects gradient brushes needs them.

Version 18 removes every per-interface ABI version field: `abiVersion`,
`interfaceAbiVersion`, `handlerInterfaceAbiVersion`,
`staticsInterfaceAbiVersion`, `factoryAbiVersion`, `brushAbiVersion` and
`commandAbiVersion`. Interfaces are unversioned (see [Native ABI](#native-abi)):
each generated IID is now derived from the projected type name alone, so every
generated IID changed once in this version and the generated header no longer
defines `*_ABI_VERSION` macros.

Version 13 adds two optional members, both host-side only. A projected property
and an attached property may carry `stringConverterTypeName`, naming the host
type that converts the member between its CLR type and the UTF-16 string in its
ABI slot, and an attached property may carry `isNullable`. Neither changes an ABI
slot: a reader that ignores both sees the same vtable it always did, and no
previously published ordinal moves.

Consumer application manifests are independently versioned by
`consumer-app-manifest.schema.json`; version 1 is validated before any build
command runs. A consumer must pin the producer checkout/submodule commit that
provides its `rustolonia` crate, projection tool, and `Avalonia.Host`. Do not mix
consumer-generated registry/adapters or Rust API from one producer revision
with a host from another.

## Native ABI

The native ABI is not a stable contract between independently released
components. A Rust application always loads the prebuilt host of the exact
release its `rustolonia-sys` was built for (see
[Crates.io and prebuilt hosts](#cratesio-and-prebuilt-hosts)), so the vtables
on both sides of every call come from the same source revision. Interfaces are
therefore not versioned: each one has a single identity that is changed in
place whenever it needs to change.

`rustolonia-sys` refuses to load a host built against a different generated
ABI. The host's `avn_get_host_info` export reports the SHA-256 of
`rustolonia-sys/include/avalonia-rust-abi.h` it was built with, and
`Host::load` compares it with the hash `rustolonia-sys` computed at build
time. Any change to the generated header therefore requires a matching host;
a mismatch (or a host without `avn_get_host_info`) fails with
`HostLoadError::IncompatibleHost` instead of reaching a stale vtable. The
hand-written interfaces (`IAvnApplication` in `host/Com` and the view-model
interfaces in `interop/Avalonia.Rust.Interop`) are not in that header; the
exact `=X.Y.Z` crate requirement and the checksum-pinned host download keep
them matched instead.

### One identity per interface

- Never extend an interface by adding a numbered successor (`IAvnFoo2`,
  `IAvnApplication16`, `IAvnRustVmSink6`). Add, remove or reorder members of
  the existing interface and update the C# definition, the Rust vtable and
  every implementation in the same change.
- Never change an IID to signal a change, and never put a version number in
  an interface name, an IID seed or the projection IR. Generated IIDs are
  derived from the projected type name alone
  (`ClrTypeExtractor.CreateDeterministicIid`); hand-written IIDs stay as they
  are. A new IID is only for a genuinely new interface with a new name.
- `UnversionedInterfaceTests` fails if a COM interface in `Avalonia.Host` or
  `Avalonia.Rust.Interop` has a name ending in a digit.

A changed vtable is still a breaking change for Rust source when the safe API
it backs changes, and it follows the semver rule in
[Crates.io and prebuilt hosts](#cratesio-and-prebuilt-hosts): every release
ships its own host whatever the bump.

### Optional functionality

When an implementation may legitimately lack a feature, say so on the same
interface rather than through a separate interface version:

- return `E_NOTIMPL` from the slot, as `IAvnApplication::SetApplicationName`
  does for platform-owned Android/iOS startup; or
- report capability flags from a method on the interface and return
  `E_NOTIMPL` from the slots a missing flag covers, as
  `IAvnRustVmSink::GetCapabilities` does with `RustVmSinkCapabilities`
  (`RicherShapes` for maps, command results/progress and windowed ranges;
  `NumberCollections` for integer and double collection elements).

Silently dropping the request is not an acceptable degradation; the caller
must see the failure. A distinct role that some objects implement and others
do not, such as `IAvnRustRangeSource` on a view model, may be its own
interface discovered through `QueryInterface`, under a descriptive name and
never as a numbered variant of an existing interface.

### Application services

`IAvnApplication` is the single host-implemented application interface. Beyond
startup and windows it carries:

- desktop file integration (see [DESKTOP_FILES.md](DESKTOP_FILES.md));
- clipboard commands, with the host-owned `IAvnClipboardData` payload builder;
- the desktop shell: shutdown mode, schema-declared tray icons, and window
  handle, topmost, transparency and capture-exclusion operations;
- code-first desktop objects: pixel surfaces (`IAvnPixelSurface` with its
  Rust-implemented `IAvnPixelSurfaceHandler`) and Rust-built tray icons
  (`IAvnShellTray`, `IAvnShellTrayHandler`);
- typed native-window descriptors, AppKit window-number and
  mouse-event-passthrough operations, and explicit show versus activate
  behavior. The macOS window number is for ScreenCaptureKit matching and does
  not promise capture exclusion;
- pre-initialization application naming through `SetApplicationName`, used by
  `App::with_application_name`. The host stages the name for its application
  factory, before Avalonia registers native application services; late
  changes fail and platform-owned Android/iOS startup returns `E_NOTIMPL`. The
  hash-pinned `avalonia-application-name.patch` also names Avalonia's default
  application-menu holder before export; setting the process name alone
  otherwise leaves its native item/submenu titles empty;
- code-first native menus: create a Rust-numbered `NativeMenu`
  (`IAvnNativeMenu`, with the Rust-implemented `IAvnNativeMenuHandler`), set
  it as a window's or the application's menu, query `IsNativeMenuExported`,
  and create a `NativeMenuBar` control.

### Projection history

What each projection wave added to the generated object model, in order.
Because nano-COM vtables are flattened, a member added to a base interface
appears in every derived interface's vtable.

The layout, chrome and control-completeness waves added the members described
in [MARSHALLING.md](MARSHALLING.md), and `IAvnControlFactory` gained
`create_solid_color_brush` alongside the read-only `IAvnBrush`. The definitions
wave added `column_definitions`/`row_definitions` to `IAvnGrid`: the
definitions cross as ordinary UTF-16 strings rather than as a new interface,
and the factory gained no slot.

The overlay-chrome pass adds only the safe, non-nullable window work-area and
dialog members to the leaf `IAvnWindow`, while blockers such as `Icon`,
`Position`, nullable geometry, and cancelable `Closing` payloads stay out of
scope (later waves below project them).

The wave P event-payload pass grows `IAvnControl` with `KeyUp`, `GotFocus`
and `LostFocus`. Three previously payload-less handlers change their `Invoke`
shape: `IAvnSelectingItemsControlSelectionChangedHandler` and
`IAvnTreeViewSelectionChangedHandler` now receive a host-implemented
`*SelectionChangedArgs` interface exposing the added/removed items as Variant
count/getter slots, and `IAvnRangeBaseValueChangedHandler` now carries the
old/new value pair as fields. The advise/unadvise slots on the control
interfaces keep their signatures throughout.

The gradient-brush pass adds three new interfaces, `IAvnGradientBrush`,
`IAvnLinearGradientBrush` and `IAvnRadialGradientBrush`, and grows
`IAvnControlFactory` with `create_linear_gradient_brush`/
`create_radial_gradient_brush`. `IAvnBrush` itself is untouched, and every
gradient object also answers `QueryInterface` for it.

The new-controls wave A is the first that widens nothing: `IAvnImage`,
`IAvnHeaderedItemsControl`, `IAvnTabControl`, `IAvnTabItem`, `IAvnTreeView`,
`IAvnTreeViewItem`, `IAvnToolTip` and `IAvnToolTipStatics` are all brand new.
The only existing interface that changes is `IAvnControlFactory`, which gains
a creator per new control plus `get_tool_tip_statics`.

Wave B is the same shape. `IAvnFlyoutBase`, `IAvnPopupFlyoutBase`, `IAvnFlyout`,
`IAvnMenuBase`, `IAvnMenu`, `IAvnHeaderedSelectingItemsControl`, `IAvnMenuItem`,
`IAvnSplitView`, `IAvnDatePicker` and `IAvnTimePicker` are all brand new; a
flyout is an `AvaloniaObject` rather than a `Control`, so `IAvnFlyoutBase` sits
directly under `IAvnAvaloniaObject` and cannot change a control interface.
`IAvnControlFactory` gains a creator per constructible wave B type; the
abstract bases get no creator and are reachable by `query_interface` only.

Wave C is the same shape again. `IAvnWrapPanel`, `IAvnUniformGrid`,
`IAvnRelativePanel`, `IAvnViewbox`, `IAvnThumb` and `IAvnGridSplitter` are all
brand new. `IAvnControlFactory` gains a creator per constructible wave C type
plus `get_relative_panel_statics`. RelativePanel's object-valued attached
properties (`Above`, `LeftOf`, …) stay gaps: a COM-valued attached property
has no ABI shape. The `Align*WithPanel` bools do cross. Flex attached
properties live on the static `Flex` class, which is not an `AvaloniaObject`, so
`Order`/`Grow`/`Shrink`/`Basis`/`AlignSelf` stay gaps too.

Wave D is the same shape. `IAvnRepeatButton`, `IAvnDropDownButton`,
`IAvnSplitButton`, `IAvnToggleSplitButton`, `IAvnHyperlinkButton`,
`IAvnContextMenu` and `IAvnMenuFlyout` are brand new. `SplitButton.Flyout` and
`Command` stay gaps. `HyperlinkButton.NavigateUri` crosses as a URI string
through `AvnUri`.

Wave E is the same shape. `IAvnSpinner` (abstract), `IAvnButtonSpinner`,
`IAvnNumericUpDown`, `IAvnAutoCompleteBox`, `IAvnMaskedTextBox` and
`IAvnSelectableTextBlock` are brand new. `NumericUpDown` decimals cross as
invariant strings through `AvnDecimal`. `PromptChar` is a `char` and stays a
gap.

Wave F is the same shape. `IAvnCalendar` and `IAvnCalendarDatePicker` are brand
new. Calendar days are `DateTime`, not `DateTimeOffset`, so they cross as
`yyyy-MM-dd` through `AvnCalendarDate` rather than the picker `"o"` form.
`SelectedDates` stays a gap.

Wave G is the same shape. `IAvnCarousel`, `IAvnTransitioningContentControl`,
`IAvnLabel`, `IAvnSeparator`, `IAvnGroupBox`, `IAvnUserControl` and
`IAvnLayoutTransformControl` are brand new. `PageTransition`, `LayoutTransform`
and `Label.Target` stay gaps.

Wave H is the same shape. `IAvnShape` (abstract) plus Rectangle, Ellipse, Line,
Path, Polygon, Polyline, Arc and Sector are brand new. Fill/Stroke are brushes.
Line points are `AvnPoint`. `Path.Data` is the path mini-language through
`AvnGeometry`. `Points` collections stay gaps.

Wave I is the same shape. `IAvnPopup`, `IAvnTrayIcon`, `IAvnWindowNotificationManager`,
`IAvnNotificationCard` and `IAvnRefreshContainer` are brand new. TrayIcon hangs
off `IAvnAvaloniaObject`. `NativeMenu`, `WindowIcon` and `ICommand` stay gaps.

Wave J is the same shape. `IAvnCommandBar`, `IAvnCommandBarButton`,
`IAvnCommandBarToggleButton`, `IAvnCommandBarSeparator`, `IAvnPipsPager` and
`IAvnThemeVariantScope` are brand new. Command lists, Icon and ThemeVariant
stay gaps.

Wave K is the same shape. `IAvnIconElement` (abstract), `IAvnPathIcon`,
`IAvnTableView`, `IAvnTableViewColumn`, `IAvnTableViewRow` and `IAvnTableViewCell`
are brand new. PathIcon.Data reuses `AvnGeometry`. Column Width is a GridLength
string. `TableView.Columns` stays a gap: a generic `AvaloniaList<T>` cannot be
assigned from the collection wrapper. Inlines stay a gap.

Wave L widens an existing leaf. `IAvnWindow` grows Hide plus SizeToContent,
ShowActivated, ShowInTaskbar, CanMinimize, CanMaximize, WindowStartupLocation,
WindowDecorations and ClosingBehavior. Nothing derives from Window, so no other
interface changes and the factory gains no slot. Icon, PixelPoint Position and ShowDialog
stay gaps. The obsolete SystemDecorations alias is not projected;
WindowDecorations is the live property.

Wave M widens a base. `IAvnTemplatedControl` grows FontFamily (UTF-16 via
`FontFamily.Parse`/`ToString`), FontStyle, FontWeight, FontStretch,
LetterSpacing and Padding, so every descendant's flattened vtable grows.
Independently, `IAvnTextBlock` grows the same fonts plus Background,
LineSpacing, MaxLines and TextWrapping, which `IAvnSelectableTextBlock`
inherits. `TextTrimming` is an abstract class without `ToString` and stays a
gap. Panels, Border, Image, shapes and flyouts are not under TemplatedControl
and are unaffected.

Wave N widens the TextBox leaf. `IAvnTextBox` grows SelectedText, content
alignments, selection/caret brushes, floating placeholder, inner left/right
content as `IAvnControl`, SelectAll/ClearSelection, and clipboard events.
`IAvnMaskedTextBox` grows MaskCompleted/MaskFull. PasswordChar, PromptChar,
CaretBlinkInterval, GetLineCount and the obsolete Watermark aliases stay gaps.

Wave O adds `AvnVector` (`MarshallingKind.Vector`, ordinal 18 after Brush) as
two doubles identical in layout to `AvnPoint`. `IAvnComboBox` grows Text,
PlaceholderForeground, Clear, DropDownOpened and DropDownClosed.
`IAvnScrollViewer` grows Extent/Viewport (`AvnSize`), Offset (`AvnVector`),
LargeChange/SmallChange and snap-point enums. ComboBoxItem is unchanged.
SelectionBoxItem, templates and PasswordChar stay gaps.

Wave P projects instance Flyout on `IAvnButton`/`IAvnSplitButton` as
`IAvnFlyoutBase` (nullable COM), so every Button descendant's flattened vtable
grows. MenuItem gains HasSubMenu, IsTopLevel, Open and Close.
Control.ContextMenu, ContextFlyout, IsLoaded and Loaded/Unloaded stay gaps:
putting them on Control would grow every control interface and is a wave of
its own. ICommand and HotKey stay gaps.

Wave overlay chrome grows `IAvnPopup` with Open/Close, Opened/Closed,
InheritsTransform, PlacementTarget, overlay-layer bools and IsPointerOverPopup.
`IAvnPopupFlyoutBase` (and Flyout/MenuFlyout) grow a read-only Popup COM slot.
`IAvnContextMenu` grows PlacementTarget. PlacementRect is nullable geometry;
PlacementAnchor/Gravity/ConstraintAdjustment are flags enums whose combined
values have no name — both stay gaps.

ItemsControl leftovers keep `Items`, `SelectedIndex` and `SelectionChanged` and
append `ItemCount` plus `ScrollIntoView(int)` on `IAvnItemsControl`.
`IAvnSelectingItemsControl` grows AutoScrollToSelectedItem, IsTextSearchEnabled
and WrapSelection. Every ItemsControl descendant inherits the new slots;
ContentControl and Window are unchanged. The object overload of
ScrollIntoView, templates and ItemsSource stay gaps.
Safe bindgen keeps the `is_` prefix on a bool builder when stripping it would
collide with a method on the same type (`Popup.IsOpen` vs `Open()`).

Text leftovers grow `IAvnTextBlock` with LineHeight and BaselineOffset,
`IAvnSelectableTextBlock` with selection brushes, SelectAll, ClearSelection and
CopyingToClipboard, and `IAvnTextBox` with ScrollToLine, which MaskedTextBox
inherits. GetLineCount returns int so it stays a gap, as do PasswordChar,
TextTrimming and Inlines.

Leaf leftovers close marshallable commands and scalars on existing types without
widening TemplatedControl or Panel. Carousel gains Next/Previous, CommandBar
overflow bools, DatePicker/TimePicker VerticalContentAlignment, NotificationCard
IsClosing/Close, ProgressBar Percentage, RefreshContainer RequestRefresh,
StackPanel snap bools and Border ClipToBoundsRadius, TableViewColumn
ActualWidth, and PipsPager SelectedIndexChanged. ComboBox content alignments
stay off ComboBox because it redeclares them with `new`. Label.Target is
IInputElement.

Control leftovers grow `IAvnControl` with ContextMenu (`IAvnContextMenu`),
ContextFlyout (`IAvnFlyoutBase`), IsLoaded, Loaded and Unloaded, so every
Control descendant's flattened vtable grows, including Image, layout panels and
shapes. StyledElement, FlyoutBase, MenuFlyout and TableViewColumn are
unchanged.

Non-void methods project as HRESULT plus an `out` slot. `GetLineCount` is the
first, on `IAvnTextBox` and its descendant `IAvnMaskedTextBox`.

`MarshallingKind.CharUtf16` is a UTF-16 code unit (`uint16_t`). PasswordChar and
PromptChar project on TextBox and MaskedTextBox.

`[Flags]` enums project as `I32` bitmasks with named constants, not closed Rust
enums. PlacementAnchor, PlacementGravity and PlacementConstraintAdjustment
append on Popup, ContextMenu, and PopupFlyoutBase plus descendants Flyout and
MenuFlyout.

Nullable geometry crosses as `AvnOptional{Kind}` (`has_value` plus the existing
blittable struct). PlacementRect appends on Popup and ContextMenu. Non-nullable
geometry slots stay `AvnRect` by value.

Window.Closing projects as a field payload: Cancel is in/out, CloseReason and
IsProgrammatic are inbound.

Thumb drag events carry `AvnVector` by value on `IAvnThumb` and
`IAvnGridSplitter`.

TextTrimming crosses as a UTF-16 name via a host converter on `IAvnTextBlock`
and `IAvnSelectableTextBlock`.

Label.Target projects as nullable `IAvnControl` (`IInputElement` values that
are Controls) on `IAvnLabel`.

`IAvnCommand` wraps `System.Windows.Input.ICommand` (`Execute` / `CanExecute`,
no CommandParameter). Command crosses on Button and its descendants (including
the wave D leaves), MenuItem, SplitButton/ToggleSplitButton and TrayIcon.

Inbound commands close the loop: `rustolonia_sys::command(execute, can_execute)`
builds a Rust CCW implementing `IAvnCommand` (ref-counted, panic-safe closures,
QI for IUnknown/IAvnCommand), and `Command::notify()` fires every live
subscription when the Rust side re-queries `CanExecute`. The host's
`CommandAdapter` (the `ToCommand` path for foreign `IAvnCommand` pointers)
subscribes eagerly with a weak-referenced handler, so a Rust command handed to
`Button::set_command` drives `CanExecuteChanged` end to end: the button
enables/disables when `notify()` runs. The generated handler constructor
`command_can_execute_changed_handler` mirrors the event-handler CCW pattern for
host-to-Rust notifications. IR schema moves 14 to 15 for the paired
`commandHandlerInterfaceName`/`commandHandlerInterfaceIid` fields. No ABI slot
moves: the advise/unadvise pair was already in the U7 vtable layout.

CommandParameter crosses as a tagged scalar, `MarshallingKind.Variant`
(ordinal 21, appended after Command): `AvnVariant` is a blittable struct
(tag + utf16 pointer + i32 + f64) carrying the closed set an `object`
parameter can hold — null, string, i32, f64, bool, with in-range longs
narrowing to i32. Getters return the struct with a host-allocated string
(Rust releases it through `take_utf16`/`free_utf16`); setters receive a
borrowed pointer the host reads synchronously. The safe crate exposes
`Variant` with `From` conversions and `button.command_parameter("save")`
ergonomics; a `VariantAbi` guard releases the UTF-16 allocation on drop.
The Button family (Button/ToggleButton/CheckBox/RadioButton/ToggleSwitch),
MenuItem, SplitButton, ToggleSplitButton and TrayIcon carry it.

`IAvnCommand` carries the parameter to the command:
`execute(AvnVariant parameter)` and `can_execute(AvnVariant parameter,
int32_t* value)`. The host's `AvnCommand` frees a UTF-16 payload
after converting it to the managed parameter; the Rust CCW does the same
after the closure returns. The Rust `command(execute, can_execute)`
closures now take the raw `AvnVariant`, and `CommandParameter` set on the
control flows end to end: `Button.CommandParameter` → managed
`ICommand.Execute(parameter)` → Rust closure argument.

Wave U13 closes CommandBar lifecycle and ScrollViewer anchoring.
CommandBar Opening/Opened/Closing/Closed cross as no-payload events.
ScrollViewer gains `ScrollBarMaximum` (a Vector, read-only direct property),
`CurrentAnchor` (a nullable `IAvnControl`), and
`RegisterAnchorCandidateWithControl`/`UnregisterAnchorCandidateWithControl`
taking an `IAvnControl`. TabItem.TabStripPlacement
stays a gap: `Dock?` is a nullable enum and the ABI has no nullable-int
shape yet; a future `NullableI32` kind would unblock it.

Wave U14 closes ContextMenu's open lifecycle.
`Open(Control?)` crosses as `OpenWithControl` — the safe layer takes
`Option<&impl AsControl>` — and Opening/Closing cross with the Cancel
write-back payload, the same `ref int Cancel` shape as PopupFlyoutBase and
Window. The extractor now suppresses a method that overrides an already
projected base method (ContextMenu's `Open()` override of MenuBase's is
reported as a gap with that reason instead of duplicating the base slot),
and the safe layer honors nullable `ComInterface` method parameters in
both directions.

Wave U15 closes AutoCompleteBox's populate lifecycle.
`MinimumPopulateDelay` introduces `MarshallingKind.TimeSpanI64`
(ordinal 22): a `TimeSpan` crosses as its int64 tick count, and the safe
layer exposes `std::time::Duration` (100ns per tick). TextChanged crosses
with no payload, and Populating crosses with the Cancel write-back plus
the `Parameter` text; DropDownOpening/DropDownClosing carry the Cancel
write-back. ItemTemplate, ValueMemberBinding, SelectedItem, the filter
and selector delegates, AsyncPopulator and ItemsSource stay gaps: they
are templates, bindings, delegates and an untyped object collection.

Wave U16 grows the base interfaces. StyledElement adds DataContextChanged (no
payload); Control adds SizeChanged, whose handler carries NewSize and
PreviousSize as `AvnSize` values — resize feedback reaches Rust on every
control. SplitView adds PaneOpening/PaneClosing (Cancel write-back) beside its
existing PaneOpened/PaneClosed; Calendar adds DisplayModeChanged
(OldMode/NewMode as i32); NotificationCard adds NotificationClosed (no
payload). Every StyledElement and Control descendant — all 100+ control
interfaces — inherits the new slots; Popup, TrayIcon and the flyout family do
not derive from Control and are unaffected.

Wave U17 widens the scalar and container surface. Two new kinds:
`DateTimeI64` (ordinal 23) carries a DateTime as its int64 tick count,
nullable as `AvnOptionalDateTime { has_value, ticks }` (SystemTime in the
sys crate, with the .NET epoch offset); `PixelPointI32` (ordinal 24)
carries an `Avalonia.PixelPoint` as the blittable `AvnPixelPoint { x, y }`
(a `PixelPoint` safe struct). Window gains Position; TextBox gains
CaretBlinkInterval via the TimeSpan shape; Calendar gains
DisplayDateChanged carrying RemovedDate/AddedDate.
Methods may now return a COM interface: ItemsControl gains
`ContainerFromIndexWithInt32` (returns a nullable `IAvnControl` as
`Result<Option<Control>>`) and `IndexFromContainerWithControl` taking
`&impl AsControl`. The object-keyed lookups stay gaps with the rest of the
untyped-collection family.

Wave U18 projects the object surface through the Variant kind. `object`
and `object?` members map to Variant everywhere, so SelectedItem and
SelectedValue (SelectingItemsControl, TreeView, AutoCompleteBox),
TabControl.SelectedContent, ItemsControl's `ContainerFromItemWithObject`
and `ItemFromContainerWithControl`, `ScrollIntoViewWithObject`, TreeView's
`TreeContainerFromItemWithObject`/`TreeItemFromContainerWithControl` and
Window's `CloseWithObject` all cross. ItemsSource (ItemsControl, MenuFlyout,
AutoCompleteBox) crosses as the new `IAvnVariantList` collection whose
element is a Variant: the interface is generated, and the host-side
`AvnObjectList` adapter owns the semantics — reads enumerate the live
source, the first mutation materializes a shadow list, and assigning the
list back through the setter persists it. `SelectedItems` uses a second
collection interface, `IAvnSelectedVariantList`, over an `IList` rather
than `IEnumerable` (SelectingItemsControl and TreeView). WindowNotification-
Manager gains CloseAll plus the object-based Show/Close overloads.

Wave U19 projects data templates, closing the templating family.
`MarshallingKind.DataTemplate` (ordinal 25) wraps
`Avalonia.Controls.Templates.IDataTemplate` as `IAvnDataTemplate`:
`match(AvnVariant data)` reports whether the template builds for an item
and `build` hands back the control. The host's `AvnDataTemplate` wraps a
managed template; a foreign `IAvnDataTemplate` converts back through the
`DataTemplateAdapter`, so a Rust CCW — `rustolonia_sys::data_template(
matches, build)` or the safe `rustolonia::data_template` — really renders
items. ItemTemplate crosses on ItemsControl, MenuFlyout, AutoCompleteBox
and ComboBox (SelectionBoxItemTemplate); HeaderTemplate on
HeaderedContentControl, HeaderedItemsControl, HeaderedSelectingItemsControl
and TableViewColumn; ContentTemplate on ContentControl, TabControl and
Flyout; ToggleSwitch gains On/OffContentTemplate. IR schema moves 15 to 16
for the paired `templateInterfaceName`/`templateInterfaceIid`.
Control.DataTemplates (the default-template list), TemplatedControl.Template
(the control template, not a data template) and the TemplateSettings getters
stay gaps by design.

Wave U20 projects the styled-element lifecycle and the payload events.
StyledElement gains IsInitialized, Parent, DataContext and TemplatedParent
(as a Variant) plus Initialized, ActualThemeVariantChanged, ResourcesChanged
and Attached/DetachedFromLogicalTree as no-payload events, which the whole
control tree inherits. NumericUpDown gains Spinned carrying
Direction/UsingMouseWheel; CalendarDatePicker gains DateValidationError
carrying Text and the ThrowException write-back. TabItem gains Icon (Variant)
and IconTemplate (DataTemplate); TabControl gains IndicatorTemplate and
SelectedContentTemplate; ComboBox gains SelectionBoxItem (Variant).
Nullable event string fields now read as Option<String> with a
null-tolerant clone.

Wave U21 projects icons as write-oriented path strings. Window.Icon and
TrayIcon.Icon cross as UTF-16 paths through the host-side `AvnWindowIcon`
converter, which loads a `WindowIcon` from the path; reading yields null
because an icon carries no source path to return.

Wave U22 projects the shape and typography scalar lists as text. Polygon
and Polyline Points cross as a UTF-16 "x,y x,y" list through the
host-side `AvnPointList` converter; Shape.StrokeDashArray crosses as
comma-separated doubles through `AvnDoubleList`; TextBlock and
TemplatedControl FontFeatures cross through `AvnFontFeatures` (the
collection's Parse exists but its ToString is inherited, so the converter
owns both halves).

Wave U23 projects the calendar date collections. Calendar.SelectedDates
and BlackoutDates, and CalendarDatePicker.BlackoutDates, cross as the
host-implemented `IAvnDateTimeList` whose elements are int64 DateTime
tick counts: the `AvnDateList` adapter reads the live collection and
writes through to it because the calendar owns the storage. The safe
crate exposes SystemTime elements with the .NET epoch offset. The
SelectedDatesChanged/SelectedDateChanged events stay gaps: their
SelectionChangedEventArgs carry added/removed collections the event
payload cannot carry yet.

Wave U24 projects AutoCompleteBox's filter delegates. Two kinds:
`ItemFilter` (ordinal 26) wraps the object predicate as `IAvnItemFilter`
whose invoke takes the search text and the item variant; `TextFilter`
(ordinal 27) wraps the string predicate as `IAvnTextFilter` over two
borrowed UTF-16 buffers. The host's `AvnItemFilter`/`AvnTextFilter` wrap
managed delegates; a foreign interface converts back into the delegate, so
the Rust CCWs — `rustolonia_sys::item_filter`/`text_filter` — really filter
items. The selector delegates and the async populator stay gaps: selectors
add another callback shape and the populator needs the async-completion
transport. CCWs now free variant payloads only when a host is loaded.

Wave U25 sweeps the last closable members and separates by-design gaps.
Expander gains the vetoable Expanding/Collapsing pair, RefreshContainer
gains RefreshRequested, TrayIcon gains Clicked, and Spinner gains Spin.
Slider.Ticks crosses through the AvnDoubleList converter, Spinner gains
ValidSpinDirection, TemplatedControl gains BackgroundSizing; TabItem
gains IndicatorTemplate, SplitView gains PaneTemplate, and the CommandBar
buttons gain Icon as a Variant. The gap report now distinguishes
"By design: reason" — an architectural exclusion with its rationale
(the property system, bindings, themes, internal plumbing, obsolete
aliases, the suppressed ContextMenu.Open override) — from the few
remaining marshalling boundaries: the INotification content overloads and
the Show overload with Action callbacks, which need a typed notification
ABI and a callback transport.

Wave Q sweeps leftover marshallable scalars on leaf input types:
`IAvnAutoCompleteBox`, `IAvnCalendar`, `IAvnCalendarDatePicker` and
`IAvnNumericUpDown`. Templates, filters, ItemsSource, SelectedDates,
NumberFormat and obsolete Watermark aliases stay gaps.

Wave U26 projects typed notifications, closing the last pending gaps.
`Notification` (ordinal 28) maps `INotification` to `IAvnNotification`
whose getter slots carry the title, message, type, expiration and the
OnClick/OnClose handler interfaces (`IAvnNotificationActionHandler`);
the host's `AvnNotification` wraps a foreign interface as a managed
`INotification`, reading the handlers through getter slots into managed
actions. The bindgen emits the handler CCW through the event-callback
pattern and RCW getters on `IAvnWindowNotificationManager`, and the
Rust side grows a ref-counted `rustolonia_sys::notification` CCW whose
string getters allocate through the host's UTF-16 provider. The INotification
content overloads and the Action-carrying Show overload cross through
the notification CCW's handler slots; every remaining gap in the report
is now by-design with its rationale.

Wave U27 projects the scalar value models. KeyGesture crosses through its
own Parse/ToString round-trip as an ordinary UTF-16 string — Button.HotKey,
SplitButton.HotKey, MenuItem.HotKey and MenuItem.InputGesture — and the
emitter now null-guards a nullable Parse so clearing the gesture works.
ThemeVariant crosses through the AvnThemeVariant host converter ("Light",
"Dark", "Default"; null = unset) for ThemeVariantScope.RequestedThemeVariant
and StyledElement.ActualThemeVariant; Border.BoxShadow crosses through
AvnBoxShadows (the comma-separated list BoxShadows prints, split on
top-level commas); TabItem.TabStripPlacement crosses through AvnDock as a
nullable dock name (read-only, the CLR setter is internal). StyledElement
grows a slot, which all 97 descendants inherit. The gap report drops from
190 to 182 entries, every one by-design.

Wave U28 crosses the scalar event payloads. The Fields payload grows two
nullable wrappers: `AvnOptionalTimeSpan` (has_value + ticks, converting
through `core::time::Duration`) joins `AvnOptionalDateTime`, and
DateTimeOffset event fields convert through `.UtcDateTime` so the wire form
stays UTC ticks. DatePicker.SelectedDateChanged crosses with the
old/new date pair, TimePicker.SelectedTimeChanged with the old/new time
pair, and TextBox.TextChanging crosses as a plain notification because
TextChangingEventArgs carries no payload members.
CalendarDatePicker.SelectedDateChanged stays by-design: it carries
SelectionChangedEventArgs, whose added/removed collection payload needs the
variant-list event shape. The gap report drops to 179 entries.

Wave U29 projects AutoCompleteBox's selector delegates. Two kinds:
`ItemSelector` (ordinal 29) wraps the object selector as `IAvnItemSelector`
whose invoke takes the search text and the item variant and returns the
display text as a host-allocated UTF-16 buffer; `TextSelector` (ordinal 30)
wraps the string selector as `IAvnTextSelector` over two borrowed buffers.
The host's `AvnItemSelector`/`AvnTextSelector` wrap managed delegates; a
foreign interface converts back into the delegate, so the Rust CCWs —
`rustolonia_sys::item_selector`/`text_selector` — really format items. The
CCWs allocate their returned string through the host's UTF-16 provider and
report E_NOTIMPL without a host, exactly as the notification CCW's string
getters do. The gap report drops to 177 entries.

Wave U30 projects the object lists and pass-through members. CommandBar's
PrimaryCommands and SecondaryCommands cross as a new IAvnControlList — a
host-implemented live adapter (AvnControlList) working through the
non-generic IList the observable command collections implement, so reads
observe the bar and mutations persist into it; VisiblePrimaryCommands and
OverflowItems ride the same adapter as read-only views. Control.Tag crosses
as a Variant, which grows Control's vtable and those of all 95 descendants.
Popup and PopupFlyoutBase's OverlayInputPassThroughElement cross as a
control reference. The gap report drops to 170 entries.

Wave U31 crosses the transition payload. TransitioningContentControl's
TransitionCompleted rides the Fields machinery with two Variants (From/To)
and the completion flag — the event-argument emitters gain a Variant case,
on both the C# side (AvnVariant.FromObject) and the bindgen
(event_argument_type), so any future object?-valued event field crosses the
same way. A transition never runs in the headless host, so the payload is
validated by the advise/unadvise pair and the generated slot signature.
The gap report drops to 169 entries.

Wave U32 crosses the collection event payloads with a new
`EventPayloadKind.Args`: the handler receives a host-implemented args
interface whose `Get{Name}Count`/`Get{Name}At` slots expose each projected
collection property as Variants, live over the CLR event args. Four events
cross — AutoCompleteBox.SelectionChanged and Populated,
Calendar.SelectedDatesChanged, CalendarDatePicker.SelectedDateChanged —
and the bindgen emits the args interface with safe count/getter methods on
the Rust side (`subscribe_selection_changed` receives a
`ComPtr<IAvnSelectionChangedArgs>`-shaped wrapper). The gap report drops
to 165 entries.

Wave U33 sweeps the column list and reason hygiene. TableView.Columns
crosses through the same IAvnControlList live adapter as the command-bar
lists — AvaloniaList<TableViewColumn> is an IList of projected controls.
The stale "INotification content" reasons on ContextMenu/Menu's Close
overwrites are corrected to name the real cause: they suppress the
projected base slot. The gap report drops to 164 entries.

Wave U34 crosses the pointer-tracking flyout overload. The insight:
`showAtPointer` is only a placement-mode switch, not pointer state, so
PopupFlyoutBase.ShowAt(Control, Boolean) crosses as an ordinary method
(`ShowAtWithControlAndBoolean`) with no payload shape at all. The sealed
ShowAt(Control) and Hide overrides of the projected base slots stay
suppressed, now with accurate reasons ("overrides the projected base
slot" / "the sealed override of the projected base Hide slot") instead of
the stale pointer-event wording. A stale wave-B forbidden pin that banned
MenuItem's gesture setters is flipped to require them now that U27
projects the gestures. The gap report drops to 163 entries.

Wave U35 crosses the custom popup placement callback. `PopupPlacement`
(ordinal 31) maps `CustomPopupPlacementCallback` to
`IAvnPopupPlacementCallback`, whose invoke passes the popup size and
anchor rectangle in and the mutated placement — offset, anchor, gravity,
constraint adjustment — back through out-parameters. The host's
`AvnPopupPlacementCallback` wraps the managed delegate both ways: a
foreign CCW converts into the delegate (the host constructs the
placement record through the new Avalonia.Host InternalsVisibleTo entry,
calls it, and reads the mutations back), and Popup, ContextMenu and
PopupFlyoutBase's CustomPopupPlacementCallback properties cross. The Rust
CCW — `rustolonia_sys::popup_placement` — returns a
`PopupPlacementResult`. The gap report drops to 160 entries.

Wave U36 crosses the async populator. `AsyncPopulator` (ordinal 32) maps
AutoCompleteBox's `Func<string?, CancellationToken,
Task<IEnumerable<object?>>`? to `IAvnAsyncPopulator`:
`BeginPopulate(requestId, IAvnAsyncPopulatorCompletion, searchText)`
launches, and the CCW reports through the completion's
`Complete(requestId, hresult, IAvnVariantList)` slot. The host wrapper
bridges both directions: a managed delegate runs under
`AvnPopulatorBridge` and reports through a host-built completion CCW,
while a foreign CCW converts into the delegate through a
`TaskCompletionSource` the completion resolves. The Rust CCW —
`rustolonia_sys::async_populator` — hands the closure a
`PopulateCompletion` reporter. ShowDialog stays by-design with an
accurate reason: its modal completion needs the dialog-owner lifetime the
projected Show overloads do not carry. The gap report drops to 159
entries.

`projection.ir.json` needs no schema change to carry a member whose CLR type is
not `string` but whose ABI slot is: the existing `kind` and `managedTypeName`
pair already says both, exactly as it does for an enum carried as `I32`. A
`StringUtf16` override on a non-string member is accepted when the managed type
declares `static T Parse(string)` and overrides `ToString()`, so the conversion
belongs to the type rather than to the generator — or, when the CLR type cannot
own either half (`Image.Source` is an `IImage`, `ToolTip.Tip` is an `object`) or
when the type's own parser is looser than the published contract
(`DatePicker.SelectedDate` and `TimePicker.SelectedTime` publish a strict ISO-8601
grammar rather than whatever `DateTimeOffset.Parse` happens to accept), when the
profile names a host-side converter in `stringConverterTypeName`. That field is a
normally a host-side detail: the ABI slot is an ordinary UTF-16 string, and the
Rust bindings do not read the converter name. Grid track definitions additionally
support an optional `@SharedSizeGroup` suffix per length. That string contract is
recorded in the generated native header so its fingerprint requires a matching
host; a previous host cannot safely consume the extended format.

## RID artifacts

Release artifacts are per RID and retain their exact names:
`rustolonia_host.dll` (`win-*`), `librustolonia_host.so` (`linux-*`), and
`librustolonia_host.dylib` (`osx-*`). Each artifact directory contains the matching
native dependencies, `licence.md`, a deterministic `sbom.cdx.json` delivery
inventory, and a `checksums.sha256` manifest covering every delivered file
except the manifest itself. Verify it before distribution or launch. Every RID
maps to an explicit native Rust target triple; packaging and executable tests
require a matching runner CPU rather than silently producing an untested cross
architecture binary.
Never substitute dependencies or combine directories from different RIDs.
Windows Rust executables use the static MSVC CRT; adding a dependency that
reintroduces a redistributable runtime DLL is a delivery-scope change and must
update packaging, SBOM coverage, and compatibility validation together.

Official desktop release binaries must be signed with the platform-appropriate
signing identity before the SBOM and checksums are generated. The optional
`AVALONIA_RUST_SIGN_COMMAND` hook is a wrapper executable/script path that
receives an artifact path as a separate argument; unsigned local developer
artifacts are not release artifacts. iOS host archives contain link inputs
rather than an installable app; the app developer signs and provisions the
final device app and its embedded frameworks.

The Rust host and standalone Rust artifacts are not NuGet packages, so
`nukebuild/SbomGenerator.cs` remains unchanged. Their per-RID delivery scope
is instead represented by `rust/generate-sbom.ps1` in `sbom.cdx.json`, including
the host, Rust executable, bundled native binaries, and licence with SHA-256
hashes after signing.

## Crates.io and prebuilt hosts

- `rustolonia` depends on `rustolonia-sys` with an exact `=X.Y.Z` requirement,
  so a dependency graph can contain only one release of the bindings.
- `rustolonia-sys` downloads only the host tarball of its own version and
  verifies it against the SHA-256 recorded in its packaged
  `host-checksums.txt`. A crate version is never re-pointed at a different
  host build; a new host means a new crate version.
- The host reports its release version and the SHA-256 of
  `avalonia-rust-abi.h` through `avn_get_host_info`. `Host::load` fails with
  `HostLoadError::IncompatibleHost` when the fingerprint differs from the one
  `rustolonia-sys` was built against, or when the export is missing (a host
  older than this check). A host found through `RUSTOLONIA_HOST_LIB`,
  `RUSTOLONIA_HOST_DIR` or next to the executable is subject to the same
  check.
- iOS uses separate `ios-arm64`, `iossimulator-arm64`, and `iossimulator-x64`
  static-host archives rather than desktop host discovery. The
  [standalone iOS builder](../samples/ios-standalone/README.md) verifies the
  archive against the matching crate's release-pinned checksum before
  extraction, then checks the host RID, version, ABI fingerprint, and file
  inventory. The in-process host checks the ABI again at startup. An explicitly
  supplied development checksum overrides the release pin and must itself be
  trusted. A future release must publish these archives and their checksums
  together with the matching crates.
- Until 1.0, a minor version bump (`0.Y`) may break the Rust API and the ABI;
  a patch bump (`0.Y.Z`) is reserved for compatible fixes. Every release still
  ships its own host, whatever the bump.

## File type associations

Registering a document type is packaging metadata owned by whatever installer
or store channel a consumer uses. The application template ships copyable
Windows/Linux/macOS snippets (see
[DESKTOP_FILES.md](DESKTOP_FILES.md#file-type-associations)); this workflow
deliberately does not introduce MSIX packaging or platform installers, and the
snippets add nothing to the delivered per-RID bundle, so the SBOM delivery
scope is unchanged.
