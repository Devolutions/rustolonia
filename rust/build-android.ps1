#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Build a Rustolonia Android app (Rust cdylib + .NET Android host) into an APK.

.DESCRIPTION
Compiles the app's Rust library for each requested Android ABI with the NDK
clang linkers, then builds the app's .NET Android project, which references
host/Android/Avalonia.Host.Android.csproj and packages the libraries. See
rust/PLATFORMS.md#android.

Requirements: the .NET `android` workload, an Android SDK (ANDROID_HOME,
ANDROID_SDK_ROOT or %LOCALAPPDATA%\Android\Sdk / ~/Android/Sdk) with an NDK
(ANDROID_NDK_HOME or the newest <sdk>/ndk/*), and the Rust targets
`aarch64-linux-android` / `x86_64-linux-android`.

.PARAMETER AppDirectory
The app's Cargo directory; its Android project is the single *.csproj in
<AppDirectory>/android. Defaults to apps/android-hello.

.PARAMETER Abi
Android ABIs to build: x86_64 (emulator) and/or arm64-v8a (devices).

.PARAMETER Install
Install the APK on the connected device or emulator (adb install -r).

.PARAMETER Launch
After installing, start the app's launcher activity.

.EXAMPLE
pwsh ./rust/build-android.ps1 -Abi x86_64 -Install -Launch
#>
param(
    [string]$AppDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) 'apps' 'android-hello'),
    [ValidateSet('x86_64', 'arm64-v8a')]
    [string[]]$Abi = @('arm64-v8a', 'x86_64'),
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release',
    [int]$MinSdk = 24,
    [switch]$Install,
    [switch]$Launch
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$AppDirectory = (Resolve-Path -LiteralPath $AppDirectory).Path
$producerProject = Join-Path $repositoryRoot 'avalonia-src' 'src' 'Android' 'Avalonia.Android' 'Avalonia.Android.csproj'
if (-not (Test-Path -LiteralPath $producerProject -PathType Leaf)) {
    throw 'Initialize the producer first: git submodule update --init --recursive; then pwsh ./avalonia-patches/apply-avalonia-patches.ps1 -AvaloniaRoot ./avalonia-src'
}

function Find-AndroidSdk {
    $candidates = @($env:ANDROID_HOME, $env:ANDROID_SDK_ROOT)
    if ($IsWindows) { $candidates += Join-Path $env:LOCALAPPDATA 'Android' 'Sdk' }
    elseif ($IsMacOS) { $candidates += Join-Path $HOME 'Library' 'Android' 'sdk' }
    else { $candidates += Join-Path $HOME 'Android' 'Sdk' }
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath (Join-Path $candidate 'platform-tools'))) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    throw 'No Android SDK found. Set ANDROID_HOME to an SDK with platform-tools, platforms;android-36 and an NDK.'
}

function Find-AndroidNdk([string]$Sdk) {
    foreach ($candidate in @($env:ANDROID_NDK_HOME, $env:ANDROID_NDK_ROOT)) {
        if ($candidate -and (Test-Path -LiteralPath (Join-Path $candidate 'toolchains'))) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    $ndkRoot = Join-Path $Sdk 'ndk'
    $newest = if (Test-Path -LiteralPath $ndkRoot) {
        Get-ChildItem -LiteralPath $ndkRoot -Directory |
            Sort-Object { [version]($_.Name -replace '[^0-9.].*$', '') } |
            Select-Object -Last 1
    }
    if (-not $newest) {
        throw "No NDK found. Install one with: sdkmanager `"ndk;28.2.13676358`" (or set ANDROID_NDK_HOME)."
    }
    return $newest.FullName
}

$sdk = Find-AndroidSdk
$ndk = Find-AndroidNdk $sdk
$hostTag = if ($IsWindows) { 'windows-x86_64' } elseif ($IsMacOS) { 'darwin-x86_64' } else { 'linux-x86_64' }
$toolchain = Join-Path $ndk 'toolchains' 'llvm' 'prebuilt' $hostTag 'bin'
if (-not (Test-Path -LiteralPath $toolchain)) {
    throw "The NDK at $ndk has no $hostTag LLVM toolchain."
}
Write-Host "Android SDK: $sdk"
Write-Host "Android NDK: $ndk"

$targets = @{
    'arm64-v8a' = 'aarch64-linux-android'
    'x86_64'    = 'x86_64-linux-android'
}
$installedTargets = rustup target list --installed
$targetDir = Join-Path $AppDirectory 'target'
$profile = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }

foreach ($abiName in $Abi) {
    $triple = $targets[$abiName]
    if ($installedTargets -notcontains $triple) {
        throw "Rust target $triple is not installed. Run: rustup target add $triple"
    }
    $suffix = if ($IsWindows) { '.cmd' } else { '' }
    $clang = Join-Path $toolchain "$triple$MinSdk-clang$suffix"
    $ar = Join-Path $toolchain "llvm-ar$(if ($IsWindows) { '.exe' })"
    $envName = $triple.ToUpperInvariant().Replace('-', '_')
    $ccName = $triple.Replace('-', '_')
    Set-Item "env:CARGO_TARGET_${envName}_LINKER" $clang
    Set-Item "env:CC_$ccName" $clang
    Set-Item "env:AR_$ccName" $ar

    Write-Host "cargo build ($triple, $profile)"
    $cargoArgs = @('build', '--lib', '--target', $triple, '--target-dir', $targetDir)
    if ($Configuration -eq 'Release') { $cargoArgs += '--release' }
    Push-Location -LiteralPath $AppDirectory
    try { cargo @cargoArgs } finally { Pop-Location }
}

$androidProject = @(Get-ChildItem -LiteralPath (Join-Path $AppDirectory 'android') -Filter '*.csproj' -File)
if ($androidProject.Count -ne 1) {
    throw "Expected exactly one .csproj in $(Join-Path $AppDirectory 'android')."
}
$project = $androidProject[0].FullName
$rids = $Abi | ForEach-Object { if ($_ -eq 'x86_64') { 'android-x64' } else { 'android-arm64' } }
$javaArgs = @()
if ($env:JAVA_HOME) { $javaArgs += "-p:JavaSdkDirectory=$env:JAVA_HOME" }

Write-Host "dotnet build $project ($Configuration; $($rids -join ', '))"
dotnet build $project -c $Configuration `
    "-p:AndroidSdkDirectory=$sdk" "-p:AndroidNdkDirectory=$ndk" @javaArgs `
    "-p:RustTargetDir=$targetDir" "-p:RustProfile=$profile" `
    "-p:RustoloniaAndroidRuntimeIdentifiers=$($rids -join '+')"

$apk = Get-ChildItem -LiteralPath (Join-Path (Split-Path -Parent $project) 'bin' $Configuration) -Recurse -Filter '*-Signed.apk' |
    Sort-Object LastWriteTime | Select-Object -Last 1
if (-not $apk) { throw 'The build produced no signed APK.' }
Write-Host "APK: $($apk.FullName)"

if ($Install -or $Launch) {
    $adb = Join-Path $sdk 'platform-tools' "adb$(if ($IsWindows) { '.exe' })"
    & $adb install -r $apk.FullName
    if ($Launch) {
        $package = $apk.BaseName -replace '-Signed$', ''
        & $adb shell monkey -p $package -c android.intent.category.LAUNCHER 1 | Out-Null
        Write-Host "Launched $package. Logs: adb logcat -s rustolonia DOTNET AndroidRuntime"
    }
}
