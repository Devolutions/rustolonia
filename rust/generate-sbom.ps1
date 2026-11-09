#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Writes a deterministic CycloneDX 1.5 delivery SBOM for a packaged Rust bundle.

.DESCRIPTION
Records the delivered files (with hashes) as before, plus a best-effort,
fully offline dependency graph resolved from already-restored/vendored
metadata: the NuGet packages resolved for the published host
(`project.assets.json`, produced by a normal `dotnet restore`/`build`, no
network access here) and the Cargo package graph resolved for the packaged
Rust binary (`Cargo.lock`, already checked in or produced by `cargo build`).
Both inputs are optional so packaging keeps working if a caller does not
have them available; their absence is recorded rather than silently ignored.
#>
param(
    [Parameter(Mandatory)][string]$Rid,
    [Parameter(Mandatory)][string]$Bundle,
    [string]$CargoLockPath,
    [string]$ProjectAssetsJsonPath,
    [string]$ProducerPin
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$bundlePath = (Resolve-Path -LiteralPath $Bundle).Path
$output = Join-Path $bundlePath 'sbom.cdx.json'
$ridBytes = [System.Text.Encoding]::UTF8.GetBytes($Rid)
$bundleId = [System.BitConverter]::ToString([System.Security.Cryptography.SHA256]::HashData($ridBytes)).Replace('-', '').ToLowerInvariant()
$serial = "urn:uuid:$($bundleId.Substring(0, 8))-$($bundleId.Substring(8, 4))-5$($bundleId.Substring(12, 3))-8$($bundleId.Substring(15, 3))-$($bundleId.Substring(18, 12))"

$fileComponents = @(
    Get-ChildItem -LiteralPath $bundlePath -File -Force -Recurse |
        ForEach-Object {
            $relative = [IO.Path]::GetRelativePath($bundlePath, $_.FullName).Replace('\', '/')
            if ($relative -in @('checksums.sha256', 'sbom.cdx.json')) { return }
            [ordered]@{
                type       = 'file'
                name       = $relative
                version    = $Rid
                hashes     = @([ordered]@{ alg = 'SHA-256'; content = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() })
                properties = @([ordered]@{ name = 'avalonia:delivery-path'; value = $relative })
            }
        } |
        Sort-Object { $_.name }
)

function Get-DependencySourceDescription {
    param([string]$Path, [string]$Description, [string]$FileName)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return "unavailable: no $FileName path was supplied"
    }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return "unavailable: supplied $FileName file does not exist"
    }
    return $Description
}

function Get-NuGetPackageComponents {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return @()
    }

    $assets = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    $libraries = @()
    if ($assets.PSObject.Properties.Name -contains 'libraries') {
        $libraries = $assets.libraries.PSObject.Properties
    }

    $libraries |
        Where-Object { $_.Value.type -eq 'package' } |
        ForEach-Object {
            $separator = $_.Name.LastIndexOf('/')
            if ($separator -lt 0) { return }
            $name = $_.Name.Substring(0, $separator)
            $version = $_.Name.Substring($separator + 1)
            [ordered]@{
                type       = 'library'
                name       = $name
                version    = $version
                purl       = "pkg:nuget/$name@$version"
                properties = @([ordered]@{ name = 'avalonia:ecosystem'; value = 'nuget' })
            }
        } |
        Sort-Object { "$($_.name)/$($_.version)" }
}

function Get-CargoPackageComponents {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return @()
    }

    $name = $null
    $version = $null
    $source = $null
    $inPackage = $false
    $components = @()
    foreach ($line in Get-Content -LiteralPath $Path) {
        $trimmed = $line.Trim()
        if ($trimmed -eq '[[package]]') {
            if ($inPackage -and $name -and $version) {
                $components += , (New-CargoComponent -Name $name -Version $version -Source $source)
            }
            $inPackage = $true
            $name = $null; $version = $null; $source = $null
            continue
        }
        if (-not $inPackage) { continue }
        if ($trimmed -match '^name\s*=\s*"(.*)"$') { $name = $Matches[1]; continue }
        if ($trimmed -match '^version\s*=\s*"(.*)"$') { $version = $Matches[1]; continue }
        if ($trimmed -match '^source\s*=\s*"(.*)"$') { $source = $Matches[1]; continue }
    }
    if ($inPackage -and $name -and $version) {
        $components += , (New-CargoComponent -Name $name -Version $version -Source $source)
    }
    $components | Sort-Object { "$($_.name)/$($_.version)" }
}

function Get-LinuxStaticPackageComponents {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return @() }
    foreach ($line in Get-Content -LiteralPath $Path) {
        if ([string]::IsNullOrWhiteSpace($line) -or $line.StartsWith('#')) { continue }
        $fields = $line -split "`t"
        if ($fields.Count -ne 5 -or $fields[0] -notmatch '^lib[A-Za-z0-9_+.-]+\.a$' -or
            $fields[1] -notmatch '^[A-Za-z0-9+.-]+:[A-Za-z0-9_-]+$' -or
            [string]::IsNullOrWhiteSpace($fields[2]) -or $fields[3] -notmatch '^[0-9a-f]{64}$' -or
            $fields[4] -notin @('debian', 'ubuntu', 'alpine', 'xorg')) {
            throw "Invalid static Linux package provenance in $Path`: $line"
        }
        $package, $architecture = $fields[1] -split ':', 2
        $packageName = [Uri]::EscapeDataString($package)
        $version = [Uri]::EscapeDataString($fields[2])
        $purl = if ($fields[4] -eq 'alpine') {
            "pkg:apk/alpine/$packageName@$version`?arch=$architecture&distro=3.24"
        }
        elseif ($fields[4] -eq 'xorg') {
            $sourceName = [Uri]::EscapeDataString($package.Substring(5))
            "pkg:generic/xorg/$sourceName@$version`?arch=$architecture"
        }
        else {
            "pkg:deb/$($fields[4])/$packageName@$version`?arch=$architecture"
        }
        [ordered]@{
            type       = 'library'
            name       = $fields[0]
            version    = $fields[2]
            purl       = $purl
            hashes     = @([ordered]@{ alg = 'SHA-256'; content = $fields[3] })
            properties = @([ordered]@{ name = 'avalonia:source-package'; value = $fields[1] })
        }
    }
}

function Get-LinuxStaticGraphicsComponents {
    param([string]$Path, [string]$Rid)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return @() }
    $metadata = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    $musl = $Rid.StartsWith('linux-musl-')
    $validLibc = if ($musl) {
        $metadata.PSObject.Properties['libc'] -and $metadata.libc -eq 'musl' -and
        $metadata.PSObject.Properties['dngPatchSha256'] -and $metadata.dngPatchSha256 -match '^[0-9a-f]{64}$'
    }
    else {
        $metadata.PSObject.Properties['glibcBuild'] -and $metadata.glibcBuild -eq '2.35'
    }
    if ($metadata.rid -ne $Rid -or -not $validLibc -or
        $metadata.skiasharpCommit -notmatch '^[0-9a-f]{40}$' -or $metadata.skiaCommit -notmatch '^[0-9a-f]{40}$') {
        throw "Invalid static Linux graphics provenance in $Path."
    }
    foreach ($archive in @('libSkiaSharp.a', 'libskia.a', 'libHarfBuzzSharp.a')) {
        $property = $metadata.archives.PSObject.Properties[$archive]
        if ($null -eq $property -or $property.Value -notmatch '^[0-9a-f]{64}$') {
            throw "Invalid static Linux graphics hash for $archive in $Path."
        }
        $repo = if ($archive -eq 'libskia.a') { 'mono/skia' } else { 'mono/SkiaSharp' }
        $commit = if ($archive -eq 'libskia.a') { $metadata.skiaCommit } else { $metadata.skiasharpCommit }
        $libcProperty = if ($musl) {
            [ordered]@{ name = 'avalonia:libc'; value = 'musl' }
        }
        else {
            [ordered]@{ name = 'avalonia:glibc-build-baseline'; value = $metadata.glibcBuild }
        }
        [ordered]@{
            type       = 'library'
            name       = $archive
            version    = $metadata.version
            purl       = "pkg:github/$repo@$commit"
            hashes     = @([ordered]@{ alg = 'SHA-256'; content = $property.Value })
            properties = @($libcProperty)
        }
    }
}

function New-CargoComponent {
    param([string]$Name, [string]$Version, [string]$Source)

    # Workspace-local crates (rustolonia, rustolonia-sys, rustolonia-bindgen, ...)
    # have no [source] entry; they are not third-party dependencies.
    if ([string]::IsNullOrWhiteSpace($Source)) {
        return $null
    }
    [ordered]@{
        type       = 'library'
        name       = $Name
        version    = $Version
        purl       = "pkg:cargo/$Name@$Version"
        properties = @([ordered]@{ name = 'avalonia:ecosystem'; value = 'cargo' })
    }
}

$nugetComponents = @(Get-NuGetPackageComponents -Path $ProjectAssetsJsonPath)
$cargoComponents = @(Get-CargoPackageComponents -Path $CargoLockPath | Where-Object { $null -ne $_ })
$linuxStaticComponents = @(Get-LinuxStaticPackageComponents -Path (Join-Path $bundlePath 'STATIC-LINUX-PROVENANCE.tsv'))
$linuxGraphicsComponents = if ($Rid.StartsWith('linux-')) {
    @(Get-LinuxStaticGraphicsComponents -Path (Join-Path $bundlePath 'STATIC-SKIASHARP-PROVENANCE.json') -Rid $Rid)
} else { @() }

$properties = @(
    [ordered]@{ name = 'avalonia:nuget-dependency-source'; value = Get-DependencySourceDescription -Path $ProjectAssetsJsonPath -FileName 'project.assets.json' -Description 'project.assets.json (offline restore metadata)' }
    [ordered]@{ name = 'avalonia:cargo-dependency-source'; value = Get-DependencySourceDescription -Path $CargoLockPath -FileName 'Cargo.lock' -Description 'Cargo.lock (offline lockfile metadata)' }
)
if ($ProducerPin) {
    $properties += [ordered]@{ name = 'avalonia:producer-pin'; value = $ProducerPin }
}

$document = [ordered]@{
    bomFormat    = 'CycloneDX'
    specVersion  = '1.5'
    serialNumber = $serial
    version      = 1
    metadata     = [ordered]@{
        component  = [ordered]@{ type = 'application'; name = 'Avalonia Rust bundle'; version = $Rid }
        properties = $properties
    }
    components   = @($fileComponents) + @($nugetComponents) + @($cargoComponents) + @($linuxStaticComponents) + @($linuxGraphicsComponents)
}

$json = $document | ConvertTo-Json -Depth 8
# ConvertTo-Json may emit CRLF; the delivery file is UTF-8 LF.
[System.IO.File]::WriteAllText($output, ($json.Replace("`r`n", "`n") + "`n"))
