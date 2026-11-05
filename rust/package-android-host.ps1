#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Builds the prebuilt Rustolonia Android host APK for a release.

.DESCRIPTION
Builds host/Android/App (the Android host with generic Application/Activity
classes and no Rust library) for arm64-v8a and x86_64 and writes

    rustolonia-host-<version>-android.apk
    rustolonia-host-<version>-android.apk.sha256

to -OutputRoot (default rust/target/android-host). Releases publish it beside
the desktop and iOS host tarballs. Code-first apps turn it into their own APK
with `cargo rustolonia-apk`, which downloads it and verifies it against
rustolonia-sys's host-checksums.txt (or takes `--host-apk <apk>` for a local
build); see rust/PLATFORMS.md#android. No .NET tooling is needed on their side.

Its build-time debug signature is irrelevant: rustolonia-apk rebuilds the
archive, which drops every existing signature, and signs the application APK.

Requirements: the .NET `android` workload and an Android SDK (ANDROID_HOME,
ANDROID_SDK_ROOT or the platform default location).
#>
param(
    [string]$OutputRoot = (Join-Path $PSScriptRoot 'target' 'android-host'),
    # Extra arguments for `dotnet build` (for example -p:RestoreConfigFile=...).
    [string[]]$DotnetArgs = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'package-shared.ps1')

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$project = Join-Path $repositoryRoot 'host' 'Android' 'App' 'Rustolonia.Android.HostApp.csproj'
$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $repositoryRoot

$sdkArgs = @()
foreach ($candidate in @($env:ANDROID_HOME, $env:ANDROID_SDK_ROOT, $(if ($IsWindows) { Join-Path $env:LOCALAPPDATA 'Android' 'Sdk' } else { Join-Path $HOME 'Android' 'Sdk' }))) {
    if ($candidate -and (Test-Path -LiteralPath (Join-Path $candidate 'platform-tools'))) {
        $sdkArgs += "-p:AndroidSdkDirectory=$candidate"
        break
    }
}
if ($env:JAVA_HOME) { $sdkArgs += "-p:JavaSdkDirectory=$env:JAVA_HOME" }

dotnet build $project -c Release @sdkArgs @DotnetArgs

$built = Join-Path (Split-Path -Parent $project) 'bin' 'Release' 'net10.0-android36.0' 'net.devolutions.rustolonia.hostapp-Signed.apk'
if (-not (Test-Path -LiteralPath $built -PathType Leaf)) {
    throw "The build produced no $built."
}
$OutputRoot = Resolve-CallerRelativePath -PathValue $OutputRoot
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$apk = Join-Path $OutputRoot (Get-AndroidHostAssetName -Version $version)
Copy-Item -LiteralPath $built -Destination $apk -Force
$hash = Write-AssetChecksum -Asset $apk
Write-Host "Android host: $apk"
Write-Host "SHA-256:      $hash"
