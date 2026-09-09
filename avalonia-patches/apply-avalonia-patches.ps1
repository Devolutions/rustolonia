#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Applies the Avalonia source patches rustolonia needs onto an Avalonia checkout.

.DESCRIPTION
Rustolonia carries a small set of additive patches against the Avalonia UI
framework (TableView windowing hooks, column width limits, friend access for
the NativeAOT host). This script applies them to the Avalonia checkout at
-AvaloniaRoot, which is typically the `avalonia-src` submodule pinned to a
release tag.

.PARAMETER AvaloniaRoot
Path to the Avalonia checkout to patch.

.PARAMETER Check
Instead of applying, verify every patch still applies cleanly (for CI and
after bumping the submodule to a newer tag).

.EXAMPLE
pwsh ./apply-avalonia-patches.ps1 -AvaloniaRoot ./avalonia-src

.EXAMPLE
pwsh ./apply-avalonia-patches.ps1 -AvaloniaRoot ./avalonia-src -Check
#>
param(
    [Parameter(Mandatory)][string]$AvaloniaRoot,
    [switch]$Check
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$avaloniaRoot = (Resolve-Path -LiteralPath $AvaloniaRoot).Path
# Absolute paths: git -C reinterprets relative patch paths against the target tree.
$patchesDirectory = (Resolve-Path -LiteralPath $PSScriptRoot).Path
if (-not (Test-Path (Join-Path $avaloniaRoot '.git'))) {
    throw "AvaloniaRoot is not a git checkout: '$avaloniaRoot'. The patches apply with git apply."
}

$patches = @(
    'avalonia-controls.patch'
)

foreach ($patch in $patches) {
    $path = Join-Path $patchesDirectory $patch
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Patch not found: $path"
    }
    # Probes are allowed to fail: a non-zero exit from --check just means
    # the patch is not applicable in that direction.
    $PSNativeCommandUseErrorActionPreference = $false
    & git -C $avaloniaRoot apply --check $path 2>$null
    $appliesCleanly = $LASTEXITCODE -eq 0
    & git -C $avaloniaRoot apply --reverse --check $path 2>$null
    $alreadyApplied = $LASTEXITCODE -eq 0
    $PSNativeCommandUseErrorActionPreference = $true
    if ($Check) {
        if ($appliesCleanly) {
            Write-Host "OK: $patch applies cleanly"
        }
        elseif ($alreadyApplied) {
            Write-Host "OK: $patch is already applied"
        }
        else {
            Write-Error "$patch neither applies cleanly nor is already applied — rebasing the patch is required for this Avalonia revision."
        }
        continue
    }
    if ($appliesCleanly) {
        & git -C $avaloniaRoot apply $path
        Write-Host "Applied $patch"
    }
    elseif ($alreadyApplied) {
        Write-Host "Skipping $patch (already applied)"
    }
    else {
        throw "$patch does not apply. Rebase it onto this Avalonia revision (see avalonia-patches/README.md)."
    }
}
