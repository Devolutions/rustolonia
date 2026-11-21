#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Build the pinned SkiaSharp & HarfBuzzSharp static libraries for Windows and macOS.
#>
param(
    [Parameter(Mandatory, Position = 0)]
    [ValidateSet('prepare', 'build-windows', 'collect-windows', 'build-macos', 'collect-macos')]
    [string]$Command,

    [ValidateSet('x64', 'arm64')]
    [string]$Arch,

    [string]$PlatformToolset,
    [string]$WorkDir,
    [string]$OutputDir
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path (Split-Path -Parent $PSScriptRoot) 'common.ps1')

$skiaConfigDir = $PSScriptRoot
$repoRoot = $script:StaticDepsRepoRoot
$winLibraries = @('skia.lib', 'SkiaSharp.lib', 'libHarfBuzzSharp.lib')
$macLibraries = @('libskia.a', 'libSkiaSharp.a', 'libHarfBuzzSharp.a')

function Invoke-Checked {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @(),
        [string]$WorkingDirectory
    )
    if ($WorkingDirectory) { Push-Location $WorkingDirectory }
    try {
        & $FilePath @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $($Arguments -join ' ')"
        }
    }
    finally {
        if ($WorkingDirectory) { Pop-Location }
    }
}

$resolvedWork = if ($WorkDir) { $WorkDir } else { Join-Path $repoRoot 'out' 'static-skiasharp' }
$sourceDir = Join-Path $resolvedWork 'src_skiasharp'
$skiaDir = Join-Path $sourceDir 'externals' 'skia'

switch ($Command) {
    'prepare' {
        if (Test-Path -LiteralPath $sourceDir) {
            throw "$sourceDir already exists; use a clean work directory for a native build."
        }
        $config = Get-StaticDepsSourceConfig -Component 'skiasharp' -RepoRoot $repoRoot
        New-Item -ItemType Directory -Force -Path $sourceDir | Out-Null
        Invoke-Checked -FilePath 'git' -Arguments @('init', $sourceDir)
        Invoke-Checked -FilePath 'git' -Arguments @('remote', 'add', 'upstream', [string]$config['repository']) -WorkingDirectory $sourceDir
        Invoke-Checked -FilePath 'git' -Arguments @('fetch', '--depth=1', 'upstream', [string]$config['commit']) -WorkingDirectory $sourceDir
        Invoke-Checked -FilePath 'git' -Arguments @('checkout', '--detach', 'FETCH_HEAD') -WorkingDirectory $sourceDir
        $actualCommit = (& git -C $sourceDir rev-parse HEAD).Trim()
        if ($actualCommit -ne [string]$config['commit']) {
            throw "SkiaSharp commit $actualCommit does not match pinned $($config['commit'])."
        }
        Invoke-Checked -FilePath 'git' -Arguments @('submodule', 'update', '--init', '--depth=1', 'externals/skia') -WorkingDirectory $sourceDir
        $actualSkia = (& git -C $skiaDir rev-parse HEAD).Trim()
        if ($actualSkia -ne [string]$config['skiaCommit']) {
            throw "Skia submodule commit $actualSkia does not match pinned $($config['skiaCommit'])."
        }
        $py = if ($IsWindows) { 'python' } else { 'python3' }
        Invoke-Checked -FilePath $py -Arguments @('tools/git-sync-deps') -WorkingDirectory $skiaDir
    }
    'build-windows' {
        if (-not $Arch) { throw '-Arch is required for build-windows.' }
        $buildDir = Join-Path $resolvedWork 'build' "win-$Arch"
        New-Item -ItemType Directory -Force -Path $buildDir | Out-Null
        $buildDirAbs = (Resolve-Path -LiteralPath $buildDir).Path
        $argsText = (Get-Content -LiteralPath (Join-Path $skiaConfigDir 'args.win.gn') -Raw -Encoding utf8).Replace('target_cpu = "x64"', "target_cpu = `"$Arch`"").Replace("`r`n", "`n")
        [System.IO.File]::WriteAllText((Join-Path $buildDirAbs 'args.gn'), $argsText, [System.Text.Encoding]::ASCII)
        Invoke-Checked -FilePath 'gn' -Arguments @('gen', $buildDirAbs) -WorkingDirectory $skiaDir
        Invoke-Checked -FilePath 'autoninja' -Arguments @('-C', $buildDirAbs, 'skia', 'SkiaSharp') -WorkingDirectory $skiaDir

        $msbuildPlatform = if ($Arch -eq 'x64') { 'x64' } else { 'ARM64' }
        $vcxproj = Join-Path $sourceDir 'native' 'windows' 'libHarfBuzzSharp' 'libHarfBuzzSharp.vcxproj'
        $msbuildArgs = @(
            $vcxproj,
            '-m',
            '-t:rebuild',
            '-p:Configuration=Release',
            "-p:Platform=$msbuildPlatform",
            '-p:ConfigurationType=StaticLibrary',
            "-p:OutDir=$buildDirAbs\"
        )
        if ($PlatformToolset) {
            $msbuildArgs += "-p:PlatformToolset=$PlatformToolset"
        }
        Invoke-Checked -FilePath 'msbuild' -Arguments $msbuildArgs -WorkingDirectory $resolvedWork
    }
    'collect-windows' {
        if (-not $Arch) { throw '-Arch is required for collect-windows.' }
        $buildDir = Join-Path $resolvedWork 'build' "win-$Arch"
        $artifactsDir = if ($OutputDir) { $OutputDir } else { Join-Path $resolvedWork 'artifacts' "win-$Arch" }
        New-Item -ItemType Directory -Force -Path $artifactsDir | Out-Null
        $expected = if ($Arch -eq 'x64') { 0x8664 } else { 0xAA64 }
        foreach ($name in $winLibraries) {
            $library = Join-Path $buildDir $name
            $machine = Get-CoffArchiveMachine -Path $library
            if ($machine -ne $expected) {
                throw ("Wrong COFF architecture in {0}: got 0x{1:X4}, expected 0x{2:X4}." -f $library, $machine, $expected)
            }
            Copy-Item -LiteralPath $library -Destination (Join-Path $artifactsDir $name) -Force
        }
        $pdb = Join-Path $buildDir 'libHarfBuzzSharp.pdb'
        if (Test-Path -LiteralPath $pdb -PathType Leaf) {
            Copy-Item -LiteralPath $pdb -Destination (Join-Path $artifactsDir 'libHarfBuzzSharp.pdb') -Force
        }
    }
    'build-macos' {
        if (-not $Arch) { throw '-Arch is required for build-macos.' }
        $buildDir = Join-Path $resolvedWork 'build' "osx-$Arch"
        New-Item -ItemType Directory -Force -Path $buildDir | Out-Null
        $buildDirAbs = (Resolve-Path -LiteralPath $buildDir).Path
        $argsText = (Get-Content -LiteralPath (Join-Path $skiaConfigDir 'args.mac.gn') -Raw -Encoding utf8).Replace('target_cpu = "arm64"', "target_cpu = `"$Arch`"").Replace("`r`n", "`n")
        [System.IO.File]::WriteAllText((Join-Path $buildDirAbs 'args.gn'), $argsText, [System.Text.UTF8Encoding]::new($false))
        Invoke-Checked -FilePath 'gn' -Arguments @('gen', $buildDirAbs) -WorkingDirectory $skiaDir
        Invoke-Checked -FilePath 'autoninja' -Arguments @('-C', $buildDirAbs, 'skia', 'SkiaSharp', 'HarfBuzzSharp') -WorkingDirectory $skiaDir
    }
    'collect-macos' {
        if (-not $Arch) { throw '-Arch is required for collect-macos.' }
        $buildDir = Join-Path $resolvedWork 'build' "osx-$Arch"
        $artifactsDir = if ($OutputDir) { $OutputDir } else { Join-Path $resolvedWork 'artifacts' "osx-$Arch" }
        New-Item -ItemType Directory -Force -Path $artifactsDir | Out-Null
        # CPU_TYPE_X86_64 = 0x01000007, CPU_TYPE_ARM64 = 0x0100000C
        $expected = if ($Arch -eq 'x64') { [uint32]0x01000007 } else { [uint32]0x0100000C }
        foreach ($name in $macLibraries) {
            $library = Join-Path $buildDir $name
            $cpu = Get-MachOArchiveCpuType -Path $library
            if ($cpu -ne $expected) {
                throw ("Wrong Mach-O architecture in {0}: got 0x{1:X8}, expected 0x{2:X8}." -f $library, $cpu, $expected)
            }
            Copy-Item -LiteralPath $library -Destination (Join-Path $artifactsDir $name) -Force
        }
    }
}
