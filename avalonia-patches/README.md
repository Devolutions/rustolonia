# avalonia-patches

Additive patches against the [Avalonia UI framework](https://github.com/AvaloniaUI/Avalonia)
that rustolonia's NativeAOT host and projected TableView surface depend on.
They are meant to be applied to the `avalonia-src` submodule checkout pinned at
the upstream `12.1.6` tag (`cf8e564674700348947e93ab19105f27afe859a9`) by
`apply-avalonia-patches.ps1`.

| Patch | Contents |
|---|---|
| `avalonia-controls.patch` | `IViewportRangeSource`, the TableView viewport hook, host friend access, and virtualization and viewport tests. |
| `avalonia-overlay-popups.patch` | Per-top-level `UseOverlayPopups` policy and optional `NativePopupShowRequested` continuation gate. Supports embedded popups or deferred native show without affecting other windows. |
| `avalonia-termination.patch` | Deferred macOS application termination for asynchronous shutdown coordination. |
| `avalonia-file-drag.patch` | Retains a macOS mouse event for a deferred native file-drag operation. |
| `avalonia-application-name.patch` | Names the default macOS application-menu holder from `Application.Name` before export, so both the menu item and submenu retain the application name when windows become key. |

## Usage

```pwsh
# after cloning submodules (avalonia-src pinned to Rustolonia's producer SHA)
pwsh ./apply-avalonia-patches.ps1 -AvaloniaRoot ../avalonia-src

# CI / after bumping the submodule to a newer producer SHA
pwsh ./apply-avalonia-patches.ps1 -AvaloniaRoot ../avalonia-src -Check
```

Applying is idempotent: an already-patched checkout is detected and skipped.

Validated producer: the patch applies cleanly to the pinned checkout at
`cf8e564674700348947e93ab19105f27afe859a9`; the patched producer build and the
relevant `Avalonia.Controls.UnitTests` suite remain the expected baseline for
Rustolonia's compatibility work.

## Maintenance

- Track upstream stable releases: pin the submodule to an Avalonia release tag
  (`release/X.Y` line, currently `12.1.6`), not `main`. Bump on each upstream
  patch release, run with `-Check` first, and rebuild/re-release.
- If a patch stops applying (typically because upstream shipped an equivalent
  change), apply with `git apply --3way`, drop the parts upstream now provides,
  regenerate the patch with `git diff --cached` in the submodule, and update
  `rust/release-manifest.json` (`producerPin`, patch `sha256`) and the
  upstreaming tracker below. Stage the new submodule gitlink before running
  Rustolonia's regeneration and build scripts.
- The projection follows the pinned API surface. Moving from `main` to `12.1.3`
  dropped master-only API (`FlexPanel`, `Calendar.IsWeekNumberVisible`/`WeekNumberRule`)
  from `AvaloniaProjectionProfiles`; re-add them when the pin reaches a release that
  ships them, then run `rust/regenerate-and-build.ps1`.
- The goal is for this directory to shrink to nothing as the changes are
  merged upstream.

## Upstreaming tracker

| Change | Status |
|---|---|
| `IViewportRangeSource` + TableView `NotifyVisibleRange` | candidate PR — additive, framework-generic |
| `TableViewColumn.MinWidth/MaxWidth` + layout/header integration | merged upstream (AvaloniaUI/Avalonia#22707, shipped in 12.1.6); removed from the patch |
| `TableViewColumn.IsVisible` | merged upstream (AvaloniaUI/Avalonia#22162, shipped in 12.1.3); removed from the patch |
| `TableViewRowAutomationPeer` | merged upstream (AvaloniaUI/Avalonia#22709, shipped in 12.1.6); removed from the patch |
| Column/row unit tests | upstream with the corresponding features; local viewport and virtualization tests remain |
| Default macOS application-menu holder uses `Application.Name` | candidate upstream fix; covered by Rustolonia's standalone and bundled native multi-window naming regression |
| `InternalsVisibleTo` for `Avalonia.Host` (+ tests) in `Avalonia.Controls.csproj` | rustolonia-specific; preferred fix is a public `CustomPopupPlacement` factory, after which this line disappears |
