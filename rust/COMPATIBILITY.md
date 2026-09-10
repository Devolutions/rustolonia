# Rust host compatibility policy

`Avalonia.Host`, the generated managed projection, and all crates in this
workspace are released in semver lockstep from one source revision. The crates
are `publish = false`; consumers must not mix a host or generated bindings from
one release with crates from another release.

[`rust/release-manifest.json`](release-manifest.json) is the single
machine-readable record tying that lockstep release together: the Rustolonia
version, the exact pinned producer commit, the additive producer patch set
(by file and content hash), and every independently-versioned schema this
repository publishes (`projection.ir.json`, the view-model IR, the released
ABI baseline, and the external consumer manifest). A test
(`ReleaseManifestTests`) fails closed if the manifest drifts from any of
those actual sources of truth, so bumping one schema or patch without
updating the manifest is caught rather than silently accepted.

## Generated IR

`projection.ir.json` (framework object model) and
`rust/avalonia-sample/view-model.ir.json` (sample/application schema) are
versioned schemas. A generator change must regenerate all checked-in managed,
Rust, and contract outputs in the same change. Sample-only Rust API that used
to be reexported from `avalonia` now lives in `avalonia-sample`. Additive, optional schema fields require a schema
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
`brushInterfaceIid` and `brushAbiVersion` members, present only when a projected
member marshals as `Brush`. `MarshallingKind.Brush` was appended after the
geometry kinds rather than grouped with the interface kinds, so every previously
published ordinal is unmoved.

Version 14 adds optional `commandInterfaceName`, `commandInterfaceIid` and
`commandAbiVersion`, present only when a member marshals as `Command`.
`MarshallingKind.Command` was appended after CharUtf16, so every previously
published ordinal is unmoved.

Version 15 adds the optional `commandHandlerInterfaceName` and
`commandHandlerInterfaceIid` pair, present whenever `commandInterfaceName` is.
It publishes the IID of `IAvnCommandCanExecuteChangedHandler` so the Rust
bindgen can emit the handler interface and its CCW constructor without
hardcoding the identity. Both fields are additive; a reader that ignores them
sees the same vtables it always did.

Version 13 adds two optional members, both host-side only. A projected property
and an attached property may carry `stringConverterTypeName`, naming the host
type that converts the member between its CLR type and the UTF-16 string in its
ABI slot, and an attached property may carry `isNullable`. Neither changes an ABI
slot: a reader that ignores both sees the same vtable it always did, and no
previously published ordinal moves.

Consumer application manifests are independently versioned by
`consumer-app-manifest.schema.json`; version 1 is validated before any build
command runs. A consumer must pin the producer checkout/submodule commit that
provides its `avalonia` crate, projection tool, and `Avalonia.Host`. Do not mix
consumer-generated registry/adapters or Rust API from one producer revision
with a host from another.

## Native ABI

The overlay-chrome pass widens the leaf `IAvnWindow` ABI from 7 to 8 to expose only the safe, non-nullable window work-area and dialog members. This stays local to the window leaf: `IAvnContentControl` remains at 6 and the factory remains at 13, while blockers such as `Icon`, `Position`, nullable geometry, and cancelable `Closing` payloads stay out of scope.

The wave P event-payload pass grows `IAvnControl` with `KeyUp`, `GotFocus`
and `LostFocus`, so `IAvnControl` and every interface below it republish on
fresh IIDs. Three previously payload-less handlers change their `Invoke`
shape and therefore mint new identities at version 2:
`IAvnSelectingItemsControlSelectionChangedHandler` and
`IAvnTreeViewSelectionChangedHandler` now receive a host-implemented
`*SelectionChangedArgs` interface exposing the added/removed items as Variant
count/getter slots, and `IAvnRangeBaseValueChangedHandler` now carries the
old/new value pair as fields. The advise/unadvise slots on the control
interfaces keep their signatures throughout; only the handler identities move.
Consumers compiled against the version 1 handlers fail `QueryInterface`
loudly (`E_NOINTERFACE`) rather than being called through a stale vtable.
After an intentional ABI wave, regenerate the frozen snapshot with
`pwsh ./rust/regenerate-and-build.ps1 -UpdateAbiBaseline` in the same change
so `abi-baseline.json` stays the reviewed record of what is released.

Published interface IIDs, vtable slot order, method signatures, calling
conventions, ownership rules, and error semantics are immutable. Never reuse an
IID for a changed interface and never insert a slot into an existing vtable.
Add a separately named, separately versioned interface with a new IID (for
example `IAvnRustVmSink2`, stage 29's `IAvnApplication3` desktop file
integration capability, stage 30's `IAvnRustVmSink4` richer data shapes,
`IAvnRustVmRangeBatch` windowed range payload, `IAvnRustRangeSource` and
`IAvnRustViewModel2`, or stage 31's `IAvnApplication4` clipboard command
capability and its host-owned `IAvnClipboardData` payload builder, or
`IAvnRustVmSink5` for scalar-number collection elements), then
negotiate/query it as optional capability.
A producer or host that predates an optional capability must report
`E_NOINTERFACE` explicitly; silently dropping the affected updates is not an
acceptable degradation.
Any unavoidable incompatible ABI change requires a new host ABI generation and
a coordinated major release.

The frozen released C ABI snapshot is [`abi-baseline.json`](abi-baseline.json)
(`kind: released`, schema version 3). It records producer pin, ABI typedef and
macro expansions (`AvnHResult` → `int32_t`, `AVN_CALL` → `__stdcall` on
Windows and empty elsewhere, struct packing default), interned callable slot
templates, value-struct layouts, and flattened IR lineage fingerprints for each
released identity. New current identities are allowed; the gate only requires
every released IID to keep its contract. `projectionIrVersion` in the snapshot
is provenance, not a live lock against `ProjectionIr.CurrentVersion`.

The pointer spelling in a slot (`const T*`, `T**`) is indirection and
constness only. It does not encode COM lifetime, free-ownership, or
allocator contracts; those remain outside this gate. Duplicate value-type
records, duplicate IIDs, and IID-constant/vtable-name mismatches fail closed
instead of associating the next marker.

The current header plus IR are an appendable inventory. Changing a resolved
typedef/macro, a parameter type, a flattened IR kind/direction/nullability
including inherited members, a value-struct layout, `abiVersion`, or name
under an unchanged IID is a break.

Intentional ABI-generation transitions retire identities instead of reusing
them: add the old IID to `retiredIdentities` with the replacement name/reason,
publish the successor under a new IID, and never reuse the retired GUID.
Missing a retired IID is expected; seeing it again on a live interface is a
reuse break. Update the released snapshot in the same change that adds a new
identity or completes a documented retirement.

The generated object model (`IAvnControl` and friends) is versioned by the
`abiVersion` recorded per interface in `projection.ir.json`, which is hashed
into the IID. Nano-COM vtables are flattened, so allowlisting a member on a base
type moves every derived interface's slots; every affected interface bumps its
`abiVersion` together and republishes under a fresh IID, and the previous IIDs
are retired rather than reused. An interface whose flattened vtable is
byte-identical keeps its IID, so a stale consumer that queries for it still
receives exactly the contract it compiled against. The layout wave took
`IAvnStyledElement`, `IAvnControl` and everything below them to version 3 while
`IAvnAvaloniaObject` stayed at version 2. The chrome wave then took
`IAvnBorder`, `IAvnPanel`, `IAvnTemplatedControl`, `IAvnTextBlock` and everything
below them to version 4, while `IAvnAvaloniaObject` stayed at 2 and
`IAvnStyledElement`, `IAvnControl` and `IAvnDecorator` stayed at 3 because their
own flattened vtables did not move. `IAvnControlFactory` gained
`create_solid_color_brush` and moved from version 1 to 2, and the new read-only
`IAvnBrush` starts at version 1. The completeness wave then took
`IAvnContentControl`, `IAvnButton`, `IAvnToggleButton`, `IAvnListBox`,
`IAvnComboBox` and everything below them to version 5; `IAvnControlFactory`
gained no slot and stays at 2, and every interface outside those subtrees —
including `IAvnItemsControl` and `IAvnSelectingItemsControl` — keeps the version
whose flattened vtable it still matches. The definitions wave then took
`IAvnGrid` alone from 4 to 5 for `column_definitions`/`row_definitions`: nothing
in the object model derives from `Grid`, the definitions cross as ordinary UTF-16
strings rather than as a new interface, and `IAvnControlFactory` again gained no
slot. All four IIDs `IAvnGrid` published at versions 1–4 are retired.

The new-controls wave A is the first that widens nothing: `IAvnImage`,
`IAvnHeaderedItemsControl`, `IAvnTabControl`, `IAvnTabItem`, `IAvnTreeView`,
`IAvnTreeViewItem`, `IAvnToolTip` and `IAvnToolTipStatics` are all brand new and
publish at version 1, and every interface that shipped before keeps the exact IID
it last published. The one thing that moves is `IAvnControlFactory`, which gains
a creator per new control plus `get_tool_tip_statics` and goes from 2 to 3. A
consumer compiled against the definitions wave therefore only has to requery the
factory; every control interface pointer it already holds stays valid.

Wave B is the same shape. `IAvnFlyoutBase`, `IAvnPopupFlyoutBase`, `IAvnFlyout`,
`IAvnMenuBase`, `IAvnMenu`, `IAvnHeaderedSelectingItemsControl`, `IAvnMenuItem`,
`IAvnSplitView`, `IAvnDatePicker` and `IAvnTimePicker` are all brand new and
publish at version 1; a flyout is an `AvaloniaObject` rather than a `Control`, so
`IAvnFlyoutBase` sits directly under `IAvnAvaloniaObject` and cannot move a control
interface. `IAvnControlFactory` gains a creator per constructible wave B type and
goes from 3 to 4; the abstract bases get no creator and are reachable by
`query_interface` only. Again, only the factory has to be requeried.

Wave C is the same shape again. `IAvnWrapPanel`, `IAvnUniformGrid`,
`IAvnRelativePanel`, `IAvnViewbox`, `IAvnFlexPanel`, `IAvnThumb` and
`IAvnGridSplitter` are all brand new and publish at version 1.
`IAvnControlFactory` gains a creator per constructible wave C type plus
`get_relative_panel_statics` and goes from 4 to 5. RelativePanel's object-valued
attached properties (`Above`, `LeftOf`, …) stay gaps: a COM-valued attached
property has no ABI shape. The `Align*WithPanel` bools do cross. Flex attached
properties live on the static `Flex` class, which is not an `AvaloniaObject`, so
`Order`/`Grow`/`Shrink`/`Basis`/`AlignSelf` stay gaps too.

Wave D is the same shape. `IAvnRepeatButton`, `IAvnDropDownButton`,
`IAvnSplitButton`, `IAvnToggleSplitButton`, `IAvnHyperlinkButton`,
`IAvnContextMenu` and `IAvnMenuFlyout` publish at version 1.
`IAvnControlFactory` moves from 5 to 6. `SplitButton.Flyout` and `Command` stay
gaps. `HyperlinkButton.NavigateUri` crosses as a URI string through `AvnUri`.

Wave E is the same shape. `IAvnSpinner` (abstract), `IAvnButtonSpinner`,
`IAvnNumericUpDown`, `IAvnAutoCompleteBox`, `IAvnMaskedTextBox` and
`IAvnSelectableTextBlock` publish at version 1. `IAvnControlFactory` moves from
6 to 7. `NumericUpDown` decimals cross as invariant strings through `AvnDecimal`.
`PromptChar` is a `char` and stays a gap.

Wave F is the same shape. `IAvnCalendar` and `IAvnCalendarDatePicker` publish at
version 1. `IAvnControlFactory` moves from 7 to 8. Calendar days are `DateTime`,
not `DateTimeOffset`, so they cross as `yyyy-MM-dd` through `AvnCalendarDate`
rather than the picker `"o"` form. `SelectedDates` stays a gap.

Wave G is the same shape. `IAvnCarousel`, `IAvnTransitioningContentControl`,
`IAvnLabel`, `IAvnSeparator`, `IAvnGroupBox`, `IAvnUserControl` and
`IAvnLayoutTransformControl` publish at version 1. `IAvnControlFactory` moves
from 8 to 9. `PageTransition`, `LayoutTransform` and `Label.Target` stay gaps.

Wave H is the same shape. `IAvnShape` (abstract) plus Rectangle, Ellipse, Line,
Path, Polygon, Polyline, Arc and Sector publish at version 1.
`IAvnControlFactory` moves from 9 to 10. Fill/Stroke are brushes. Line points
are `AvnPoint`. `Path.Data` is the path mini-language through `AvnGeometry`.
`Points` collections stay gaps.

Wave I is the same shape. `IAvnPopup`, `IAvnTrayIcon`, `IAvnWindowNotificationManager`,
`IAvnNotificationCard` and `IAvnRefreshContainer` publish at version 1.
`IAvnControlFactory` moves from 10 to 11. TrayIcon hangs off `IAvnAvaloniaObject`.
`NativeMenu`, `WindowIcon` and `ICommand` stay gaps.

Wave J is the same shape. `IAvnCommandBar`, `IAvnCommandBarButton`,
`IAvnCommandBarToggleButton`, `IAvnCommandBarSeparator`, `IAvnPipsPager` and
`IAvnThemeVariantScope` publish at version 1. `IAvnControlFactory` moves from
11 to 12. Command lists, Icon and ThemeVariant stay gaps.

Wave K is the same shape. `IAvnIconElement` (abstract), `IAvnPathIcon`,
`IAvnTableView`, `IAvnTableViewColumn`, `IAvnTableViewRow` and `IAvnTableViewCell`
publish at version 1. `IAvnControlFactory` moves from 12 to 13. PathIcon.Data
reuses `AvnGeometry`. Column Width is a GridLength string. `TableView.Columns`
stays a gap: a generic `AvaloniaList<T>` cannot be assigned from the collection
wrapper. Inlines stay a gap.

Wave L widens an existing leaf. `IAvnWindow` grows Hide plus SizeToContent,
ShowActivated, ShowInTaskbar, CanMinimize, CanMaximize, WindowStartupLocation,
WindowDecorations and ClosingBehavior, and moves from 5 to 6. Nothing derives
from Window, so no other interface and no factory slot move. Icon, PixelPoint
Position and ShowDialog stay gaps. The obsolete SystemDecorations alias is not
projected; WindowDecorations is the live property.

Wave M widens a base. `IAvnTemplatedControl` grows FontFamily (UTF-16 via
`FontFamily.Parse`/`ToString`), FontStyle, FontWeight, FontStretch,
LetterSpacing and Padding, and moves from 4 to 5. Every descendant republishes:
chrome-era types at 4 go to 5, completeness-era types at 5 go to 6, Window goes
from 6 to 7, and previously version-1 templated types go to 2. Independently,
`IAvnTextBlock` grows the same fonts plus Background, LineSpacing, MaxLines and
TextWrapping, and moves from 4 to 5; `IAvnSelectableTextBlock` goes from 1 to 2.
`TextTrimming` is an abstract class without `ToString` and stays a gap. The
factory is unmoved at 13. Panels, Border, Image, shapes and flyouts are not
under TemplatedControl and keep their IIDs.

Wave N widens the TextBox leaf. `IAvnTextBox` grows SelectedText, content
alignments, selection/caret brushes, floating placeholder, inner left/right
content as `IAvnControl`, SelectAll/ClearSelection, and clipboard events, and
moves from 5 to 6. `IAvnMaskedTextBox` grows MaskCompleted/MaskFull and moves
from 2 to 3. PasswordChar, PromptChar, CaretBlinkInterval, GetLineCount and
the obsolete Watermark aliases stay gaps. The factory is unmoved.

Wave O adds `AvnVector` (`MarshallingKind.Vector`, ordinal 18 after Brush) as
two doubles identical in layout to `AvnPoint`. `IAvnComboBox` grows Text,
PlaceholderForeground, Clear, DropDownOpened and DropDownClosed, and moves
from 6 to 7. `IAvnScrollViewer` grows Extent/Viewport (`AvnSize`), Offset
(`AvnVector`), LargeChange/SmallChange and snap-point enums, and moves from
6 to 7. ComboBoxItem is unchanged. SelectionBoxItem, templates and
PasswordChar stay gaps. The factory is unmoved at 13.

Wave P projects instance Flyout on `IAvnButton`/`IAvnSplitButton` as
`IAvnFlyoutBase` (nullable COM). Button and every descendant move one version
(Button 6 to 7). SplitButton/MenuItem move from 2 to 3 and MenuItem gains
HasSubMenu, IsTopLevel, Open and Close. Control.ContextMenu, ContextFlyout,
IsLoaded and Loaded/Unloaded stay gaps: putting them on Control would
republish every control interface and is a wave of its own. ICommand and
HotKey stay gaps. The factory is unmoved at 13.

Wave overlay chrome grows `IAvnPopup` from 1 to 2 with Open/Close, Opened/Closed,
InheritsTransform, PlacementTarget, overlay-layer bools and IsPointerOverPopup.
`IAvnPopupFlyoutBase` (and Flyout/MenuFlyout) grow a read-only Popup COM slot
and move from 1 to 2. `IAvnContextMenu` grows PlacementTarget and moves from
2 to 3. PlacementRect is nullable geometry; PlacementAnchor/Gravity/ConstraintAdjustment
are flags enums whose combined values have no name — both stay gaps. Factory 13.

ItemsControl leftovers keep `Items`, `SelectedIndex` and `SelectionChanged` and
append `ItemCount` plus `ScrollIntoView(int)` on `IAvnItemsControl` (5 to 6).
`IAvnSelectingItemsControl` grows AutoScrollToSelectedItem, IsTextSearchEnabled
and WrapSelection (5 to 6). Every ItemsControl descendant republishes:
ComboBox 7 to 8, ListBox 6 to 7, ContextMenu 3 to 4, MenuItem 3 to 4,
Menu/MenuBase/Headered* 2 to 3, TabControl/TableView/TreeView/TreeViewItem/
Carousel 2 to 3. ContentControl and Window are unchanged. The object overload
of ScrollIntoView, templates and ItemsSource stay gaps. Factory 13.
Safe bindgen keeps the `is_` prefix on a bool builder when stripping it would
collide with a method on the same type (`Popup.IsOpen` vs `Open()`).

Text leftovers grow `IAvnTextBlock` from 5 to 6 with LineHeight and
BaselineOffset, `IAvnSelectableTextBlock` from 2 to 3 with selection brushes,
SelectAll, ClearSelection and CopyingToClipboard, and `IAvnTextBox` from 6 to 7
with ScrollToLine. MaskedTextBox republishes 3 to 4. GetLineCount returns int
so it stays a gap, as do PasswordChar, TextTrimming and Inlines. Factory 13.

Leaf leftovers close marshallable commands and scalars on existing types without
widening TemplatedControl or Panel. Carousel gains Next/Previous (3 to 4),
CommandBar overflow bools (2 to 3), DatePicker/TimePicker VerticalContentAlignment (2 to 3), NotificationCard
IsClosing/Close (2 to 3), ProgressBar Percentage (5 to 6), RefreshContainer
RequestRefresh (2 to 3), StackPanel snap bools and Border ClipToBoundsRadius
(4 to 5), TableViewColumn ActualWidth (1 to 2), PipsPager SelectedIndexChanged
(2 to 3). ComboBox content alignments stay off ComboBox because it redeclares
them with `new`. Label.Target is IInputElement. Factory 13.

Control leftovers grow `IAvnControl` from 3 to 4 with ContextMenu
(`IAvnContextMenu`), ContextFlyout (`IAvnFlyoutBase`), IsLoaded, Loaded and
Unloaded. Every Control descendant republishes, including previously unpinned
v1 types now at 2 (Image, layout panels, shapes). StyledElement, FlyoutBase,
MenuFlyout and TableViewColumn are unchanged. Factory 13.

Non-void methods project as HRESULT plus an `out` slot. `GetLineCount` is the
first: `IAvnTextBox` 8 to 9, `IAvnMaskedTextBox` 5 to 6. Factory 13.

`MarshallingKind.CharUtf16` is a UTF-16 code unit (`uint16_t`). PasswordChar and
PromptChar project on TextBox (9 to 10) and MaskedTextBox (6 to 7). Factory 13.

`[Flags]` enums project as `I32` bitmasks with named constants, not closed Rust
enums. PlacementAnchor, PlacementGravity and PlacementConstraintAdjustment
append on Popup (3 to 4), ContextMenu (5 to 6), and PopupFlyoutBase plus
descendants Flyout and MenuFlyout (2 to 3). Factory 13.

Nullable geometry crosses as `AvnOptional{Kind}` (`has_value` plus the existing
blittable struct). PlacementRect appends on Popup (4 to 5) and ContextMenu
(6 to 7). Factory 13. Non-nullable geometry slots stay `AvnRect` by value.

Window.Closing projects as a field payload: Cancel is in/out, CloseReason and
IsProgrammatic are inbound. `IAvnWindow` 9 to 10. Factory 13.

Thumb drag events carry `AvnVector` by value. `IAvnThumb` and `IAvnGridSplitter`
3 to 4. Factory 13.

TextTrimming crosses as a UTF-16 name via a host converter. `IAvnTextBlock` 7
to 8, `IAvnSelectableTextBlock` 4 to 5. Factory 13.

Label.Target projects as nullable `IAvnControl` (`IInputElement` values that
are Controls). `IAvnLabel` 3 to 4. Factory 13.

`IAvnCommand` wraps `System.Windows.Input.ICommand` (`Execute` / `CanExecute`,
no CommandParameter). Button and descendants 8 to 9 (or 4 to 5 for the wave D
leaves), MenuItem 5 to 6, SplitButton/ToggleSplitButton 4 to 5, TrayIcon 1 to 2.
Factory 13.

Inbound commands close the loop: `avalonia_sys::command(execute, can_execute)`
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
Button family 9 to 10 (Button/ToggleButton/CheckBox/RadioButton/ToggleSwitch),
MenuItem 6 to 7, SplitButton and ToggleSplitButton 5 to 6, TrayIcon 2 to 3,
each republished under its fresh deterministic IID. Factory stays 13.

`IAvnCommand` moves to ABI version 2 so the parameter reaches the command:
`execute(AvnVariant parameter)` and `can_execute(AvnVariant parameter,
int32_t* value)` replace their parameterless v1 shapes, and the IID is
republished accordingly. The host's `AvnCommand` frees a UTF-16 payload
after converting it to the managed parameter; the Rust CCW does the same
after the closure returns. The Rust `command(execute, can_execute)`
closures now take the raw `AvnVariant`, and `CommandParameter` set on the
control flows end to end: `Button.CommandParameter` → managed
`ICommand.Execute(parameter)` → Rust closure argument.

Wave U13 closes CommandBar lifecycle and ScrollViewer anchoring.
CommandBar Opening/Opened/Closing/Closed cross as no-payload events
(CommandBar 4 to 5). ScrollViewer gains `ScrollBarMaximum` (a Vector,
read-only direct property), `CurrentAnchor` (a nullable `IAvnControl`),
and `RegisterAnchorCandidateWithControl`/`UnregisterAnchorCandidateWithControl`
taking an `IAvnControl` (ScrollViewer 8 to 9). TabItem.TabStripPlacement
stays a gap: `Dock?` is a nullable enum and the ABI has no nullable-int
shape yet; a future `NullableI32` kind would unblock it.

Wave U14 closes ContextMenu's open lifecycle (ContextMenu 7 to 8).
`Open(Control?)` crosses as `OpenWithControl` — the safe layer takes
`Option<&impl AsControl>` — and Opening/Closing cross with the Cancel
write-back payload, the same `ref int Cancel` shape as PopupFlyoutBase and
Window. The extractor now suppresses a method that overrides an already
projected base method (ContextMenu's `Open()` override of MenuBase's is
reported as a gap with that reason instead of duplicating the base slot),
and the safe layer honors nullable `ComInterface` method parameters in
both directions.

Wave U15 closes AutoCompleteBox's populate lifecycle (AutoCompleteBox 4 to
5). `MinimumPopulateDelay` introduces `MarshallingKind.TimeSpanI64`
(ordinal 22): a `TimeSpan` crosses as its int64 tick count, and the safe
layer exposes `std::time::Duration` (100ns per tick). TextChanged crosses
with no payload, and Populating crosses with the Cancel write-back plus
the `Parameter` text; DropDownOpening/DropDownClosing carry the Cancel
write-back. ItemTemplate, ValueMemberBinding, SelectedItem, the filter
and selector delegates, AsyncPopulator and ItemsSource stay gaps: they
are templates, bindings, delegates and an untyped object collection.

Wave U16 grows the base interfaces, so every control republishes.
StyledElement 3 to 4 adds DataContextChanged (no payload); Control 4 to 5
adds SizeChanged, whose handler carries NewSize and PreviousSize as
`AvnSize` values — resize feedback reaches Rust on every control.
SplitView 3 to 4 adds PaneOpening/PaneClosing (Cancel write-back) beside
its existing PaneOpened/PaneClosed; Calendar 4 to 5 adds DisplayModeChanged
(OldMode/NewMode as i32); NotificationCard 4 to 5 adds NotificationClosed
(no payload). Every StyledElement and Control descendant — all 100+
control interfaces — bumps once and republishes under a fresh
deterministic IID; Popup, TrayIcon and the flyout family do not derive
from Control and keep their versions. Factory stays 13.

Wave U17 widens the scalar and container surface. Two new kinds:
`DateTimeI64` (ordinal 23) carries a DateTime as its int64 tick count,
nullable as `AvnOptionalDateTime { has_value, ticks }` (SystemTime in the
sys crate, with the .NET epoch offset); `PixelPointI32` (ordinal 24)
carries an `Avalonia.PixelPoint` as the blittable `AvnPixelPoint { x, y }`
(a `PixelPoint` safe struct). Window gains Position (Window 11 to 12);
TextBox gains CaretBlinkInterval via the TimeSpan shape (TextBox 11 to
12, MaskedTextBox and AutoCompleteBox republish); Calendar gains
DisplayDateChanged carrying RemovedDate/AddedDate (Calendar 5 to 6).
Methods may now return a COM interface: ItemsControl gains
`ContainerFromIndexWithInt32` (returns a nullable `IAvnControl` as
`Result<Option<Control>>`) and `IndexFromContainerWithControl` taking
`&impl AsControl` (ItemsControl and every items descendant 8 to 9). The
object-keyed lookups stay gaps with the rest of the untyped-collection
family. Factory stays 13.

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
Versions: the whole items lineage moves once more (ItemsControl 9 to 10
with every descendant), AutoCompleteBox 7 to 8, MenuFlyout 3 to 4,
TabControl 6 to 7, TreeView 6 to 7, Window 12 to 13, WindowNotification-
Manager 4 to 5. Factory stays 13.

Wave U19 projects data templates, closing the templating family.
`MarshallingKind.DataTemplate` (ordinal 25) wraps
`Avalonia.Controls.Templates.IDataTemplate` as `IAvnDataTemplate`:
`match(AvnVariant data)` reports whether the template builds for an item
and `build` hands back the control. The host's `AvnDataTemplate` wraps a
managed template; a foreign `IAvnDataTemplate` converts back through the
`DataTemplateAdapter`, so a Rust CCW — `avalonia_sys::data_template(
matches, build)` or the safe `avalonia::data_template` — really renders
items. ItemTemplate crosses on ItemsControl, MenuFlyout, AutoCompleteBox
and ComboBox (SelectionBoxItemTemplate); HeaderTemplate on
HeaderedContentControl, HeaderedItemsControl, HeaderedSelectingItemsControl
and TableViewColumn; ContentTemplate on ContentControl, TabControl and
Flyout; ToggleSwitch gains On/OffContentTemplate. IR schema moves 15 to 16
for the paired `templateInterfaceName`/`templateInterfaceIid`. The control
tree republishes once more (ContentControl 8 to 9 cascades everywhere; the
items lineage reaches 11), Flyout 3 to 4, MenuFlyout 4 to 5, Window 13 to
14. Control.DataTemplates (the default-template list),
TemplatedControl.Template (the control template, not a data template) and
the TemplateSettings getters stay gaps by design. Factory stays 13.

Wave U20 projects the styled-element lifecycle and the payload events.
StyledElement gains IsInitialized, Parent, DataContext and TemplatedParent
(as a Variant) plus Initialized, ActualThemeVariantChanged, ResourcesChanged
and Attached/DetachedFromLogicalTree as no-payload events (StyledElement 4
to 5, cascading the whole control tree once more). NumericUpDown gains
Spinned carrying Direction/UsingMouseWheel; CalendarDatePicker gains
DateValidationError carrying Text and the ThrowException write-back
(both at their republished versions). TabItem gains Icon (Variant) and
IconTemplate (DataTemplate); TabControl gains IndicatorTemplate and
SelectedContentTemplate; ComboBox gains SelectionBoxItem (Variant).
Nullable event string fields now read as Option<String> with a
null-tolerant clone. Factory stays 13.

Wave U21 projects icons as write-oriented path strings. Window.Icon and
TrayIcon.Icon cross as UTF-16 paths through the host-side `AvnWindowIcon`
converter, which loads a `WindowIcon` from the path; reading yields null
because an icon carries no source path to return (Window 15 to 16,
TrayIcon 4 to 5). Factory stays 13.

Wave U22 projects the shape and typography scalar lists as text. Polygon
and Polyline Points cross as a UTF-16 "x,y x,y" list through the
host-side `AvnPointList` converter; Shape.StrokeDashArray crosses as
comma-separated doubles through `AvnDoubleList`; TextBlock and
TemplatedControl FontFeatures cross through `AvnFontFeatures` (the
collection's Parse exists but its ToString is inherited, so the converter
owns both halves). The templated-control tree and the shape family
republish under their fresh IIDs. Factory stays 13.

Wave U23 projects the calendar date collections. Calendar.SelectedDates
and BlackoutDates, and CalendarDatePicker.BlackoutDates, cross as the
host-implemented `IAvnDateTimeList` whose elements are int64 DateTime
tick counts: the `AvnDateList` adapter reads the live collection and
writes through to it because the calendar owns the storage (Calendar 9
to 10, CalendarDatePicker 8 to 9). The safe crate exposes
SystemTime elements with the .NET epoch offset. The
SelectedDatesChanged/SelectedDateChanged events stay gaps: their
SelectionChangedEventArgs carry added/removed collections the event
payload cannot carry yet. Factory stays 13.

Wave U24 projects AutoCompleteBox's filter delegates. Two kinds:
`ItemFilter` (ordinal 26) wraps the object predicate as `IAvnItemFilter`
whose invoke takes the search text and the item variant; `TextFilter`
(ordinal 27) wraps the string predicate as `IAvnTextFilter` over two
borrowed UTF-16 buffers. The host's `AvnItemFilter`/`AvnTextFilter` wrap
managed delegates; a foreign interface converts back into the delegate, so
the Rust CCWs — `avalonia_sys::item_filter`/`text_filter` — really filter
items (AutoCompleteBox 11 to 12). The selector delegates and the async
populator stay gaps: selectors add another callback shape and the
populator needs the async-completion transport. CCWs now free variant
payloads only when a host is loaded. Factory stays 13.

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
ABI and a callback transport. The templated-control tree and the shape
family republish under their fresh IIDs. Factory stays 13.

Wave Q sweeps leftover marshallable scalars on leaf input types.
`IAvnAutoCompleteBox`, `IAvnCalendar`, `IAvnCalendarDatePicker` and
`IAvnNumericUpDown` each move from 2 to 3. Templates, filters, ItemsSource,
SelectedDates, NumberFormat and obsolete Watermark aliases stay gaps.
The factory is unmoved at 13.

Wave U26 projects typed notifications, closing the last pending gaps.
`Notification` (ordinal 28) maps `INotification` to `IAvnNotification`
whose getter slots carry the title, message, type, expiration and the
OnClick/OnClose handler interfaces (`IAvnNotificationActionHandler`);
the host's `AvnNotification` wraps a foreign interface as a managed
`INotification`, reading the handlers through getter slots into managed
actions. The bindgen emits the handler CCW through the event-callback
pattern and RCW getters on `IAvnWindowNotificationManager`, and the
Rust side grows a ref-counted `avalonia_sys::notification` CCW whose
string getters allocate through the host's UTF-16 provider. The INotification
content overloads and the Action-carrying Show overload cross through
the notification CCW's handler slots; every remaining gap in the report
is now by-design with its rationale. Factory stays 13.

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
grows a slot, so it and all 97 descendants republish under fresh IIDs.
The gap report drops from 190 to 182 entries, every one by-design. Factory
stays 13.

Wave U28 crosses the scalar event payloads. The Fields payload grows two
nullable wrappers: `AvnOptionalTimeSpan` (has_value + ticks, converting
through `core::time::Duration`) joins `AvnOptionalDateTime`, and
DateTimeOffset event fields convert through `.UtcDateTime` so the wire form
stays UTC ticks. DatePicker.SelectedDateChanged crosses with the
old/new date pair, TimePicker.SelectedTimeChanged with the old/new time
pair, and TextBox.TextChanging crosses as a plain notification because
TextChangingEventArgs carries no payload members. TextBox (17) and its
MaskedTextBox descendant (14) republish, and the pickers move to 10.
CalendarDatePicker.SelectedDateChanged stays by-design: it carries
SelectionChangedEventArgs, whose added/removed collection payload needs the
variant-list event shape. The gap report drops to 179 entries. Factory
stays 13.

Wave U29 projects AutoCompleteBox's selector delegates. Two kinds:
`ItemSelector` (ordinal 29) wraps the object selector as `IAvnItemSelector`
whose invoke takes the search text and the item variant and returns the
display text as a host-allocated UTF-16 buffer; `TextSelector` (ordinal 30)
wraps the string selector as `IAvnTextSelector` over two borrowed buffers.
The host's `AvnItemSelector`/`AvnTextSelector` wrap managed delegates; a
foreign interface converts back into the delegate, so the Rust CCWs —
`avalonia_sys::item_selector`/`text_selector` — really format items
(AutoCompleteBox 13 to 14). The CCWs allocate their returned string through
the host's UTF-16 provider and report E_NOTIMPL without a host, exactly as
the notification CCW's string getters do. The gap report drops to 177
entries. Factory stays 13.

Wave U30 projects the object lists and pass-through members. CommandBar's
PrimaryCommands and SecondaryCommands cross as a new IAvnControlList — a
host-implemented live adapter (AvnControlList) working through the
non-generic IList the observable command collections implement, so reads
observe the bar and mutations persist into it; VisiblePrimaryCommands and
OverflowItems ride the same adapter as read-only views. Control.Tag crosses
as a Variant, which grows Control's vtable, so Control and all 95
descendants republish under fresh IIDs. Popup and PopupFlyoutBase's
OverlayInputPassThroughElement cross as a control reference. The gap
report drops to 170 entries. Factory stays 13.

Wave U31 crosses the transition payload. TransitioningContentControl's
TransitionCompleted rides the Fields machinery with two Variants (From/To)
and the completion flag — the event-argument emitters gain a Variant case,
on both the C# side (AvnVariant.FromObject) and the bindgen
(event_argument_type), so any future object?-valued event field crosses the
same way. A transition never runs in the headless host, so the payload is
validated by the advise/unadvise pair and the generated slot signature.
TransitioningContentControl republishes at 10. The gap report drops to 169
entries. Factory stays 13.

Wave U32 crosses the collection event payloads with a new
`EventPayloadKind.Args`: the handler receives a host-implemented args
interface whose `Get{Name}Count`/`Get{Name}At` slots expose each projected
collection property as Variants, live over the CLR event args. Four events
cross — AutoCompleteBox.SelectionChanged and Populated,
Calendar.SelectedDatesChanged, CalendarDatePicker.SelectedDateChanged —
and the bindgen emits the args interface with safe count/getter methods on
the Rust side (`subscribe_selection_changed` receives a
`ComPtr<IAvnSelectionChangedArgs>`-shaped wrapper). AutoCompleteBox (15),
Calendar (12) and CalendarDatePicker (11) republish. The gap report drops
to 165 entries. Factory stays 13.

Wave U33 sweeps the column list and reason hygiene. TableView.Columns
crosses through the same IAvnControlList live adapter as the command-bar
lists — AvaloniaList<TableViewColumn> is an IList of projected controls
(TableView 12 to 13). The stale "INotification content" reasons on
ContextMenu/Menu's Close overwrites are corrected to name the real cause:
they suppress the projected base slot. The gap report drops to 164
entries. Factory stays 13.

Wave U34 crosses the pointer-tracking flyout overload. The insight:
`showAtPointer` is only a placement-mode switch, not pointer state, so
PopupFlyoutBase.ShowAt(Control, Boolean) crosses as an ordinary method
(`ShowAtWithControlAndBoolean`) with no payload shape at all. The sealed
ShowAt(Control) and Hide overrides of the projected base slots stay
suppressed, now with accurate reasons ("overrides the projected base
slot" / "the sealed override of the projected base Hide slot") instead of
the stale pointer-event wording. PopupFlyoutBase republishes at 5; a
stale wave-B forbidden pin that banned MenuItem's gesture setters is
flipped to require them now that U27 projects the gestures. The gap
report drops to 163 entries. Factory stays 13.

Wave U35 crosses the custom popup placement callback. `PopupPlacement`
(ordinal 31) maps `CustomPopupPlacementCallback` to
`IAvnPopupPlacementCallback`, whose invoke passes the popup size and
anchor rectangle in and the mutated placement — offset, anchor, gravity,
constraint adjustment — back through out-parameters. The host's
`AvnPopupPlacementCallback` wraps the managed delegate both ways: a
foreign CCW converts into the delegate (the host constructs the
placement record through the new Avalonia.Host InternalsVisibleTo entry,
calls it, and reads the mutations back), and Popup, ContextMenu and
PopupFlyoutBase's CustomPopupPlacementCallback properties cross (the
flyout base republishes at 6, Popup at 8, ContextMenu at 15). The Rust
CCW — `avalonia_sys::popup_placement` — returns a
`PopupPlacementResult`. The gap report drops to 160 entries. Factory
stays 13.

Wave U36 crosses the async populator. `AsyncPopulator` (ordinal 32) maps
AutoCompleteBox's `Func<string?, CancellationToken,
Task<IEnumerable<object?>>`? to `IAvnAsyncPopulator`:
`BeginPopulate(requestId, IAvnAsyncPopulatorCompletion, searchText)`
launches, and the CCW reports through the completion's
`Complete(requestId, hresult, IAvnVariantList)` slot. The host wrapper
bridges both directions: a managed delegate runs under
`AvnPopulatorBridge` and reports through a host-built completion CCW,
while a foreign CCW converts into the delegate through a
`TaskCompletionSource` the completion resolves (AutoCompleteBox 15 to
16). The Rust CCW — `avalonia_sys::async_populator` — hands the closure a
`PopulateCompletion` reporter. ShowDialog stays by-design with an
accurate reason: its modal completion needs the dialog-owner lifetime the
projected Show overloads do not carry. The gap report drops to 159
entries. Factory stays 13.

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
host-side detail only: the ABI slot is an ordinary UTF-16 string, so neither the
native header nor the Rust bindings read it, and a consumer that ignores it sees
no difference.

## RID artifacts

Release artifacts are per RID and retain their exact names:
`Avalonia.Host.dll` (`win-*`), `Avalonia.Host.so` (`linux-*`), and
`Avalonia.Host.dylib` (`osx-*`). Each artifact directory contains the matching
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

Official release artifacts must be signed with the platform-appropriate
signing identity before the SBOM and checksums are generated. The optional
`AVALONIA_RUST_SIGN_COMMAND` hook is a wrapper executable/script path that
receives an artifact path as a separate argument; unsigned local developer
artifacts are not release artifacts.

The Rust host and standalone Rust artifacts are not NuGet packages, so
`nukebuild/SbomGenerator.cs` remains unchanged. Their per-RID delivery scope
is instead represented by `rust/generate-sbom.ps1` in `sbom.cdx.json`, including
the host, Rust executable, bundled native binaries, and licence with SHA-256
hashes after signing.

## File type associations

Registering a document type is packaging metadata owned by whatever installer
or store channel a consumer uses. The application template ships copyable
Windows/Linux/macOS snippets (see
[DESKTOP_FILES.md](DESKTOP_FILES.md#file-type-associations)); this workflow
deliberately does not introduce MSIX packaging or platform installers, and the
snippets add nothing to the delivered per-RID bundle, so the SBOM delivery
scope is unchanged.
