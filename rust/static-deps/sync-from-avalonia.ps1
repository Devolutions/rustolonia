#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Validate or update pinned static dependency metadata against Avalonia's Directory.Packages.props.

.DESCRIPTION
By default, verifies that rust/static-deps/skiasharp/source.json,
rust/static-deps/angle/source.json, and rust/build-linux-static-graphics.sh match
the SkiaSharp, HarfBuzzSharp, and Avalonia.Angle.Windows.Natives versions declared
in avalonia-src/Directory.Packages.props.

Pass -Update after bumping avalonia-src to resolve the new mono/SkiaSharp tag and
externals/skia submodule commit, update the checked-in pins, and flag any ANGLE
package version bump for patch review.
#>
param(
    [string]$AvaloniaPackagesProps,
    [switch]$Update
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'common.ps1')

$repoRoot = $script:StaticDepsRepoRoot
$propsPath = if ($AvaloniaPackagesProps) {
    $AvaloniaPackagesProps
} else {
    Join-Path $repoRoot 'avalonia-src' 'Directory.Packages.props'
}

if (-not $Update) {
    if (-not (Test-Path -LiteralPath $propsPath -PathType Leaf)) {
        throw "Avalonia Directory.Packages.props not found at $propsPath. Initialize the avalonia-src submodule or pass -AvaloniaPackagesProps."
    }
    Assert-StaticDepsSyncedWithAvalonia -RepoRoot $repoRoot -AvaloniaPackagesProps $propsPath
    Write-Host 'Static dependency pins match Avalonia Directory.Packages.props.'
    return
}

$avalonia = Get-AvaloniaPackageVersions -PackagesPropsPath $propsPath
$skiaVersion = $avalonia['SkiaSharp']
$harfbuzzVersion = $avalonia['HarfBuzzSharp']
$anglePkgVersion = $avalonia['Avalonia.Angle.Windows.Natives']

$skiaConfigPath = Join-Path $repoRoot 'rust' 'static-deps' 'skiasharp' 'source.json'
$skiaConfig = Read-StaticDepsJson -Path $skiaConfigPath
$skiaChanged = ($skiaConfig['version'] -ne $skiaVersion) -or ($skiaConfig['harfbuzzVersion'] -ne $harfbuzzVersion)

if ($skiaChanged) {
    $tag = "v$skiaVersion"
    $refs = & git ls-remote --exit-code ([string]$skiaConfig['repository']) "refs/tags/$tag" "refs/tags/${tag}^{}"
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to resolve tag '$tag' in $($skiaConfig['repository'])."
    }
    $resolved = @{}
    foreach ($line in $refs) {
        $parts = $line -split '\s+'
        if ($parts.Count -ge 2) { $resolved[$parts[1]] = $parts[0] }
    }
    $skiaSharpCommit = if ($resolved.ContainsKey("refs/tags/${tag}^{}")) {
        $resolved["refs/tags/${tag}^{}"]
    } else {
        $resolved["refs/tags/$tag"]
    }
    if (-not $skiaSharpCommit -or $skiaSharpCommit -notmatch '^[0-9a-f]{40}$') {
        throw "Failed to resolve 40-char commit for mono/SkiaSharp tag '$tag'."
    }
    $skiaSubmoduleRes = Invoke-GhCapture -Arguments @('api', "repos/mono/SkiaSharp/contents/externals/skia?ref=$skiaSharpCommit", '--jq', '.sha')
    if ($skiaSubmoduleRes.ExitCode -ne 0) {
        throw "Failed to resolve externals/skia submodule commit at mono/SkiaSharp@${skiaSharpCommit}: $($skiaSubmoduleRes.StdErr)"
    }
    $skiaSubmoduleCommit = $skiaSubmoduleRes.StdOut.Trim()
    if ($skiaSubmoduleCommit -notmatch '^[0-9a-f]{40}$') {
        throw "Unexpected externals/skia commit '$skiaSubmoduleCommit'."
    }

    $skiaConfig['ref'] = $tag
    $skiaConfig['version'] = $skiaVersion
    $skiaConfig['harfbuzzVersion'] = $harfbuzzVersion
    $skiaConfig['commit'] = $skiaSharpCommit
    $skiaConfig['skiaCommit'] = $skiaSubmoduleCommit
    Write-StaticDepsJson -Path $skiaConfigPath -Value $skiaConfig
    Write-Host "Updated rust/static-deps/skiasharp/source.json -> SkiaSharp $skiaVersion ($skiaSharpCommit), Skia $skiaSubmoduleCommit, HarfBuzzSharp $harfbuzzVersion."

    $linuxScriptPath = Join-Path $repoRoot 'rust' 'build-linux-static-graphics.sh'
    $linuxScript = Get-Content -LiteralPath $linuxScriptPath -Raw -Encoding utf8
    $linuxScript = [regex]::Replace($linuxScript, '(?m)^skiasharp_commit=[0-9a-f]{40}', "skiasharp_commit=$skiaSharpCommit")
    $linuxScript = [regex]::Replace($linuxScript, '(?m)^skia_commit=[0-9a-f]{40}', "skia_commit=$skiaSubmoduleCommit")
    $linuxScript = [regex]::Replace($linuxScript, '--branch\s+v[^\s]+', "--branch $tag")
    [System.IO.File]::WriteAllText($linuxScriptPath, $linuxScript.Replace("`r`n", "`n"), [System.Text.UTF8Encoding]::new($false))
    Write-Host "Updated rust/build-linux-static-graphics.sh -> v$skiaVersion."
}

$angleConfigPath = Join-Path $repoRoot 'rust' 'static-deps' 'angle' 'source.json'
$angleConfig = Read-StaticDepsJson -Path $angleConfigPath
if ([string]$angleConfig['avaloniaAnglePackageVersion'] -ne $anglePkgVersion) {
    $previous = [string]$angleConfig['avaloniaAnglePackageVersion']
    $angleConfig['avaloniaAnglePackageVersion'] = $anglePkgVersion
    Write-StaticDepsJson -Path $angleConfigPath -Value $angleConfig
    Write-Warning "Avalonia.Angle.Windows.Natives changed from '$previous' to '$anglePkgVersion'. Review rust/static-deps/angle/source.json commit and patches/build/* before merging."
}

Assert-StaticDepsSyncedWithAvalonia -RepoRoot $repoRoot -AvaloniaPackagesProps $propsPath
Write-Host 'Static dependency sync complete.'
