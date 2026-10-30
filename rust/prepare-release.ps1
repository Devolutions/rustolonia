#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Prepares a rustolonia release.

.DESCRIPTION
Two steps, each followed by a commit:

  1. -Version <x.y.z>
     Sets the version of rustolonia, rustolonia-sys and rustolonia-bindgen,
     the exact `rustolonia-sys` pin in rustolonia, `rustoloniaVersion` in
     rust/release-manifest.json and <Version> in build/SharedVersion.props,
     clears rust/rustolonia-sys/host-checksums.txt and refreshes Cargo.lock
     (and the crates' entries in the in-repository apps' lock files).
     Commit to master, then dispatch .github/workflows/release.yml. With
     phase=all (the default) that one run does everything below; with
     phase=build it only stages host tarballs and SHA256SUMS as artifacts.

  2. -Checksums (-SumsFile <SHA256SUMS> | -FromRelease)
     Writes host-checksums.txt from SHA256SUMS (symbol tarballs are left out).
     For initial publication use -SumsFile from the staged build artifact.
     -FromRelease reads an existing v<x.y.z> release.
     Commit, then push `v<x.y.z>` on this commit. Dispatch phase=publish
     with build_run_id to create the GitHub release (non-draft by default).
     crates.io publishing and draft releases are opt-in.

     -CommitAndTag also makes that commit (only host-checksums.txt) and the
     annotated tag v<x.y.z>, without pushing. It refuses a dirty tree, an
     existing tag, or a HEAD other than -SourceRevision (the build source).
     phase=all runs it, then pushes master and the tag atomically.
#>
[CmdletBinding(DefaultParameterSetName = 'Version')]
param(
    [Parameter(Mandatory, ParameterSetName = 'Version')]
    [ValidatePattern('^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$')]
    [string]$Version,
    [Parameter(ParameterSetName = 'Version')]
    [switch]$SkipCargoUpdate,

    [Parameter(Mandatory, ParameterSetName = 'Checksums')]
    [switch]$Checksums,
    [Parameter(ParameterSetName = 'Checksums')]
    [string]$SumsFile,
    [Parameter(ParameterSetName = 'Checksums')]
    [switch]$FromRelease,
    [Parameter(ParameterSetName = 'Checksums')]
    [switch]$CommitAndTag,
    [Parameter(ParameterSetName = 'Checksums')]
    [string]$SourceRevision,

    [string]$RustoloniaRoot = (Split-Path -Parent $PSScriptRoot)
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'package-shared.ps1')

$root = (Resolve-Path -LiteralPath $RustoloniaRoot).Path
$checksumFile = Join-Path $root 'rust' 'rustolonia-sys' 'host-checksums.txt'
$checksumHeader = @'
# SHA-256 of the prebuilt rustolonia host tarballs for this rustolonia-sys
# version, in `sha256sum` format. build.rs refuses to use a downloaded host
# whose hash is not listed here. rust/prepare-release.ps1 fills this in from
# the release build before the consumer tag or crates are published.
'@ -replace "`r`n", "`n"

function Write-Utf8Lf([string]$Path, [string]$Text) {
    [IO.File]::WriteAllText($Path, ($Text -replace "`r`n", "`n"), [Text.UTF8Encoding]::new($false))
}

function Set-Content-Replace([string]$Path, [string]$Pattern, [string]$Replacement) {
    $text = [IO.File]::ReadAllText($Path)
    $regex = [regex]::new($Pattern, [Text.RegularExpressions.RegexOptions]::Multiline)
    if (-not $regex.IsMatch($text)) { throw "Pattern '$Pattern' not found in $Path" }
    [IO.File]::WriteAllText($Path, $regex.Replace($text, $Replacement, 1), [Text.UTF8Encoding]::new($false))
}

if ($PSCmdlet.ParameterSetName -eq 'Version') {
    foreach ($crate in @('rustolonia-sys', 'rustolonia', 'rustolonia-bindgen')) {
        Set-Content-Replace (Join-Path $root 'rust' $crate 'Cargo.toml') '^version\s*=\s*"[^"]+"' "version = `"$Version`""
    }
    Set-Content-Replace (Join-Path $root 'rust' 'rustolonia' 'Cargo.toml') '^(rustolonia-sys\s*=\s*\{\s*version\s*=\s*)"=[^"]+"' "`${1}`"=$Version`""
    Set-Content-Replace (Join-Path $root 'rust' 'release-manifest.json') '("rustoloniaVersion"\s*:\s*)"[^"]+"' "`${1}`"$Version`""
    Set-Content-Replace (Join-Path $root 'build' 'SharedVersion.props') '<Version>[^<]+</Version>' "<Version>$Version</Version>"
    Write-Utf8Lf $checksumFile "$checksumHeader`n"

    if (-not $SkipCargoUpdate) {
        & cargo update --workspace --manifest-path (Join-Path $root 'rust' 'Cargo.toml')
        # The in-repository apps use the crates by path with their own lock
        # files; only the crates' own entries move (CI builds them --locked).
        # Lock files without rustolonia (an app's other workspaces) stay.
        foreach ($lock in @(& git -C $root ls-files 'apps/*Cargo.lock' 'apps/**/Cargo.lock' | Sort-Object -Unique)) {
            $manifest = Join-Path $root (Split-Path -Parent $lock) 'Cargo.toml'
            $usesRustolonia = Select-String -LiteralPath (Join-Path $root $lock) -Pattern '^name = "rustolonia"$' -Quiet
            if ($usesRustolonia -and (Test-Path -LiteralPath $manifest)) {
                & cargo update -p rustolonia -p rustolonia-sys --manifest-path $manifest
            }
        }
    }
    $null = Get-RustoloniaReleaseVersion -RustoloniaRoot $root
    Write-Host "Version set to $Version. Merge to master, then dispatch release.yml with phase=all (no tag needed)."
    return
}

$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $root
$checksumPath = 'rust/rustolonia-sys/host-checksums.txt'
if ($SourceRevision -and -not $CommitAndTag) { throw '-SourceRevision is only used with -CommitAndTag.' }
if ($CommitAndTag) {
    $head = (& git -C $root rev-parse HEAD).Trim()
    if ($SourceRevision) {
        $source = (& git -C $root rev-parse --verify "$SourceRevision^{commit}").Trim()
        if ($head -ne $source) { throw "HEAD $head is not the build source revision $source; rebuild from the current master." }
    }
    if (@(& git -C $root status --porcelain --untracked-files=no).Count -gt 0) { throw 'The working tree has changes; -CommitAndTag commits only host-checksums.txt.' }
    if (@(& git -C $root tag --list "v$version").Count -gt 0) { throw "Tag v$version already exists." }
}
if ($FromRelease) {
    if ($SumsFile) { throw 'Pass either -SumsFile or -FromRelease, not both.' }
    $download = Join-Path ([IO.Path]::GetTempPath()) ('rustolonia-sums-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $download | Out-Null
    try {
        & gh release download "v$version" --repo Devolutions/rustolonia --pattern SHA256SUMS --dir $download
        $sumsLines = @(Get-Content -LiteralPath (Join-Path $download 'SHA256SUMS'))
    }
    finally { Remove-Item -LiteralPath $download -Recurse -Force }
}
elseif (-not $SumsFile) {
    throw 'Pass -SumsFile <SHA256SUMS> or -FromRelease.'
}
else {
    $sumsLines = @(Get-Content -LiteralPath $SumsFile)
}

$entries = [ordered]@{}
foreach ($line in $sumsLines) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    $hash, $name = $line.Trim() -split '\s+\*?', 2
    if ($hash -notmatch '^[0-9a-f]{64}$') { throw "Invalid SHA256SUMS line: $line" }
    if ($name -notlike "rustolonia-host-$version-*.tar.gz" -or $name -like '*-symbols.tar.gz') { continue }
    $entries[$name] = $hash
}
foreach ($rid in $script:RidTargets.Keys) {
    $asset = Get-HostAssetName -Version $version -Rid $rid
    if (-not $entries.Contains($asset)) { throw "SHA256SUMS has no entry for $asset." }
}
$lines = @($entries.Keys | Sort-Object -CaseSensitive | ForEach-Object { "$($entries[$_])  $_" })
Write-Utf8Lf $checksumFile ("$checksumHeader`n" + ($lines -join "`n") + "`n")
Write-Host "Wrote $($lines.Count) checksums for v$version to $checksumFile."
if (-not $CommitAndTag) { return }

$changed = @(& git -C $root status --porcelain --untracked-files=no | ForEach-Object { $_.Substring(3) })
if ($changed.Count -eq 0) { throw "host-checksums.txt already lists these hashes; the build source must not carry v$version checksums." }
if ($changed.Count -ne 1 -or $changed[0] -ne $checksumPath) { throw "Unexpected changes: $($changed -join ', ')" }
& git -C $root add -- $checksumPath
& git -C $root commit --quiet -m "Pin rustolonia $version release host checksums"
& git -C $root tag -a "v$version" -m "rustolonia $version"
$parent = (& git -C $root rev-parse 'HEAD^').Trim()
$committed = @(& git -C $root diff --name-only $head HEAD)
if ($parent -ne $head -or $committed.Count -ne 1 -or $committed[0] -ne $checksumPath) { throw 'The checksum commit must change only host-checksums.txt on top of the build source.' }
Write-Host "Committed $((& git -C $root rev-parse HEAD).Trim()) and tagged v$version. Push with: git push --atomic origin HEAD:refs/heads/master v$version"
