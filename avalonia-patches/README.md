# avalonia-patches

Additive patches against the [Avalonia UI framework](https://github.com/AvaloniaUI/Avalonia)
that rustolonia's NativeAOT host and projected TableView surface depend on.
They are meant to be applied to the `avalonia-src` submodule checkout pinned at
the upstream `12.1.3` tag (`8eeda4f6f546165b3f72e63c9f42247abb306905`) by
`apply-avalonia-patches.ps1`.

| Patch | Contents |
|---|---|
| `avalonia-controls.patch` | `IViewportRangeSource` (new viewport-range interface), `TableViewColumn.MinWidth/MaxWidth` + `ClampWidth`, `TableViewRowAutomationPeer` (new), TableView layout/row/column-header hooks, and their unit tests. `TableViewColumn.IsVisible` is upstream since 12.1.3 (AvaloniaUI/Avalonia#22162) and is no longer patched. |

## Usage

```pwsh
# after cloning submodules (avalonia-src pinned to Rustolonia's producer SHA)
pwsh ./apply-avalonia-patches.ps1 -AvaloniaRoot ../avalonia-src

# CI / after bumping the submodule to a newer producer SHA
pwsh ./apply-avalonia-patches.ps1 -AvaloniaRoot ../avalonia-src -Check
```

Applying is idempotent: an already-patched checkout is detected and skipped.

Validated producer: the patch applies cleanly to the pinned checkout at
`8eeda4f6f546165b3f72e63c9f42247abb306905`; the patched producer build and the
relevant `Avalonia.Controls.UnitTests` suite remain the expected baseline for
Rustolonia's compatibility work.

## Maintenance

- Track upstream stable releases: pin the submodule to an Avalonia release tag
  (`release/X.Y` line, currently `12.1.3`), not `main`. Bump on each upstream
  patch release, run with `-Check` first, and rebuild/re-release.
- If a patch stops applying (typically because upstream shipped an equivalent
  change), apply with `git apply --3way`, drop the parts upstream now provides,
  regenerate the patch with `git diff --cached` in the submodule, and update
  `rust/release-manifest.json` (`producerPin`, patch `sha256`) and the
  upstreaming tracker below.
- The goal is for this directory to shrink to nothing as the changes are
  merged upstream.

## Upstreaming tracker

| Change | Status |
|---|---|
| `IViewportRangeSource` + TableView `NotifyVisibleRange` | candidate PR — additive, framework-generic |
| `TableViewColumn.MinWidth/MaxWidth` + `ClampWidth` + layout/header integration | candidate PR — additive, standalone control feature |
| `TableViewColumn.IsVisible` | merged upstream (AvaloniaUI/Avalonia#22162, shipped in 12.1.3); removed from the patch |
| `TableViewRowAutomationPeer` | candidate PR — accessibility, follows existing peer patterns |
| Column/row unit tests | rides with the PRs above |
| `InternalsVisibleTo` for `Avalonia.Host` (+ tests) in `Avalonia.Controls.csproj` | rustolonia-specific; preferred fix is a public `CustomPopupPlacement` factory, after which this line disappears |
