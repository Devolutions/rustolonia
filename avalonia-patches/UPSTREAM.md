# Upstream Avalonia: what rustolonia needs merged

Everything else in the branch lives under `rustolonia/` and is ours alone.
This is the complete list of changes touching shared Avalonia framework
files (`git diff <merge-base> HEAD -- src/Avalonia.Controls
tests/Avalonia.Controls.UnitTests`: 558 insertions across 11 files), and the
shape each piece would take as an upstream PR.

## 1. `IViewportRangeSource` + TableView viewport hook (framework-generic, easy sell)
- **New file** `IViewportRangeSource.cs` (16 lines): an opt-in interface an
  items source implements to be told the realized row range.
- **`TableView.cs`** (+26): `ArrangeOverride` override calls
  `NotifyVisibleRange()`, which forwards `VirtualizingStackPanel`'s
  first/last realized index to an `IViewportRangeSource` items source.
- Why upstream should want it: it is the standard hook virtualized windowed
  collections need, additive, control-local, no behavior change otherwise.

## 2. `TableViewColumn` width limits + visibility (standalone control feature)
- **`TableViewColumn.cs`** (+89): `MinWidth`/`MaxWidth` styled properties with
  validation, `IsVisible` (column-level hide; column is not a `Visual` so
  `Visual.IsVisible` is unavailable), internal `ClampWidth`.
- **`Presenters/TableViewLayoutHelper.cs`** (+27), **`TableViewColumnHeader.cs`**
  (+4), **`TableViewRow.cs`** (+15): layout clamps widths, hides render the
  column at zero width, disables the resize thumb, and rows consult
  visibility/limits.
- Upstream shape: a self-contained TableView feature PR.

## 3. `TableViewRowAutomationPeer` (accessibility)
- **New file** (+113) + tests (+135): automation peer for table rows following
  the existing peer patterns.
- Upstream shape: accessibility PR, pattern-conformant.

## 4. Tests riding with the above (+24 to `TableViewTests.cs`, +109
`TableViewColumnWidthLimitTests.cs`): ship inside PRs 1–2.

## 5. `InternalsVisibleTo` in `Avalonia.Controls.csproj` (rustolonia-specific — different treatment)
Two lines granting `Avalonia.Host` / `Avalonia.Host.Tests` friend access,
needed *only* by the generated popup-placement wrapper constructing the
internal `CustomPopupPlacement(Size, Visual)` record (one call site).

**Preferred upstream ask**: make placement construction public — a
`CustomPopupPlacement` public constructor or factory — since the type is
public and its mutability is the point of the callback. If that lands, the
`InternalsVisibleTo` lines disappear from the patch set entirely. If not,
these two lines stay a local patch (they are not a reasonable upstream ask
as-is).

## Patch set

`avalonia-patches/avalonia-controls.patch` carries items 1–4 today
(regenerate with `git diff <merge-base> HEAD -- src/Avalonia.Controls
tests/Avalonia.Controls.UnitTests`). Item 5 rides in the same patch until
the public-constructor ask lands upstream.
