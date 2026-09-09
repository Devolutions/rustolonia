#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Package the NativeAOT host and a Rust example for one RID.
#>
param(
    [Parameter(Mandatory)]
    [ValidateSet('win-x64', 'win-arm64', 'linux-x64', 'linux-arm64', 'osx-x64', 'osx-arm64')]
    [string]$Rid,
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release',
    [string]$Example = 'hello_world',
    [string]$OutputRoot,
    [switch]$SkipCargoBuild,
    [string]$RustoloniaRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$ProducerRoot,
    [string]$PresentationProject,
    [string]$ViewRegistryFile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'package-shared.ps1')

$resolvedRustoloniaRoot = (Resolve-Path -LiteralPath (Resolve-CallerRelativePath -PathValue $RustoloniaRoot)).Path
if ([string]::IsNullOrWhiteSpace($ProducerRoot)) {
    $resolvedProducerRoot = Join-Path $resolvedRustoloniaRoot 'avalonia-src'
}
else {
    $resolvedProducerRoot = (Resolve-Path -LiteralPath (Resolve-CallerRelativePath -PathValue $ProducerRoot)).Path
}
if (-not (Test-Path -LiteralPath $resolvedProducerRoot -PathType Container)) {
    throw "Producer root does not exist: $resolvedProducerRoot"
}
$target = Get-RidTargetInfo -Rid $Rid
$hostProject = Join-Path $resolvedRustoloniaRoot 'host' 'Avalonia.Host.csproj'
$artifactsRoot = if ($env:AVN_DOTNET_ARTIFACTS) { $env:AVN_DOTNET_ARTIFACTS } else { Join-Path $PSScriptRoot 'target' "dotnet-$Rid" }

Ensure-RidPrerequisites -Rid $Rid -ProducerRoot $resolvedProducerRoot -Configuration $Configuration

if (-not $OutputRoot) {
    $OutputRoot = if ($env:AVN_PACKAGE_OUTPUT) { $env:AVN_PACKAGE_OUTPUT } else { Join-Path $PSScriptRoot 'artifacts' }
}
$OutputRoot = Resolve-CallerRelativePath -PathValue $OutputRoot
$destination = Join-Path $OutputRoot $Rid
Assert-ArtifactBundleDestination -BundlePath $destination

$publishProperties = New-HostPublishProperties -ProducerRoot $resolvedProducerRoot -RustoloniaRoot $resolvedRustoloniaRoot -Rid $Rid -HostPlatform $($target.Platform)
if (-not [string]::IsNullOrWhiteSpace($PresentationProject)) {
    $resolvedPresentationProject = (Resolve-Path -LiteralPath (Resolve-CallerRelativePath -PathValue $PresentationProject)).Path
    $publishProperties += "-p:AvaloniaRustPresentationProjects=$resolvedPresentationProject"
}
if (-not [string]::IsNullOrWhiteSpace($ViewRegistryFile)) {
    $resolvedViewRegistryFile = (Resolve-Path -LiteralPath (Resolve-CallerRelativePath -PathValue $ViewRegistryFile)).Path
    $publishProperties += "-p:AvaloniaRustViewRegistryFile=$resolvedViewRegistryFile"
}

Write-Host "==> Publishing Avalonia.Host ($Rid, $Configuration)"
$publishCommand = New-DotnetPublishCommand -Project $hostProject -Configuration $Configuration -Rid $Rid -ArtifactsPath $artifactsRoot -AdditionalProperties $publishProperties
Invoke-Logged -Command $publishCommand
$hostAssets = Get-PublishedProjectAssetsFile -Project $hostProject -Configuration $Configuration -Rid $Rid -ArtifactsPath $artifactsRoot -AdditionalProperties $publishProperties

$publishDir = Join-Path $artifactsRoot 'publish' 'Avalonia.Host' "$($Configuration.ToLowerInvariant())_$Rid"
$hostFile = Join-Path $publishDir "Avalonia.Host$($target.HostExtension)"
if (-not (Test-Path -LiteralPath $hostFile -PathType Leaf)) {
    throw "NativeAOT host was not produced at $hostFile"
}

$finalDestination = $destination
Invoke-ArtifactBundleTransaction -BundlePath $finalDestination -Rid $Rid -Build {
param($destination)
Write-Host "==> Copying host and native dependencies into $destination"
Copy-BundleFiles -SourceDirectory $publishDir -DestinationDirectory $destination -HostFile $hostFile -Rid $Rid
Copy-BundleNotices -ProducerRoot $resolvedProducerRoot -RustoloniaRoot $resolvedRustoloniaRoot -DestinationDirectory $destination

$signTargets = @(
    (Join-Path $destination (Split-Path -Leaf $hostFile))
)

if (-not $SkipCargoBuild -and -not $env:AVN_PACKAGE_SKIP_CARGO_BUILD) {
    Write-Host "==> Building Rust example '$Example' ($Configuration) next to the host"
    $cargoTarget = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $PSScriptRoot 'target' "cargo-$Rid" }
    $previousCargo = [Environment]::GetEnvironmentVariable('CARGO_TARGET_DIR', 'Process')
    [Environment]::SetEnvironmentVariable('CARGO_TARGET_DIR', $cargoTarget, 'Process')
    try {
        Set-WindowsStaticCrt -Triple $target.Triple
        $cargoCommand = New-CargoBuildCommand -ManifestPath (Join-Path $resolvedRustoloniaRoot 'rust' 'Cargo.toml') -PackageName 'avalonia' -TargetTriple $target.Triple -Configuration $Configuration -Example $Example
        Invoke-Logged -Command $cargoCommand
    }
    finally {
        if ($null -eq $previousCargo) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable('CARGO_TARGET_DIR', $previousCargo, 'Process') }
    }
    $profile = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }
    $exePath = Join-Path $cargoTarget $target.Triple $profile 'examples' "$Example$($target.ExeExtension)"
    if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
        throw "Rust example binary was not produced at $exePath"
    }
    Copy-Item -LiteralPath $exePath -Destination $destination
    $signTargets += (Join-Path $destination (Split-Path -Leaf $exePath))
}

Invoke-ArtifactSigning -ArtifactDirectory $destination -SignCommand $env:AVALONIA_RUST_SIGN_COMMAND -ExplicitFiles $signTargets
Write-Host '==> Writing deterministic CycloneDX delivery SBOM'
$producerPin = git -C $resolvedProducerRoot rev-parse HEAD 2>$null
& (Join-Path $PSScriptRoot 'generate-sbom.ps1') -Rid $Rid -Bundle $destination `
    -CargoLockPath (Join-Path $resolvedRustoloniaRoot 'rust' 'Cargo.lock') `
    -ProjectAssetsJsonPath $hostAssets `
    -ProducerPin $producerPin
Write-Checksums -Bundle $destination
}

Write-Host "Package layout ready at $finalDestination"
Get-ChildItem -LiteralPath $finalDestination | Select-Object Name, Length | Format-Table -AutoSize
