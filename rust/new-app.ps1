#!/usr/bin/env pwsh
#Requires -Version 7.0
param(
    [Parameter(Mandatory)]
    [ValidatePattern('^[a-z][a-z0-9_]*$')]
    [string]$Name,
    [Parameter(Mandatory)]
    [string]$Destination,
    [string]$ProducerRoot,
    [string]$Rid,
    [string]$RustoloniaRoot = (Split-Path -Parent $PSScriptRoot)
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'package-shared.ps1')
if ([string]::IsNullOrWhiteSpace($Rid)) { $Rid = Get-DefaultConsumerRid }
$null = Get-RidTargetInfo -Rid $Rid

$templateDir = Join-Path $PSScriptRoot 'templates' 'avalonia-app'
if (-not (Test-Path -LiteralPath $templateDir -PathType Container)) {
    throw "Template directory not found at $templateDir"
}

$resolvedRustoloniaRoot = [System.IO.Path]::GetFullPath(
    $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($RustoloniaRoot)
)
$resolvedProducerRoot = if ([string]::IsNullOrWhiteSpace($ProducerRoot)) {
    Join-Path $resolvedRustoloniaRoot 'avalonia-src'
} else {
    [System.IO.Path]::GetFullPath(
        $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($ProducerRoot)
    )
}

$destinationReference = if ([System.IO.Path]::IsPathRooted($Destination)) {
    $Destination
} else {
    $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Destination)
}
$resolvedDestination = [System.IO.Path]::GetFullPath($destinationReference)
$destinationParent = Split-Path -Parent $resolvedDestination
if (-not (Test-Path -LiteralPath $resolvedProducerRoot -PathType Container)) {
    throw "Producer root does not exist: $resolvedProducerRoot"
}
if (-not (Test-Path -LiteralPath $resolvedRustoloniaRoot -PathType Container)) {
    throw "Rustolonia root does not exist: $resolvedRustoloniaRoot"
}
if (Test-Path -LiteralPath $resolvedDestination -PathType Any) {
    throw "Destination '$resolvedDestination' already exists."
}

$producerDisplayPath = $resolvedProducerRoot.Replace('\', '/')
$rustoloniaDisplayPath = $resolvedRustoloniaRoot.Replace('\', '/')
$relativeRustolonia = [IO.Path]::GetRelativePath($resolvedDestination, $resolvedRustoloniaRoot).Replace('\', '/')
$relativeProducer = [IO.Path]::GetRelativePath($resolvedDestination, $resolvedProducerRoot).Replace('\', '/')
$managedDirectory = Join-Path $resolvedDestination 'managed'
$managedRustolonia = [IO.Path]::GetRelativePath($managedDirectory, $resolvedRustoloniaRoot).Replace('\', '/')
$managedProducer = [IO.Path]::GetRelativePath($managedDirectory, $resolvedProducerRoot).Replace('\', '/')
if (-not [IO.Path]::IsPathRooted($managedRustolonia)) { $managedRustolonia = '$(MSBuildThisFileDirectory)' + $managedRustolonia }
if (-not [IO.Path]::IsPathRooted($managedProducer)) { $managedProducer = '$(MSBuildThisFileDirectory)' + $managedProducer }
if ([IO.Path]::IsPathRooted($relativeRustolonia) -or [IO.Path]::IsPathRooted($relativeProducer)) {
    Write-Warning 'Roots are on different volumes; generated references are absolute. Keep consumer and source checkouts on one volume for relocation.'
}
$manifestDisplayPath = (Join-Path $resolvedDestination 'avalonia-app.json').Replace('\', '/')
$buildAppPath = Join-Path $resolvedRustoloniaRoot 'rust' 'build-app.ps1'
$buildCommand = @(
    'pwsh',
    ('"{0}"' -f $buildAppPath.Replace('\', '/')),
    '-ProducerRoot',
    ('"{0}"' -f $producerDisplayPath),
    '-Manifest',
    ('"{0}"' -f $manifestDisplayPath),
    '-UpdateLockFile'
) -join ' '
$tempDestination = Join-Path $destinationParent ('.' + [System.IO.Path]::GetFileName($resolvedDestination) + '.tmp-' + [guid]::NewGuid().ToString('N'))

try {
    New-Item -ItemType Directory -Force -Path $destinationParent | Out-Null
    Copy-Item -LiteralPath $templateDir -Destination $tempDestination -Recurse

    $items = Get-ChildItem -LiteralPath $tempDestination -Recurse -File
    foreach ($item in $items) {
        $content = Get-Content -LiteralPath $item.FullName -Raw
        $content = $content.Replace('__AVALONIA_APP_NAME__', $Name)
        $content = $content.Replace('__AVALONIA_PRODUCER_ROOT__', $relativeProducer)
        $content = $content.Replace('__RUSTOLONIA_ROOT__', $relativeRustolonia)
        $content = $content.Replace('__MANAGED_PRODUCER_ROOT__', [Security.SecurityElement]::Escape($managedProducer))
        $content = $content.Replace('__MANAGED_RUSTOLONIA_ROOT__', [Security.SecurityElement]::Escape($managedRustolonia))
        $content = $content.Replace('__CONSUMER_RID__', $Rid)
        Set-Content -LiteralPath $item.FullName -Value $content -NoNewline
    }
    Copy-Item -LiteralPath (Join-Path $resolvedRustoloniaRoot 'global.json') -Destination (Join-Path $tempDestination 'global.json')

    $gitIgnorePath = Join-Path $tempDestination '.gitignore'
    @(
        '.avalonia/',
        'bin/',
        'obj/',
        'target/',
        '*.user',
        '*.suo'
    ) | Set-Content -LiteralPath $gitIgnorePath -Encoding utf8

    Get-ChildItem -LiteralPath $tempDestination -Recurse -Directory -Filter 'target' |
        Remove-Item -Recurse -Force
    Remove-Item -LiteralPath (Join-Path $tempDestination 'Cargo.lock') -ErrorAction SilentlyContinue

    Move-Item -LiteralPath $tempDestination -Destination $resolvedDestination
}
catch {
    if (Test-Path -LiteralPath $tempDestination -PathType Container) {
        Remove-Item -LiteralPath $tempDestination -Recurse -Force -ErrorAction SilentlyContinue
    }
    throw
}

Write-Host "Created '$Name' at $resolvedDestination."
Write-Host 'Next steps:'
Write-Host "  1. Pin '$producerDisplayPath' to the compatible Avalonia producer commit/submodule."
Write-Host "  2. $buildCommand"
