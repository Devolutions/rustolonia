#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Content-addressed build identities, verification, and GitHub Release cache for static native deps.
#>
param(
    [Parameter(Mandatory, Position = 0)]
    [ValidateSet('plan', 'bundle', 'store-component', 'fetch-component', 'extract-local', 'check-sync')]
    [string]$Command,

    [ValidateSet('angle', 'skiasharp')]
    [string]$Component,

    [string]$Tag,
    [string]$InputPath = 'out/component.json',
    [string]$Incoming = 'out/incoming',
    [string]$Output = 'out/component',
    [string]$AvaloniaPackagesProps,
    [switch]$Probe,
    [switch]$SkipSyncCheck
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'common.ps1')

switch ($Command) {
    'check-sync' {
        Assert-StaticDepsSyncedWithAvalonia -AvaloniaPackagesProps $AvaloniaPackagesProps
        Write-Host 'Static dependency pins are in sync with Avalonia.'
    }
    'plan' {
        if (-not $Component) { throw '-Component is required for plan.' }
        if (-not $SkipSyncCheck) {
            Assert-StaticDepsSyncedWithAvalonia -AvaloniaPackagesProps $AvaloniaPackagesProps
        }
        $manifest = Get-StaticDepsIdentity -Component $Component
        Write-StaticDepsJson -Path $Output -Value $manifest
        $exists = if ($Probe) { Test-StaticDepsCachedRelease -Expected $manifest } else { $false }
        Write-GitHubWorkflowOutputs -Values ([ordered]@{
            tag     = $manifest['tag']
            commit  = $manifest['commit']
            version = $manifest['version']
            exists  = $exists
        })
    }
    'bundle' {
        $manifest = Read-StaticDepsJson -Path $InputPath
        $bundled = New-StaticDepsBundle -Manifest $manifest -IncomingDir $Incoming -OutputDir $Output
        Write-GitHubWorkflowOutputs -Values ([ordered]@{
            tag = $bundled['tag']
        })
    }
    'store-component' {
        $manifestPath = Join-Path $Output 'component.json'
        $manifest = Read-StaticDepsJson -Path $manifestPath
        Assert-StaticDepsManifest -Manifest $manifest
        $archivePath = Join-Path $Output ([string]$manifest['archive'])
        Publish-StaticDepsRelease `
            -Tag ([string]$manifest['tag']) `
            -Files @($manifestPath, $archivePath) `
            -Title ([string]$manifest['tag']) `
            -Notes "Verified static $($manifest['component']) native libraries. Source: $($manifest['commit'])."
        Write-GitHubWorkflowOutputs -Values ([ordered]@{
            tag     = $manifest['tag']
            version = $manifest['version']
        })
    }
    'fetch-component' {
        if (-not $Component) { throw '-Component is required for fetch-component.' }
        $resolvedTag = if ($Tag) { $Tag } else { [string](Get-StaticDepsIdentity -Component $Component)['tag'] }
        if (-not $resolvedTag.StartsWith("native-$Component-")) {
            throw "Unexpected $Component component tag: $resolvedTag"
        }
        $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("rustolonia-fetch-" + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Force -Path $tmp | Out-Null
        try {
            $manifestPath = Receive-StaticDepsReleaseAsset -Tag $resolvedTag -Name 'component.json' -Directory $tmp
            $manifest = Read-StaticDepsJson -Path $manifestPath
            if ([string]$manifest['tag'] -ne $resolvedTag -or [string]$manifest['component'] -ne $Component) {
                throw 'Downloaded component does not match the requested tag.'
            }
            $archivePath = Receive-StaticDepsReleaseAsset -Tag $resolvedTag -Name ([string]$manifest['archive']) -Directory $tmp
            $null = Expand-StaticDepsBundle -Manifest $manifest -ArchivePath $archivePath -DestinationDir $Output
            Write-GitHubWorkflowOutputs -Values ([ordered]@{
                tag     = $manifest['tag']
                version = $manifest['version']
            })
        }
        finally {
            Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
    'extract-local' {
        $manifestPath = Join-Path $InputPath 'component.json'
        $manifest = Read-StaticDepsJson -Path $manifestPath
        $archivePath = Join-Path $InputPath ([string]$manifest['archive'])
        $null = Expand-StaticDepsBundle -Manifest $manifest -ArchivePath $archivePath -DestinationDir $Output
        Write-GitHubWorkflowOutputs -Values ([ordered]@{
            tag     = $manifest['tag']
            version = $manifest['version']
        })
    }
}
