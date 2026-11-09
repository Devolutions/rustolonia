#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Builds the prebuilt rustolonia host release tarball for one RID.

.DESCRIPTION
Publishes the code-first NativeAOT host (no application view registry), copies
its native dependencies and notices, signs the binaries with
AVALONIA_RUST_SIGN_COMMAND when set, writes a CycloneDX SBOM and
host-manifest.json, and archives everything as

    rustolonia-host-<version>-<rid>[-drm][-static][-devtools].tar.gz
    rustolonia-host-<version>-<rid>[-drm][-devtools]-symbols.tar.gz

with a `.sha256` file next to each. rustolonia-sys downloads the default
flavor from the GitHub release `v<version>`.

-Linkage Static publishes the host as a static library (NativeLib=Static)
together with the NativeAOT runtime libraries and rustolonia-host-link.txt,
for the rustolonia-sys `static-host` feature. Its debug information stays in
the static library, so no separate symbols archive is produced.

The archives are deterministic for identical inputs; the timestamp comes from SOURCE_DATE_EPOCH or the HEAD
commit.

Pass -PublishDirectory to archive an already published host instead of
running `dotnet publish` (used by CI smoke tests).
#>
param(
    [Parameter(Mandatory)]
    [ValidateSet('win-x64', 'win-arm64', 'linux-x64', 'linux-arm64', 'linux-musl-x64', 'linux-musl-arm64', 'osx-x64', 'osx-arm64')]
    [string]$Rid,
    [ValidateSet('X11', 'Drm')]
    [string]$LinuxBackend = 'X11',
    [ValidateSet('default', 'devtools')]
    [string]$Flavor = 'default',
    [ValidateSet('Shared', 'Static')]
    [string]$Linkage = 'Shared',
    [string]$OutputRoot,
    [string]$RustoloniaRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$ProducerRoot,
    [string]$PublishDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'package-shared.ps1')

function Get-GitRevision([string]$Directory) {
    $PSNativeCommandUseErrorActionPreference = $false
    $revision = & git -C $Directory rev-parse HEAD 2>$null
    if ($LASTEXITCODE -eq 0) { return $revision } else { return $null }
}

$resolvedRustoloniaRoot = (Resolve-Path -LiteralPath (Resolve-CallerRelativePath -PathValue $RustoloniaRoot)).Path
$resolvedProducerRoot = if ([string]::IsNullOrWhiteSpace($ProducerRoot)) {
    Join-Path $resolvedRustoloniaRoot 'avalonia-src'
}
else {
    (Resolve-Path -LiteralPath (Resolve-CallerRelativePath -PathValue $ProducerRoot)).Path
}
if (-not (Test-Path -LiteralPath $resolvedProducerRoot -PathType Container)) {
    throw "Producer root does not exist: $resolvedProducerRoot"
}

$target = Get-RidTargetInfo -Rid $Rid -LinuxBackend $LinuxBackend
Assert-RidLinkage -Rid $Rid -Linkage $Linkage
$backendSuffix = if ($LinuxBackend -eq 'Drm') { '-drm' } else { '' }
$static = $Linkage -eq 'Static'
$linkageSuffix = if ($static) { '-static' } else { '' }
$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $resolvedRustoloniaRoot
$abiFingerprint = Get-AbiFingerprint -RustoloniaRoot $resolvedRustoloniaRoot
$sourceRevision = Get-GitRevision $resolvedRustoloniaRoot
$timestamp = if ($env:SOURCE_DATE_EPOCH) { [long]$env:SOURCE_DATE_EPOCH }
    elseif ($sourceRevision) { [long](& git -C $resolvedRustoloniaRoot log -1 --format=%ct HEAD) }
    else { 0 }

if (-not $OutputRoot) {
    $OutputRoot = Join-Path $PSScriptRoot 'artifacts' 'host'
}
$OutputRoot = Resolve-CallerRelativePath -PathValue $OutputRoot
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

$hostAssets = $null
if ([string]::IsNullOrWhiteSpace($PublishDirectory)) {
    Ensure-RidPrerequisites -Rid $Rid -ProducerRoot $resolvedProducerRoot -Configuration Release
    $hostProject = Join-Path $resolvedRustoloniaRoot 'host' 'Avalonia.Host.csproj'
    $artifactsRoot = if ($env:AVN_DOTNET_ARTIFACTS) { $env:AVN_DOTNET_ARTIFACTS } else { Join-Path $PSScriptRoot 'target' "dotnet-host-$Rid$backendSuffix$linkageSuffix-$Flavor" }
    $publishProperties = New-HostPublishProperties -ProducerRoot $resolvedProducerRoot -RustoloniaRoot $resolvedRustoloniaRoot `
        -Rid $Rid -HostPlatform $target.Platform -DeveloperTools:($Flavor -eq 'devtools')
    if ($static) { $publishProperties += '-p:NativeLib=Static' }

    Write-Host "==> Publishing code-first host ($Rid, $Flavor, $Linkage)"
    Invoke-Logged -Command (New-DotnetPublishCommand -Project $hostProject -Configuration Release -Rid $Rid -ArtifactsPath $artifactsRoot -AdditionalProperties $publishProperties)
    $hostAssets = Get-PublishedProjectAssetsFile -Project $hostProject -Configuration Release -Rid $Rid -ArtifactsPath $artifactsRoot -AdditionalProperties $publishProperties
    $PublishDirectory = Join-Path $artifactsRoot 'publish' 'Avalonia.Host' "release_$Rid"
}
$PublishDirectory = (Resolve-Path -LiteralPath (Resolve-CallerRelativePath -PathValue $PublishDirectory)).Path
if ($static -and -not $hostAssets) {
    $hostAssets = Join-Path $PublishDirectory 'rustolonia-project-assets.json'
    if (-not (Test-Path -LiteralPath $hostAssets -PathType Leaf)) {
        throw "The static publish is missing its restored package metadata: $hostAssets. Republish the host before packaging it."
    }
}
$hostFileName = if ($static) { $target.StaticHostFileName } else { $target.HostFileName }
$hostFile = Join-Path $PublishDirectory $hostFileName
if (-not (Test-Path -LiteralPath $hostFile -PathType Leaf)) {
    throw "NativeAOT host was not produced at $hostFile"
}

$assetName = Get-HostAssetName -Version $version -Rid $Rid -Flavor $Flavor -LinuxBackend $LinuxBackend -Linkage $Linkage
$symbolsName = if (-not $static) { Get-HostAssetName -Version $version -Rid $Rid -Flavor $Flavor -LinuxBackend $LinuxBackend -Symbols }
$staging = New-IsolatedPackageStagingRoot -OutputRoot $OutputRoot -Rid $Rid
try {
    $bundle = Prepare-ArtifactBundle -BundlePath (Join-Path $staging 'host')
    Write-Host "==> Staging host and native dependencies"
    Copy-BundleFiles -SourceDirectory $PublishDirectory -DestinationDirectory $bundle -HostFile $hostFile -Rid $Rid -Static:$static
    Copy-BundleNotices -ProducerRoot $resolvedProducerRoot -RustoloniaRoot $resolvedRustoloniaRoot -DestinationDirectory $bundle
    if ($static) {
        Copy-StaticNativeNotices -ProjectAssetsJsonPath $hostAssets -DestinationDirectory $bundle -Rid $Rid -StaticPublishDirectory $PublishDirectory
    }
    Remove-Item -LiteralPath (Join-Path $bundle '.rustolonia-bundle-owner') -Force
    # Copy-BundleFiles matches *.so.*, which includes Linux .so.dbg symbol files.
    Get-ChildItem -LiteralPath $bundle -File -Filter '*.dbg' | Remove-Item -Force

    # A static library is signed as part of the application that links it.
    $explicitSigning = if ($static) { @() } else { @(Join-Path $bundle $hostFileName) }
    Invoke-ArtifactSigning -ArtifactDirectory $bundle -SignCommand $env:AVALONIA_RUST_SIGN_COMMAND -ExplicitFiles $explicitSigning

    Write-Host '==> Writing CycloneDX SBOM and host-manifest.json'
    $sbomArguments = @{ Rid = $Rid; Bundle = $bundle; ProducerPin = (Get-GitRevision $resolvedProducerRoot) }
    if ($hostAssets) { $sbomArguments.ProjectAssetsJsonPath = $hostAssets }
    & (Join-Path $PSScriptRoot 'generate-sbom.ps1') @sbomArguments
    Write-HostManifest -Directory $bundle -Version $version -Rid $Rid -AbiFingerprint $abiFingerprint `
        -RustoloniaRoot $resolvedRustoloniaRoot -Flavor $Flavor -LinuxBackend $LinuxBackend -Linkage $Linkage -SourceRevision $sourceRevision

    $assetPath = Join-Path $OutputRoot $assetName
    $files = @(Get-ChildItem -LiteralPath $bundle -File -Force | ForEach-Object Name)
    New-DeterministicTarGz -SourceDirectory $bundle -RelativePaths $files -Destination $assetPath -Timestamp $timestamp
    $hash = Write-AssetChecksum -Asset $assetPath
    Write-Host "==> $assetName ($hash)"

    $symbols = @(Get-HostSymbolFiles -PublishDirectory $PublishDirectory)
    if ($static) {
        # Static libraries keep their debug information; there is nothing to split out.
    }
    elseif ($symbols.Count -gt 0) {
        $symbolsPath = Join-Path $OutputRoot $symbolsName
        New-DeterministicTarGz -SourceDirectory $PublishDirectory -RelativePaths $symbols -Destination $symbolsPath -Timestamp $timestamp
        $symbolsHash = Write-AssetChecksum -Asset $symbolsPath
        Write-Host "==> $symbolsName ($symbolsHash)"
    }
    else {
        Write-Warning "No debug symbols found in $PublishDirectory; skipping $symbolsName."
    }
}
finally {
    Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
}
