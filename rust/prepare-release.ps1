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
     clears rust/rustolonia-sys/host-checksums.txt and refreshes Cargo.lock.
     Commit, then push the tag `v<x.y.z>`; .github/workflows/release.yml
     builds the host tarballs into a draft GitHub release.

  2. -Checksums (-SumsFile <SHA256SUMS> | -FromRelease)
     Writes host-checksums.txt from the draft release's SHA256SUMS (symbol
     tarballs are left out). Commit, then run the release workflow manually
     to publish the GitHub release and the crates.
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
# the release build before the crates are published.
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

    if (-not $SkipCargoUpdate) { & cargo update --workspace --manifest-path (Join-Path $root 'rust' 'Cargo.toml') }
    $null = Get-RustoloniaReleaseVersion -RustoloniaRoot $root
    Write-Host "Version set to $Version. Commit, then push the tag v$Version."
    return
}

$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $root
if ($FromRelease) {
    if ($SumsFile) { throw 'Pass either -SumsFile or -FromRelease, not both.' }
    $download = Join-Path ([IO.Path]::GetTempPath()) ('rustolonia-sums-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $download | Out-Null
    & gh release download "v$version" --repo Devolutions/rustolonia --pattern SHA256SUMS --dir $download
    $SumsFile = Join-Path $download 'SHA256SUMS'
}
elseif (-not $SumsFile) {
    throw 'Pass -SumsFile <SHA256SUMS> or -FromRelease.'
}

$entries = [ordered]@{}
foreach ($line in Get-Content -LiteralPath $SumsFile) {
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
