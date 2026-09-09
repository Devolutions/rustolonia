#!/usr/bin/env pwsh
#Requires -Version 7.0
[CmdletBinding()]
param(
    [switch]$RunNativeSmoke
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Throws {
    param([scriptblock]$Action, [string]$MessagePattern)
    $failure = $null
    try { & $Action | Out-Null }
    catch { $failure = $_ }
    Assert-True ($null -ne $failure) "Expected an error matching '$MessagePattern'."
    Assert-True ($failure.Exception.Message -match $MessagePattern) "Unexpected error: $($failure.Exception.Message)"
}

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $root 'rust' 'package-shared.ps1')

function Test-NativeConsumer {
    param([string]$Scratch)

    if ($IsLinux -and [string]::IsNullOrWhiteSpace($env:DISPLAY)) {
        throw 'Native Linux smoke execution requires a display. Run this script with xvfb-run -a pwsh -NoProfile -File ... -RunNativeSmoke.'
    }

    $architecture = Get-CurrentRuntimeArchitecture
    $platform = if ($IsWindows) { 'win' } elseif ($IsLinux) { 'linux' } elseif ($IsMacOS) { 'osx' } else { throw 'Unsupported native smoke platform.' }
    $rid = "$platform-$architecture"
    $target = Get-RidTargetInfo -Rid $rid
    $producer = Join-Path $root 'avalonia-src'
    $consumer = Join-Path $Scratch 'native consumer'
    & (Join-Path $root 'rust' 'new-app.ps1') -Name smoke_app -Destination $consumer -ProducerRoot $producer -RustoloniaRoot $root
    $manifestPath = Join-Path $consumer 'avalonia-app.json'
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json -AsHashtable
    $manifest.rid = $rid
    $manifest.configuration = 'Release'
    $manifest.outputDirectory = "artifacts/$rid"
    $manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $manifestPath -Encoding utf8

    $previousSigner = $env:AVALONIA_RUST_SIGN_COMMAND
    try {
        Remove-Item Env:AVALONIA_RUST_SIGN_COMMAND -ErrorAction SilentlyContinue
        & (Join-Path $root 'rust' 'build-app.ps1') -ProducerRoot $producer -Manifest $manifestPath -UpdateLockFile
    }
    finally {
        if ($null -ne $previousSigner) { $env:AVALONIA_RUST_SIGN_COMMAND = $previousSigner }
        else { Remove-Item Env:AVALONIA_RUST_SIGN_COMMAND -ErrorAction SilentlyContinue }
    }

    $bundle = Join-Path $consumer 'artifacts' $rid
    $executable = Join-Path $bundle "smoke_app$($target.ExeExtension)"
    Assert-True (Test-Path -LiteralPath $executable -PathType Leaf) "Packaged executable is missing: $executable"
    Assert-True (Test-Path -LiteralPath (Join-Path $bundle "Avalonia.Host$($target.HostExtension)") -PathType Leaf) 'Packaged host is missing.'
    $inventory = Get-Content -LiteralPath (Join-Path $bundle 'sbom.cdx.json') -Raw | ConvertFrom-Json
    Assert-True (@($inventory.components | Where-Object { $_.type -eq 'library' -and $_.purl -like 'pkg:nuget/*' }).Count -gt 0) 'Packaged inventory must contain the published host dependencies.'
    foreach ($line in Get-Content -LiteralPath (Join-Path $bundle 'checksums.sha256')) {
        $expected, $file = $line -split '\s+\*', 2
        $actual = (Get-FileHash -LiteralPath (Join-Path $bundle $file) -Algorithm SHA256).Hash.ToLowerInvariant()
        Assert-True ($actual -eq $expected) "Checksum mismatch for $file"
    }

    $previousHost = $env:AVN_HOST_NATIVE_LIB
    $process = $null
    try {
        Remove-Item Env:AVN_HOST_NATIVE_LIB -ErrorAction SilentlyContinue
        $process = Start-Process -FilePath $executable -WorkingDirectory $bundle -PassThru
        Assert-True (-not $process.WaitForExit(5000)) "Packaged consumer exited during startup."
    }
    finally {
        if ($null -ne $previousHost) { $env:AVN_HOST_NATIVE_LIB = $previousHost }
        else { Remove-Item Env:AVN_HOST_NATIVE_LIB -ErrorAction SilentlyContinue }
        if ($null -ne $process) {
            try {
                if (-not $process.HasExited) {
                    Stop-Process -Id $process.Id
                    Assert-True ($process.WaitForExit(10000)) 'Packaged consumer did not stop.'
                }
            }
            finally { $process.Dispose() }
        }
    }
    Write-Host "Native consumer smoke passed for $rid."
}

$scratch = Join-Path $root 'rust' 'target' ('test-build-app-' + [guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Path $scratch | Out-Null
    $productionScripts = Get-ChildItem -LiteralPath (Join-Path $root 'rust'), (Join-Path $root 'avalonia-patches') -Filter '*.ps1' -File
    foreach ($scriptFile in $productionScripts) {
        $tokens = $null
        $parseErrors = $null
        [System.Management.Automation.Language.Parser]::ParseFile($scriptFile.FullName, [ref]$tokens, [ref]$parseErrors) | Out-Null
        $messages = @($parseErrors | ForEach-Object { $_.Message }) -join '; '
        Assert-True ($parseErrors.Count -eq 0) "Invalid script $($scriptFile.FullName): $messages"
    }

    $publishProperties = @('-p:AvaloniaProducerRoot=producer path', '-p:RustoloniaRoot=rustolonia path')
    $publishCommand = New-DotnetPublishCommand -Project 'host project.csproj' -Configuration Release -Rid win-x64 -ArtifactsPath 'artifacts path' -AdditionalProperties $publishProperties
    Assert-True ($publishCommand[0] -eq 'dotnet' -and $publishCommand[2] -eq 'host project.csproj') 'Publish argv must preserve command and spaced paths.'
    Assert-True ($publishCommand[-1] -eq $publishProperties[-1]) 'Publish argv must include the final override.'
    & {
        $assetsDirectory = Join-Path $scratch 'custom artifacts' 'obj' 'Avalonia.Host'
        New-Item -ItemType Directory -Force -Path $assetsDirectory | Out-Null
        $assetsFile = Join-Path $assetsDirectory 'project.assets.json'
        Set-Content -LiteralPath $assetsFile -Value '{"libraries":{}}'
        function dotnet {
            Assert-True ($args[0] -eq 'msbuild' -and $args -contains '-getProperty:ProjectAssetsFile') 'Restore metadata must be queried from MSBuild.'
            Assert-True ($args -contains '-p:Configuration=Release' -and $args -contains '-p:RuntimeIdentifier=win-x64') 'Metadata evaluation must match publish configuration and RID.'
            Assert-True ($args -contains '-p:AvaloniaProducerRoot=producer path' -and $args -contains '-p:RustoloniaRoot=rustolonia path') 'Metadata evaluation must preserve publish overrides.'
            Assert-True (($args -contains "-p:ArtifactsPath=$(Resolve-CallerRelativePath -PathValue 'artifacts path')") -eq $expectArtifacts) 'Metadata evaluation must preserve the optional artifacts path.'
            $global:LASTEXITCODE = 0
            $assetsFile
        }
        $expectArtifacts = $true
        $resolvedAssets = Get-PublishedProjectAssetsFile -Project 'host project.csproj' -Configuration Release -Rid win-x64 -ArtifactsPath 'artifacts path' -AdditionalProperties $publishProperties
        Assert-True ($resolvedAssets -eq $assetsFile) 'Metadata must come from the evaluated path, not host/obj.'
        $expectArtifacts = $false
        $resolvedAssets = Get-PublishedProjectAssetsFile -Project 'host project.csproj' -Configuration Release -Rid win-x64 -AdditionalProperties $publishProperties
        Assert-True ($resolvedAssets -eq $assetsFile) 'Consumer publication must also query its restore metadata.'
        $assetsFile = Join-Path $scratch 'missing.assets.json'
        Assert-Throws { Get-PublishedProjectAssetsFile -Project 'host project.csproj' -Configuration Release -Rid win-x64 -AdditionalProperties $publishProperties } 'metadata does not exist'
        $assetsFile = ''
        Assert-Throws { Get-PublishedProjectAssetsFile -Project 'host project.csproj' -Configuration Release -Rid win-x64 -AdditionalProperties $publishProperties } 'ProjectAssetsFile was empty'
    }
    $cargoCommand = New-CargoBuildCommand -ManifestPath 'app path/Cargo.toml' -PackageName example_app -TargetTriple x86_64-pc-windows-msvc -Example hello_world
    Assert-True ($cargoCommand.Count -eq 11 -and $cargoCommand[0] -eq 'cargo' -and $cargoCommand[3] -eq 'app path/Cargo.toml') 'Cargo argv must be flat and preserve spaced paths.'

    $hostProperties = New-HostPublishProperties -ProducerRoot producer -RustoloniaRoot rustolonia -Rid win-x64 -HostPlatform Win32
    Assert-True ($hostProperties -contains '-p:AvaloniaProducerRoot=producer') 'Host publish must use the declared producer.'
    Assert-True ($hostProperties -contains '-p:AvaloniaRustDeveloperTools=false') 'Normal publication must explicitly disable developer tools.'
    $diagnosticProperties = New-HostPublishProperties -ProducerRoot producer -RustoloniaRoot rustolonia -Rid win-x64 -HostPlatform Win32 -DeveloperTools
    Assert-True ($diagnosticProperties -contains '-p:AvaloniaRustDeveloperTools=true') 'Opt-in publication must enable developer tools even in Release.'
    Assert-True (@($hostProperties | Where-Object { $_ -like '-p:ObjCopyName=*' }).Count -eq 0) 'Windows publish must not set objcopy.'
    $linuxProperties = New-HostPublishProperties -ProducerRoot producer -RustoloniaRoot rustolonia -Rid linux-arm64 -HostPlatform X11 -CurrentArchitecture x64 -ObjCopyName aarch64-linux-gnu-objcopy
    Assert-True ($linuxProperties -contains '-p:ObjCopyName=aarch64-linux-gnu-objcopy') 'Linux publish must pass its cross objcopy.'
    & {
        $availableTools = @('aarch64-linux-gnu-objcopy', 'x86_64-linux-gnu-objcopy')
        function Get-Command {
            [CmdletBinding()]
            param([string]$Name)
            if ($Name -in $availableTools) { [pscustomobject]@{ Name = $Name } }
        }
        Assert-True ((Resolve-LinuxCrossObjCopyName -Rid linux-arm64 -CurrentArchitecture x64) -eq $availableTools[0]) 'ARM64 cross tool selection is incorrect.'
        Assert-True ((Resolve-LinuxCrossObjCopyName -Rid linux-x64 -CurrentArchitecture arm64) -eq $availableTools[1]) 'x64 cross tool selection is incorrect.'
        Assert-True ($null -eq (Resolve-LinuxCrossObjCopyName -Rid linux-x64 -CurrentArchitecture x64)) 'Native Linux builds must not need a cross tool.'
        $availableTools = @()
        Assert-Throws { Resolve-LinuxCrossObjCopyName -Rid linux-x64 -CurrentArchitecture arm64 } 'binutils-x86-64-linux-gnu'
    }

    $fakeProducer = Join-Path $scratch 'fake producer'
    $headerDirectory = Join-Path $fakeProducer 'nukebuild'
    New-Item -ItemType Directory -Force -Path $headerDirectory | Out-Null
    Set-Content -LiteralPath (Join-Path $headerDirectory '_build.csproj') -Value '<Project />'
    $headerCommand = New-ProducerHeaderGenerationCommand -ProducerRoot $fakeProducer
    Assert-True ($headerCommand[-1] -eq 'GenerateCppHeaders') 'Native preparation must generate COM headers.'
    $xcodeProject = Join-Path $fakeProducer 'native' 'Avalonia.Native' 'src' 'OSX' 'Avalonia.Native.OSX.xcodeproj'
    New-Item -ItemType Directory -Force -Path $xcodeProject | Out-Null
    foreach ($configuration in @('Debug', 'Release')) {
        $xcodeCommand = New-XcodeBuildCommand -ProducerRoot $fakeProducer -Rid osx-arm64 -Configuration $configuration
        Assert-True ($xcodeCommand[4] -eq $configuration -and $xcodeCommand -contains 'ARCHS=arm64') 'Xcode must use the requested configuration and architecture.'
        Assert-True ($xcodeCommand -contains "CONFIGURATION_BUILD_DIR=$(Join-Path $fakeProducer 'Build' 'Products' 'Release')") 'Xcode output must match the pinned producer layout.'
    }

    Push-Location $scratch
    try {
        Assert-True ((Resolve-CallerRelativePath -PathValue 'relative-output') -eq (Join-Path $scratch 'relative-output')) 'Output paths must follow the caller location.'
        Assert-True ((Resolve-CallerRelativePath -PathValue (Join-Path 'nested' 'output')) -eq (Join-Path $scratch 'nested' 'output')) 'Nested output paths must follow the caller location.'
    }
    finally { Pop-Location }

    $firstStaging = New-IsolatedPackageStagingRoot -OutputRoot $scratch -Rid win-x64
    $secondStaging = New-IsolatedPackageStagingRoot -OutputRoot $scratch -Rid win-x64
    Assert-True ($firstStaging -ne $secondStaging -and (Test-Path -LiteralPath $firstStaging) -and (Test-Path -LiteralPath $secondStaging)) 'Staging must not remove another active invocation.'
    $sourceBundle = Join-Path $scratch 'source-bundle'
    New-Item -ItemType Directory -Path $sourceBundle | Out-Null
    $sourceHost = Join-Path $sourceBundle 'Avalonia.Host.dll'
    Set-Content -LiteralPath $sourceHost -Value 'stub host'
    Set-Content -LiteralPath (Join-Path $sourceBundle 'libexample.so.1') -Value 'versioned native library'
    $copyBundle = Join-Path $scratch 'source-bundle-copy'
    Prepare-ArtifactBundle -BundlePath $copyBundle | Out-Null
    $metadata = Join-Path $copyBundle 'metadata.txt'
    Set-Content -LiteralPath $metadata -Value 'preserve me'
    Copy-BundleFiles -SourceDirectory $sourceBundle -DestinationDirectory $copyBundle -HostFile $sourceHost -Rid win-x64
    Assert-True ((Get-Content -LiteralPath $metadata -Raw).Trim() -eq 'preserve me') 'Copying must preserve existing metadata.'
    Assert-True (Test-Path -LiteralPath (Join-Path $copyBundle 'libexample.so.1')) 'Versioned native libraries must be copied.'
    Assert-Throws { Copy-BundleFiles -SourceDirectory $sourceBundle -DestinationDirectory $sourceBundle -HostFile $sourceHost -Rid win-x64 } 'differ|overlap|nested'
    $protectedBundle = Join-Path $scratch 'protected-bundle'
    New-Item -ItemType Directory -Path $protectedBundle | Out-Null
    Set-Content -LiteralPath (Join-Path $protectedBundle 'user.data') -Value 'preserve me'
    Assert-Throws { Prepare-ArtifactBundle -BundlePath $protectedBundle } 'Refusing'
    Assert-True (Test-Path -LiteralPath (Join-Path $protectedBundle 'user.data')) 'Refused output must preserve user data.'

    $signingRoot = Join-Path $scratch 'signing'
    New-Item -ItemType Directory -Path $signingRoot | Out-Null
    $extensionlessApp = Join-Path $signingRoot 'app'
    Set-Content -LiteralPath $extensionlessApp -Value 'app'
    Set-Content -LiteralPath (Join-Path $signingRoot 'LICENSE') -Value 'license'
    Set-Content -LiteralPath (Join-Path $signingRoot '.rustolonia-bundle-owner') -Value 'marker'
    $signerScript = Join-Path $scratch 'fake-sign.ps1'
    $signLog = Join-Path $scratch 'signatures.log'
    @'
param([Parameter(Mandatory, Position = 0)][string]$ArtifactPath)
Add-Content -LiteralPath (Join-Path $PSScriptRoot 'signatures.log') -Value $ArtifactPath
'@ | Set-Content -LiteralPath $signerScript
    Invoke-ArtifactSigning -ArtifactDirectory $signingRoot -SignCommand $signerScript -ExplicitFiles @($extensionlessApp)
    $signed = @(Get-Content -LiteralPath $signLog)
    Assert-True ($signed.Count -eq 1 -and $signed[0] -eq $extensionlessApp) 'An explicit extensionless app must be signable without any native-library matches.'
    $nativeLibrary = Join-Path $signingRoot 'libexample.so.1'
    Set-Content -LiteralPath $nativeLibrary -Value 'library'
    Invoke-ArtifactSigning -ArtifactDirectory $signingRoot -SignCommand $signerScript -ExplicitFiles @($extensionlessApp)
    $signed = @(Get-Content -LiteralPath $signLog)
    Assert-True ($signed -contains $nativeLibrary -and $signed.Count -eq 3) 'Signing must include native libraries, not LICENSE or owner markers.'
    Assert-Throws { Invoke-ArtifactSigning -ArtifactDirectory $signingRoot -SignCommand $signerScript -ExplicitFiles @('missing-app') } 'missing'
    Assert-True (@(Get-Content -LiteralPath $signLog).Count -eq 3) 'Missing signing inputs must fail before signing anything.'

    $transactionBundle = Join-Path $scratch 'transaction bundle'
    Prepare-ArtifactBundle -BundlePath $transactionBundle | Out-Null
    Set-Content -LiteralPath (Join-Path $transactionBundle 'old.bin') -Value 'last good package'
    Write-Checksums -Bundle $transactionBundle
    $originalChecksums = Get-Content -LiteralPath (Join-Path $transactionBundle 'checksums.sha256') -Raw
    foreach ($stage in @('cargo', 'signing', 'sbom', 'checksums')) {
        Assert-Throws {
            Invoke-ArtifactBundleTransaction -BundlePath $transactionBundle -Rid win-x64 -Build {
                param($workingBundle)
                Set-Content -LiteralPath (Join-Path $workingBundle 'new.bin') -Value 'incomplete replacement'
                throw "$stage failed"
            }
        } "$stage failed"
        Assert-True ((Get-Content -LiteralPath (Join-Path $transactionBundle 'checksums.sha256') -Raw) -eq $originalChecksums) "$stage failure changed the old package."
        Assert-True ((Get-Content -LiteralPath (Join-Path $transactionBundle 'old.bin') -Raw).Trim() -eq 'last good package') "$stage failure lost the old binary."
        Assert-True (-not (Test-Path -LiteralPath (Join-Path $transactionBundle 'new.bin'))) 'Partial bundles must not be published.'
    }
    & {
        function Move-ArtifactBundleDirectory {
            param([string]$LiteralPath, [string]$Destination)
            if ((Split-Path -Leaf $LiteralPath) -eq 'bundle') { throw 'simulated replacement failure' }
            [IO.Directory]::Move($LiteralPath, $Destination)
        }
        Assert-Throws {
            Invoke-ArtifactBundleTransaction -BundlePath $transactionBundle -Rid win-x64 -Build {
                param($workingBundle)
                Set-Content -LiteralPath (Join-Path $workingBundle 'new.bin') -Value 'replacement'
            }
        } 'simulated replacement failure'
    }
    Assert-True (Test-Path -LiteralPath (Join-Path $transactionBundle 'old.bin')) 'Failed replacement must restore the previous bundle.'
    Assert-Throws {
        Invoke-ArtifactBundleTransaction -BundlePath $protectedBundle -Rid win-x64 -Build { throw 'must not build' }
    } 'Refusing'
    Invoke-ArtifactBundleTransaction -BundlePath $transactionBundle -Rid win-x64 -Build {
        param($workingBundle)
        Assert-True (Test-Path -LiteralPath (Join-Path $transactionBundle 'old.bin')) 'Previous bundle must remain available while building.'
        Set-Content -LiteralPath (Join-Path $workingBundle 'new.bin') -Value 'complete replacement'
        Write-Checksums -Bundle $workingBundle
    }
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $transactionBundle 'old.bin'))) 'Successful replacement must remove stale outputs.'
    Assert-True (Test-Path -LiteralPath (Join-Path $transactionBundle 'new.bin')) 'Successful replacement must publish the new bundle.'
    Assert-True (@(Get-ChildItem -LiteralPath $scratch -Force -Filter '.win-x64.avalonia-staging.*').Count -eq 2) 'Transactions must clean their own staging without removing other invocations.'
    $uncreatedBundle = Join-Path $scratch 'uncreated bundle'
    Assert-Throws {
        Invoke-ArtifactBundleTransaction -BundlePath $uncreatedBundle -Rid win-x64 -Build { throw 'first build failed' }
    } 'first build failed'
    Assert-True (-not (Test-Path -LiteralPath $uncreatedBundle)) 'A failed first build must not expose a partial bundle.'

    $racedBundle = Join-Path $scratch 'raced bundle'
    Prepare-ArtifactBundle -BundlePath $racedBundle | Out-Null
    Set-Content -LiteralPath (Join-Path $racedBundle 'old.bin') -Value 'preserve on rollback failure'
    & {
        function Move-ArtifactBundleDirectory {
            param([string]$LiteralPath, [string]$Destination)
            if ((Split-Path -Leaf $LiteralPath) -eq 'bundle') {
                New-Item -ItemType Directory -Path $Destination | Out-Null
                Set-Content -LiteralPath (Join-Path $Destination 'user.data') -Value 'concurrent directory'
            }
            [IO.Directory]::Move($LiteralPath, $Destination)
        }
        Assert-Throws {
            Invoke-ArtifactBundleTransaction -BundlePath $racedBundle -Rid win-x64 -Build {} 3>$null
        } 'already exists|already in use'
    }
    Assert-True (Test-Path -LiteralPath (Join-Path $racedBundle 'user.data')) 'A replacement race must not overwrite concurrent user data.'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $racedBundle 'bundle'))) 'A replacement race must not nest the staged bundle.'
    $retained = @(Get-ChildItem -LiteralPath $scratch -Directory -Force -Filter '.win-x64.avalonia-staging.*' |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'previous' 'old.bin') })
    Assert-True ($retained.Count -eq 1) 'A rollback failure must retain the previous package for recovery.'
    Remove-Item -LiteralPath $retained[0].FullName -Recurse -Force

    $archive = Join-Path $scratch 'package.tar.gz'
    $downloadedBundle = Join-Path $scratch 'downloaded bundle'
    New-Item -ItemType Directory -Path $downloadedBundle | Out-Null
    if (-not $IsWindows) {
        [IO.File]::SetUnixFileMode((Join-Path $transactionBundle 'new.bin'), [IO.UnixFileMode]'UserRead,UserWrite,UserExecute,GroupRead,GroupExecute,OtherRead,OtherExecute')
    }
    tar -czf $archive -C $transactionBundle .
    tar -xzf $archive -C $downloadedBundle
    Assert-True (Test-Path -LiteralPath (Join-Path $downloadedBundle '.rustolonia-bundle-owner')) 'Archive roundtrip must preserve the hidden ownership marker.'
    foreach ($line in Get-Content -LiteralPath (Join-Path $downloadedBundle 'checksums.sha256')) {
        $expected, $file = $line -split '\s+\*', 2
        Assert-True ((Get-FileHash -LiteralPath (Join-Path $downloadedBundle $file) -Algorithm SHA256).Hash.ToLowerInvariant() -eq $expected) "Archive checksum mismatch for $file."
    }
    if (-not $IsWindows) {
        Assert-True ([IO.File]::GetUnixFileMode((Join-Path $downloadedBundle 'new.bin')) -eq [IO.File]::GetUnixFileMode((Join-Path $transactionBundle 'new.bin'))) 'Archive roundtrip must preserve executable permissions.'
    }

    $consumer = Join-Path $scratch 'external app directory' 'My App'
    $scriptOutput = & (Join-Path $root 'rust' 'new-app.ps1') -Name demo_app -Destination $consumer -ProducerRoot $fakeProducer -RustoloniaRoot $root 6>&1
    $scriptText = $scriptOutput | Out-String
    $rustoloniaPath = $root.Replace('\', '/')
    $producerPath = $fakeProducer.Replace('\', '/')
    $manifestPath = (Join-Path $consumer 'avalonia-app.json').Replace('\', '/')
    $expectedCommand = "pwsh `"$rustoloniaPath/rust/build-app.ps1`" -ProducerRoot `"$producerPath`" -Manifest `"$manifestPath`" -UpdateLockFile"
    Assert-True ($scriptText.Contains($expectedCommand)) 'Printed build arguments must preserve exact spaced paths.'
    $cargoText = Get-Content -LiteralPath (Join-Path $consumer 'Cargo.toml') -Raw
    $relativeRustolonia = [IO.Path]::GetRelativePath($consumer, $root).Replace('\', '/')
    Assert-True ($cargoText.Contains("path = `"$relativeRustolonia/rust/avalonia`"")) 'Cargo must reference Rustolonia relatively rather than the producer.'
    $projectText = Get-Content -LiteralPath (Join-Path $consumer 'managed' 'Consumer.Presentation.csproj') -Raw
    $managedRoot = Join-Path $consumer 'managed'
    Assert-True ($projectText.Contains([IO.Path]::GetRelativePath($managedRoot, $fakeProducer).Replace('\', '/')) -and
        $projectText.Contains([IO.Path]::GetRelativePath($managedRoot, $root).Replace('\', '/')) -and
        $projectText.Contains('$(MSBuildThisFileDirectory)') -and -not $projectText.Contains('__MANAGED_')) 'Presentation roots must be project-relative.'
    Assert-True ((Get-Content (Join-Path $consumer 'global.json') -Raw) -eq (Get-Content (Join-Path $root 'global.json') -Raw)) 'Consumer must inherit the supported SDK policy.'
    $scaffoldManifest = Get-Content (Join-Path $consumer 'avalonia-app.json') -Raw | ConvertFrom-Json
    Assert-True ($scaffoldManifest.rid -eq (Get-DefaultConsumerRid) -and $scaffoldManifest.outputDirectory -eq "artifacts/$($scaffoldManifest.rid)") 'Scaffold must default to host OS/architecture.'
    Assert-True (-not (Test-Path (Join-Path $consumer 'Cargo.lock'))) 'Scaffold must leave initial dependency resolution explicit.'
    Assert-Throws { & (Join-Path $root 'rust' 'new-app.ps1') -Name demo_app -Destination $consumer -ProducerRoot $fakeProducer -RustoloniaRoot $root } 'already exists'
    $invalidDestination = Join-Path $scratch 'invalid-consumer'
    Assert-Throws { & (Join-Path $root 'rust' 'new-app.ps1') -Name demo_app -Destination $invalidDestination -ProducerRoot (Join-Path $scratch 'missing-producer') -RustoloniaRoot $root } 'does not exist'
    Assert-True (-not (Test-Path -LiteralPath $invalidDestination)) 'Invalid roots must not create an output directory.'
    Push-Location $scratch
    try {
        & (Join-Path $root 'rust' 'new-app.ps1') -Name relative_app -Destination 'relative app' -ProducerRoot $fakeProducer -RustoloniaRoot $root -Rid linux-arm64 | Out-Null
        Assert-True (Test-Path -LiteralPath (Join-Path $scratch 'relative app' 'Cargo.toml')) 'Relative scaffolds must use the caller location.'
        Assert-True ((Get-Content (Join-Path $scratch 'relative app' 'avalonia-app.json') -Raw | ConvertFrom-Json).rid -eq 'linux-arm64') 'Explicit RID must override host default.'
    }
    finally { Pop-Location }

    & {
        $release = Get-Content (Join-Path $root 'rust' 'release-manifest.json') -Raw | ConvertFrom-Json
        $sourceCase = 'valid'
        $treeWrites = @{ Value = 0 }
        function git {
            $global:LASTEXITCODE = 0
            if ($args -contains 'rev-parse') {
                if ($sourceCase -eq 'revision') { return 'wrong-revision' }
                return $release.producerPin
            }
            if ($args -contains 'apply' -and $sourceCase -eq 'patch') { $global:LASTEXITCODE = 1 }
            if ($args -contains 'write-tree') {
                $treeWrites.Value++
                if ($sourceCase -eq 'source' -and $treeWrites.Value % 2 -eq 0) { return 'unexpected-tree' }
                return 'expected-tree'
            }
            if ($args -contains 'submodule' -and $sourceCase -eq 'submodule') { return '-012345 external/missing' }
            if ($args -contains 'foreach' -and $sourceCase -eq 'dirty-submodule') { return 'external/dirty' }
        }
        Assert-ConsumerSource -ProducerRoot $fakeProducer -RustoloniaRoot $root
        foreach ($case in @(
                @{ Name = 'revision'; Error = 'Producer revision must be' },
                @{ Name = 'patch'; Error = 'Cannot construct the expected producer tree' },
                @{ Name = 'source'; Error = 'changes beyond the declared Rustolonia patches' },
                @{ Name = 'submodule'; Error = 'submodules are missing or at the wrong revision' },
                @{ Name = 'dirty-submodule'; Error = 'submodules contain local changes' }
            )) {
            $sourceCase = $case.Name
            $treeWrites.Value = 0
            Assert-Throws { Assert-ConsumerSource -ProducerRoot $fakeProducer -RustoloniaRoot $root } $case.Error
        }
        $sourceCase = 'valid'
        $badReleaseRoot = Join-Path $scratch 'bad release'
        New-Item -ItemType Directory (Join-Path $badReleaseRoot 'rust') -Force | Out-Null
        $release.patches[0].path = 'tampered.patch'
        Set-Content (Join-Path $badReleaseRoot 'tampered.patch') 'tampered'
        $release | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $badReleaseRoot 'rust' 'release-manifest.json')
        Assert-Throws { Assert-ConsumerSource -ProducerRoot $fakeProducer -RustoloniaRoot $badReleaseRoot } 'patch hash mismatch'
        function dotnet { $global:LASTEXITCODE = 0; '9.0.100' }
        Assert-Throws { Assert-ConsumerTools -Rid (Get-DefaultConsumerRid) -RustoloniaRoot $root -ConsumerRoot $consumer } 'Unsupported .NET SDK'
    }

    # A tiny local dependency exercises real Cargo lock and relocation behavior without network access.
    $layout = Join-Path $scratch 'portable layout'
    $portableRoot = Join-Path $layout 'rustolonia'
    $portableCrate = Join-Path $portableRoot 'rust' 'avalonia'
    New-Item -ItemType Directory (Join-Path $portableCrate 'src') -Force | Out-Null
    Copy-Item (Join-Path $root 'global.json') (Join-Path $portableRoot 'global.json')
    Set-Content (Join-Path $portableCrate 'Cargo.toml') "[package]`nname = `"avalonia`"`nversion = `"0.1.0`"`nedition = `"2021`""
    Set-Content (Join-Path $portableCrate 'src' 'lib.rs') ''
    $portableConsumer = Join-Path $layout 'consumer'
    & (Join-Path $root 'rust' 'new-app.ps1') -Name portable_app -Destination $portableConsumer -ProducerRoot $portableRoot -RustoloniaRoot $portableRoot | Out-Null
    $portableManifest = Join-Path $portableConsumer 'Cargo.toml'
    Assert-Throws { Invoke-Logged -Command @('cargo', 'metadata', '--offline', '--locked', '--format-version', '1', '--manifest-path', $portableManifest) } 'exit code'
    Invoke-Logged -Command @('cargo', 'metadata', '--offline', '--format-version', '1', '--manifest-path', $portableManifest) | Out-Null
    $lockHash = (Get-FileHash (Join-Path $portableConsumer 'Cargo.lock')).Hash
    $movedLayout = Join-Path $scratch 'moved layout'
    Move-Item -LiteralPath $layout -Destination $movedLayout
    $portableConsumer = Join-Path $movedLayout 'consumer'
    $portableManifest = Join-Path $portableConsumer 'Cargo.toml'
    $relocatedMetadata = Invoke-Logged -Command @('cargo', 'metadata', '--offline', '--locked', '--format-version', '1', '--manifest-path', $portableManifest) | ConvertFrom-Json
    Assert-True ($relocatedMetadata.packages.Count -eq 2) 'Relative Cargo references must resolve after relocating the entire layout.'
    $relocatedProject = [xml](Get-Content (Join-Path $portableConsumer 'managed' 'Consumer.Presentation.csproj') -Raw)
    $relocatedManagedDirectory = (Join-Path $portableConsumer 'managed') + [IO.Path]::DirectorySeparatorChar
    $relocatedRoot = $relocatedProject.Project.PropertyGroup.RustoloniaRoot.InnerText.Replace('$(MSBuildThisFileDirectory)', $relocatedManagedDirectory)
    Assert-True ([IO.Path]::GetFullPath($relocatedRoot) -eq (Join-Path $movedLayout 'rustolonia')) 'Managed references must remain project-relative after relocation.'
    foreach ($property in @('AvaloniaProducerRoot', 'RustoloniaRoot')) {
        Assert-True ($relocatedProject.Project.PropertyGroup.$property.Condition -eq "'`$($property)' == ''") "Explicit $property overrides must not be overwritten."
    }
    Assert-True ((Get-FileHash (Join-Path $portableConsumer 'Cargo.lock')).Hash -eq $lockHash) 'Locked metadata must not change the dependency lock.'
    Set-Content (Join-Path $movedLayout 'rustolonia' 'rust' 'avalonia' 'Cargo.toml') "[package]`nname = `"avalonia`"`nversion = `"0.2.0`"`nedition = `"2021`""
    Assert-Throws { Invoke-Logged -Command @('cargo', 'metadata', '--offline', '--locked', '--format-version', '1', '--manifest-path', $portableManifest) } 'exit code'

    $badManifest = Get-Content -LiteralPath (Join-Path $consumer 'avalonia-app.json') -Raw | ConvertFrom-Json -AsHashtable
    $badManifest.unexpected = $true
    $badManifestPath = Join-Path $consumer 'bad.json'
    $badManifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $badManifestPath
    Assert-Throws { & (Join-Path $root 'rust' 'build-app.ps1') -ProducerRoot $fakeProducer -Manifest $badManifestPath } 'unknown field'

    $outsideNoticeDirectory = Join-Path $scratch 'outside notices'
    New-Item -ItemType Directory -Force -Path $outsideNoticeDirectory | Out-Null
    $outsideNotice = Join-Path $outsideNoticeDirectory 'OUTSIDE.txt'
    Set-Content -LiteralPath $outsideNotice -Value 'must not be packaged'
    Set-Content -LiteralPath (Join-Path $consumer 'sbom.cdx.json') -Value 'reserved'
    Set-Content -LiteralPath (Join-Path $consumer 'SBOM.CDX.JSON') -Value 'reserved case variant'
    $firstNoticeDirectory = Join-Path $consumer 'first notices'
    $secondNoticeDirectory = Join-Path $consumer 'second notices'
    New-Item -ItemType Directory -Force -Path $firstNoticeDirectory, $secondNoticeDirectory | Out-Null
    Set-Content -LiteralPath (Join-Path $firstNoticeDirectory 'NOTICE.txt') -Value 'first'
    Set-Content -LiteralPath (Join-Path $secondNoticeDirectory 'notice.txt') -Value 'second'
    $noticeValidationCases = @(
        @{ Value = @($outsideNotice); Error = 'must be relative' },
        @{ Value = @([IO.Path]::GetRelativePath($consumer, $outsideNotice)); Error = 'must remain within' },
        @{ Value = @("NOTICE`nINJECTED"); Error = 'must not contain line terminators' },
        @{ Value = @('SBOM.CDX.JSON'); Error = 'reserved bundle filename' },
        @{ Value = @('first notices/NOTICE.txt', 'second notices/notice.txt'); Error = 'unique filenames' }
    )
    foreach ($case in $noticeValidationCases) {
        $badManifest = Get-Content -LiteralPath (Join-Path $consumer 'avalonia-app.json') -Raw | ConvertFrom-Json -AsHashtable
        $badManifest.noticeFiles = $case.Value
        $badManifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $badManifestPath
        Assert-Throws { & (Join-Path $root 'rust' 'build-app.ps1') -ProducerRoot $fakeProducer -Manifest $badManifestPath } $case.Error
    }
    $linkedNoticeDirectory = Join-Path $consumer 'linked notices'
    try {
        $linkType = if ($IsWindows) { 'Junction' } else { 'SymbolicLink' }
        New-Item -ItemType $linkType -Path $linkedNoticeDirectory -Target $outsideNoticeDirectory -ErrorAction Stop | Out-Null
        $badManifest = Get-Content -LiteralPath (Join-Path $consumer 'avalonia-app.json') -Raw | ConvertFrom-Json -AsHashtable
        $badManifest.noticeFiles = @('linked notices/OUTSIDE.txt')
        $badManifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $badManifestPath
        Assert-Throws { & (Join-Path $root 'rust' 'build-app.ps1') -ProducerRoot $fakeProducer -Manifest $badManifestPath } 'must not traverse'
    }
    finally {
        if (Test-Path -LiteralPath $linkedNoticeDirectory) {
            Remove-Item -LiteralPath $linkedNoticeDirectory -Force
        }
    }

    $invalidFields = @(
        @{ Field = 'version'; Value = '1' },
        @{ Field = 'version'; Value = '2026-01-01T00:00:00Z' },
        @{ Field = 'version'; Value = $true },
        @{ Field = 'version'; Value = 1.4 },
        @{ Field = 'version'; Value = $null },
        @{ Field = 'version'; Value = @(1) },
        @{ Field = 'packageName'; Value = '' },
        @{ Field = 'packageName'; Value = $null },
        @{ Field = 'packageName'; Value = 123 },
        @{ Field = 'binary'; Value = '' },
        @{ Field = 'binary'; Value = $null },
        @{ Field = 'binary'; Value = @('demo_app') },
        @{ Field = 'noticeFiles'; Value = 'LICENSE.app' },
        @{ Field = 'noticeFiles'; Value = @('') },
        @{ Field = 'rid'; Value = 'WIN-X64' },
        @{ Field = 'rid'; Value = @('win-x64') },
        @{ Field = 'configuration'; Value = 'release' },
        @{ Field = 'configuration'; Value = @('Release') },
        @{ Field = 'generatedRegistryFile'; Value = 'generated/CustomRegistry.g.cs' },
        @{ Field = 'generatedRegistryFile'; Value = 'generated/rustviewregistry.g.cs' }
    )
    foreach ($field in @('presentationProject', 'viewModelIr', 'generatedAdaptersDirectory', 'generatedRegistryFile', 'generatedRustFile', 'generatedContractFile', 'cargoManifest', 'outputDirectory')) {
        foreach ($value in @($null, '', 123, @('path'), @{ path = 'value' })) {
            $invalidFields += @{ Field = $field; Value = $value }
        }
    }
    foreach ($case in $invalidFields) {
        $badManifest = Get-Content -LiteralPath (Join-Path $consumer 'avalonia-app.json') -Raw | ConvertFrom-Json -AsHashtable
        $badManifest[$case.Field] = $case.Value
        $badManifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $badManifestPath
        Assert-Throws { & (Join-Path $root 'rust' 'build-app.ps1') -ProducerRoot $fakeProducer -Manifest $badManifestPath } "Invalid consumer manifest: $($case.Field)"
    }
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $consumer '.avalonia'))) 'Invalid manifests must not create build outputs.'

    & {
        $nativeOs = if ($IsWindows) { 'win' } elseif ($IsLinux) { 'linux' } else { 'osx' }
        $nativeRid = "$nativeOs-$(Get-CurrentRuntimeArchitecture)"
        $nativeTarget = Get-RidTargetInfo -Rid $nativeRid
        $mockArtifacts = Join-Path $scratch 'mock dotnet artifacts'
        $mockAssets = Join-Path $scratch 'mock.assets.json'
        Set-Content -LiteralPath $mockAssets -Value '{"libraries":{}}'
        Set-Content -LiteralPath (Join-Path $fakeProducer 'licence.md') -Value 'producer license'
        $dbusDirectory = Join-Path $fakeProducer 'external' 'Avalonia.DBus' 'src' 'Avalonia.DBus'
        New-Item -ItemType Directory -Force -Path $dbusDirectory | Out-Null
        Set-Content -LiteralPath (Join-Path $dbusDirectory 'Avalonia.DBus.csproj') -Value '<Project />'
        $failingSigner = Join-Path $scratch 'failing-sign.ps1'
        Set-Content -LiteralPath $failingSigner -Value "throw 'injected signing failure'"
        function rustup { $global:LASTEXITCODE = 0; $nativeTarget.Triple }
        function xcodebuild {}
        function git {
            $global:LASTEXITCODE = 0
            if ($args -contains 'rev-parse') { (Get-Content (Join-Path $root 'rust' 'release-manifest.json') -Raw | ConvertFrom-Json).producerPin }
        }
        function dotnet {
            $global:LASTEXITCODE = 0
            if ($args[0] -eq '--version') { return '10.0.400' }
            if ($args -contains '-getProperty:ProjectAssetsFile') {
                Assert-True ($args -contains "-p:AvaloniaRustDeveloperTools=$expectedDeveloperTools") 'SBOM metadata evaluation must preserve diagnostics mode.'
                return $mockAssets
            }
            if ($args[0] -ne 'publish') { return }
            Assert-True ($args -contains "-p:AvaloniaRustDeveloperTools=$expectedDeveloperTools") 'Publish must preserve diagnostics mode.'
            $publishOverride = @($args | Where-Object { $_ -like '-p:PublishDir=*' })
            $publish = if ($publishOverride.Count) { $publishOverride[0].Substring('-p:PublishDir='.Length) }
                else { Join-Path $mockArtifacts 'publish' 'Avalonia.Host' "release_$nativeRid" }
            New-Item -ItemType Directory -Force -Path $publish | Out-Null
            Set-Content -LiteralPath (Join-Path $publish "Avalonia.Host$($nativeTarget.HostExtension)") -Value 'new host'
            if ($IsMacOS) { Set-Content -LiteralPath (Join-Path $publish 'libAvaloniaNative.dylib') -Value 'native library' }
        }
        function cargo {
            $global:LASTEXITCODE = 0
            if ($args[0] -eq 'fmt') { throw 'Consumer builds must not format handwritten Rust.' }
            if ($args[0] -eq 'metadata') {
                Assert-True (($args -contains '--locked') -eq $expectLockedMetadata) 'Only explicit lockfile updates may resolve without --locked.'
                $workspaceJson = ConvertTo-Json -InputObject $consumer -Compress
                return '{"workspace_root":' + $workspaceJson + ',"packages":[{"features":{"USB":[],"usb":[]}}]}'
            }
            if ($args -contains '--bin') { Assert-True ($args -contains '--locked') 'Consumer Cargo builds must be locked.' }
            if ($failureStage -eq 'cargo') { throw 'injected cargo failure' }
            $output = Join-Path $env:CARGO_TARGET_DIR $nativeTarget.Triple 'release'
            $name = 'demo_app'
            if ($args -contains '--example') {
                $output = Join-Path $output 'examples'
                $name = 'hello_world'
            }
            New-Item -ItemType Directory -Force -Path $output | Out-Null
            Set-Content -LiteralPath (Join-Path $output "$name$($nativeTarget.ExeExtension)") -Value 'new executable'
        }
        $savedEnvironment = @{}
        foreach ($key in @('AVN_DOTNET_ARTIFACTS', 'CARGO_TARGET_DIR', 'AVALONIA_RUST_SIGN_COMMAND', 'AVN_PACKAGE_SKIP_CARGO_BUILD')) {
            $savedEnvironment[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
        }
        try {
            $env:AVN_DOTNET_ARTIFACTS = $mockArtifacts
            $env:CARGO_TARGET_DIR = Join-Path $scratch 'mock cargo artifacts'
            $env:AVALONIA_RUST_SIGN_COMMAND = $failingSigner
            Remove-Item Env:AVN_PACKAGE_SKIP_CARGO_BUILD -ErrorAction SilentlyContinue
            $sampleOutput = Join-Path $scratch 'sample package output'
            $sampleBundle = Join-Path $sampleOutput $nativeRid
            $consumerBundle = Join-Path $consumer 'package output'
            foreach ($output in @($sampleBundle, $consumerBundle)) {
                Prepare-ArtifactBundle -BundlePath $output | Out-Null
                Set-Content -LiteralPath (Join-Path $output 'previous.bin') -Value 'last good release'
            }
            $mockManifest = Get-Content -LiteralPath (Join-Path $consumer 'avalonia-app.json') -Raw | ConvertFrom-Json -AsHashtable
            $mockManifest.rid = $nativeRid
            $mockManifest.configuration = 'Release'
            $mockManifest.outputDirectory = $consumerBundle
            Set-Content -LiteralPath (Join-Path $consumer 'LICENSE.app') -Value 'application license'
            $mockManifest.noticeFiles = @('LICENSE.app')
            $mockManifestPath = Join-Path $consumer 'mock-build.json'
            $mockManifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $mockManifestPath
            $expectLockedMetadata = $true
            $expectedDeveloperTools = 'false'
            foreach ($failureStage in @('cargo', 'signing')) {
                Assert-Throws {
                    & (Join-Path $root 'rust' 'package.ps1') -Rid $nativeRid -ProducerRoot $fakeProducer -OutputRoot $sampleOutput
                } "injected $failureStage failure"
                Assert-Throws {
                    & (Join-Path $root 'rust' 'build-app.ps1') -ProducerRoot $fakeProducer -Manifest $mockManifestPath -SkipGenerate
                    $expectLockedMetadata = $false
                    & (Join-Path $root 'rust' 'build-app.ps1') -ProducerRoot $fakeProducer -Manifest $mockManifestPath -SkipGenerate -UpdateLockFile
                } "injected $failureStage failure"
                foreach ($output in @($sampleBundle, $consumerBundle)) {
                    Assert-True ((Get-Content -LiteralPath (Join-Path $output 'previous.bin') -Raw).Trim() -eq 'last good release') "$failureStage failure in a packaging entry point lost the previous bundle."
                    Assert-True (@(Get-ChildItem -LiteralPath $output -Force).Count -eq 2) 'A failed packaging entry point exposed partial output.'
                }
            }
            $failureStage = 'none'
            Remove-Item Env:AVALONIA_RUST_SIGN_COMMAND
            & (Join-Path $root 'rust' 'package.ps1') -Rid $nativeRid -ProducerRoot $fakeProducer -OutputRoot $sampleOutput
            & (Join-Path $root 'rust' 'build-app.ps1') -ProducerRoot $fakeProducer -Manifest $mockManifestPath -SkipGenerate
            Assert-True ((Get-Content -LiteralPath (Join-Path $consumerBundle 'LICENSE.app') -Raw).Trim() -eq 'application license') 'Consumer bundles must include application-specific notice files.'
            $releaseChecksum = (Get-FileHash -LiteralPath (Join-Path $consumerBundle 'checksums.sha256')).Hash
            $expectedDeveloperTools = 'true'
            & (Join-Path $root 'rust' 'build-app.ps1') -ProducerRoot $fakeProducer -Manifest $mockManifestPath -SkipGenerate -DeveloperTools
            Assert-True (Test-Path -LiteralPath (Join-Path "$consumerBundle-devtools" 'checksums.sha256')) 'Diagnostics must publish to a separate bundle.'
            Assert-True ((Get-FileHash -LiteralPath (Join-Path $consumerBundle 'checksums.sha256')).Hash -eq $releaseChecksum) 'Diagnostics publication must preserve the normal release bundle.'
            foreach ($output in @($sampleBundle, $consumerBundle)) {
                Assert-True (-not (Test-Path -LiteralPath (Join-Path $output 'previous.bin'))) 'Successful entry points must replace stale bundles.'
                Assert-True (Test-Path -LiteralPath (Join-Path $output 'sbom.cdx.json')) 'Successful entry points must publish an SBOM.'
                foreach ($line in Get-Content -LiteralPath (Join-Path $output 'checksums.sha256')) {
                    $expected, $file = $line -split '\s+\*', 2
                    Assert-True ((Get-FileHash -LiteralPath (Join-Path $output $file) -Algorithm SHA256).Hash.ToLowerInvariant() -eq $expected) "Published checksum mismatch for $file."
                }
            }
        }
        finally {
            foreach ($key in $savedEnvironment.Keys) {
                if ($null -eq $savedEnvironment[$key]) { Remove-Item "Env:$key" -ErrorAction SilentlyContinue }
                else { [Environment]::SetEnvironmentVariable($key, $savedEnvironment[$key], 'Process') }
            }
        }
    }

    $inventory = Join-Path $scratch 'inventory'
    New-Item -ItemType Directory -Path $inventory | Out-Null
    [IO.File]::WriteAllBytes((Join-Path $inventory 'z.bin'), [byte[]](0x7A))
    [IO.File]::WriteAllBytes((Join-Path $inventory 'a.bin'), [byte[]](0x61))
    Set-Content -LiteralPath (Join-Path $inventory '.hidden-metadata') -Value 'inventory metadata'
    & (Join-Path $root 'rust' 'generate-sbom.ps1') -Rid win-x64 -Bundle $inventory
    $first = [IO.File]::ReadAllBytes((Join-Path $inventory 'sbom.cdx.json'))
    & (Join-Path $root 'rust' 'generate-sbom.ps1') -Rid win-x64 -Bundle $inventory
    $second = [IO.File]::ReadAllBytes((Join-Path $inventory 'sbom.cdx.json'))
    Assert-True ([Linq.Enumerable]::SequenceEqual($first, $second)) 'Delivery inventory must be repeatable.'
    $sbom = Get-Content -LiteralPath (Join-Path $inventory 'sbom.cdx.json') -Raw | ConvertFrom-Json
    Assert-True ($sbom.components.name -contains '.hidden-metadata') 'Delivery inventory must include hidden metadata on every OS.'
    Write-Checksums -Bundle $inventory
    Assert-True (@(Get-Content -LiteralPath (Join-Path $inventory 'checksums.sha256') | Where-Object { $_.EndsWith('*.hidden-metadata') }).Count -eq 1) 'Checksums must include hidden metadata on every OS.'

    $depsInventory = Join-Path $scratch 'deps-inventory'
    New-Item -ItemType Directory -Path $depsInventory | Out-Null
    [IO.File]::WriteAllBytes((Join-Path $depsInventory 'a.bin'), [byte[]](0x61))
    $fakeAssets = Join-Path $scratch 'project.assets.json'
    Set-Content -LiteralPath $fakeAssets -Value '{"libraries":{"Some.Package/1.2.3":{"type":"package"},"local-project/1.0.0":{"type":"project"}}}'
    $fakeCargoLock = Join-Path $scratch 'Cargo.lock'
    Set-Content -LiteralPath $fakeCargoLock -Value @'
[[package]]
name = "third-party-crate"
version = "0.4.1"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "avalonia"
version = "0.1.0"
'@
    & (Join-Path $root 'rust' 'generate-sbom.ps1') -Rid win-x64 -Bundle $depsInventory -CargoLockPath $fakeCargoLock -ProjectAssetsJsonPath $fakeAssets -ProducerPin 'deadbeef'
    $depsSbom = Get-Content -LiteralPath (Join-Path $depsInventory 'sbom.cdx.json') -Raw | ConvertFrom-Json
    Assert-True (@($depsSbom.components | Where-Object { $_.type -eq 'library' -and $_.purl -eq 'pkg:nuget/Some.Package@1.2.3' }).Count -eq 1) 'SBOM must include resolved NuGet package dependencies.'
    Assert-True (@($depsSbom.components | Where-Object { $_.type -eq 'library' -and $_.name -eq 'local-project' }).Count -eq 0) 'SBOM must not list local project references as third-party dependencies.'
    Assert-True (@($depsSbom.components | Where-Object { $_.type -eq 'library' -and $_.purl -eq 'pkg:cargo/third-party-crate@0.4.1' }).Count -eq 1) 'SBOM must include resolved Cargo dependencies.'
    Assert-True (@($depsSbom.components | Where-Object { $_.type -eq 'library' -and $_.name -eq 'avalonia' -and $_.version -eq '0.1.0' }).Count -eq 0) 'SBOM must not list workspace-local crates without a [source] as third-party dependencies.'
    Assert-True (@($depsSbom.metadata.properties | Where-Object { $_.name -eq 'avalonia:producer-pin' -and $_.value -eq 'deadbeef' }).Count -eq 1) 'SBOM must record the producer pin used to build the bundle.'
    & (Join-Path $root 'rust' 'generate-sbom.ps1') -Rid win-x64 -Bundle $depsInventory -CargoLockPath (Join-Path $scratch 'missing.lock') -ProjectAssetsJsonPath (Join-Path $scratch 'missing.assets.json')
    $missingSbom = Get-Content -LiteralPath (Join-Path $depsInventory 'sbom.cdx.json') -Raw | ConvertFrom-Json
    foreach ($ecosystem in @('nuget', 'cargo')) {
        $source = @($missingSbom.metadata.properties | Where-Object { $_.name -eq "avalonia:$ecosystem-dependency-source" })
        Assert-True ($source.Count -eq 1 -and $source[0].value -like 'unavailable:*does not exist') 'Missing supplied dependency files must not be reported as available.'
    }

    if ($RunNativeSmoke) {
        $nativeTempRoot = [IO.Path]::GetTempPath()
        if ($IsMacOS) {
            # /var is a symlink to /private/var. MSBuild canonicalizes the project
            # directory before resolving references, so create the external smoke
            # consumer under the physical path as well.
            Push-Location -LiteralPath $nativeTempRoot
            try { $nativeTempRoot = (& /bin/pwd -P).Trim() }
            finally { Pop-Location }
        }
        $nativeScratch = Join-Path $nativeTempRoot ('rustolonia-native-' + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $nativeScratch | Out-Null
        try { Test-NativeConsumer -Scratch $nativeScratch }
        finally {
            if (Test-Path -LiteralPath $nativeScratch) { Remove-Item -LiteralPath $nativeScratch -Recurse -Force }
        }
    }
}
finally {
    if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
}

Write-Host 'Helper and scaffold regressions passed.'
