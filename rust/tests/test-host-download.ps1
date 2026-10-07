#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Builds and launches a rustolonia example through the rustolonia-sys download path.

.DESCRIPTION
Serves the host tarball produced by package-host.ps1 from a file:// release
tree, points rustolonia-sys at it with checksums taken from the `.sha256`
sidecars, builds the example with an empty download cache and no
RUSTOLONIA_HOST_DIR, asserts the host was cached, and launches the example
(under xvfb-run on Linux) for a few seconds.

With -AppManifest, builds that standalone Cargo package (binary -AppName)
instead of a repository example. -ShipHost copies the cached host next to the
executable before launching, as a distributable build without the
`dev-host-path` feature requires.
#>
param(
    [Parameter(Mandatory)]
    [ValidateSet('win-x64', 'win-arm64', 'linux-x64', 'linux-arm64', 'osx-x64', 'osx-arm64')]
    [string]$Rid,
    [Parameter(Mandatory)][string]$TarballDirectory,
    [string]$Example = 'hello_world',
    [string]$RustoloniaRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [string]$WorkDirectory = (Join-Path ([IO.Path]::GetTempPath()) "rustolonia-host-download-$Rid"),
    [int]$LaunchSeconds = 5,
    [string]$AppManifest,
    [string]$AppName,
    [switch]$ShipHost,
    # Release validation must use the checksums shipped in the tagged source.
    [switch]$UseCommittedChecksums
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $RustoloniaRoot 'rust' 'package-shared.ps1')

$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $RustoloniaRoot
$asset = Get-HostAssetName -Version $version -Rid $Rid
$tarball = Join-Path $TarballDirectory $asset
if (-not (Test-Path -LiteralPath $tarball -PathType Leaf)) { throw "Host tarball not found: $tarball" }

if (Test-Path -LiteralPath $WorkDirectory) { Remove-Item -LiteralPath $WorkDirectory -Recurse -Force }
$releaseDir = Join-Path $WorkDirectory 'release' "v$version"
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
Copy-Item -LiteralPath $tarball -Destination $releaseDir
$checksums = Join-Path $WorkDirectory 'host-checksums.txt'
if (-not $UseCommittedChecksums) {
    Get-Content -LiteralPath "$tarball.sha256" | Set-Content -LiteralPath $checksums
}
$cache = Join-Path $WorkDirectory 'cache'

$saved = @{}
$overrides = @{
    RUSTOLONIA_NO_DOWNLOAD    = '0'
    RUSTOLONIA_HOST_BASE_URL  = [Uri]::new((Join-Path $WorkDirectory 'release')).AbsoluteUri
    RUSTOLONIA_HOST_CHECKSUMS = if ($UseCommittedChecksums) { $null } else { $checksums }
    RUSTOLONIA_CACHE_DIR      = $cache
    RUSTOLONIA_HOST_DIR       = $null
    RUSTOLONIA_HOST_LIB       = $null
}
# pwsh 7 turns [Environment]::SetEnvironmentVariable(name, $null) into an
# empty-but-defined variable, which child processes still see; remove instead.
function Set-ProcessEnvironment([string]$Name, [AllowNull()][string]$Value) {
    if ([string]::IsNullOrEmpty($Value)) {
        Remove-Item -LiteralPath "env:$Name" -ErrorAction SilentlyContinue
    } else {
        Set-Item -LiteralPath "env:$Name" -Value $Value
    }
}
foreach ($name in $overrides.Keys) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name)
    Set-ProcessEnvironment -Name $name -Value $overrides[$name]
}
try {
    $targetDir = Join-Path $WorkDirectory 'target'
    $exeSuffix = if ($IsWindows) { '.exe' } else { '' }
    if ($AppManifest) {
        if (-not $AppName) { throw '-AppName is required with -AppManifest.' }
        cargo build --manifest-path $AppManifest --release --target-dir $targetDir
        $exe = Join-Path $targetDir 'release' ($AppName + $exeSuffix)
        $Example = $AppName
    } else {
        cargo build --locked --manifest-path (Join-Path $RustoloniaRoot 'rust' 'Cargo.toml') `
            -p rustolonia --example $Example --release --target-dir $targetDir
        $exe = Join-Path $targetDir 'release' 'examples' ($Example + $exeSuffix)
    }

    $hostFile = Join-Path $cache $version $Rid (Get-RidTargetInfo -Rid $Rid).HostFileName
    if (-not (Test-Path -LiteralPath $hostFile -PathType Leaf)) {
        throw "The downloaded host was not cached at $hostFile."
    }
    if ($ShipHost) {
        Copy-Item -Path (Join-Path (Split-Path -Parent $hostFile) '*') -Destination (Split-Path -Parent $exe) -Recurse -Force
        # Prove the copy beside the executable is what loads.
        Remove-Item -LiteralPath $cache -Recurse -Force
    }
    $stdout = Join-Path $WorkDirectory "$Example.stdout.log"
    $stderr = Join-Path $WorkDirectory "$Example.stderr.log"
    $redirect = @{ PassThru = $true; RedirectStandardOutput = $stdout; RedirectStandardError = $stderr }
    $process = if ($IsLinux) {
        Start-Process xvfb-run -ArgumentList '-a', $exe @redirect
    } else {
        Start-Process $exe @redirect
    }
    Start-Sleep -Seconds $LaunchSeconds
    if ($process.HasExited) {
        Get-Content -LiteralPath $stdout, $stderr -ErrorAction SilentlyContinue | Write-Host
        throw "$Example exited during startup with code $($process.ExitCode)."
    }
    Stop-Process -Id $process.Id
    Write-Host "$Example built and launched with the downloaded $Rid host ($hostFile)."
}
finally {
    foreach ($name in $saved.Keys) { Set-ProcessEnvironment -Name $name -Value $saved[$name] }
}
