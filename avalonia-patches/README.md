# avalonia-patches

Additive patches against the [Avalonia UI framework](https://github.com/AvaloniaUI/Avalonia)
that rustolonia's NativeAOT host and projected TableView surface depend on.
They are meant to be applied to the `avalonia-src` submodule checkout pinned at
`9654332a79f637473da96054f75b2a16deaa557e` by `apply-avalonia-patches.ps1`.

| Patch | Contents |
|---|---|
| `avalonia-controls.patch` | `IViewportRangeSource` (new viewport-range interface), `TableViewColumn.MinWidth/MaxWidth/IsVisible` + `ClampWidth`, `TableViewRowAutomationPeer` (new), TableView layout/row/column-header hooks, and their unit tests |

## Usage

```pwsh
# after cloning submodules (avalonia-src pinned to Rustolonia's producer SHA)
pwsh ./apply-avalonia-patches.ps1 -AvaloniaRoot ../avalonia-src

# CI / after bumping the submodule to a newer producer SHA
pwsh ./apply-avalonia-patches.ps1 -AvaloniaRoot ../avalonia-src -Check
```

Applying is idempotent: an already-patched checkout is detected and skipped.

Validated producer: the patch applies cleanly to the pinned checkout at
`9654332a79f637473da96054f75b2a16deaa557e`; the patched producer build and the
relevant `Avalonia.Controls.UnitTests` suite remain the expected baseline for
Rustolonia's compatibility work.

## Maintenance

- Bump the submodule to a new Avalonia tag, then run with `-Check` first.
- If a patch stops applying, regenerate it from the rustolonia branch
  (`git diff <merge-base> HEAD -- src/Avalonia.Controls tests/Avalonia.Controls.UnitTests`)
  and update the upstreaming tracker below.
- The goal is for this directory to shrink to nothing as the changes are
  merged upstream.

## Upstreaming tracker

| Change | Status |
|---|---|
| `IViewportRangeSource` + TableView `NotifyVisibleRange` | candidate PR — additive, framework-generic |
| `TableViewColumn.MinWidth/MaxWidth/IsVisible` + `ClampWidth` + layout/header integration | candidate PR — additive, standalone control feature |
| `TableViewRowAutomationPeer` | candidate PR — accessibility, follows existing peer patterns |
| Column/row unit tests | rides with the PRs above |
| `InternalsVisibleTo` for `Avalonia.Host` (+ tests) in `Avalonia.Controls.csproj` | rustolonia-specific; preferred fix is a public `CustomPopupPlacement` factory, after which this line disappears |
