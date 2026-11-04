#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Build a Rustolonia app for the browser (WebAssembly).

.DESCRIPTION
Compiles a Rust crate (crate-type = ["staticlib"], exporting
`rustolonia_main`) for wasm32-unknown-emscripten, then publishes the .NET
WebAssembly browser host (host/Browser) with the static library linked in.
The resulting static site is written to <OutputDirectory>/wwwroot.

Requirements: the Rust target wasm32-unknown-emscripten
(`rustup target add wasm32-unknown-emscripten`) and a .NET SDK with the
wasm-tools workload (`dotnet workload install wasm-tools`). See
rust/PLATFORMS.md#browser-webassembly.
#>
param(
    # Cargo package to build. Defaults to the in-repo browser sample.
    [string]$Package = 'browser-sample',
    # Cargo manifest of the workspace or crate containing -Package.
    [string]$Manifest = (Join-Path $PSScriptRoot 'Cargo.toml'),
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Release',
    [string]$OutputDirectory = (Join-Path $PSScriptRoot 'target/browser/publish'),
    # llvm-objcopy used to strip the target_features section; defaults to the
    # one shipped with the .NET Emscripten workload pack.
    [string]$LlvmObjcopy,
    # Serve the published site on http://localhost:<Port> after building.
    [switch]$Serve,
    [int]$Port = 8080,
    # Extra arguments for `dotnet publish` (for example --configfile).
    [string[]]$DotnetArgs = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$repoRoot = Split-Path -Parent $PSScriptRoot
$target = 'wasm32-unknown-emscripten'
$profileDir = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }

function Find-LlvmObjcopy {
    if ($LlvmObjcopy) { return $LlvmObjcopy }
    $dotnet = (Get-Command dotnet -ErrorAction Stop).Source
    $dotnetRoot = if ($env:DOTNET_ROOT) { $env:DOTNET_ROOT } else {
        Split-Path -Parent ([IO.Path]::GetFullPath((Get-Item $dotnet).ResolvedTarget ?? $dotnet))
    }
    $exe = if ($IsWindows) { 'llvm-objcopy.exe' } else { 'llvm-objcopy' }
    $packs = Join-Path $dotnetRoot 'packs'
    $found = Get-ChildItem -Path $packs -Directory -Filter 'Microsoft.NET.Runtime.Emscripten.*.Sdk.*' -ErrorAction SilentlyContinue |
        ForEach-Object { Get-ChildItem -Path $_.FullName -Recurse -Filter $exe -ErrorAction SilentlyContinue } |
        Select-Object -First 1
    if ($found) { return $found.FullName }
    $onPath = Get-Command $exe -ErrorAction SilentlyContinue
    if ($onPath) { return $onPath.Source }
    throw "llvm-objcopy was not found in the .NET Emscripten workload ($packs) or on PATH; install the wasm-tools workload or pass -LlvmObjcopy."
}

# 1. Rust: static library for Emscripten. Unwinding needs Emscripten
#    exception support in the final link, so browser builds abort on panic.
$env:CARGO_PROFILE_RELEASE_PANIC = 'abort'
$env:CARGO_PROFILE_DEV_PANIC = 'abort'
$cargoArgs = @('build', '--manifest-path', $Manifest, '-p', $Package, '--target', $target)
if ($Configuration -eq 'Release') { $cargoArgs += '--release' }
& cargo @cargoArgs

$metadata = & cargo metadata --manifest-path $Manifest --format-version 1 --no-deps | ConvertFrom-Json
$libName = ($metadata.packages | Where-Object name -eq $Package | Select-Object -First 1).targets |
    Where-Object { $_.crate_types -contains 'staticlib' } | Select-Object -First 1 -ExpandProperty name
if (-not $libName) { throw "Package '$Package' has no staticlib target." }
$staticLib = Join-Path $metadata.target_directory "$target/$profileDir/lib$($libName.Replace('-', '_')).a"
if (-not (Test-Path $staticLib)) { throw "Expected static library was not produced: $staticLib" }

# 2. Recent Rust toolchains record wasm target features that the Emscripten
#    wasm-opt bundled with .NET does not know; drop the section before linking.
#    The host imports the entry point from module 'rustolonia_app'.
$appDir = Join-Path $metadata.target_directory 'browser'
New-Item -ItemType Directory -Force -Path $appDir | Out-Null
$appLib = Join-Path $appDir 'rustolonia_app.a'
& (Find-LlvmObjcopy) --remove-section=target_features $staticLib $appLib

# 3. Avalonia.Browser's JavaScript modules (avalonia.js, ...) are bundled by a
#    target that only runs in its multi-targeting build, and the host picks
#    them up at evaluation time, so produce them before publishing.
$avaloniaBrowser = Join-Path $repoRoot 'avalonia-src/src/Browser/Avalonia.Browser/Avalonia.Browser.csproj'
& dotnet msbuild $avaloniaBrowser -t:BunRunBuild -nologo -v:minimal @DotnetArgs

# 4. .NET: publish the browser host with the Rust app linked in.
& dotnet publish (Join-Path $repoRoot 'host/Browser/Avalonia.Host.Browser.csproj') `
    -c $Configuration -o $OutputDirectory "-p:RustoloniaBrowserApp=$appLib" @DotnetArgs

$site = Join-Path $OutputDirectory 'wwwroot'
Write-Host "Browser app published to $site"

if ($Serve) {
    $python = Get-Command python3, python -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $python) { throw 'Serving requires python3 on PATH; serve the wwwroot directory with any static web server instead.' }
    Write-Host "Serving http://localhost:$Port (Ctrl+C to stop)"
    & $python.Source -m http.server $Port --directory $site
}
