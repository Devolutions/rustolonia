#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Builds the prebuilt Rustolonia browser (WebAssembly) host for a release.

.DESCRIPTION
The browser host statically links the Rust app into the .NET runtime's
dotnet.native.wasm, so it cannot ship as a finished binary. This script
publishes host/Browser once (with rust/browser-sample as a stand-in app),
records the exact Emscripten link through the project's
RustoloniaBrowserKitCapture hook, and packages a relink kit:

    rustolonia-host-<version>-browser-wasm.tar.gz (+ .sha256)
      site/               the published static site without dotnet.native.*
      link/emcc.rsp       the Emscripten link, with @KIT@/@APP@/@OUT@ placeholders
      link/NN-<name>      every link input: .NET runtime archives, the
                          generated interop objects, SkiaSharp/HarfBuzz and
                          the runtime's JavaScript libraries
      build-site.py       relinks with an app's staticlib and writes a site
      host-manifest.json  version, ABI fingerprint, Emscripten version, files

Consumers need Rust and the Emscripten SDK at the recorded version, not .NET;
see rust/PLATFORMS.md#prebuilt-browser-host.

This is a producer-side command: it needs the patched Avalonia source, the
.NET SDK with the wasm-tools workload, and the wasm32-unknown-emscripten Rust
target (rust/build-browser.ps1 requirements).
#>
param(
    [string]$OutputRoot,
    [string]$RustoloniaRoot = (Split-Path -Parent $PSScriptRoot),
    # Extra arguments for the dotnet invocations (for example -p:RestoreConfigFile=...).
    [string[]]$DotnetArgs = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'package-shared.ps1')

$root = (Resolve-Path -LiteralPath (Resolve-CallerRelativePath -PathValue $RustoloniaRoot)).Path
$producer = Join-Path $root 'avalonia-src'
Assert-ConsumerSource -ProducerRoot $producer -RustoloniaRoot $root
$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $root
$abi = Get-AbiFingerprint -RustoloniaRoot $root
$release = Get-Content -LiteralPath (Join-Path $root 'rust' 'release-manifest.json') -Raw | ConvertFrom-Json
$revision = (& git -C $root rev-parse HEAD).Trim()
$timestamp = if ($env:SOURCE_DATE_EPOCH) { [long]$env:SOURCE_DATE_EPOCH }
    else { [long](& git -C $root log -1 --format=%ct HEAD) }
$rid = $script:BrowserHostRid

if (-not $OutputRoot) { $OutputRoot = Join-Path $root 'rust' 'artifacts' 'host' }
$OutputRoot = Resolve-CallerRelativePath -PathValue $OutputRoot
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$assetName = Get-HostAssetName -Version $version -Rid $rid
$assetPath = Join-Path $OutputRoot $assetName
if ((Test-Path -LiteralPath $assetPath) -or (Test-Path -LiteralPath "$assetPath.sha256")) {
    throw "Refusing to overwrite an existing host archive or checksum: $assetPath"
}
$staging = Join-Path $OutputRoot (".browser-staging-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $staging | Out-Null

function Copy-RequiredFile([string]$Source, [string]$Destination) {
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Required host input is missing: $Source" }
    Copy-Item -LiteralPath $Source -Destination $Destination
}

function Read-Rsp([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Response file is missing: $Path" }
    return @(Get-Content -LiteralPath $Path | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

try {
    # 1. Publish the host with the stand-in app, recording the link.
    $publish = Join-Path $staging 'publish'
    $capture = Join-Path $staging 'link-capture.txt'
    & (Join-Path $root 'rust' 'build-browser.ps1') -Package browser-sample -Configuration Release `
        -OutputDirectory $publish -DotnetArgs (@("-p:RustoloniaBrowserKitCapture=$capture") + $DotnetArgs)
    if (-not (Test-Path -LiteralPath $capture -PathType Leaf)) {
        throw 'The host publish did not record its Emscripten link (RustoloniaBrowserKitCapture).'
    }
    $facts = @{}
    # Feature switches the runtime's JavaScript library reads from emcc's
    # environment; toolchain locations (PATH, PYTHON*, EM_CACHE, ...) are the
    # consumer's own Emscripten SDK's business.
    $linkEnvironment = [ordered]@{}
    foreach ($line in Get-Content -LiteralPath $capture) {
        $key, $value = $line -split '=', 2
        if ($key -like 'Env:*') {
            $name = $key.Substring(4)
            if ($name -match '^(WASM_ENABLE_[A-Z]+|ENABLE_[A-Z_]+|RUN_AOT_COMPILATION)$') {
                if ($value -notmatch '^[0-9A-Za-z_.-]*$') { throw "Unexpected value for $name in the recorded link environment: $value" }
                $linkEnvironment[$name] = $value
            }
            continue
        }
        $facts[$key] = $value
    }
    if (-not $linkEnvironment.Contains('WASM_ENABLE_SIMD')) { throw 'The recorded link environment has no WASM_ENABLE_SIMD.' }
    $runtimeNative = $facts['RuntimePackNativeDir']
    if ($runtimeNative -notmatch '[/\\]microsoft\.netcore\.app\.runtime\.mono\.browser-wasm[/\\](\d+\.\d+\.\d+[^/\\]*)[/\\]') {
        throw "Cannot determine the .NET runtime pack version from $runtimeNative"
    }
    $runtimeVersion = $Matches[1]
    $runtimePack = (Resolve-Path -LiteralPath (Join-Path $runtimeNative '..' '..' '..')).Path
    $props = Get-Content -LiteralPath (Join-Path $runtimeNative 'src' 'wasm-props.json') -Raw | ConvertFrom-Json
    $emscriptenVersion = @($props.items.EmccProperties | Where-Object identity -eq 'RuntimeEmccVersion')[0].value
    if ($emscriptenVersion -notmatch '^\d+\.\d+\.\d+$') { throw "Unexpected Emscripten version '$emscriptenVersion'." }

    # 2. One response file with every input copied into link/.
    $bundle = Join-Path $staging 'host'
    $linkDirectory = Join-Path $bundle 'link'
    foreach ($directory in @('site', 'link', 'licenses')) {
        New-Item -ItemType Directory -Path (Join-Path $bundle $directory) -Force | Out-Null
    }
    $rustApp = [IO.Path]::GetFullPath($facts['RustApp'])
    $copied = [ordered]@{}
    $rewrite = {
        param([string]$Line)
        [regex]::Replace($Line, '"([^"]+)"', {
            param($match)
            $value = $match.Groups[1].Value
            if (-not [IO.Path]::IsPathFullyQualified($value)) { return $match.Value }
            $full = [IO.Path]::GetFullPath($value)
            if ($full -eq $rustApp) { return '"@APP@"' }
            if ($full.EndsWith('dotnet.native.js', [StringComparison]::OrdinalIgnoreCase) -and -not (Test-Path -LiteralPath $full -PathType Leaf)) {
                return '"@OUT@"'
            }
            if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { return $match.Value }
            if (-not $copied.Contains($full)) {
                $name = '{0:D2}-{1}' -f $copied.Count, [IO.Path]::GetFileName($full)
                Copy-Item -LiteralPath $full -Destination (Join-Path $linkDirectory $name)
                $copied[$full] = $name
            }
            return '"@KIT@/link/' + $copied[$full] + '"'
        })
    }
    $lines = @()
    $lines += Read-Rsp $facts['DefaultFlagsRsp']
    if ($facts['DefaultFlags']) { $lines += $facts['DefaultFlags'] }
    $lines += Read-Rsp $facts['DefaultLinkFlagsRsp']
    $lines += Read-Rsp $facts['LinkRsp']
    $rspLines = @()
    foreach ($line in $lines) {
        if ($line -match '^\s*-o\s+"') { $rspLines += '-o "@OUT@"'; continue }
        $rspLines += & $rewrite $line
    }
    $unresolved = @($rspLines | Where-Object { $_ -match '"[A-Za-z]:[\\/]|"/(home|Users|tmp|usr|opt|mnt)/' })
    if ($unresolved.Count -gt 0) { throw "The link refers to files that were not packaged:`n$($unresolved -join "`n")" }
    if (-not ($rspLines -contains '"@APP@"')) { throw 'The recorded link does not contain the Rust app archive.' }
    if (-not ($rspLines -contains '-o "@OUT@"')) { throw 'The recorded link has no dotnet.native.js output.' }
    [IO.File]::WriteAllText((Join-Path $linkDirectory 'emcc.rsp'), (($rspLines -join "`n") + "`n"), [Text.UTF8Encoding]::new($false))

    # 3. The site without the native runtime, which build-site.py relinks,
    #    nor dotnet.js's precompressed copies (it rewrites the boot config).
    $site = Join-Path $publish 'wwwroot'
    Copy-Item -Path (Join-Path $site '*') -Destination (Join-Path $bundle 'site') -Recurse
    $framework = Join-Path $bundle 'site' '_framework'
    Get-ChildItem -LiteralPath $framework -File | Where-Object {
        $_.Name -like 'dotnet.native.*' -or $_.Name -in @('dotnet.js.br', 'dotnet.js.gz')
    } | Remove-Item
    $bootConfig = Join-Path $framework 'dotnet.js'
    $bootText = Get-Content -LiteralPath $bootConfig -Raw
    if (-not $bootText.Contains('/*json-start*/') -or -not $bootText.Contains('"wasmNative"')) {
        throw 'dotnet.js does not embed the expected boot configuration.'
    }

    # 4. Licenses, the link script and the manifest.
    Copy-BundleNotices -ProducerRoot $producer -RustoloniaRoot $root -DestinationDirectory $bundle
    Copy-RequiredFile (Join-Path $runtimePack 'LICENSE.TXT') (Join-Path $bundle 'licenses' 'dotnet-LICENSE.txt')
    Copy-RequiredFile (Join-Path $runtimePack 'THIRD-PARTY-NOTICES.TXT') (Join-Path $bundle 'licenses' 'dotnet-THIRD-PARTY-NOTICES.txt')
    $assetsPath = Join-Path $root 'host' 'Browser' 'obj' 'project.assets.json'
    $assets = Get-Content -LiteralPath $assetsPath -Raw | ConvertFrom-Json
    foreach ($packageName in @('SkiaSharp.NativeAssets.WebAssembly', 'HarfBuzzSharp.NativeAssets.WebAssembly')) {
        $package = @($assets.libraries.PSObject.Properties | Where-Object { $_.Name -like "$packageName/*" })
        if ($package.Count -ne 1) { throw "Cannot find the restored $packageName package." }
        $packagePath = $null
        foreach ($folder in $assets.packageFolders.PSObject.Properties.Name) {
            $candidate = Join-Path $folder $package[0].Value.path
            if (Test-Path -LiteralPath $candidate -PathType Container) { $packagePath = $candidate; break }
        }
        if (-not $packagePath) { throw "Cannot locate the restored $packageName package on disk." }
        Copy-RequiredFile (Join-Path $packagePath 'LICENSE.txt') (Join-Path $bundle 'licenses' "$packageName-LICENSE.txt")
        $notice = Join-Path $packagePath 'THIRD-PARTY-NOTICES.txt'
        if (Test-Path -LiteralPath $notice -PathType Leaf) {
            Copy-Item -LiteralPath $notice -Destination (Join-Path $bundle 'licenses' "$packageName-THIRD-PARTY-NOTICES.txt")
        }
    }
    Copy-RequiredFile (Join-Path $root 'rust' 'browser' 'build-site.py') (Join-Path $bundle 'build-site.py')
    & (Join-Path $root 'rust' 'generate-sbom.ps1') -Rid $rid -Bundle $bundle `
        -ProjectAssetsJsonPath $assetsPath -ProducerPin $release.producerPin

    $files = @(Get-ChildItem -LiteralPath $bundle -File -Force -Recurse |
        ForEach-Object {
            $name = [IO.Path]::GetRelativePath($bundle, $_.FullName).Replace('\', '/')
            [ordered]@{ name = $name; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
        } | Sort-Object { $_.name } -CaseSensitive)
    $wasmOptFlags = @(($facts['WasmOptFlags'] -split '\s+') | Where-Object { $_ })
    $manifest = [ordered]@{
        schemaVersion = 1
        kind = 'rustolonia-browser-host'
        version = $version
        rid = $rid
        abiFingerprint = $abi
        rustTarget = 'wasm32-unknown-emscripten'
        emscriptenVersion = $emscriptenVersion
        dotnetRuntimeVersion = $runtimeVersion
        sourceRevision = $revision
        producerPin = [string]$release.producerPin
        link = [ordered]@{
            responseFile = 'link/emcc.rsp'
            runWasmOpt = ($facts['RunWasmOpt'] -eq 'true')
            wasmOptFlags = $wasmOptFlags
            environment = $linkEnvironment
        }
        site = [ordered]@{
            directory = 'site'
            bootConfig = '_framework/dotnet.js'
        }
        files = $files
    }
    $json = ($manifest | ConvertTo-Json -Depth 6) -replace "`r`n", "`n"
    [IO.File]::WriteAllText((Join-Path $bundle 'host-manifest.json'), "$json`n", [Text.UTF8Encoding]::new($false))

    $paths = @(Get-ChildItem -LiteralPath $bundle -File -Force -Recurse |
        ForEach-Object { [IO.Path]::GetRelativePath($bundle, $_.FullName) })
    $stagedAsset = Join-Path $staging $assetName
    New-DeterministicTarGz -SourceDirectory $bundle -RelativePaths $paths -Destination $stagedAsset `
        -Timestamp $timestamp -ExecutablePaths @('build-site.py')
    $hash = Write-AssetChecksum -Asset $stagedAsset
    [IO.File]::Move($stagedAsset, $assetPath)
    [IO.File]::Move("$stagedAsset.sha256", "$assetPath.sha256")
    Write-Host "==> $assetName ($hash)"
}
finally {
    Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
}
