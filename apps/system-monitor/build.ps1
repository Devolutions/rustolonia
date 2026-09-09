#!/usr/bin/env pwsh
#Requires -Version 7.0
[CmdletBinding()]
param(
    [switch]$UpdateLockFile,
    [switch]$DeveloperTools
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Push-Location $PSScriptRoot
try {
    & (Join-Path $repositoryRoot 'rust\build-app.ps1') `
        -ProducerRoot (Join-Path $repositoryRoot 'avalonia-src') `
        -RustoloniaRoot $repositoryRoot `
        -Manifest (Join-Path $PSScriptRoot 'avalonia-app.json') `
        -UpdateLockFile:$UpdateLockFile `
        -DeveloperTools:$DeveloperTools
}
finally {
    Pop-Location
}
