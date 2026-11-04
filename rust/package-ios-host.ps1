#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Builds a reusable NativeAOT iOS host for one device or simulator RID.

.DESCRIPTION
This is a producer-side command and requires the patched Avalonia source,
.NET iOS workloads, Rust and Xcode. The resulting archive can be consumed
using Xcode and Rust without building C# or installing .NET.
#>
param(
    [Parameter(Mandatory)]
    [ValidateSet('ios-arm64', 'iossimulator-arm64', 'iossimulator-x64')]
    [string]$Rid,
    [string]$OutputRoot,
    [string]$RustoloniaRoot = (Split-Path -Parent $PSScriptRoot)
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'package-shared.ps1')

if (-not $IsMacOS) { throw 'Building an iOS host requires macOS and Xcode.' }
$root = (Resolve-Path -LiteralPath (Resolve-CallerRelativePath -PathValue $RustoloniaRoot)).Path
$producer = Join-Path $root 'avalonia-src'
Assert-ConsumerSource -ProducerRoot $producer -RustoloniaRoot $root
$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $root
$abi = Get-AbiFingerprint -RustoloniaRoot $root
$release = Get-Content -LiteralPath (Join-Path $root 'rust' 'release-manifest.json') -Raw | ConvertFrom-Json
$revision = (& git -C $root rev-parse HEAD).Trim()
$timestamp = if ($env:SOURCE_DATE_EPOCH) { [long]$env:SOURCE_DATE_EPOCH }
    else { [long](& git -C $root log -1 --format=%ct HEAD) }

$mode, $arch = switch ($Rid) {
    'ios-arm64' { 'device', 'arm64' }
    'iossimulator-arm64' { 'simulator-arm64', 'arm64' }
    'iossimulator-x64' { 'simulator-x64', 'x86_64' }
}
$previousSmoke = $env:RUSTOLONIA_IOS_SMOKE
try {
    $env:RUSTOLONIA_IOS_SMOKE = '0'
    $arguments = @($mode, '-p:RustoloniaIosSmoke=false')
    if ($Rid -eq 'ios-arm64') { $arguments += '-p:EnableCodeSigning=false' }
    & bash (Join-Path $root 'samples' 'ios' 'build.sh') @arguments
}
finally {
    if ($null -eq $previousSmoke) { Remove-Item Env:RUSTOLONIA_IOS_SMOKE -ErrorAction SilentlyContinue }
    else { $env:RUSTOLONIA_IOS_SMOKE = $previousSmoke }
}

$managed = Join-Path $root 'samples' 'ios' 'managed'
$obj = Join-Path $managed 'obj' 'Release' 'net10.0-ios' $Rid
$bin = Join-Path $managed 'bin' 'Release' 'net10.0-ios' $Rid
$app = Join-Path $bin 'Rustolonia.IosSample.app'
$hostObject = Join-Path $obj 'native' 'Rustolonia.IosSample.o'
$assetsPath = Join-Path $managed 'obj' 'project.assets.json'
$assets = Get-Content -LiteralPath $assetsPath -Raw | ConvertFrom-Json
$compiler = @($assets.libraries.PSObject.Properties.Name | Where-Object { $_ -match '^Microsoft\.DotNet\.ILCompiler/' })
if ($compiler.Count -ne 1 -or $compiler[0] -notmatch '^Microsoft\.DotNet\.ILCompiler/(\d+\.\d+\.\d+)$') {
    throw 'Cannot determine the NativeAOT compiler version from project.assets.json.'
}
$compilerVersion = $Matches[1]
$response = Get-Content -LiteralPath (Join-Path $obj 'native' 'Rustolonia.IosSample.ilc.rsp')
$coreLib = @($response | Where-Object {
    $_ -match 'microsoft\.netcore\.app\.runtime\.nativeaot\..*System\.Private\.CoreLib\.dll"?$'
})
if ($coreLib.Count -ne 1 -or $coreLib[0] -notmatch
    'microsoft\.netcore\.app\.runtime\.nativeaot\.[^/\\]+[/\\](\d+\.\d+\.\d+)[/\\]') {
    throw 'Cannot determine the NativeAOT runtime version from the compiler response file.'
}
$runtimeVersion = $Matches[1]
if ([version]$runtimeVersion -lt [version]'10.0.14' -or [version]$compilerVersion -lt [version]'10.0.14') {
    throw "NativeAOT 10.0.14 or later is required; found runtime $runtimeVersion and compiler $compilerVersion."
}
$coreLibPath = ($coreLib[0] -replace '^-r:', '').Trim('"')
$runtimePackage = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $coreLibPath)))
$linkerOptions = Get-Content -LiteralPath (Join-Path $obj 'custom-linker-options.txt') -Raw
if ($linkerOptions -notmatch '(?m)^\s*XamarinNativeLibraryDirectory=(.+?)\s*$') {
    throw 'Cannot locate the .NET iOS runtime pack from the linker metadata.'
}
$iosNativeDirectory = $Matches[1].Trim().TrimEnd([IO.Path]::DirectorySeparatorChar)
$iosPack = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $iosNativeDirectory))

$PSNativeCommandUseErrorActionPreference = $false
try {
    & /usr/bin/grep -a -q 'IosSmokeTest' $hostObject
    $grepResult = $LASTEXITCODE
}
finally { $PSNativeCommandUseErrorActionPreference = $true }
if ($grepResult -eq 0) { throw 'Refusing to package a smoke-enabled managed host.' }
if ($grepResult -ne 1) { throw "Could not inspect the managed host for smoke-test code (grep exit $grepResult)." }

if (-not $OutputRoot) { $OutputRoot = Join-Path $root 'rust' 'artifacts' 'host' }
$OutputRoot = Resolve-CallerRelativePath -PathValue $OutputRoot
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$assetName = Get-HostAssetName -Version $version -Rid $Rid
$assetPath = Join-Path $OutputRoot $assetName
if ((Test-Path -LiteralPath $assetPath) -or (Test-Path -LiteralPath "$assetPath.sha256")) {
    throw "Refusing to overwrite an existing host archive or checksum: $assetPath"
}
$staging = Join-Path $OutputRoot (".ios-staging-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $staging | Out-Null

function Copy-RequiredFile([string]$Source, [string]$Destination) {
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Required host input is missing: $Source" }
    Copy-Item -LiteralPath $Source -Destination $Destination
}

function Write-Lines([string]$Path, [string[]]$Lines) {
    [IO.File]::WriteAllText($Path, (($Lines -join "`n") + "`n"), [Text.UTF8Encoding]::new($false))
}

try {
    $bundle = Join-Path $staging 'host'
    foreach ($directory in @('native', 'bundle', 'link', 'licenses')) {
        New-Item -ItemType Directory -Path (Join-Path $bundle $directory) -Force | Out-Null
    }
    Copy-RequiredFile (Join-Path $obj 'nativelibraries' "main.$arch.o") (Join-Path $bundle 'native' 'entry.o')
    Copy-RequiredFile (Join-Path $obj 'nativelibraries' 'registrar.o') (Join-Path $bundle 'native' 'registrar.o')
    Copy-RequiredFile $hostObject (Join-Path $bundle 'native' 'host.o')
    Copy-RequiredFile (Join-Path $bin 'libbootstrapperdll.o') (Join-Path $bundle 'native' 'libbootstrapperdll.o')

    $libraries = @(
        'libxamarin-dotnet-nativeaot.a', 'libRuntime.WorkstationGC.a', 'libaotminipal.a',
        'libeventpipe-disabled.a', 'libstandalonegc-disabled.a'
    )
    if ($Rid -eq 'iossimulator-x64') { $libraries += 'libRuntime.VxsortEnabled.a' }
    if ($Rid -eq 'ios-arm64') {
        $libraries += @(
            'libSystem.Globalization.Native.a', 'libSystem.IO.Compression.Native.a',
            'libSystem.Native.a', 'libSystem.Net.Security.Native.a',
            'libSystem.Security.Cryptography.Native.Apple.a',
            'libbrotlienc.a', 'libbrotlidec.a', 'libbrotlicommon.a'
        )
    }
    foreach ($name in $libraries) {
        Copy-RequiredFile (Join-Path $bin $name) (Join-Path $bundle 'native' $name)
    }
    Copy-RequiredFile (Join-Path $app 'runtimeconfig.bin') (Join-Path $bundle 'bundle' 'runtimeconfig.bin')
    $frameworkDirectory = Join-Path $app 'Frameworks'
    if (-not (Test-Path -LiteralPath $frameworkDirectory -PathType Container)) {
        throw "The generated app has no native frameworks: $frameworkDirectory"
    }
    Copy-Item -LiteralPath $frameworkDirectory -Destination (Join-Path $bundle 'bundle') -Recurse -Force

    $dylibs = @()
    if ($Rid -ne 'ios-arm64') {
        $linkedBinary = Join-Path $obj 'nativelibraries' 'Rustolonia.IosSample'
        $dependencies = & xcrun otool -L $linkedBinary
        $dylibs = @($dependencies | ForEach-Object {
            if ($_ -match '@executable_path/(lib[A-Za-z0-9._-]+\.dylib)') { $Matches[1] }
        } | Sort-Object -Unique)
        if ($dylibs.Count -eq 0) { throw 'No .NET simulator dynamic libraries were found in the linked host.' }
        foreach ($name in $dylibs) {
            Copy-RequiredFile (Join-Path $app $name) (Join-Path $bundle 'bundle' $name)
        }
    }
    Write-Lines (Join-Path $bundle 'link' 'dylibs.txt') $dylibs

    [xml]$frameworkMetadata = Get-Content -LiteralPath (Join-Path $obj 'linker-items' '_LinkerFrameworks.items') -Raw
    $frameworks = [ordered]@{}
    foreach ($item in @($frameworkMetadata.Project.ItemGroup._LinkerFrameworks)) {
        $name = [string]$item.Include
        if ($name -notmatch '^[A-Za-z0-9._-]+$') { throw "Invalid generated framework name: $name" }
        $strength = if ([string]$item.IsWeak -eq 'true') { 'weak' } else { 'strong' }
        $frameworks[$name] = $strength
    }
    if ($frameworks.Count -eq 0) { throw 'The generated framework list is empty.' }
    foreach ($name in @('CFNetwork', 'Security')) {
        if (-not $frameworks.Contains($name)) { $frameworks[$name] = 'strong' }
    }
    Write-Lines (Join-Path $bundle 'link' 'frameworks.txt') @(
        $frameworks.Keys | ForEach-Object { "$($frameworks[$_]) $_" }
    )
    $symbols = @(Get-Content -LiteralPath (Join-Path $obj 'mtouch-symbols.list') | Where-Object { $_ })
    if ($symbols.Count -eq 0) { throw 'The generated linker symbol list is empty.' }
    Write-Lines (Join-Path $bundle 'link' 'symbols.txt') $symbols

    Copy-BundleNotices -ProducerRoot $producer -RustoloniaRoot $root -DestinationDirectory $bundle
    Copy-RequiredFile (Join-Path $runtimePackage 'LICENSE.TXT') (Join-Path $bundle 'licenses' 'dotnet-LICENSE.txt')
    Copy-RequiredFile (Join-Path $runtimePackage 'THIRD-PARTY-NOTICES.TXT') (Join-Path $bundle 'licenses' 'dotnet-THIRD-PARTY-NOTICES.txt')
    Copy-RequiredFile (Join-Path $iosPack 'LICENSE') (Join-Path $bundle 'licenses' 'dotnet-ios-LICENSE.txt')
    foreach ($packageName in @('SkiaSharp.NativeAssets.iOS', 'HarfBuzzSharp.NativeAssets.iOS')) {
        $package = @($assets.libraries.PSObject.Properties | Where-Object { $_.Name -like "$packageName/*" })
        if ($package.Count -ne 1) { throw "Cannot find the restored $packageName package." }
        $packagePath = $null
        foreach ($folder in $assets.packageFolders.PSObject.Properties.Name) {
            $candidate = Join-Path $folder $package[0].Value.path
            if (Test-Path -LiteralPath $candidate -PathType Container) { $packagePath = $candidate; break }
        }
        if (-not $packagePath) { throw "Cannot locate the restored $packageName package on disk." }
        Copy-RequiredFile (Join-Path $packagePath 'LICENSE.txt') (Join-Path $bundle 'licenses' "$packageName-LICENSE.txt")
        $notice = Join-Path $packagePath 'THIRD-PARTY-NOTICES.txt'
        if (Test-Path -LiteralPath $notice -PathType Leaf) {
            Copy-Item -LiteralPath $notice -Destination (Join-Path $bundle 'licenses' "$packageName-THIRD-PARTY-NOTICES.txt")
        }
    }
    Copy-RequiredFile (Join-Path $root 'rust' 'ios' 'build-app.sh') (Join-Path $bundle 'build-app.sh')

    & (Join-Path $root 'rust' 'generate-sbom.ps1') -Rid $Rid -Bundle $bundle `
        -ProjectAssetsJsonPath $assetsPath -ProducerPin $release.producerPin
    $files = @(Get-ChildItem -LiteralPath $bundle -File -Force -Recurse |
        ForEach-Object {
            $name = [IO.Path]::GetRelativePath($bundle, $_.FullName).Replace('\', '/')
            [ordered]@{ name = $name; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
        } | Sort-Object { $_.name } -CaseSensitive)
    $manifest = [ordered]@{
        schemaVersion = 1
        kind = 'rustolonia-ios-host'
        version = $version
        rid = $Rid
        abiFingerprint = $abi
        nativeAotRuntimeVersion = $runtimeVersion
        nativeAotCompilerVersion = $compilerVersion
        sourceRevision = $revision
        producerPin = [string]$release.producerPin
        files = $files
    }
    $json = ($manifest | ConvertTo-Json -Depth 6) -replace "`r`n", "`n"
    [IO.File]::WriteAllText((Join-Path $bundle 'host-manifest.json'), "$json`n", [Text.UTF8Encoding]::new($false))
    $paths = @(Get-ChildItem -LiteralPath $bundle -File -Force -Recurse |
        ForEach-Object { [IO.Path]::GetRelativePath($bundle, $_.FullName) })
    $executables = @('build-app.sh')
    foreach ($framework in @(Get-ChildItem -LiteralPath (Join-Path $bundle 'bundle' 'Frameworks') -Directory -Filter '*.framework')) {
        $binary = Join-Path $framework.FullName ([IO.Path]::GetFileNameWithoutExtension($framework.Name))
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Framework binary is missing: $binary" }
        $executables += [IO.Path]::GetRelativePath($bundle, $binary)
    }
    $stagedAsset = Join-Path $staging $assetName
    New-DeterministicTarGz -SourceDirectory $bundle -RelativePaths $paths -Destination $stagedAsset `
        -Timestamp $timestamp -ExecutablePaths $executables
    $hash = Write-AssetChecksum -Asset $stagedAsset
    [IO.File]::Move($stagedAsset, $assetPath)
    [IO.File]::Move("$stagedAsset.sha256", "$assetPath.sha256")
    Write-Host "==> $assetName ($hash)"
}
finally {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
