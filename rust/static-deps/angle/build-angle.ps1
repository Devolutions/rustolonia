#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Build the pinned ANGLE source as Windows static libraries.
#>
param(
    [Parameter(Mandatory, Position = 0)]
    [ValidateSet('prepare', 'build', 'collect')]
    [string]$Command,

    [Parameter(Mandatory)]
    [ValidateSet('x64', 'arm64')]
    [string]$Arch,

    [string]$WorkDir,
    [string]$OutputDir
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path (Split-Path -Parent $PSScriptRoot) 'common.ps1')

$angleDir = $PSScriptRoot
$repoRoot = $script:StaticDepsRepoRoot
$libraries = @('libANGLE_static.lib', 'libGLESv2_static.lib')

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

function Apply-AnglePatches {
    param(
        [Parameter(Mandatory)][string]$SourceDir,
        [Parameter(Mandatory)][ValidateSet('deps', 'build')][string]$Stage
    )
    $patchDir = Join-Path $angleDir 'patches' $Stage
    foreach ($patch in @(Get-ChildItem -LiteralPath $patchDir -Filter '*.patch' | Sort-Object Name -CaseSensitive)) {
        Invoke-Checked -FilePath 'git' -Arguments @('apply', '--check', $patch.FullName) -WorkingDirectory $SourceDir
        Invoke-Checked -FilePath 'git' -Arguments @('apply', $patch.FullName) -WorkingDirectory $SourceDir
    }
}

function Write-AngleNotices {
    param(
        [Parameter(Mandatory)][string]$SourceDir,
        [Parameter(Mandatory)][string]$DestinationFile
    )
    $resolvedSource = (Resolve-Path -LiteralPath $SourceDir).Path
    $candidates = @(Get-ChildItem -LiteralPath $resolvedSource -File -Recurse | Where-Object {
        $upper = $_.Name.ToUpperInvariant()
        if (-not ($upper.StartsWith('LICENSE') -or $upper.StartsWith('COPYING') -or $upper.StartsWith('COPYRIGHT'))) {
            return $false
        }
        $rel = [System.IO.Path]::GetRelativePath($resolvedSource, $_.FullName).Replace('\', '/')
        $parts = $rel -split '/'
        return ($parts -notcontains '.git') -and ($parts -notcontains 'out')
    })
    if ($candidates.Count -eq 0) {
        throw "No ANGLE license files found under $resolvedSource"
    }
    $entries = @($candidates | ForEach-Object {
        [pscustomobject]@{
            Rel  = [System.IO.Path]::GetRelativePath($resolvedSource, $_.FullName).Replace('\', '/')
            Path = $_.FullName
        }
    } | Sort-Object -Property Rel -CaseSensitive)

    $sb = [System.Text.StringBuilder]::new()
    for ($i = 0; $i -lt $entries.Count; $i++) {
        if ($i -gt 0) { [void]$sb.Append("`n") }
        [void]$sb.Append("===== angle/$($entries[$i].Rel) =====`n")
        $text = [System.IO.File]::ReadAllText($entries[$i].Path).TrimEnd("`r", "`n").Replace("`r`n", "`n")
        [void]$sb.Append($text)
        [void]$sb.Append("`n")
    }
    $parent = Split-Path -Parent $DestinationFile
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    [System.IO.File]::WriteAllText($DestinationFile, $sb.ToString(), [System.Text.UTF8Encoding]::new($false))
    return $entries.Count
}

$resolvedWork = if ($WorkDir) { $WorkDir } else { Join-Path $repoRoot 'out' 'static-angle' }
$sourceDir = Join-Path $resolvedWork 'src_angle'
$buildDir = Join-Path $resolvedWork 'build' "win-$Arch"

switch ($Command) {
    'prepare' {
        if (Test-Path -LiteralPath $sourceDir) {
            throw "$sourceDir already exists; use a clean work directory for a native build."
        }
        $config = Get-StaticDepsSourceConfig -Component 'angle' -RepoRoot $repoRoot
        New-Item -ItemType Directory -Force -Path $sourceDir | Out-Null
        Invoke-Checked -FilePath 'git' -Arguments @('init', $sourceDir)
        Invoke-Checked -FilePath 'git' -Arguments @('remote', 'add', 'origin', [string]$config['repository']) -WorkingDirectory $sourceDir
        Invoke-Checked -FilePath 'git' -Arguments @('fetch', '--depth=1', 'origin', [string]$config['commit']) -WorkingDirectory $sourceDir
        Invoke-Checked -FilePath 'git' -Arguments @('checkout', '--detach', 'FETCH_HEAD') -WorkingDirectory $sourceDir
        Apply-AnglePatches -SourceDir $sourceDir -Stage 'deps'
        Invoke-Checked -FilePath 'python' -Arguments @('scripts/bootstrap.py') -WorkingDirectory $sourceDir
        Invoke-Checked -FilePath 'gclient' -Arguments @('sync', '-f', '-D', '-R') -WorkingDirectory $sourceDir
        Apply-AnglePatches -SourceDir $sourceDir -Stage 'build'
        New-Item -ItemType Directory -Force -Path $buildDir | Out-Null
        $argsText = (Get-Content -LiteralPath (Join-Path $angleDir 'args.win.gn') -Raw -Encoding utf8).Replace('target_cpu = "x64"', "target_cpu = `"$Arch`"").Replace("`r`n", "`n")
        [System.IO.File]::WriteAllText((Join-Path $buildDir 'args.gn'), $argsText, [System.Text.UTF8Encoding]::new($false))
    }
    'build' {
        Invoke-Checked -FilePath 'gn' -Arguments @('gen', $buildDir) -WorkingDirectory $sourceDir
        Invoke-Checked -FilePath 'autoninja' -Arguments @('-C', $buildDir, 'libANGLE_static', 'libGLESv2_static') -WorkingDirectory $sourceDir
    }
    'collect' {
        $artifactsDir = if ($OutputDir) { $OutputDir } else { Join-Path $resolvedWork 'artifacts' "angle-win-$Arch" }
        New-Item -ItemType Directory -Force -Path $artifactsDir | Out-Null
        $expected = if ($Arch -eq 'x64') { 0x8664 } else { 0xAA64 }
        foreach ($name in $libraries) {
            $library = Join-Path $buildDir 'obj' $name
            $machine = Get-CoffArchiveMachine -Path $library
            if ($machine -ne $expected) {
                throw ("Wrong COFF architecture in {0}: got 0x{1:X4}, expected 0x{2:X4}." -f $library, $machine, $expected)
            }
            Copy-Item -LiteralPath $library -Destination (Join-Path $artifactsDir $name) -Force
        }
        $config = Get-StaticDepsSourceConfig -Component 'angle' -RepoRoot $repoRoot
        Write-StaticDepsJson -Path (Join-Path $artifactsDir 'ANGLE-source.json') -Value $config
        $null = Write-AngleNotices -SourceDir $sourceDir -DestinationFile (Join-Path $artifactsDir 'STATIC-ANGLE-NOTICES.txt')
    }
}
