#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Publish the NativeAOT host for this OS and run the Rust workspace tests.
#>
param(
    [ValidateSet('x64', 'arm64')]
    [string]$Architecture = 'x64',
    [switch]$RunExamples
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

# The Rustolonia repository root is the parent of rust/.
$repositoryRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'package-shared.ps1')
$avaloniaRoot = Join-Path $repositoryRoot 'avalonia-src'
Initialize-LocalProducerSubmodule -RustoloniaRoot $repositoryRoot -ProducerRoot $avaloniaRoot
$nativeArchitecture = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    'X64' { 'x64' }
    'Arm64' { 'arm64' }
    default { throw "Unsupported CPU architecture: $_" }
}
if ($Architecture -ne $nativeArchitecture) {
    throw "This RID requires a runner with a matching $Architecture CPU because cargo tests execute native binaries."
}

if ($IsWindows) {
    $platform = 'Win32'
    $rid = "win-$Architecture"
    $hostExtension = '.dll'
}
elseif ($IsLinux) {
    $platform = 'X11'
    $rid = "linux-$Architecture"
    $hostExtension = '.so'
    $dbus = Join-Path $avaloniaRoot 'external' 'Avalonia.DBus' 'src' 'Avalonia.DBus' 'Avalonia.DBus.csproj'
    if (-not (Test-Path -LiteralPath $dbus -PathType Leaf)) {
        throw 'Initialize Linux sources with: git submodule update --init external/Avalonia.DBus'
    }
}
elseif ($IsMacOS) {
    $platform = 'OSX'
    $rid = "osx-$Architecture"
    $hostExtension = '.dylib'
    $xcodeArch = if ($Architecture -eq 'x64') { 'x86_64' } else { 'arm64' }
    # The COM header avalonia-native.h is generated from avn.idl by the producer's NUKE
    # build; a fresh checkout does not carry it, so generate before xcodebuild.
    Push-Location $avaloniaRoot
    try {
        dotnet run --project (Join-Path $avaloniaRoot 'nukebuild' '_build.csproj') --target GenerateCppHeaders
    }
    finally {
        Pop-Location
    }
    $xcodeProject = Join-Path $avaloniaRoot 'native' 'Avalonia.Native' 'src' 'OSX' 'Avalonia.Native.OSX.xcodeproj'
    $products = Join-Path $avaloniaRoot 'Build' 'Products' 'Release'
    xcodebuild -project $xcodeProject -configuration Release "ARCHS=$xcodeArch" "CONFIGURATION_BUILD_DIR=$products"
}
else {
    throw 'rust/build.ps1 supports Windows, Linux, and macOS.'
}

if ($IsWindows) {
    $rustTarget = if ($Architecture -eq 'x64') { 'x86_64-pc-windows-msvc' } else { 'aarch64-pc-windows-msvc' }
    $rustFlagsVariable = "CARGO_TARGET_$($rustTarget.ToUpperInvariant().Replace('-', '_'))_RUSTFLAGS"
    $rustFlags = [Environment]::GetEnvironmentVariable($rustFlagsVariable, 'Process')
    if ($rustFlags -notlike '*target-feature=+crt-static*') {
        [Environment]::SetEnvironmentVariable($rustFlagsVariable, "$rustFlags -C target-feature=+crt-static".Trim(), 'Process')
    }
}

$artifacts = if ($env:AVN_DOTNET_ARTIFACTS) { $env:AVN_DOTNET_ARTIFACTS } else { Join-Path $PSScriptRoot 'target' "dotnet-$rid" }
dotnet publish (Join-Path $repositoryRoot 'host' 'Avalonia.Host.csproj') `
    -c Release -r $rid "-p:AvaloniaRustHostPlatform=$platform" --artifacts-path $artifacts

$hostFile = Join-Path $artifacts 'publish' 'Avalonia.Host' "release_$rid" "Avalonia.Host$hostExtension"
if (-not (Test-Path -LiteralPath $hostFile -PathType Leaf)) {
    throw "NativeAOT host was not produced at $hostFile"
}

$env:AVN_HOST_NATIVE_LIB = $hostFile
if (-not $IsWindows) {
    $cargoTarget = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $PSScriptRoot 'target' "cargo-$rid" }
    $env:CARGO_TARGET_DIR = $cargoTarget
}

cargo test --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml') --workspace
Write-Host "$rid NativeAOT host: $hostFile"

if ($RunExamples) {
    cargo run --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml') -p avalonia --example hello_world
}
