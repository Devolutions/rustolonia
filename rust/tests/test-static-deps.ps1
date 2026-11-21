#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Regression tests for in-repo static dependency PowerShell scripts and Avalonia version sync.
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $root 'rust' 'static-deps' 'common.ps1')

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "Assertion failed: $Message" }
}

function Assert-Throws {
    param([scriptblock]$Action, [string]$Pattern)
    $threw = $false
    try {
        & $Action | Out-Null
    }
    catch {
        $threw = $true
        if ($Pattern -and $_.Exception.Message -notmatch $Pattern) {
            throw "Expected error matching '$Pattern', got: $($_.Exception.Message)"
        }
    }
    if (-not $threw) {
        throw "Expected scriptblock to throw (pattern: '$Pattern')."
    }
}

function New-CoffArchiveBytes {
    param([uint16]$Machine)
    $obj = [byte[]]::new(20)
    [Array]::Copy([System.BitConverter]::GetBytes($Machine), 0, $obj, 0, 2)
    $hdrText = ('{0,-16}{1,-12}{2,-6}{3,-6}{4,-8}{5,-10}`' -f 'test.obj/', '0', '0', '0', '100644', $obj.Length) + "`n"
    $magic = [System.Text.Encoding]::ASCII.GetBytes("!<arch>`n")
    $hdr = [System.Text.Encoding]::ASCII.GetBytes($hdrText)
    $ms = [System.IO.MemoryStream]::new()
    try {
        $ms.Write($magic, 0, $magic.Length)
        $ms.Write($hdr, 0, $hdr.Length)
        $ms.Write($obj, 0, $obj.Length)
        if ($obj.Length % 2 -ne 0) { $ms.WriteByte(0x0A) }
        return $ms.ToArray()
    }
    finally { $ms.Dispose() }
}

function New-MachOArchiveBytes {
    param([uint32]$CpuType)
    $obj = [byte[]]::new(24)
    [Array]::Copy([System.BitConverter]::GetBytes([uint32]0xFEEDFACFu), 0, $obj, 0, 4)
    [Array]::Copy([System.BitConverter]::GetBytes($CpuType), 0, $obj, 4, 4)
    $hdrText = ('{0,-16}{1,-12}{2,-6}{3,-6}{4,-8}{5,-10}`' -f 'test.o', '0', '0', '0', '100644', $obj.Length) + "`n"
    $magic = [System.Text.Encoding]::ASCII.GetBytes("!<arch>`n")
    $hdr = [System.Text.Encoding]::ASCII.GetBytes($hdrText)
    $ms = [System.IO.MemoryStream]::new()
    try {
        $ms.Write($magic, 0, $magic.Length)
        $ms.Write($hdr, 0, $hdr.Length)
        $ms.Write($obj, 0, $obj.Length)
        if ($obj.Length % 2 -ne 0) { $ms.WriteByte(0x0A) }
        return $ms.ToArray()
    }
    finally { $ms.Dispose() }
}

$scratch = Join-Path $root 'rust' 'target' ("test-static-deps-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $scratch | Out-Null

try {
    # 1. Deterministic content-addressed identity for angle and skiasharp.
    foreach ($component in @('angle', 'skiasharp')) {
        $first = Get-StaticDepsIdentity -Component $component -RepoRoot $root
        $second = Get-StaticDepsIdentity -Component $component -RepoRoot $root
        Assert-True ($first['tag'] -eq $second['tag']) "Identity tag for $component must be deterministic."
        Assert-True ($first['tag'].StartsWith("native-$component-")) "Tag must start with native-$component-."
        Assert-True ($first['tag'].Length -eq ("native-$component-".Length + 32)) "Tag must include 32-char hex digest."
        Assert-True ($first['recipe'].Count -gt 0) "Recipe map for $component must be non-empty."
        Assert-StaticDepsManifest -Manifest $first
    }

    # 2. ANGLE bundle + extract roundtrip + tamper detection.
    $angleManifest = Get-StaticDepsIdentity -Component 'angle' -RepoRoot $root
    $angleIncoming = Join-Path $scratch 'angle-incoming'
    foreach ($pair in @(@('win-x64', [uint16]0x8664), @('win-arm64', [uint16]0xAA64))) {
        $dir = Join-Path $angleIncoming "native-angle-$($pair[0])"
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        [System.IO.File]::WriteAllBytes((Join-Path $dir 'libANGLE_static.lib'), (New-CoffArchiveBytes -Machine $pair[1]))
        [System.IO.File]::WriteAllBytes((Join-Path $dir 'libGLESv2_static.lib'), (New-CoffArchiveBytes -Machine $pair[1]))
        [System.IO.File]::WriteAllText((Join-Path $dir 'STATIC-ANGLE-NOTICES.txt'), "ANGLE BSD license`n", [System.Text.UTF8Encoding]::new($false))
    }
    $angleBundledDir = Join-Path $scratch 'angle-bundled'
    $angleBundled = New-StaticDepsBundle -Manifest $angleManifest -IncomingDir $angleIncoming -OutputDir $angleBundledDir
    Assert-True ($angleBundled['archive'] -eq 'angle-native.zip') 'ANGLE bundle must emit angle-native.zip.'
    Assert-True (Test-Path -LiteralPath (Join-Path $angleBundledDir 'component.json') -PathType Leaf) 'ANGLE bundle must emit component.json.'

    $angleExtracted = Join-Path $scratch 'angle-extracted'
    $null = & (Join-Path $root 'rust' 'static-deps' 'artifacts.ps1') extract-local -InputPath $angleBundledDir -Output $angleExtracted
    foreach ($rid in @('win-x64', 'win-arm64')) {
        Assert-True (Test-Path -LiteralPath (Join-Path $angleExtracted $rid 'libANGLE_static.lib') -PathType Leaf) "Extracted $rid must contain libANGLE_static.lib."
        Assert-True (Test-Path -LiteralPath (Join-Path $angleExtracted $rid 'libGLESv2_static.lib') -PathType Leaf) "Extracted $rid must contain libGLESv2_static.lib."
        Assert-True (Test-Path -LiteralPath (Join-Path $angleExtracted $rid 'STATIC-ANGLE-NOTICES.txt') -PathType Leaf) "Extracted $rid must contain STATIC-ANGLE-NOTICES.txt."
        $prov = Read-StaticDepsJson -Path (Join-Path $angleExtracted $rid 'STATIC-ANGLE-PROVENANCE.json')
        Assert-True ($prov['tag'] -eq $angleBundled['tag']) "Extracted $rid provenance tag must match bundle."
    }

    # Tampering with zip must fail checksum verification.
    $zipToTamper = Join-Path $angleBundledDir 'angle-native.zip'
    $tamperZip = [System.IO.Compression.ZipFile]::Open($zipToTamper, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $extra = $tamperZip.CreateEntry('win-x64/extra.txt')
        $w = [System.IO.StreamWriter]::new($extra.Open())
        try { $w.Write('tampered') } finally { $w.Dispose() }
    }
    finally { $tamperZip.Dispose() }
    Assert-Throws {
        Expand-StaticDepsBundle -Manifest $angleBundled -ArchivePath $zipToTamper -DestinationDir (Join-Path $scratch 'angle-tampered')
    } 'checksum mismatch'

    # 3. SkiaSharp bundle + extract preserves Linux glibc 2.35 provenance while writing Win/macOS provenance.
    $skiaManifest = Get-StaticDepsIdentity -Component 'skiasharp' -RepoRoot $root
    $skiaIncoming = Join-Path $scratch 'skia-incoming'
    foreach ($rid in @('win-x64', 'win-arm64')) {
        $dir = Join-Path $skiaIncoming "native-skiasharp-$rid"
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        foreach ($lib in @('skia.lib', 'SkiaSharp.lib', 'libHarfBuzzSharp.lib')) {
            [System.IO.File]::WriteAllText((Join-Path $dir $lib), "fake $lib")
        }
    }
    foreach ($rid in @('linux-x64', 'linux-arm64')) {
        $dir = Join-Path $skiaIncoming "native-skiasharp-$rid"
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        foreach ($lib in @('libskia.a', 'libSkiaSharp.a', 'libHarfBuzzSharp.a')) {
            [System.IO.File]::WriteAllText((Join-Path $dir $lib), "fake $lib")
        }
        Write-StaticDepsJson -Path (Join-Path $dir 'STATIC-SKIASHARP-PROVENANCE.json') -Value @{ component = 'skiasharp'; rid = $rid; glibcBuild = '2.35' }
        [System.IO.File]::WriteAllText((Join-Path $dir 'STATIC-SKIASHARP-SOURCE-NOTICES.txt'), "linux notices`n")
    }
    foreach ($rid in @('osx-x64', 'osx-arm64')) {
        $dir = Join-Path $skiaIncoming 'native-skiasharp-osx' $rid
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        foreach ($lib in @('libskia.a', 'libSkiaSharp.a', 'libHarfBuzzSharp.a')) {
            [System.IO.File]::WriteAllText((Join-Path $dir $lib), "fake $lib")
        }
    }
    $skiaBundledDir = Join-Path $scratch 'skia-bundled'
    $skiaBundled = New-StaticDepsBundle -Manifest $skiaManifest -IncomingDir $skiaIncoming -OutputDir $skiaBundledDir
    $skiaExtracted = Join-Path $scratch 'skia-extracted'
    $null = Expand-StaticDepsBundle -Manifest $skiaBundled -ArchivePath (Join-Path $skiaBundledDir 'skiasharp-native.zip') -DestinationDir $skiaExtracted
    $linuxProv = Read-StaticDepsJson -Path (Join-Path $skiaExtracted 'linux-x64' 'STATIC-SKIASHARP-PROVENANCE.json')
    Assert-True ($linuxProv['glibcBuild'] -eq '2.35') 'Linux provenance must preserve glibc 2.35 metadata.'
    $winProv = Read-StaticDepsJson -Path (Join-Path $skiaExtracted 'win-x64' 'STATIC-SKIASHARP-PROVENANCE.json')
    Assert-True ($winProv['tag'] -eq $skiaBundled['tag']) 'Windows provenance must record component tag.'
    $osxProv = Read-StaticDepsJson -Path (Join-Path $skiaExtracted 'osx-arm64' 'STATIC-SKIASHARP-PROVENANCE.json')
    Assert-True ($osxProv['tag'] -eq $skiaBundled['tag']) 'macOS provenance must record component tag.'

    # 4. COFF and Mach-O architecture checks.
    $archDir = Join-Path $scratch 'arch'
    New-Item -ItemType Directory -Force -Path $archDir | Out-Null
    $x64Lib = Join-Path $archDir 'x64.lib'
    $arm64Lib = Join-Path $archDir 'arm64.lib'
    [System.IO.File]::WriteAllBytes($x64Lib, (New-CoffArchiveBytes -Machine 0x8664))
    [System.IO.File]::WriteAllBytes($arm64Lib, (New-CoffArchiveBytes -Machine 0xAA64))
    Assert-True ((Get-CoffArchiveMachine -Path $x64Lib) -eq 0x8664) 'COFF x64 machine check failed.'
    Assert-True ((Get-CoffArchiveMachine -Path $arm64Lib) -eq 0xAA64) 'COFF arm64 machine check failed.'

    $x64A = Join-Path $archDir 'x64.a'
    $arm64A = Join-Path $archDir 'arm64.a'
    [System.IO.File]::WriteAllBytes($x64A, (New-MachOArchiveBytes -CpuType 0x01000007))
    [System.IO.File]::WriteAllBytes($arm64A, (New-MachOArchiveBytes -CpuType 0x0100000C))
    Assert-True ((Get-MachOArchiveCpuType -Path $x64A) -eq [uint32]0x01000007) 'Mach-O x64 cpu check failed.'
    Assert-True ((Get-MachOArchiveCpuType -Path $arm64A) -eq [uint32]0x0100000C) 'Mach-O arm64 cpu check failed.'

    # 5. Avalonia Directory.Packages.props sync check (matching + drift rejection).
    $skiaCfg = Get-StaticDepsSourceConfig -Component 'skiasharp' -RepoRoot $root
    $angleCfg = Get-StaticDepsSourceConfig -Component 'angle' -RepoRoot $root
    $matchingProps = Join-Path $scratch 'Directory.Packages.matching.props'
    @"
<Project>
  <ItemGroup>
    <PackageVersion Include="SkiaSharp" Version="$($skiaCfg['version'])" />
    <PackageVersion Include="HarfBuzzSharp" Version="$($skiaCfg['harfbuzzVersion'])" />
    <PackageVersion Include="Avalonia.Angle.Windows.Natives" Version="$($angleCfg['avaloniaAnglePackageVersion'])" />
  </ItemGroup>
</Project>
"@ | Set-Content -LiteralPath $matchingProps -Encoding utf8
    Assert-StaticDepsSyncedWithAvalonia -RepoRoot $root -AvaloniaPackagesProps $matchingProps

    $driftedSkiaProps = Join-Path $scratch 'Directory.Packages.skia-drift.props'
    @"
<Project>
  <ItemGroup>
    <PackageVersion Include="SkiaSharp" Version="3.120.0" />
    <PackageVersion Include="HarfBuzzSharp" Version="$($skiaCfg['harfbuzzVersion'])" />
    <PackageVersion Include="Avalonia.Angle.Windows.Natives" Version="$($angleCfg['avaloniaAnglePackageVersion'])" />
  </ItemGroup>
</Project>
"@ | Set-Content -LiteralPath $driftedSkiaProps -Encoding utf8
    Assert-Throws {
        Assert-StaticDepsSyncedWithAvalonia -RepoRoot $root -AvaloniaPackagesProps $driftedSkiaProps
    } 'SkiaSharp version drift'

    $driftedAngleProps = Join-Path $scratch 'Directory.Packages.angle-drift.props'
    @"
<Project>
  <ItemGroup>
    <PackageVersion Include="SkiaSharp" Version="$($skiaCfg['version'])" />
    <PackageVersion Include="HarfBuzzSharp" Version="$($skiaCfg['harfbuzzVersion'])" />
    <PackageVersion Include="Avalonia.Angle.Windows.Natives" Version="2.1.99999.20269999" />
  </ItemGroup>
</Project>
"@ | Set-Content -LiteralPath $driftedAngleProps -Encoding utf8
    Assert-Throws {
        Assert-StaticDepsSyncedWithAvalonia -RepoRoot $root -AvaloniaPackagesProps $driftedAngleProps
    } 'ANGLE version drift'

    # 6. Workflow YAML must not use unallowed third-party actions or invalid needs.*.result expressions.
    foreach ($wf in @('build-angle.yml', 'build-skiasharp.yml', 'release.yml', 'avalonia-rust.yml')) {
        $wfPath = Join-Path $root '.github' 'workflows' $wf
        $wfText = Get-Content -LiteralPath $wfPath -Raw
        Assert-True ($wfText -notmatch 'setup-msvc-dev') "$wf must not use unallowed third-party action 'setup-msvc-dev'."
        Assert-True ($wfText -notmatch 'needs\.\*\.result') "$wf must not use invalid 'needs.*.result' in job conditions."
    }

    # 7. build-angle.ps1 and build-skiasharp.ps1 must resolve repoRoot to the repository root.
    $angleCollectWork = Join-Path $scratch 'angle-collect-work'
    $angleCollectBuildObj = Join-Path $angleCollectWork 'build' 'win-x64' 'obj'
    $angleCollectSrc = Join-Path $angleCollectWork 'src_angle'
    New-Item -ItemType Directory -Force -Path $angleCollectBuildObj | Out-Null
    New-Item -ItemType Directory -Force -Path $angleCollectSrc | Out-Null
    [System.IO.File]::WriteAllBytes((Join-Path $angleCollectBuildObj 'libANGLE_static.lib'), (New-CoffArchiveBytes -Machine 0x8664))
    [System.IO.File]::WriteAllBytes((Join-Path $angleCollectBuildObj 'libGLESv2_static.lib'), (New-CoffArchiveBytes -Machine 0x8664))
    [System.IO.File]::WriteAllText((Join-Path $angleCollectSrc 'LICENSE'), "BSD license`n")
    $angleCollectOut = Join-Path $scratch 'angle-collect-out'
    & (Join-Path $root 'rust' 'static-deps' 'angle' 'build-angle.ps1') collect -Arch x64 -WorkDir $angleCollectWork -OutputDir $angleCollectOut
    Assert-True (Test-Path -LiteralPath (Join-Path $angleCollectOut 'ANGLE-source.json') -PathType Leaf) 'build-angle.ps1 collect must write ANGLE-source.json from repoRoot.'

    $skiaCollectWork = Join-Path $scratch 'skia-collect-work'
    $skiaCollectBuild = Join-Path $skiaCollectWork 'build' 'win-x64'
    New-Item -ItemType Directory -Force -Path $skiaCollectBuild | Out-Null
    foreach ($lib in @('skia.lib', 'SkiaSharp.lib', 'libHarfBuzzSharp.lib')) {
        [System.IO.File]::WriteAllBytes((Join-Path $skiaCollectBuild $lib), (New-CoffArchiveBytes -Machine 0x8664))
    }
    $skiaCollectOut = Join-Path $scratch 'skia-collect-out'
    & (Join-Path $root 'rust' 'static-deps' 'skiasharp' 'build-skiasharp.ps1') collect-windows -Arch x64 -WorkDir $skiaCollectWork -OutputDir $skiaCollectOut
    Assert-True (Test-Path -LiteralPath (Join-Path $skiaCollectOut 'skia.lib') -PathType Leaf) 'build-skiasharp.ps1 collect-windows must collect skia.lib.'

    Write-Host 'Static native dependency tests passed.'
}
finally {
    Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue
}
