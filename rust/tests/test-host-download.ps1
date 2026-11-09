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
With -LinuxBackend Drm, select the separate DRM archive/cache and enable
linux-drm for repository examples. -Linkage Static selects the static host
archive/cache and enables `static-host` for repository examples; -ShipHost
then launches a copy of the executable without any companion libraries.
Standalone app manifests declare the feature themselves. -BuildOnly stages
the app without requiring a display device.

An osx-x64 -Rid on Apple silicon builds for x86_64-apple-darwin and
launches through Rosetta 2, which must be installed.
#>
param(
    [Parameter(Mandatory)]
    [ValidateSet('win-x64', 'win-arm64', 'linux-x64', 'linux-arm64', 'linux-musl-x64', 'linux-musl-arm64', 'osx-x64', 'osx-arm64')]
    [string]$Rid,
    [ValidateSet('X11', 'Drm')][string]$LinuxBackend = 'X11',
    [ValidateSet('Shared', 'Static')][string]$Linkage = 'Shared',
    [Parameter(Mandatory)][string]$TarballDirectory,
    [string]$Example = 'hello_world',
    [string]$RustoloniaRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [string]$WorkDirectory = (Join-Path ([IO.Path]::GetTempPath()) "rustolonia-host-download-$Rid"),
    [int]$LaunchSeconds = 5,
    [string]$AppManifest,
    [string]$AppName,
    [switch]$ShipHost,
    # Hosted CI has no DRM display. Build/stage there; run the QEMU fixture separately.
    [switch]$BuildOnly,
    # Release validation must use the checksums shipped in the tagged source.
    [switch]$UseCommittedChecksums,
    [switch]$MuslInteraction
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $RustoloniaRoot 'rust' 'package-shared.ps1')

$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $RustoloniaRoot
$asset = Get-HostAssetName -Version $version -Rid $Rid -LinuxBackend $LinuxBackend -Linkage $Linkage
$static = $Linkage -eq 'Static'
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
    $ridTarget = Get-RidTargetInfo -Rid $Rid -LinuxBackend $LinuxBackend
    if ($static -and $IsWindows) {
        $crtEnvName = "CARGO_TARGET_$($ridTarget.Triple.ToUpperInvariant().Replace('-', '_'))_RUSTFLAGS"
        $saved[$crtEnvName] = [Environment]::GetEnvironmentVariable($crtEnvName)
        Set-WindowsStaticCrt -Triple $ridTarget.Triple
    }
    if ($static -and $IsMacOS) {
        $saved.MACOSX_DEPLOYMENT_TARGET = [Environment]::GetEnvironmentVariable('MACOSX_DEPLOYMENT_TARGET')
        if ([string]::IsNullOrWhiteSpace($saved.MACOSX_DEPLOYMENT_TARGET)) {
            Set-ProcessEnvironment -Name MACOSX_DEPLOYMENT_TARGET -Value '12.0'
        }
    }
    $exampleFeatures = @()
    if ($LinuxBackend -eq 'Drm') { $exampleFeatures += 'linux-drm' }
    if ($static) { $exampleFeatures += 'static-host' }
    $backendFeatures = if ($exampleFeatures) { @('--features', ($exampleFeatures -join ',')) } else { @() }
    $cargoTarget = @()
    $outputDir = Join-Path $targetDir 'release'
    if ($Rid -cin $script:MuslStaticHostRids -or $ridTarget.Arch -ne (Get-CurrentRuntimeArchitecture)) {
        # Apple silicon runs x64 apps through Rosetta 2; other cross RIDs cannot launch here.
        if ($ridTarget.Arch -ne (Get-CurrentRuntimeArchitecture) -and -not ($IsMacOS -and $ridTarget.Arch -eq 'x64')) { throw "A $Rid app cannot be launched on this $(Get-CurrentRuntimeArchitecture) machine." }
        $cargoTarget = @('--target', $ridTarget.Triple)
        $outputDir = Join-Path $targetDir $ridTarget.Triple 'release'
    }
    if ($AppManifest) {
        if (-not $AppName) { throw '-AppName is required with -AppManifest.' }
        cargo build --manifest-path $AppManifest --release --target-dir $targetDir @cargoTarget
        $exe = Join-Path $outputDir ($AppName + $exeSuffix)
        $Example = $AppName
    } else {
        cargo build --locked --manifest-path (Join-Path $RustoloniaRoot 'rust' 'Cargo.toml') `
            -p rustolonia --example $Example --release --target-dir $targetDir @cargoTarget @backendFeatures
        $exe = Join-Path $outputDir 'examples' ($Example + $exeSuffix)
    }

    $variant = if ($LinuxBackend -eq 'Drm') { "$Rid-drm" } else { $Rid }
    if ($static) { $variant += '-static' }
    $hostFile = Join-Path $cache $version $variant $(if ($static) { $ridTarget.StaticHostFileName } else { $ridTarget.HostFileName })
    if (-not (Test-Path -LiteralPath $hostFile -PathType Leaf)) {
        throw "The downloaded host was not cached at $hostFile."
    }
    if ($ShipHost -and $static) {
        $standaloneDirectory = Join-Path $WorkDirectory 'static-app'
        New-Item -ItemType Directory -Force -Path $standaloneDirectory | Out-Null
        $standaloneExe = Join-Path $standaloneDirectory (Split-Path -Leaf $exe)
        Copy-Item -LiteralPath $exe -Destination $standaloneExe
        $exe = $standaloneExe
        Remove-Item -LiteralPath $cache -Recurse -Force
    }
    elseif ($ShipHost) {
        Copy-Item -Path (Join-Path (Split-Path -Parent $hostFile) '*') -Destination (Split-Path -Parent $exe) -Recurse -Force
        # Prove the copy beside the executable is what loads.
        Remove-Item -LiteralPath $cache -Recurse -Force
    }
    if ($Rid -cin $script:MuslStaticHostRids) {
        if (-not (Get-Command readelf -ErrorAction SilentlyContinue)) { throw 'The musl static smoke test requires readelf.' }
        $segments = & readelf -l $exe
        if ($LASTEXITCODE -ne 0 -or ($segments -match '\sINTERP\s')) { throw 'The musl executable has a dynamic ELF interpreter.' }
        $dynamic = & readelf -d $exe
        if ($LASTEXITCODE -ne 0 -or ($dynamic -match '\(NEEDED\)')) { throw 'The musl executable has dynamic ELF dependencies.' }
    }
    if ($BuildOnly) {
        Write-Host "$Example built and staged with the downloaded $variant host (not launched)."
        return
    }
    if ($MuslInteraction) {
        if ($Rid -cnotin $script:MuslStaticHostRids) { throw '-MuslInteraction requires a musl host.' }
        & sh (Join-Path $PSScriptRoot 'test-static-musl-app.sh') $exe (Join-Path $WorkDirectory 'musl-smoke')
        if ($LASTEXITCODE -ne 0) { throw "The $Rid interactive static smoke test failed." }
        return
    }
    $stdout = Join-Path $WorkDirectory "$Example.stdout.log"
    $stderr = Join-Path $WorkDirectory "$Example.stderr.log"
    $redirect = @{ PassThru = $true; RedirectStandardOutput = $stdout; RedirectStandardError = $stderr }
    $traceVariable = if ($static -and $IsMacOS) { 'DYLD_PRINT_LIBRARIES' }
        elseif ($static -and $IsLinux) { 'LD_DEBUG' }
        else { $null }
    $usesXvfb = $IsLinux -and $LinuxBackend -ne 'Drm'
    $timeoutSeconds = [Math]::Max(1, $LaunchSeconds + 2)
    $previousTrace = if ($traceVariable) { [Environment]::GetEnvironmentVariable($traceVariable) } else { $null }
    try {
        if ($traceVariable) { Set-ProcessEnvironment -Name $traceVariable -Value $(if ($IsMacOS) { '1' } else { 'libs' }) }
        $process = if ($usesXvfb) {
            # Let xvfb-run exit normally so its cleanup trap stops Xvfb, even
            # when the test fails after launching the application.
            $quotedExe = '"' + $exe.Replace('\', '\\').Replace('"', '\"') + '"'
            Start-Process xvfb-run -ArgumentList '-a', 'timeout', '--signal=TERM', '--kill-after=3s', "${timeoutSeconds}s", $quotedExe @redirect
        } else {
            Start-Process $exe @redirect
        }
    }
    finally {
        if ($traceVariable) { Set-ProcessEnvironment -Name $traceVariable -Value $previousTrace }
    }
    try {
        Start-Sleep -Seconds $LaunchSeconds
        if ($process.HasExited) {
            Get-Content -LiteralPath $stdout, $stderr -ErrorAction SilentlyContinue | Write-Host
            throw "$Example exited during startup with code $($process.ExitCode)."
        }
        if ($static) {
            $forbidden = '(?i)(AvaloniaNative|SkiaSharp|HarfBuzzSharp|av_libGLESv2)(\.dll|\.dylib|\.so(\.\d+)*)'
            $loaded = @(
                if ($IsWindows) {
                    $process.Modules | Where-Object { $_.ModuleName -match $forbidden } | ForEach-Object ModuleName
                } else {
                    Select-String -LiteralPath $stderr -Pattern $forbidden
                }
            )
            if ($loaded.Count -gt 0) { throw "Static app loaded a project native library dynamically: $($loaded -join ', ')" }
        }
    }
    finally {
        if (-not $process.HasExited) {
            if ($usesXvfb) {
                if (-not $process.WaitForExit(($timeoutSeconds + 6) * 1000)) {
                    Stop-Process -Id $process.Id
                    throw 'The Xvfb smoke-test process did not stop after its timeout.'
                }
            } else {
                Stop-Process -Id $process.Id
            }
        }
    }
    Write-Host "$Example built and launched with the downloaded $Rid host ($hostFile)."
}
finally {
    foreach ($name in $saved.Keys) { Set-ProcessEnvironment -Name $name -Value $saved[$name] }
}
