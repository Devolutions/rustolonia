# Value-type and solid brush marshalling

Avalonia geometry value types cross the nano-COM ABI **by value** as blittable,
sequential structs instead of as COM objects. This keeps `Margin`, `Padding`,
`BorderThickness`, `CornerRadius` and friends cheap to read and write, and it
avoids a COM object per rectangle. Brushes are the one chrome member that is
*not* a value type: they cross as a small read-only COM interface that carries a
solid colour and an opacity. `Grid`'s track definitions are the one member that
crosses as a **string**: they use the same comma-separated length list Avalonia
already parses and prints (see [Grid track definitions](#grid-track-definitions)).

## Nullable safe inputs

Nullable interface-valued setters and builders accept `Option`: pass
`Some(&control)` to assign a control, or `None` to clear it. Root-control
properties accept `Option<&dyn AsControl>`; more specific interfaces accept
`Option<&SpecificType>`. Nonnullable interface properties still require a value.
For example, `button.set_content(None)?` clears the button's content.

String method inputs accept `impl AsRef<str>` when required and `Option<&str>`
when nullable. The generated wrapper keeps null-terminated UTF-16 buffers alive
through the ABI call; `None` remains a null pointer, distinct from an empty string.

## The structs

Every struct is `LayoutKind.Sequential` / `#[repr(C)]`, contains only `double`
(`f64`) or `uint32_t` (`u32`) fields, and therefore has an identical layout in
the C header, the C# host, and Rust.

| `MarshallingKind` | CLR type               | ABI struct        | Safe Rust type | Fields (in order)                          | Size |
| ----------------- | ---------------------- | ----------------- | -------------- | ------------------------------------------ | ---- |
| `Thickness`       | `Avalonia.Thickness`   | `AvnThickness`    | `Thickness`    | `left`, `top`, `right`, `bottom`           | 32   |
| `CornerRadius`    | `Avalonia.CornerRadius`| `AvnCornerRadius` | `CornerRadius` | `top_left`, `top_right`, `bottom_right`, `bottom_left` | 32 |
| `Size`            | `Avalonia.Size`        | `AvnSize`         | `Size`         | `width`, `height`                          | 16   |
| `Point`           | `Avalonia.Point`       | `AvnPoint`        | `Point`        | `x`, `y`                                   | 16   |
| `Rect`            | `Avalonia.Rect`        | `AvnRect`         | `Rect`         | `x`, `y`, `width`, `height`                | 32   |
| `Color`           | `Avalonia.Media.Color` | `AvnColor`        | `Color`        | `argb` (packed `u32`)                      | 4    |

`AvnColor` carries a single packed **ARGB** integer that matches
`Avalonia.Media.Color.ToUInt32()` exactly: `(A << 24) | (R << 16) | (G << 8) | B`.
The safe Rust `Color` splits it back into `a`, `r`, `g`, `b` bytes; converting in
either direction is lossless.

## Where each declaration comes from

Everything is generated from the shared projection IR by
`rust/regenerate-and-build.ps1`:

- `projection/Avalonia.Projection.Ir/GeometryMarshalling.cs` is the managed source of
  truth (kind, CLR type, ABI name, field list, conversion style).
- `projection/Avalonia.Projection.Ir/BrushMarshalling.cs` is the equivalent for the
  solid brush interface (CLR types, ABI interface name, factory method name).
- `ComSourceEmitter` writes the C# structs into
  `host/Generated/ObjectModel/ProjectionStructs.g.cs`, each with
  `FromAvalonia`/`ToAvalonia` helpers, and the brush interface plus its
  `AvnBrush` wrapper into `IAvnBrush.g.cs`.
- `NativeHeaderEmitter` writes the `typedef struct Avn*` declarations into
  `rust/avalonia-sys/include/avalonia-rust-abi.h`.
- `rust/avalonia-bindgen/src/geometry.rs` is the Rust-side source of truth; it
  emits the `#[repr(C)]` structs into `avalonia-sys` and the ergonomic structs
  with `From`/`Into` bridges into the safe `avalonia` crate.

## ABI shape in vtables

A projected property of a geometry kind occupies the usual pair of slots and
passes the struct by value on the way in and by pointer on the way out:

```c
AvnHResult (AVN_CALL *get_margin)(IAvnControl* self, AvnThickness* value);
AvnHResult (AVN_CALL *set_margin)(IAvnControl* self, AvnThickness value);
```

## Layout members

The layout wave allowlists the first live geometry-carrying members, alongside
the scalar and enum layout members that belong with them:

| Projected interface | Members |
| ------------------- | ------- |
| `IAvnStyledElement` | `Name` (`string?`) |
| `IAvnControl`       | `Margin` (`Thickness`), `HorizontalAlignment`, `VerticalAlignment`, `MinWidth`, `MinHeight`, `MaxWidth`, `MaxHeight`, `IsVisible`, `Opacity` |
| `IAvnDecorator`     | `Padding` (`Thickness`) |
| `IAvnWindow`        | `CanResize`, `WindowState` |

`Avalonia.Controls.Control` has no `Padding`, so `Padding` sits on `Decorator`
(and therefore on `Border`), matching Avalonia's own hierarchy. Width and Height
were already on `Control`, so `Window` inherits them.

`HorizontalAlignment`, `VerticalAlignment` and `WindowState` are carried as
`int32_t` and projected into the safe crate as Rust enums with `TryFrom<i32>`.

In safe Rust each member is a `set_*`/`get_*` pair plus a chaining builder, and
`Thickness` gains `const` helpers so a caller never repeats a scalar:

```rust
let readout = TextBlock::new()?
    .text("Laid out from Rust")?
    .margin(Thickness::symmetric(12.0, 4.0))?
    .horizontal_alignment(HorizontalAlignment::Right)?
    .max_width(320.0)?;
readout.set_margin(Thickness::uniform(8.0))?;
```

`Thickness::uniform`, `Thickness::symmetric` and `CornerRadius::uniform` are
emitted from the `helpers` column of `avalonia-bindgen`'s geometry table, so a
new geometry struct opts into them declaratively.

## Solid brushes

`IBrush` is an interface, not a value type, and a real brush graph (gradient
stops, tile modes, drawings, visuals) has no blittable shape. Rather than
project that graph, the ABI projects the one case that chrome actually needs: a
**solid colour**.

`MarshallingKind.Brush` maps `Avalonia.Media.IBrush` onto a single generated COM
interface:

```c
struct IAvnBrushVtbl {
    /* IUnknown slots 0-2 */
    AvnHResult (AVN_CALL *get_color)(IAvnBrush* self, AvnColor* value);   /* slot 3 */
    AvnHResult (AVN_CALL *get_opacity)(IAvnBrush* self, double* value);   /* slot 4 */
};
```

`IAvnBrush` is deliberately **read-only**. The managed side hands out immutable
brushes that several controls may share, so a setter would let one caller repaint
another control. A brush is instead minted by the factory, which appends one slot:

```c
AvnHResult (AVN_CALL *create_solid_color_brush)(
    IAvnControlFactory* self, AvnColor color, double opacity, IAvnBrush** value);
```

Unlike the `create_*` control slots it has no dispatcher affinity, so it does not
verify UI-thread access.

### What crosses, and what fails

| Managed value                                   | `get_*`                                 | `set_*` |
| ----------------------------------------------- | --------------------------------------- | ------- |
| `null`                                          | `S_OK`, null pointer                    | clears the property |
| `ISolidColorBrush` (including `SolidColorBrush`) | `S_OK`, colour + opacity                | writes an `ImmutableSolidColorBrush` |
| gradient / drawing / visual brush                | `AVN_E_NONSOLIDBRUSH` (`0xA7A70002`)    | unreachable — the ABI cannot express one |

Reading a non-solid brush fails explicitly rather than degrading to a "nearest"
colour, because a silently wrong colour is worse than an error a caller can act
on. Inbound, every brush is materialised as
`Avalonia.Media.Immutable.ImmutableSolidColorBrush(color, opacity)`, so a set is
always a solid brush by construction and anything a source brush carried beyond
colour and opacity (transforms, gradient stops) is dropped rather than guessed.

`DrawingBrush` and `VisualBrush` remain out of scope for this ABI. A future wave
that needs them must add a separately named, separately versioned interface
rather than widening `IAvnBrush`, following the pattern gradients use below.

In safe Rust the brush is a plain value; only crossing the ABI needs the factory:

```rust
let accent = Brush::solid(Color::rgb(0x00, 0x7A, 0xCC));
let card = Border::new()?
    .background(accent)?
    .border_brush(Brush::new(Color::rgb(0xAA, 0xBB, 0xCC), 0.5))?
    .border_thickness(Thickness::uniform(1.0))?
    .corner_radius(CornerRadius::uniform(4.0))?;
assert_eq!(card.get_background()?, Some(accent));
card.set_border_brush(None)?;
```

Getters return `Result<Option<Brush>>` and setters take `impl Into<Option<Brush>>`,
so `None` clears the property and a `Brush` needs no `Some`. `Color::rgb` is a
`const` opaque-colour constructor, so a consumer can define a palette as
constants.

## Gradient brushes

Linear and radial gradients follow the sibling-interface pattern rather than
widening `IAvnBrush`: `IAvnGradientBrush` carries the members every gradient
shares (`Opacity`, `SpreadMethod`, `StopCount`, `Stops`), and
`IAvnLinearGradientBrush`/`IAvnRadialGradientBrush` each derive from it and add
their own geometry (`StartPoint`/`EndPoint`, or `Center`/`GradientOrigin`/
`RadiusX`/`RadiusY`). Every gradient object also answers `QueryInterface` for
plain `IAvnBrush`, so it can be assigned anywhere a solid brush could be —
reading `Color` back off it is what returns `AVN_E_NONSOLIDBRUSH`.

Stops cross the ABI as a fixed-capacity buffer rather than a COM collection:

```c
struct AvnGradientStop { double offset; AvnColor color; };
struct AvnGradientStopBuffer { AvnGradientStop stops[8]; };
```

Eight stops is enough for chrome gradients without a heap-allocated
marshalled array; `StopCount` says how many of the eight slots are populated,
so a wrapper must always read count and buffer together and never assume all
eight are meaningful. `IAvnControlFactory` mints both shapes:

```c
AvnHResult (AVN_CALL *create_linear_gradient_brush)(
    IAvnControlFactory* self, double opacity, int32_t spreadMethod,
    int32_t stopCount, AvnGradientStopBuffer stops,
    AvnRelativePoint startPoint, AvnRelativePoint endPoint,
    IAvnLinearGradientBrush** value);

AvnHResult (AVN_CALL *create_radial_gradient_brush)(
    IAvnControlFactory* self, double opacity, int32_t spreadMethod,
    int32_t stopCount, AvnGradientStopBuffer stops,
    AvnRelativePoint center, AvnRelativePoint gradientOrigin,
    AvnRelativeScalar radiusX, AvnRelativeScalar radiusY,
    IAvnRadialGradientBrush** value);
```

`AvnRelativePoint`/`AvnRelativeScalar` carry a `RelativeUnit` ordinal
(`Relative = 0`, `Absolute = 1`) alongside their coordinates, mirroring
`Avalonia.RelativePoint`/`RelativeUnit`. Growing `IAvnControlFactory` to add
these two slots required minting a new IID for it (the ABI-baseline check
treats any slot-list change under a published IID as a break, even an
additive one) — see [Versioning of the widened vtables](#versioning-of-the-widened-vtables)
for the general rule.

In safe Rust, `LinearGradientBrush` and `RadialGradientBrush` are plain values
alongside `Brush`, and `Paint` is the enum that spans all three shapes a
control property can hold:

```rust
let sunset = LinearGradientBrush::new(
    vec![
        GradientStop::new(0.0, Color::rgb(0xFF, 0x45, 0x00)),
        GradientStop::new(1.0, Color::rgb(0xFF, 0xD7, 0x00)),
    ],
    RelativePoint::new(0.0, 0.0, RelativeUnit::Relative),
    RelativePoint::new(1.0, 1.0, RelativeUnit::Relative),
);
let paint: Paint = sunset.into();
```

`Paint`, `LinearGradientBrush` and `RadialGradientBrush` are additive: no
existing `Brush`-typed property getter or setter was rewired to accept or
return `Paint`. Reading `Background`/`BorderBrush`/`Foreground` off a
gradient-backed control still fails with `AVN_E_NONSOLIDBRUSH` today, matching
"reads fail rather than degrade" above; wiring chrome properties to accept a
`Paint` is left to a future wave.

## Chrome members

The chrome wave allowlists the members that make a control look like part of an
application rather than an unstyled box:

| Projected interface     | Members |
| ----------------------- | ------- |
| `IAvnBorder`            | `Background`, `BorderBrush`, `BorderThickness` (`Thickness`), `CornerRadius` |
| `IAvnPanel`             | `Background` |
| `IAvnTemplatedControl`  | `Background`, `BorderBrush`, `BorderThickness`, `CornerRadius`, `FontSize`, `Foreground` |
| `IAvnTextBlock`         | `FontSize`, `FontWeight`, `Foreground`, `Padding` (`Thickness`), `TextAlignment` |

Each brush member sits on the type that declares it in Avalonia, so `Background`
is published once on `Panel` and once on `TemplatedControl` rather than being
duplicated onto `Grid`, `Button`, `Window` and the rest — a derived interface
that re-declared it would spend a second pair of slots on one property.
`FontWeight` and `TextAlignment` join the IR enums and project as Rust enums with
`TryFrom<i32>`. Rust enums cannot carry two names for one discriminant, so
Avalonia's weight aliases collapse onto the first declared name: write
`FontWeight::DemiBold` rather than `SemiBold`, and `FontWeight::Black` rather
than `Heavy`.

## Control completeness members

The completeness wave allowlists the remaining scalar and enum members that a
consumer needs before it has to reach for XAML, on types the object model already
projects:

| Projected interface   | Members |
| --------------------- | ------- |
| `IAvnContentControl`  | `HorizontalContentAlignment`, `VerticalContentAlignment` |
| `IAvnButton`          | `ClickMode`, `IsDefault`, `IsCancel`, `IsPressed` (read-only) |
| `IAvnToggleButton`    | `IsThreeState` |
| `IAvnListBox`         | `SelectionMode`, `SelectAll()`, `UnselectAll()` |
| `IAvnComboBox`        | `IsDropDownOpen`, `IsEditable`, `MaxDropDownHeight` |

As with every other wave, each member sits on the type that declares it in
Avalonia. `ComboBox` re-declares `HorizontalContentAlignment` and
`VerticalContentAlignment` with `new`, so they stay published once on
`ContentControl` rather than costing a second pair of slots.

`IsPressed` is a read-only direct property that Avalonia raises from its own
input handling, so the ABI publishes `get_is_pressed` and no setter. In safe Rust
a read-only member drops the `get_` prefix and has no builder: `button.is_pressed()?`.

`ClickMode` and `SelectionMode` join the IR enums. `SelectionMode` is a `[Flags]`
enum, and — like `KeyModifiers` — it projects as a plain Rust enum carrying only
the declared modes. A combined value such as `Multiple | Toggle` therefore cannot
be written from Rust, and reading one that XAML set reports
`Error::InvalidEnumValue`. Bitmask-shaped enums need their own representation and
are deliberately left to a later wave.

`Command`, `Flyout`, `ISelectionModel` and `PasswordChar` remain out of scope:
the first three need reference graphs the ABI does not marshal, and `char` is not
a marshalling kind.

## Grid track definitions

`Grid.ColumnDefinitions` and `Grid.RowDefinitions` are collections of
`ColumnDefinition`/`RowDefinition` objects, but Avalonia already has a canonical
textual form for them — the comma-separated length list AXAML writes — and both
collections own both halves of that conversion: `static Parse(string)` and an
overridden `ToString()`. The ABI publishes that string rather than a projected
collection:

| Projected interface | Members |
| ------------------- | ------- |
| `IAvnGrid`          | `ColumnDefinitions`, `RowDefinitions` (both UTF-16 strings) |

```c
AvnHResult (AVN_CALL *get_column_definitions)(IAvnGrid* self, uint16_t** value);
AvnHResult (AVN_CALL *set_column_definitions)(IAvnGrid* self, const uint16_t* value);
```

```rust
grid.set_column_definitions("*,Auto,120")?;
assert_eq!(grid.get_column_definitions()?, "1*,Auto,120");
```

The host wrapper converts on both sides: a get is `ColumnDefinitions.ToString()`
and a set is `ColumnDefinitions.Parse(value)`. This is expressed in the profile
as an ordinary marshalling override to `StringUtf16` on a member whose CLR type
is not `string`; the extractor refuses such an override unless the managed type
declares `static T Parse(string)` and overrides `ToString()`, so the emitter
never guesses at a conversion. No definition object, list interface or factory
slot is minted, which is why this costs `IAvnGrid` four slots and nothing else.

### What the string does and does not carry

The list is **normalising, not byte-preserving**. `*` is shorthand for `1*`, and
whitespace between entries is a separator, so writing `"*, Auto, 120"` and
reading it back reports `"1*,Auto,120"`. A list that has already been normalised
is a fixed point: writing what the getter returned changes nothing. An empty grid
reports an empty string rather than a null pointer, and writing an empty string
clears the tracks — there is no null definition list.

The list carries only each track's `Width`/`Height`, because that is all
`ColumnDefinitions.ToString()` prints. `MinWidth`, `MaxWidth`, `MinHeight`,
`MaxHeight` and `SharedSizeGroup` on an individual definition are **not** in
scope of this ABI, and a set replaces the whole collection, so anything a
previous definition carried beyond its length is dropped rather than merged. That
is a deliberate v1 boundary rather than an oversight: a per-definition surface
needs its own separately named, separately versioned interfaces and is left to a
later wave. A consumer that needs shared size groups should keep those tracks in
AXAML.

A malformed entry fails the call — `Parse` throws and the `HRESULT` carries it
back — rather than being silently dropped or rounded to a nearest length, and the
previous definitions stay in place.

## Image sources

`Image.Source` is an `IImage`, which is a managed *interface*: it has no fields,
no canonical text form and no ABI shape. Wave A projects it as the **source
string** the host resolves into a bitmap rather than as a projected image object:

| Projected interface | Members |
| ------------------- | ------- |
| `IAvnImage`         | `Source` (nullable UTF-16 string), `Stretch`, `StretchDirection`, `BlendMode` |

```c
AvnHResult (AVN_CALL *get_source)(IAvnImage* self, uint16_t** value);
AvnHResult (AVN_CALL *set_source)(IAvnImage* self, const uint16_t* value);
```

```rust
let image = Image::new()?
    .source(r"C:\assets\logo.png")?
    .stretch(Stretch::UniformToFill)?;
assert_eq!(image.get_source()?.as_deref(), Some(r"C:\assets\logo.png"));
```

Four spellings resolve: an absolute filesystem path, a path relative to the
process working directory, a `file://` URI, and an `avares://` or `resm:` URI
resolved through Avalonia's asset loader. Any other scheme — `http` included —
fails the call with `NotSupportedException` rather than silently doing nothing;
fetching over the network is the caller's job, and a byte-buffer entry point is
left to a later wave. An unreadable or undecodable file fails the call too, and
the previous source stays in place.

### What the string does and does not carry

Reading is narrower than writing, and this is the honest boundary of the v1
shape. A `Bitmap` does not remember where it came from, so the host remembers
instead: `AvnImageSource` keeps the string that produced an image in a weak table
and hands it back on read. Consequences:

* An image the ABI set reads back as **the exact string that was written** — not
  a normalised path, and not the bitmap's type name.
* An image the ABI never set reads back as **`null`**, even though the control is
  drawing it. An `Image` whose `Source` came from AXAML, from a style or from
  managed code has no source string to report.
* Writing an empty string or a null **clears** the source; there is no
  "empty path" state.
* Nothing is cached or deduplicated. Setting the same path twice decodes twice.

No `IAvnBitmap`, `IAvnImageSource` or image-list interface is minted, so this
costs `IAvnImage` two slots plus the three enums and nothing else. Projecting a
real image object — with pixel size, DPI, and construction from a byte buffer —
needs its own separately named, separately versioned interface and is left to a
later wave.

## ToolTip

`ToolTip` is projected for its **attached** properties, the same pipeline
`Canvas.Left` and `Grid.Row` use. The statics interface hangs off the factory:

| Statics interface     | Members |
| --------------------- | ------- |
| `IAvnToolTipStatics`  | `Tip` (nullable UTF-16 string), `IsOpen`, `Placement`, `HorizontalOffset`, `VerticalOffset`, `ShowDelay`, `BetweenShowDelay`, `ShowOnDisabled`, `ServiceEnabled` |

```c
AvnHResult (AVN_CALL *get_tip)(IAvnToolTipStatics* self, IAvnControl* target, uint16_t** value);
AvnHResult (AVN_CALL *set_tip)(IAvnToolTipStatics* self, IAvnControl* target, const uint16_t* value);
```

```rust
ToolTip::set_tip(&button, "Save the document")?;
ToolTip::set_show_delay(&button, 250)?;
assert_eq!(ToolTip::get_tip(&button)?.as_deref(), Some("Save the document"));
```

`ToolTip.Tip` is an `object` managed-side so that AXAML can hang a whole control
off it. Over the ABI it carries **text and nothing else**: a tip that is a control
reads back as `null` rather than as its type name, and an empty string clears the
tip rather than storing an empty tooltip. Control-as-tip is a later wave.

`ToolTip` itself is also projected as a control (`IAvnToolTip`, a
`ContentControl`) because the attached-property pipeline requires the owner type
to be in the projection; it adds no members of its own.

## Tabs and trees

| Projected interface | Base | Members it declares |
| ------------------- | ---- | ------------------- |
| `IAvnTabControl`    | `IAvnSelectingItemsControl` | `TabStripPlacement`, `HorizontalContentAlignment`, `VerticalContentAlignment` |
| `IAvnTabItem`       | `IAvnHeaderedContentControl` | `IsSelected` |
| `IAvnHeaderedItemsControl` | `IAvnItemsControl` | `Header` (as `IAvnControl`) |
| `IAvnTreeView`      | `IAvnItemsControl` | `AutoScrollToSelectedItem`, `SelectionMode`, `SelectAll`, `UnselectAll`, `ExpandSubTree`, `CollapseSubTree`, `SelectionChanged` |
| `IAvnTreeViewItem`  | `IAvnHeaderedItemsControl` | `IsExpanded`, `IsSelected`, `Level` (read-only), `Expanded`, `Collapsed` |

`Items` and `SelectedIndex` are **inherited**, not redeclared: `TabControl`
derives from `SelectingItemsControl` so it gets both, and `TreeView` derives from
`ItemsControl` so it gets `Items` but has **no** `SelectedIndex`. `TreeView`'s
own `SelectedItem`/`SelectedItems` are `object`/`IList` and stay in the gap
report; selecting a tree node from Rust is a later wave.

`TabItem.TabStripPlacement` is a `Dock?`. A nullable enum has no ABI shape — the
projection has `NullableBool` but no nullable-enum kind — so it is a gap rather
than a non-nullable `Dock` that silently loses the null. The `TabControl` writes
it anyway, so nothing is lost in practice.

`ExpandSubTree`/`CollapseSubTree` take a projected `IAvnTreeViewItem` and unwrap
it back to the Avalonia object, so a tree node crosses as a real control rather
than an opaque handle. `TreeView.SelectionChanged` is `TreeView`'s own event
rather than the `SelectingItemsControl` one, and like `TreeViewItem.Expanded`
and `TreeViewItem.Collapsed` it carries no payload: the handler is a bare
notification and the consumer reads back whatever it needs.

## Flyouts

A `Flyout` is not a `Control`. `FlyoutBase` derives from `AvaloniaObject`, so
`IAvnFlyoutBase` hangs directly off `IAvnAvaloniaObject`, implements no
`AsControl`, and is never a child of a panel:

| Projected interface   | Base                  | Members it declares |
| --------------------- | --------------------- | ------------------- |
| `IAvnFlyoutBase`      | `IAvnAvaloniaObject`  | `IsOpen`, `Target` (read-only, as `IAvnControl`), `ShowAt`, `Hide`, `Opened`, `Closed` |
| `IAvnPopupFlyoutBase` | `IAvnFlyoutBase`      | `Placement`, `ShowMode`, `HorizontalOffset`, `VerticalOffset`, `OverlayDismissEventPassThrough`, `Opening`, `Closing` |
| `IAvnFlyout`          | `IAvnPopupFlyoutBase` | `Content` (as `IAvnControl`) |

```c
AvnHResult (AVN_CALL *show_at_with_control)(IAvnFlyoutBase* self, IAvnControl* placement_target);
AvnHResult (AVN_CALL *hide)(IAvnFlyoutBase* self);
```

```rust
let flyout = Flyout::new()?
    .content(Some(&TextBlock::new()?.text("Pick one")?))?
    .placement(PlacementMode::BottomEdgeAlignedLeft)?
    .show_mode(FlyoutShowMode::Transient)?;
flyout.show_at_with_control(&button)?;
assert!(flyout.get_is_open()?);
flyout.hide()?;
```

### Why there is no attached flyout

`FlyoutBase.AttachedFlyout` and `Button.Flyout` are both **gaps**, and this is the
honest boundary of the wave rather than an oversight. The attached-property
pipeline carries scalars and strings only: a COM-valued attached property has no
ABI shape here, and `IAvnControlFactory` mints no `IAvnFlyoutBaseStatics`. What
crosses instead is `ShowAt`, which takes any projected control and unwraps it back
to the Avalonia object, so a flyout still reaches a button — imperatively, which is
what a Rust host is doing anyway. Attaching a flyout declaratively is a later wave.

`ShowAt(Control, bool)` — the show-at-pointer overload — does not cross either;
only the single-argument form does. `Target` is read-only because Avalonia sets it
from `ShowAt`, and a flyout that has never been shown reports `None` rather than a
placeholder.

`PlacementAnchor`, `PlacementGravity` and `PlacementConstraintAdjustment` are
`[Flags]` enums. A combined value has no name, and the projection's enum shape is a
closed set of named values that fails a `try_from` on anything else, so all three
stay in the gap report rather than crossing as an enum that cannot round trip.
`CustomPopupPlacementCallback` and `OverlayInputPassThroughElement` are a delegate
and an `IInputElement`, and neither has an ABI shape.

`Closing` is the only wave B event with a payload. `Cancel` is an in/out field,
exactly like `Control.KeyDown`'s `Handled`: a handler vetoes the close by writing
it back rather than by returning a magic HRESULT.

```rust
flyout.subscribe_closing(|arguments| arguments.cancel = !ready_to_close())?;
```

## Menus as controls

This is the **imperative** menu, distinct from the view-model `NativeMenu` in
[MENUS.md](MENUS.md). A `Menu` is an `ItemsControl` whose items are real projected
controls, built and driven from Rust:

| Projected interface                 | Base                                | Members it declares |
| ----------------------------------- | ----------------------------------- | ------------------- |
| `IAvnMenuBase`                      | `IAvnSelectingItemsControl`         | `IsOpen` (read-only), `Open`, `Close`, `Opened`, `Closed` |
| `IAvnMenu`                          | `IAvnMenuBase`                      | nothing — it inherits all of it |
| `IAvnHeaderedSelectingItemsControl` | `IAvnSelectingItemsControl`         | `Header` (as `IAvnControl`) |
| `IAvnMenuItem`                      | `IAvnHeaderedSelectingItemsControl` | `Icon` (as `IAvnControl`), `IsSelected`, `IsSubMenuOpen`, `StaysOpenOnClick`, `ToggleType`, `IsChecked`, `GroupName`, `Click`, `SubmenuOpened` |

```rust
let menu = Menu::new()?.item(
    MenuItem::new()?
        .header(Some(&TextBlock::new()?.text("File")?))?
        .item(
            MenuItem::new()?
                .header(Some(&TextBlock::new()?.text("Save")?))?
                .toggle_type(MenuItemToggleType::CheckBox)?
                .checked(true)?,
        )?,
)?;
menu.open()?;
assert!(menu.is_open()?);
menu.close()?;
```

`Items` and `SelectedIndex` are **inherited** from `ItemsControl` and
`SelectingItemsControl`; `IsEnabled` is inherited from `Control`. `MenuBase.IsOpen`
has a protected setter managed-side, so the ABI publishes a getter and no setter:
opening a menu goes through `Open`, not through assigning a flag.

`Menu` overrides `Open`/`Close` but declares nothing new, so the flattened vtable
publishes each exactly once, from `IAvnMenuBase`.

What does **not** cross: `MenuItem.Command` is an `ICommand`,
`MenuItem.CommandParameter` is an `object`, and `HotKey`/`InputGesture` are
`KeyGesture`s. All four are gaps. `Click` is the equivalent that does cross, and it
carries no payload — the handler is a bare notification and the consumer reads back
whatever it needs.

## SplitView

| Projected interface | Base                | Members it declares |
| ------------------- | ------------------- | ------------------- |
| `IAvnSplitView`     | `IAvnContentControl` | `IsPaneOpen`, `DisplayMode`, `PanePlacement`, `OpenPaneLength`, `CompactPaneLength`, `Pane` (as `IAvnControl`), `PaneBackground` (as `IAvnBrush`), `UseLightDismissOverlayMode`, `PaneOpened`, `PaneClosed` |

```rust
let split_view = SplitView::new()?
    .display_mode(SplitViewDisplayMode::CompactOverlay)?
    .pane_placement(SplitViewPanePlacement::Left)?
    .open_pane_length(220.0)?
    .pane(Some(&StackPanel::new()?.child(TextBlock::new()?.text("Pane")?)?))?
    .content(Some(&TextBlock::new()?.text("Body")?))?;
split_view.set_pane_open(true)?;
```

`Pane` is an `object` managed-side and crosses as a control, the same shape
`ContentControl.Content` already uses; `Content` itself is inherited from
`IAvnContentControl`. `PaneOpening`/`PaneClosing` carry a cancellable
`CancelRoutedEventArgs`; they are gaps in this wave, so a pane opens and closes
without a veto point. `PaneTemplate` is an `IDataTemplate` and `TemplateSettings` is
a template-only object; neither crosses.

## Dates and times

`DateTimeOffset` and `TimeSpan` have **no ABI shape** in this projection: there is
no date struct, no epoch integer and no tick count. A date crosses as an ISO-8601
string through the same host-side converter mechanism `Image.Source` uses, and the
parse rules are part of the contract rather than whatever the ambient culture does.

| Projected interface | Base                  | Members it declares |
| ------------------- | --------------------- | ------------------- |
| `IAvnDatePicker`    | `IAvnTemplatedControl` | `SelectedDate` (nullable), `MinYear`, `MaxYear`, `DayVisible`, `MonthVisible`, `YearVisible`, `DayFormat`, `MonthFormat`, `YearFormat`, `Clear` |
| `IAvnTimePicker`    | `IAvnTemplatedControl` | `SelectedTime` (nullable), `MinuteIncrement`, `SecondIncrement`, `ClockIdentifier`, `UseSeconds`, `Clear` |

```rust
let date_picker = DatePicker::new()?.selected_date("2027-01-15")?;
// Reading is normalising: the round-trip "o" form, not the string that was written.
assert_eq!(
    date_picker.get_selected_date()?.as_deref(),
    Some("2027-01-15T00:00:00.0000000-05:00")
);

let time_picker = TimePicker::new()?.selected_time("17:04")?;
assert_eq!(time_picker.get_selected_time()?.as_deref(), Some("17:04:00"));
```

### What the string does and does not carry

**Dates** (`AvnDateTimeOffset`, `AvnDateTimeOffsetValue`):

* Reading always produces the invariant round-trip `"o"` form,
  `yyyy-MM-ddTHH:mm:ss.fffffffK`. It is **normalising, not byte-preserving**: what
  is read back is rarely the exact string that was written.
* Writing accepts exactly these ISO-8601 spellings: `yyyy-MM-dd`, plus optional
  `THH:mm`, `THH:mm:ss` or `THH:mm:ss.fffffff`, plus an optional `Z` or `+hh:mm`
  offset. **Nothing else.** A locale spelling such as `03/09/2026` is ambiguous
  between March 9th and the 3rd of September, so it fails the call rather than
  being resolved by whichever culture happens to be installed.
* A date written without an offset is read as a **local-time** date, which is what
  a `DatePicker` means by "the selected day". `Z` is honoured and read back as
  `+00:00`.
* `SelectedDate` is nullable: a null or empty string **clears** it, and no selection
  reads back as `null` rather than as a default date. `MinYear`/`MaxYear` have no
  absent state, so clearing one fails with `ArgumentNullException` instead of
  quietly meaning "today".

**Times** (`AvnTimeSpan`):

* The wire form is `HH:mm:ss`, 24-hour and invariant — ISO-8601's extended
  *time-of-day* spelling, **not** an ISO-8601 `PnDTnHnMnS` duration. `PT8H15M` fails
  the call.
* Writing also accepts `HH:mm`; reading always produces `HH:mm:ss`. Sub-second
  precision is not part of the wire form and is rejected rather than truncated.
* The value must be within `[00:00:00, 24:00:00)`. Managed code may store any
  `TimeSpan`, so a span outside a day — set from AXAML or from C# — **fails the
  read** instead of being wrapped into a plausible-looking time.

`SelectedDateChanged` and `SelectedTimeChanged` carry `DateTimeOffset?` and
`TimeSpan?` fields. An event payload has no converter hook — field payloads are
scalars and strings the emitter converts itself — so both events are gaps rather
than a payload that silently loses the date. A consumer that needs to observe a
selection polls the property; wiring a converter through event payloads is a later
wave.

`ClockIdentifier` is a plain string that Avalonia validates as `"12HourClock"` or
`"24HourClock"`; an unknown value fails the call because the managed setter throws.

## Versioning of the widened vtables
Nano-COM vtables are flattened, so widening a base type moves every slot of every
interface that inherits from it. Each wave therefore republishes the affected
interfaces at a new `abiVersion` under a fresh IID; the retired IIDs are never
reused. An interface whose flattened vtable is byte-identical keeps its IID, so a
stale consumer that queries for it still gets exactly the contract it was
compiled against.

| Wave     | Widened                                                     | Version | Unchanged |
| -------- | ----------------------------------------------------------- | ------- | --------- |
| Layout   | `IAvnStyledElement`, `IAvnControl` and everything below them | 2 → 3   | `IAvnAvaloniaObject` (2) |
| Chrome   | `IAvnBorder`, `IAvnPanel`, `IAvnTemplatedControl`, `IAvnTextBlock` and everything below them | 3 → 4 | `IAvnAvaloniaObject` (2), `IAvnStyledElement`, `IAvnControl`, `IAvnDecorator` (3) |
| Completeness | `IAvnContentControl`, `IAvnButton`, `IAvnToggleButton`, `IAvnListBox`, `IAvnComboBox` and everything below them | 4 → 5 | `IAvnAvaloniaObject` (2), `IAvnStyledElement`, `IAvnControl`, `IAvnDecorator` (3), `IAvnBorder`, `IAvnPanel`, `IAvnGrid`, `IAvnCanvas`, `IAvnDockPanel`, `IAvnStackPanel`, `IAvnTextBlock`, `IAvnTemplatedControl`, `IAvnItemsControl`, `IAvnSelectingItemsControl`, `IAvnTextBox`, `IAvnRangeBase`, `IAvnSlider`, `IAvnProgressBar` (4) |
| Definitions | `IAvnGrid` alone | 4 → 5 | everything else, including `IAvnPanel`, `IAvnCanvas`, `IAvnDockPanel`, `IAvnStackPanel` (4) |
| New controls A | nothing — seven brand-new interfaces | — | every interface that shipped before, at the version it last published |
| New controls B | nothing — ten brand-new interfaces | — | every interface that shipped before, at the version it last published |
| New controls C | nothing — seven brand-new interfaces | — | every interface that shipped before, at the version it last published |

Nothing in the object model derives from `Grid`, so the definitions wave moves
exactly one interface. `IAvnGrid` has published versions 1–4 and now publishes 5;
all four earlier IIDs are retired for good.

Wave A is the first wave that widens **nothing**. `IAvnImage`,
`IAvnHeaderedItemsControl`, `IAvnTabControl`, `IAvnTabItem`, `IAvnTreeView`,
`IAvnTreeViewItem`, `IAvnToolTip` and `IAvnToolTipStatics` are all new, so they
publish at version 1, and every interface they derive from keeps the IID it last
shipped. A consumer compiled against the definitions wave can keep every
interface pointer it already holds.

Wave B widens nothing either. `IAvnFlyoutBase`, `IAvnPopupFlyoutBase`,
`IAvnFlyout`, `IAvnMenuBase`, `IAvnMenu`, `IAvnHeaderedSelectingItemsControl`,
`IAvnMenuItem`, `IAvnSplitView`, `IAvnDatePicker` and `IAvnTimePicker` are all new
and publish at version 1. The flyout trio hangs off `IAvnAvaloniaObject` rather
than off `IAvnControl`, so it cannot move a control interface even in principle;
`IAvnHeaderedSelectingItemsControl` is inserted below `IAvnSelectingItemsControl`
and nothing that shipped derives from it. Even the wave A interfaces are unmoved,
so a consumer compiled against wave A keeps every control pointer it holds.

`IAvnControlFactory` grew `create_solid_color_brush` and moved from version 1 to
2. Neither the completeness nor the definitions wave adds a factory slot, so it
stayed at 2; wave A gives it a creator per new control plus
`get_tool_tip_statics`, so it moves to 3; wave B gives it a creator per
constructible new type — `Flyout`, `Menu`, `MenuItem`,
`HeaderedSelectingItemsControl`, `SplitView`, `DatePicker` and `TimePicker` — so it
moves to 4. Wave C gives it a creator per constructible new type — `WrapPanel`,
`UniformGrid`, `RelativePanel`, `Viewbox`, `FlexPanel`, `Thumb` and `GridSplitter`
— plus `get_relative_panel_statics`, so it moves to 5. The abstract bases
(`FlyoutBase`, `PopupFlyoutBase`, `MenuBase`) get no creator; they are reachable
by `query_interface` only. `IAvnBrush` is brand new, so it starts at version 1.
The collection interfaces and the event handler interfaces are unchanged, because
they carry interface pointers rather than the widened layouts.

The gradient wave grows `IAvnControlFactory` again, for
`create_linear_gradient_brush` and `create_radial_gradient_brush`, so it
publishes another new IID. `IAvnGradientBrush`, `IAvnLinearGradientBrush` and
`IAvnRadialGradientBrush` are brand new, so each starts at version 1; `IAvnBrush`
itself is untouched and keeps the IID it published with.

`Decorator` sits between `Control` and `Border`. The chrome wave added members to
`Border`, not to `Decorator`, and nothing was added to `Decorator`'s bases either,
so `IAvnDecorator` keeps the version 3 IID it published with the layout wave even
though `IAvnBorder` moved to 4. `ListBox` and `ComboBox` derive from
`SelectingItemsControl`, not from `ContentControl`, so they moved to 5 for their
own members while `IAvnItemsControl` and `IAvnSelectingItemsControl` stayed at 4.

## Host constraint: same-assembly structs

`Avalonia.Host` builds with `EnableRuntimeMarshalling=true`, so it does **not**
carry `[assembly: DisableRuntimeMarshalling]`. Under that setting the COM
interface source generator only accepts a user-defined struct in a
`[GeneratedComInterface]` signature when the struct is declared in the same
compilation. That is why `ProjectionStructs.g.cs` is emitted directly into
`Avalonia.Host` alongside the generated interfaces. A separate assembly that
wants to declare its own COM interface over these structs must apply
`[assembly: DisableRuntimeMarshalling]` (equivalently, leave
`EnableRuntimeMarshalling` unset so `build/TrimmingEnable.props` applies it).

`IAvnGeometryEcho` / `AvnGeometryEcho` are ABI fixtures in `Avalonia.Host` that
echo every struct back through a real CCW/RCW round trip. They are not part of
the projected object model and are not reachable from `IAvnControlFactory`, so
adding members there never widens a published control vtable.
