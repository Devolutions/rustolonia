# Upstream Avalonia: what Rustolonia needs merged

The producer is pinned to Avalonia `12.1.6`. The patches in this directory
contain the remaining changes to upstream framework files.

## TableView viewport hook

`avalonia-controls.patch` adds `IViewportRangeSource` and calls it with the
realized row range after TableView layout and scrolling. A windowed collection
can request pages for the viewport without waiting for individual indexer
misses. The patch also carries viewport and large-table virtualization tests.

## Host friend access

`avalonia-controls.patch` grants `Avalonia.Host` and `Avalonia.Host.Tests`
access to internals for the generated popup-placement wrapper. A public
`CustomPopupPlacement` constructor or factory would allow these
Rustolonia-specific friend declarations to be removed.

## Popup and macOS integration

- `avalonia-overlay-popups.patch` adds a per-top-level overlay-popup policy and
  an optional continuation gate before showing a native popup.
- `avalonia-termination.patch` supports deferred macOS application termination.
- `avalonia-file-drag.patch` retains the originating macOS mouse event for
  deferred native file-drag operations.
- `avalonia-application-name.patch` initializes the default macOS application
  menu holder with `Application.Name`.
- `avalonia-background-launch.patch` stages the desired activation policy from
  `ShowInDock`, reports policy-change failures, and prevents accessory launches
  from activating either the Dock or the application. Delegate lifecycle
  callbacks remain installed.

## Grid definition changes invalidate cached cells

Replacing or mutating row/column definitions must invalidate the grid's cell
structure as well as measure. Otherwise a previously measured child can retain
an index into a removed track and the next layout can throw. The separate
`avalonia-grid-invalidation.patch` uses the existing Grid invalidation method;
managed and native layout regressions cover the change.

## Changes merged upstream

- `TableViewColumn.IsVisible`: AvaloniaUI/Avalonia#22162, shipped in 12.1.3.
- TableView column width limits and layout/resizing integration:
  AvaloniaUI/Avalonia#22707, shipped in 12.1.6.
- TableView row automation peer: AvaloniaUI/Avalonia#22709, shipped in 12.1.6.

The upstream width implementation redistributes available space among star
columns when a limit is reached, instead of independently clamping each share.
The upstream row peer also excludes hidden and raw content and respects
explicit accessibility names. The upstream control tests cover both behaviors.
