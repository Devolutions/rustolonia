#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Builds the prebuilt Rustolonia Android host APK for a release.

.DESCRIPTION
Builds host/Android/App (the Android host with generic Application/Activity
classes and no Rust library) for arm64-v8a and x86_64 and writes

    rustolonia-android-host-<version>.apk
    rustolonia-android-host-<version>.apk.sha256

to -OutputRoot (default rust/target/android-host). Code-first apps turn it
into their own APK with `cargo rustolonia-apk --host-apk <apk>` (see
rust/PLATFORMS.md#android); no .NET tooling is needed on their side.

Its build-time debug signature is irrelevant: rustolonia-apk rebuilds the
archive, which drops every existing signature, and signs the application APK.

Requirements: the .NET `android` workload and an Android SDK (ANDROID_HOME,
ANDROID_SDK_ROOT or the platform default location).
#>
param(
    [string]$OutputRoot = (Join-Path $PSScriptRoot 'target' 'android-host')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$project = Join-Path $repositoryRoot 'host' 'Android' 'App' 'Rustolonia.Android.HostApp.csproj'
$version = (Select-String -LiteralPath (Join-Path $PSScriptRoot 'rustolonia-sys' 'Cargo.toml') -Pattern '^version = "(.+)"$' |
    Select-Object -First 1).Matches[0].Groups[1].Value

$sdkArgs = @()
foreach ($candidate in @($env:ANDROID_HOME, $env:ANDROID_SDK_ROOT, $(if ($IsWindows) { Join-Path $env:LOCALAPPDATA 'Android' 'Sdk' } else { Join-Path $HOME 'Android' 'Sdk' }))) {
    if ($candidate -and (Test-Path -LiteralPath (Join-Path $candidate 'platform-tools'))) {
        $sdkArgs += "-p:AndroidSdkDirectory=$candidate"
        break
    }
}
if ($env:JAVA_HOME) { $sdkArgs += "-p:JavaSdkDirectory=$env:JAVA_HOME" }

dotnet build $project -c Release @sdkArgs

$built = Join-Path (Split-Path -Parent $project) 'bin' 'Release' 'net10.0-android36.0' 'net.devolutions.rustolonia.hostapp-Signed.apk'
if (-not (Test-Path -LiteralPath $built -PathType Leaf)) {
    throw "The build produced no $built."
}
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$apk = Join-Path $OutputRoot "rustolonia-android-host-$version.apk"
Copy-Item -LiteralPath $built -Destination $apk -Force
$hash = (Get-FileHash -LiteralPath $apk -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -LiteralPath "$apk.sha256" -Value "$hash  $(Split-Path -Leaf $apk)" -NoNewline
Write-Host "Android host: $apk"
Write-Host "SHA-256:      $hash"
