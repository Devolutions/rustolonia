#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Build and package an external Rust/AXAML consumer against a pinned producer.
#>
param(
    [Parameter(Mandatory)][string]$ProducerRoot,
    [Parameter(Mandatory)][string]$Manifest,
    # The rustolonia repository root (defaults to the checkout this script lives in).
    [string]$RustoloniaRoot = (Split-Path -Parent $PSScriptRoot),
    [switch]$SkipGenerate,
    [switch]$UpdateLockFile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'package-shared.ps1')

$script:PathFields = @(
    'presentationProject', 'viewModelIr', 'generatedAdaptersDirectory',
    'generatedRegistryFile', 'generatedRustFile', 'generatedContractFile',
    'cargoManifest', 'outputDirectory'
)
$script:Required = @('version') + $script:PathFields[0..6] + @('packageName', 'rid', 'configuration', 'outputDirectory')

function Resolve-ManifestPath {
    param($ManifestDirectory, [string]$Value)
    if ([System.IO.Path]::IsPathRooted($Value)) { return [System.IO.Path]::GetFullPath($Value) }
    return [System.IO.Path]::GetFullPath((Join-Path $ManifestDirectory $Value))
}

function Resolve-ManifestNoticePath {
    param([string]$ManifestDirectory, [string]$Value)
    if ([System.IO.Path]::IsPathRooted($Value)) {
        throw 'Invalid consumer manifest: noticeFiles entries must be relative to the manifest directory'
    }

    $candidate = [IO.Path]::GetFullPath((Join-Path $ManifestDirectory $Value))
    $relative = [IO.Path]::GetRelativePath($ManifestDirectory, $candidate)
    $separator = [IO.Path]::DirectorySeparatorChar
    $alternateSeparator = [IO.Path]::AltDirectorySeparatorChar
    if ([IO.Path]::IsPathRooted($relative) -or
        $relative -eq '..' -or
        $relative.StartsWith("..$separator", [StringComparison]::Ordinal) -or
        $relative.StartsWith("..$alternateSeparator", [StringComparison]::Ordinal)) {
        throw 'Invalid consumer manifest: noticeFiles entries must remain within the manifest directory'
    }
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "Invalid consumer manifest: noticeFiles entry does not exist: $candidate"
    }

    $current = $ManifestDirectory
    foreach ($segment in $relative -split '[\\/]') {
        $current = Join-Path $current $segment
        $item = Get-Item -LiteralPath $current -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw 'Invalid consumer manifest: noticeFiles entries must not traverse symbolic links or reparse points'
        }
    }
    return $candidate
}

function Read-ConsumerManifest {
    param([string]$ManifestPath)
    if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
        throw "Invalid consumer manifest: manifest does not exist: $ManifestPath"
    }
    $document = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json -AsHashtable
    if ($document -isnot [hashtable]) { throw 'Invalid consumer manifest: the document must be an object' }
    $unknown = @($document.Keys | Where-Object { $_ -cnotin ($script:Required + @('binary', 'noticeFiles')) })
    if ($unknown.Count -gt 0) { throw "Invalid consumer manifest: unknown field(s): $($unknown -join ', ')" }
    $missing = @($script:Required | Where-Object { -not $document.ContainsKey($_) })
    if ($missing.Count -gt 0) { throw "Invalid consumer manifest: missing required field(s): $($missing -join ', ')" }
    if (($document.version -isnot [long] -and $document.version -isnot [int] -and
         $document.version -isnot [double] -and $document.version -isnot [decimal]) -or $document.version -ne 1) {
        throw 'Invalid consumer manifest: version must be the number 1'
    }
    foreach ($field in $script:PathFields) {
        if ($document[$field] -isnot [string] -or [string]::IsNullOrWhiteSpace($document[$field])) {
            throw "Invalid consumer manifest: $field must be a non-empty string"
        }
    }
    if ($document.rid -isnot [string] -or $document.rid -cnotin $script:RidTargets.Keys) {
        throw "Invalid consumer manifest: rid must be one of: $(($script:RidTargets.Keys | Sort-Object) -join ', ')"
    }
    if ($document.configuration -isnot [string] -or $document.configuration -cnotin @('Debug', 'Release')) { throw 'Invalid consumer manifest: configuration must be Debug or Release' }
    foreach ($field in @('packageName', 'binary')) {
        if ($document.ContainsKey($field) -and ($document[$field] -isnot [string] -or $document[$field] -notmatch '^[A-Za-z0-9][A-Za-z0-9_-]*$')) {
            throw "Invalid consumer manifest: $field must be a Cargo package/binary name"
        }
    }
    if (-not $document.ContainsKey('binary')) {
        $document.binary = $document.packageName
    }
    $manifestDirectory = Split-Path -Parent $ManifestPath
    $noticePaths = @()
    if ($document.ContainsKey('noticeFiles')) {
        if ($document.noticeFiles -is [string] -or $document.noticeFiles -isnot [System.Collections.IList]) {
            throw 'Invalid consumer manifest: noticeFiles must be an array of non-empty paths'
        }
        foreach ($notice in $document.noticeFiles) {
            if ($notice -isnot [string] -or [string]::IsNullOrWhiteSpace($notice)) {
                throw 'Invalid consumer manifest: noticeFiles must be an array of non-empty paths'
            }
            if ($notice.Contains("`r") -or $notice.Contains("`n")) {
                throw 'Invalid consumer manifest: noticeFiles entries must not contain line terminators'
            }
            $noticePath = Resolve-ManifestNoticePath $manifestDirectory $notice
            $noticeName = Split-Path -Leaf $noticePath
            # Bundle metadata names stay portable across case-sensitive and
            # case-insensitive destination filesystems.
            $comparison = [StringComparison]::OrdinalIgnoreCase
            if (@('sbom.cdx.json', 'checksums.sha256') | Where-Object { $noticeName.Equals($_, $comparison) }) {
                throw "Invalid consumer manifest: noticeFiles entry uses a reserved bundle filename: $noticeName"
            }
            if (@($noticePaths | Where-Object { $noticeName.Equals((Split-Path -Leaf $_), $comparison) }).Count -gt 0) {
                throw "Invalid consumer manifest: noticeFiles entries must have unique filenames: $noticeName"
            }
            $noticePaths += $noticePath
        }
    }
    $paths = @{}
    foreach ($field in $script:PathFields) {
        $paths[$field] = Resolve-ManifestPath $manifestDirectory ([string]$document[$field])
    }
    if ([IO.Path]::GetFileName($paths.generatedRegistryFile) -cne 'RustViewRegistry.g.cs') {
        throw 'Invalid consumer manifest: generatedRegistryFile must be named RustViewRegistry.g.cs'
    }
    foreach ($field in @('presentationProject', 'viewModelIr', 'cargoManifest')) {
        if (-not (Test-Path -LiteralPath $paths[$field] -PathType Leaf)) {
            throw "Invalid consumer manifest: $field does not exist: $($paths[$field])"
        }
    }
    $document._paths = $paths
    $document._manifestDirectory = $manifestDirectory
    $document._noticeFiles = $noticePaths
    return $document
}

function Invoke-ConsumerPackage {
    param([string]$ProducerRootPath, $Document)
    $paths = $Document._paths
    Assert-ArtifactBundleDestination -BundlePath $paths.outputDirectory
    $rid = [string]$Document.rid
    $target = Get-RidTargetInfo -Rid $rid
    $rustoloniaRootPath = (Resolve-Path -LiteralPath $RustoloniaRoot).Path
    Assert-ConsumerTools -Rid $rid -RustoloniaRoot $rustoloniaRootPath -ConsumerRoot $Document._manifestDirectory
    Assert-ConsumerSource -ProducerRoot $ProducerRootPath -RustoloniaRoot $rustoloniaRootPath
    # Cargo finds the owning workspace lockfile, which may be above the manifest.
    $metadataCommand = @('cargo', 'metadata', '--format-version', '1', '--manifest-path', $paths.cargoManifest)
    if (-not $UpdateLockFile) { $metadataCommand += '--locked' }
    $metadata = Invoke-Logged -WorkingDirectory $Document._manifestDirectory -Command $metadataCommand | ConvertFrom-Json -AsHashtable
    $cargoLockPath = Join-Path $metadata.workspace_root 'Cargo.lock'
    $hostProject = Join-Path $rustoloniaRootPath 'host' 'Avalonia.Host.csproj'
    $projectionTool = Join-Path $rustoloniaRootPath 'projection' 'Avalonia.ViewModelProjection.Tool' 'Avalonia.ViewModelProjection.Tool.csproj'
    $licenseFile = Join-Path $ProducerRootPath 'licence.md'
    foreach ($pair in @(
            @{ Path = $hostProject; Name = 'Avalonia.Host project' },
            @{ Path = $projectionTool; Name = 'view-model projection tool' },
            @{ Path = $licenseFile; Name = 'licence' }
        )) {
        if (-not (Test-Path -LiteralPath $pair.Path -PathType Leaf)) {
            throw "Producer root is invalid; missing $($pair.Name): $($pair.Path)"
        }
    }

    foreach ($field in @('generatedAdaptersDirectory', 'generatedRegistryFile', 'generatedRustFile', 'generatedContractFile')) {
        $parent = Split-Path -Parent $paths[$field]
        if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    }

    if (-not $SkipGenerate) {
        Invoke-Logged -WorkingDirectory $rustoloniaRootPath -Command @(
            'dotnet', 'run', '--project', $projectionTool, '-c', [string]$Document.configuration, '--',
            $paths.viewModelIr, $paths.generatedAdaptersDirectory, (Split-Path -Parent $paths.generatedRegistryFile),
            $paths.generatedRustFile, $paths.generatedContractFile, '--external-rust'
        )
    }

    Ensure-RidPrerequisites -Rid $rid -ProducerRoot $ProducerRootPath -Configuration ([string]$Document.configuration)

    Invoke-Logged -WorkingDirectory $Document._manifestDirectory -Command @(
        'dotnet', 'build', $paths.presentationProject, '-c', [string]$Document.configuration,
        "-p:AvaloniaProducerRoot=$ProducerRootPath",
        "-p:RustoloniaRoot=$rustoloniaRootPath"
    )

    $cargoTarget = Join-Path $Document._manifestDirectory '.avalonia' 'cargo-target'
    $previousCargo = [Environment]::GetEnvironmentVariable('CARGO_TARGET_DIR', 'Process')
    [Environment]::SetEnvironmentVariable('CARGO_TARGET_DIR', $cargoTarget, 'Process')
    Set-WindowsStaticCrt -Triple $target.Triple
    try {
        $cargoArgs = New-CargoBuildCommand -ManifestPath $paths.cargoManifest -PackageName $Document.packageName -BinaryName $Document.binary -TargetTriple $target.Triple -Configuration $Document.configuration
        $cargoArgs += '--locked'
        $profile = if ($Document.configuration -eq 'Release') { 'release' } else { 'debug' }
        Invoke-Logged -WorkingDirectory $Document._manifestDirectory -Command $cargoArgs
    }
    finally {
        if ($null -eq $previousCargo) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable('CARGO_TARGET_DIR', $previousCargo, 'Process') }
    }
    $executable = Join-Path $cargoTarget $target.Triple $profile "$($Document.binary)$($target.ExeExtension)"
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "Cargo did not produce the declared binary: $executable"
    }

    $bundle = $paths.outputDirectory
    $staging = New-IsolatedPackageStagingRoot -OutputRoot (Split-Path -Parent $bundle) -Rid $rid
    try {
        $publishProperties = New-HostPublishProperties -ProducerRoot $ProducerRootPath -RustoloniaRoot $rustoloniaRootPath -Rid $rid -HostPlatform $($target.Platform)
        $publishProperties += @(
            "-p:AvaloniaRustPresentationProjects=$($paths.presentationProject)",
            "-p:AvaloniaRustViewRegistryFile=$($paths.generatedRegistryFile)",
            "-p:PublishDir=$staging"
        )
        $publishCommand = @('dotnet', 'publish', $hostProject, '-c', [string]$Document.configuration, '-r', $rid) + $publishProperties
        Invoke-Logged -WorkingDirectory $rustoloniaRootPath -Command $publishCommand
        Push-Location $rustoloniaRootPath
        try {
            $consumerHostAssets = Get-PublishedProjectAssetsFile -Project $hostProject -Configuration $Document.configuration -Rid $rid -AdditionalProperties $publishProperties
        }
        finally { Pop-Location }
        $hostFile = Join-Path $staging "Avalonia.Host$($target.HostExtension)"
        if (-not (Test-Path -LiteralPath $hostFile -PathType Leaf)) {
            throw "NativeAOT host was not produced: $hostFile"
        }
        Invoke-ArtifactBundleTransaction -BundlePath $bundle -Rid $rid -Build {
        param($bundle)
        Copy-BundleFiles -SourceDirectory $staging -DestinationDirectory $bundle -HostFile $hostFile -Rid $rid
        Copy-Item -LiteralPath $executable -Destination (Join-Path $bundle (Split-Path -Leaf $executable))
        Copy-BundleNotices -ProducerRoot $ProducerRootPath -RustoloniaRoot $rustoloniaRootPath -DestinationDirectory $bundle
        foreach ($notice in $Document._noticeFiles) {
            $noticeDestination = Join-Path $bundle (Split-Path -Leaf $notice)
            if (Test-Path -LiteralPath $noticeDestination) {
                throw "Application notice conflicts with another bundle file: $noticeDestination"
            }
            Copy-Item -LiteralPath $notice -Destination $noticeDestination
        }
        $signTargets = @(
            (Join-Path $bundle (Split-Path -Leaf $executable)),
            (Join-Path $bundle (Split-Path -Leaf $hostFile))
        )
        Invoke-ArtifactSigning -ArtifactDirectory $bundle -SignCommand $env:AVALONIA_RUST_SIGN_COMMAND -ExplicitFiles $signTargets
        $consumerProducerPin = git -C $ProducerRootPath rev-parse HEAD 2>$null
        & (Join-Path $PSScriptRoot 'generate-sbom.ps1') -Rid $rid -Bundle $bundle `
            -CargoLockPath $cargoLockPath `
            -ProjectAssetsJsonPath $consumerHostAssets `
            -ProducerPin $consumerProducerPin
        Write-Checksums -Bundle $bundle
        }
    }
    finally {
        if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
    }
    Write-Host "Package layout ready at $bundle"
}

$rustoloniaRootPath = (Resolve-Path -LiteralPath $RustoloniaRoot).Path
Initialize-LocalProducerSubmodule -RustoloniaRoot $rustoloniaRootPath -ProducerRoot $ProducerRoot
$producer = (Resolve-Path -LiteralPath $ProducerRoot).Path
$manifestPath = (Resolve-Path -LiteralPath $Manifest).Path
$document = Read-ConsumerManifest $manifestPath
Invoke-ConsumerPackage $producer $document
