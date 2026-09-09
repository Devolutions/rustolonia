#!/usr/bin/env pwsh
#Requires -Version 7.0

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Invoke-Logged {
    param(
        [Parameter(Mandatory)][ValidateNotNullOrEmpty()][string[]]$Command,
        [string]$WorkingDirectory
    )

    Write-Host "==> $($Command -join ' ')"
    $executable = $Command[0]
    $arguments = if ($Command.Length -gt 1) { $Command[1..($Command.Length - 1)] } else { @() }
    if ($WorkingDirectory) {
        Push-Location $WorkingDirectory
        try { & $executable @arguments }
        finally { Pop-Location }
    }
    else { & $executable @arguments }
}

$script:RidTargets = @{
    'win-x64'     = @{ Triple = 'x86_64-pc-windows-msvc'; Platform = 'Win32'; HostExtension = '.dll'; ExeExtension = '.exe'; OS = 'Windows'; Arch = 'x64' }
    'win-arm64'   = @{ Triple = 'aarch64-pc-windows-msvc'; Platform = 'Win32'; HostExtension = '.dll'; ExeExtension = '.exe'; OS = 'Windows'; Arch = 'arm64' }
    'linux-x64'   = @{ Triple = 'x86_64-unknown-linux-gnu'; Platform = 'X11'; HostExtension = '.so'; ExeExtension = ''; OS = 'Linux'; Arch = 'x64' }
    'linux-arm64' = @{ Triple = 'aarch64-unknown-linux-gnu'; Platform = 'X11'; HostExtension = '.so'; ExeExtension = ''; OS = 'Linux'; Arch = 'arm64' }
    'osx-x64'     = @{ Triple = 'x86_64-apple-darwin'; Platform = 'OSX'; HostExtension = '.dylib'; ExeExtension = ''; OS = 'macOS'; Arch = 'x64' }
    'osx-arm64'   = @{ Triple = 'aarch64-apple-darwin'; Platform = 'OSX'; HostExtension = '.dylib'; ExeExtension = ''; OS = 'macOS'; Arch = 'arm64' }
}

function Get-RidTargetInfo {
    param(
        [Parameter(Mandatory)][string]$Rid
    )

    if (-not $script:RidTargets.ContainsKey($Rid)) {
        throw "Unsupported RID '$Rid'. Supported values: $((($script:RidTargets.Keys | Sort-Object) -join ', '))"
    }

    return $script:RidTargets[$Rid]
}

function New-DotnetPublishCommand {
    param(
        [Parameter(Mandatory)][string]$Project,
        [Parameter(Mandatory)][string]$Configuration,
        [Parameter(Mandatory)][string]$Rid,
        [Parameter(Mandatory)][string]$ArtifactsPath,
        [string[]]$AdditionalProperties = @()
    )

    $command = @(
        'dotnet', 'publish', $Project,
        '-c', $Configuration,
        '-r', $Rid,
        '--artifacts-path', $ArtifactsPath
    ) + @($AdditionalProperties)

    return $command
}

function Get-PublishedProjectAssetsFile {
    param(
        [Parameter(Mandatory)][string]$Project,
        [Parameter(Mandatory)][string]$Configuration,
        [Parameter(Mandatory)][string]$Rid,
        [string]$ArtifactsPath,
        [string[]]$AdditionalProperties = @()
    )

    # Evaluate with the same properties as publication, including custom output layouts.
    $arguments = @(
        'msbuild', $Project, '-nologo', '-verbosity:quiet',
        '-getProperty:ProjectAssetsFile',
        "-p:Configuration=$Configuration", "-p:RuntimeIdentifier=$Rid"
    )
    if (-not [string]::IsNullOrWhiteSpace($ArtifactsPath)) {
        $arguments += "-p:ArtifactsPath=$(Resolve-CallerRelativePath -PathValue $ArtifactsPath)"
    }
    $arguments += $AdditionalProperties
    $output = & dotnet @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Could not resolve publish restore metadata for '$Project' (exit code $LASTEXITCODE)."
    }
    $path = ($output -join "`n").Trim()
    if ([string]::IsNullOrWhiteSpace($path)) {
        throw "ProjectAssetsFile was empty for '$Project'."
    }
    if (-not [IO.Path]::IsPathRooted($path)) {
        $path = Join-Path (Split-Path -Parent (Resolve-CallerRelativePath -PathValue $Project)) $path
    }
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Publish restore metadata does not exist: $path"
    }
    return (Resolve-Path -LiteralPath $path).Path
}

function New-ProducerHeaderGenerationCommand {
    param(
        [Parameter(Mandatory)][string]$ProducerRoot
    )

    $project = Join-Path $ProducerRoot 'nukebuild' '_build.csproj'
    if (-not (Test-Path -LiteralPath $project -PathType Leaf)) {
        throw "Producer root is missing the NUKE build project: $project"
    }

    return @('dotnet', 'run', '--project', $project, '--target', 'GenerateCppHeaders')
}

function New-XcodeBuildCommand {
    param(
        [Parameter(Mandatory)][string]$ProducerRoot,
        [Parameter(Mandatory)][string]$Rid,
        [ValidateSet('Debug', 'Release')]
        [string]$Configuration = 'Release'
    )

    $target = Get-RidTargetInfo -Rid $Rid
    if ($target.Platform -ne 'OSX') {
        throw "$Rid is not an macOS target."
    }

    $xcodeArch = if ($target.Arch -eq 'x64') { 'x86_64' } else { 'arm64' }
    $projectPath = Join-Path $ProducerRoot 'native' 'Avalonia.Native' 'src' 'OSX' 'Avalonia.Native.OSX.xcodeproj'
    if (-not (Test-Path -LiteralPath $projectPath -PathType Container)) {
        throw "macOS native bootstrap project is missing: $projectPath"
    }

    # The pinned producer expects the native dylib under Build/Products/Release regardless of the xcode configuration.
    $products = Join-Path $ProducerRoot 'Build' 'Products' 'Release'
    return @('xcodebuild', '-project', $projectPath, '-configuration', $Configuration, "ARCHS=$xcodeArch", "CONFIGURATION_BUILD_DIR=$products")
}

function Resolve-LinuxCrossObjCopyName {
    param(
        [Parameter(Mandatory)][string]$Rid,
        [string]$CurrentArchitecture = (Get-CurrentRuntimeArchitecture)
    )

    $target = Get-RidTargetInfo -Rid $Rid
    if ($target.OS -ne 'Linux') {
        return $null
    }

    if ($target.Arch -eq $CurrentArchitecture) {
        return $null
    }

    $name = switch ($target.Arch) {
        'x64' { 'x86_64-linux-gnu-objcopy' }
        'arm64' { 'aarch64-linux-gnu-objcopy' }
        default { throw "Unsupported Linux cross-build target architecture: $($target.Arch)" }
    }

    if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
        $hint = if ($target.Arch -eq 'x64') { 'sudo apt-get install binutils-x86-64-linux-gnu' } else { 'sudo apt-get install gcc-aarch64-linux-gnu' }
        throw "Cross-building $Rid requires $name on PATH. Install it with: $hint"
    }

    return $name
}

function New-HostPublishProperties {
    param(
        [Parameter(Mandatory)][string]$ProducerRoot,
        [Parameter(Mandatory)][string]$RustoloniaRoot,
        [Parameter(Mandatory)][string]$Rid,
        [Parameter(Mandatory)][string]$HostPlatform,
        [string]$CurrentArchitecture = (Get-CurrentRuntimeArchitecture),
        [string]$ObjCopyName
    )

    $properties = @(
        "-p:AvaloniaProducerRoot=$ProducerRoot",
        "-p:RustoloniaRoot=$RustoloniaRoot",
        "-p:AvaloniaRustHostPlatform=$HostPlatform"
    )

    if (-not [string]::IsNullOrWhiteSpace($ObjCopyName)) {
        $properties += "-p:ObjCopyName=$ObjCopyName"
        return $properties
    }

    if ($IsLinux) {
        $crossObjCopy = Resolve-LinuxCrossObjCopyName -Rid $Rid -CurrentArchitecture $CurrentArchitecture
        if (-not [string]::IsNullOrWhiteSpace($crossObjCopy)) {
            $properties += "-p:ObjCopyName=$crossObjCopy"
        }
    }

    return $properties
}

function New-CargoBuildCommand {
    param(
        [Parameter(Mandatory)][string]$ManifestPath,
        [Parameter(Mandatory)][string]$PackageName,
        [Parameter(Mandatory)][string]$TargetTriple,
        [string]$Configuration = 'Release',
        [string]$Example,
        [string]$BinaryName
    )

    $args = @('build', '--manifest-path', $ManifestPath, '-p', $PackageName)
    if ($PSBoundParameters.ContainsKey('Example')) {
        $args += @('--example', $Example)
    }
    elseif ($PSBoundParameters.ContainsKey('BinaryName')) {
        $args += @('--bin', $BinaryName)
    }
    $args += @('--target', $TargetTriple)
    if ($Configuration -eq 'Release') {
        $args += '--release'
    }
    return @('cargo') + $args
}

function Get-CurrentRuntimeArchitecture {
    switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
        'X64' { return 'x64' }
        'Arm64' { return 'arm64' }
        default { throw "Unsupported CPU architecture: $([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)" }
    }
}

function Get-DefaultConsumerRid {
    $platform = if ($IsWindows) { 'win' } elseif ($IsLinux) { 'linux' } elseif ($IsMacOS) { 'osx' }
        else { throw 'Unsupported consumer platform.' }
    return "$platform-$(Get-CurrentRuntimeArchitecture)"
}

function Assert-ConsumerSource {
    param([string]$ProducerRoot, [string]$RustoloniaRoot)

    $release = Get-Content -LiteralPath (Join-Path $RustoloniaRoot 'rust' 'release-manifest.json') -Raw | ConvertFrom-Json
    $revision = & git -C $ProducerRoot rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $revision -ne $release.producerPin) {
        throw "Producer revision must be $($release.producerPin); found '$revision'. Initialize the pinned producer before building."
    }
    foreach ($patch in $release.patches) {
        $path = Join-Path $RustoloniaRoot $patch.path
        if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $patch.sha256) {
            throw "Producer patch hash mismatch: $path"
        }
    }

    $previousIndex = $env:GIT_INDEX_FILE
    $expectedIndex = [IO.Path]::GetTempFileName()
    $actualIndex = [IO.Path]::GetTempFileName()
    Remove-Item -LiteralPath $expectedIndex, $actualIndex
    try {
        $env:GIT_INDEX_FILE = $expectedIndex
        & git -C $ProducerRoot read-tree HEAD
        if ($LASTEXITCODE -ne 0) { throw 'Cannot initialize the expected producer tree.' }
        foreach ($patch in $release.patches) {
            $path = Join-Path $RustoloniaRoot $patch.path
            & git -C $ProducerRoot apply --cached --whitespace=nowarn $path
            if ($LASTEXITCODE -ne 0) { throw "Cannot construct the expected producer tree from patch: $path" }
        }
        $expectedTree = & git -C $ProducerRoot write-tree
        if ($LASTEXITCODE -ne 0) { throw 'Cannot record the expected producer tree.' }

        $env:GIT_INDEX_FILE = $actualIndex
        & git -C $ProducerRoot read-tree HEAD
        if ($LASTEXITCODE -ne 0) { throw 'Cannot initialize the actual producer tree.' }
        & git -C $ProducerRoot add --all
        if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect producer source changes.' }
        $actualTree = & git -C $ProducerRoot write-tree
        if ($LASTEXITCODE -ne 0) { throw 'Cannot record the actual producer tree.' }
        if ($actualTree -ne $expectedTree) {
            throw 'Producer source contains changes beyond the declared Rustolonia patches. Restore the pinned producer and reapply the declared patches.'
        }
    }
    finally {
        if ($null -eq $previousIndex) { Remove-Item Env:GIT_INDEX_FILE -ErrorAction SilentlyContinue }
        else { $env:GIT_INDEX_FILE = $previousIndex }
        Remove-Item -LiteralPath $expectedIndex, "$expectedIndex.lock", $actualIndex, "$actualIndex.lock" -Force -ErrorAction SilentlyContinue
    }

    $submodules = & git -C $ProducerRoot submodule status --recursive
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect producer submodule revisions.' }
    if (@($submodules | Where-Object { $_ -match '^[-+U]' }).Count -gt 0) {
        throw "Producer submodules are missing or at the wrong revision. Run: git -C `"$ProducerRoot`" submodule update --init --recursive"
    }
    $dirtySubmodules = & git -C $ProducerRoot submodule foreach --recursive --quiet 'if test -n "$(git status --porcelain --untracked-files=all)"; then printf "%s\n" "$displaypath"; fi'
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect producer submodule worktrees.' }
    if (@($dirtySubmodules).Count -gt 0) {
        throw "Producer submodules contain local changes: $($dirtySubmodules -join ', '). Restore them to their pinned revisions."
    }
}

function Assert-ConsumerTools {
    param([string]$Rid, [string]$RustoloniaRoot, [string]$ConsumerRoot)

    Assert-RidMatchesHost -Rid $Rid
    foreach ($tool in @('git', 'dotnet', 'cargo', 'rustc', 'rustup')) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Required consumer build tool '$tool' is missing." }
    }
    $sdk = (Get-Content -LiteralPath (Join-Path $RustoloniaRoot 'global.json') -Raw | ConvertFrom-Json).sdk
    $minimum = [version]$sdk.version
    foreach ($directory in @($RustoloniaRoot, $ConsumerRoot)) {
        Push-Location $directory
        try {
            $selected = & dotnet --version
            if ($LASTEXITCODE -ne 0) { throw "Cannot select the supported .NET SDK in $directory." }
            $version = $null
            if (-not [version]::TryParse($selected, [ref]$version) -or
                $version.Major -ne $minimum.Major -or $version.Minor -ne $minimum.Minor -or $version -lt $minimum) {
                throw "Unsupported .NET SDK '$selected' in $directory. Use Rustolonia global.json ($($sdk.version), $($sdk.rollForward))."
            }
        }
        finally { Pop-Location }
    }
    $target = Get-RidTargetInfo -Rid $Rid
    $installed = & rustup target list --installed
    if ($LASTEXITCODE -ne 0 -or $installed -notcontains $target.Triple) {
        throw "Rust target '$($target.Triple)' is required. Run: rustup target add $($target.Triple)"
    }
    if ($IsWindows) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio' 'Installer' 'vswhere.exe'
        if (-not (Test-Path -LiteralPath $vswhere)) { throw 'NativeAOT requires Visual Studio with Desktop development with C++ and a Windows SDK (vswhere not found).' }
        $component = if ($target.Arch -eq 'arm64') { 'Microsoft.VisualStudio.Component.VC.Tools.ARM64' } else { 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64' }
        $installation = & $vswhere -latest -products '*' -requires $component -property installationPath
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($installation)) { throw "Install Visual Studio C++ tools for $($target.Arch) and a Windows SDK." }
        $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits' '10' 'Lib'
        if (-not (Test-Path -LiteralPath $kits) -or
            @(Get-ChildItem -LiteralPath $kits -Directory | Where-Object { Test-Path (Join-Path $_.FullName 'um' $target.Arch 'kernel32.lib') }).Count -eq 0) {
            throw "Install the Windows SDK libraries for $($target.Arch)."
        }
    }
    else {
        $tools = if ($IsMacOS) { @('xcodebuild', 'xcrun', 'clang') } else { @('clang', 'cc', 'objcopy', 'pkg-config') }
        foreach ($tool in $tools) {
            if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "NativeAOT requires '$tool' for $Rid." }
        }
        if ($IsLinux) {
            & pkg-config --exists zlib
            if ($LASTEXITCODE -ne 0) { throw 'NativeAOT requires zlib development files (zlib1g-dev or zlib-devel).' }
            $null = Resolve-LinuxCrossObjCopyName -Rid $Rid -CurrentArchitecture (Get-CurrentRuntimeArchitecture)
        }
        else {
            & xcrun --find clang
            if ($LASTEXITCODE -ne 0) { throw 'Select an installed Xcode toolchain with xcode-select.' }
        }
    }
}

function Assert-RidMatchesHost {
    param(
        [Parameter(Mandatory)][string]$Rid
    )

    $target = Get-RidTargetInfo -Rid $Rid
    if ($IsWindows -and $target.OS -ne 'Windows') { throw "$Rid packaging must run on Windows." }
    if ($IsLinux -and $target.OS -ne 'Linux') { throw "$Rid packaging must run on Linux." }
    if ($IsMacOS -and $target.OS -ne 'macOS') { throw "$Rid packaging must run on macOS." }

    $runnerArch = Get-CurrentRuntimeArchitecture
    if ($target.Arch -ne $runnerArch) {
        Write-Warning "$Rid is being cross-built on a $runnerArch runner. The packaged sample cannot be smoke-launched here."
    }
}

function Ensure-RidPrerequisites {
    param(
        [Parameter(Mandatory)][string]$Rid,
        [Parameter(Mandatory)][string]$ProducerRoot,
        [ValidateSet('Debug', 'Release')]
        [string]$Configuration = 'Release'
    )

    $target = Get-RidTargetInfo -Rid $Rid
    Assert-RidMatchesHost -Rid $Rid

    if ($target.Platform -eq 'X11') {
        $dbusProject = Join-Path $ProducerRoot 'external' 'Avalonia.DBus' 'src' 'Avalonia.DBus' 'Avalonia.DBus.csproj'
        if (-not (Test-Path -LiteralPath $dbusProject -PathType Leaf)) {
            throw 'Initialize Linux sources with: git submodule update --init external/Avalonia.DBus'
        }
    }

    if ($target.Platform -eq 'OSX') {
        $headerCommand = New-ProducerHeaderGenerationCommand -ProducerRoot $ProducerRoot
        Invoke-Logged -Command $headerCommand -WorkingDirectory $ProducerRoot
        $xcodeBuild = New-XcodeBuildCommand -ProducerRoot $ProducerRoot -Rid $Rid -Configuration $Configuration
        Invoke-Logged -Command $xcodeBuild -WorkingDirectory $ProducerRoot
    }

    $installed = rustup target list --installed
    if ($installed -notcontains $target.Triple) {
        throw "Rust target '$($target.Triple)' required for RID '$Rid' is missing. Install it with: rustup target add $($target.Triple)"
    }
}

function Set-WindowsStaticCrt {
    param(
        [Parameter(Mandatory)][string]$Triple
    )

    if (-not $Triple.EndsWith('-pc-windows-msvc')) { return }

    $key = "CARGO_TARGET_$($Triple.ToUpperInvariant().Replace('-', '_'))_RUSTFLAGS"
    $required = '-C target-feature=+crt-static'
    $current = [Environment]::GetEnvironmentVariable($key, 'Process')
    if ([string]::IsNullOrWhiteSpace($current)) { $current = '' }
    if ($current -notlike "*$required*") {
        [Environment]::SetEnvironmentVariable($key, "$($current.Trim()) $required".Trim(), 'Process')
    }
}

function Resolve-CallerRelativePath {
    param(
        [Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$PathValue
    )

    return [System.IO.Path]::GetFullPath(
        $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($PathValue)
    )
}

function Invoke-ArtifactSigning {
    param(
        [Parameter(Mandatory)][string]$ArtifactDirectory,
        [string]$SignCommand,
        [string[]]$ExplicitFiles = @()
    )

    if ([string]::IsNullOrWhiteSpace($SignCommand)) {
        Write-Host 'AVALONIA_RUST_SIGN_COMMAND is not set; skipping signing. Set it to a trusted signing wrapper; it receives each artifact path as its only argument. This script never downloads a signing tool.'
        return
    }

    if (-not (Test-Path -LiteralPath $SignCommand -PathType Leaf)) {
        throw 'AVALONIA_RUST_SIGN_COMMAND must be the path to a signing wrapper executable or script.'
    }

    $missingExplicit = @()
    $explicitArtifacts = @(
        foreach ($candidate in $ExplicitFiles) {
            if ([string]::IsNullOrWhiteSpace($candidate)) {
                throw 'Explicit signing inputs must name nonempty artifact paths.'
            }
            $resolved = if ([System.IO.Path]::IsPathRooted($candidate)) { $candidate } else { Join-Path $ArtifactDirectory $candidate }
            if (Test-Path -LiteralPath $resolved -PathType Leaf) { $resolved }
            else { $missingExplicit += $resolved }
        }
    )

    if ($missingExplicit.Count -gt 0) {
        throw "Signing inputs are missing from '$ArtifactDirectory': $($missingExplicit -join ', '). Check the explicit artifact paths and ensure they exist before packaging."
    }

    $bundleArtifacts = @(Get-ChildItem -LiteralPath $ArtifactDirectory -File | Where-Object {
        $_.Extension -in @('.dll', '.exe', '.so', '.dylib') -or
        $_.Name -like '*.so.*' -or
        $_.Name -like '*.dylib' -or
        $_.Name -like '*.dll' -or
        $_.Name -like '*.exe'
    })
    $bundleArtifactNames = @()
    if ($bundleArtifacts.Count -gt 0) {
        $bundleArtifactNames = @($bundleArtifacts | ForEach-Object { $_.FullName })
    }

    $toSign = @()
    foreach ($artifact in @($explicitArtifacts + $bundleArtifactNames)) {
        if ([string]::IsNullOrWhiteSpace($artifact)) { continue }
        $path = [System.IO.Path]::GetFullPath($artifact)
        if ($toSign -notcontains $path) { $toSign += $path }
    }

    if ($toSign.Count -eq 0) {
        Write-Host '==> No executable or native artifacts to sign in the bundle.'
        return
    }

    Write-Host '==> Signing executable artifacts with AVALONIA_RUST_SIGN_COMMAND'
    foreach ($artifact in $toSign) {
        Write-Host "    signing $([System.IO.Path]::GetFileName($artifact))"
        & $SignCommand $artifact
    }
}

function Write-Checksums {
    param(
        [Parameter(Mandatory)][string]$Bundle
    )

    $lines = @(Get-ChildItem -LiteralPath $Bundle -File -Force |
        Where-Object { $_.Name -ne 'checksums.sha256' } |
        Sort-Object Name |
        ForEach-Object {
            $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            "$hash *$($_.Name)"
        })

    [System.IO.File]::WriteAllLines((Join-Path $Bundle 'checksums.sha256'), $lines, [System.Text.UTF8Encoding]::new($false))
}

function Assert-ArtifactBundleDestination {
    param(
        [Parameter(Mandatory)][string]$BundlePath
    )

    $resolvedBundle = Resolve-CallerRelativePath -PathValue $BundlePath
    if ([System.IO.Path]::GetPathRoot($resolvedBundle) -eq $resolvedBundle) {
        throw 'A filesystem root cannot be used as an artifact bundle.'
    }
    if ((Test-Path -LiteralPath $resolvedBundle) -and -not (Test-Path -LiteralPath $resolvedBundle -PathType Container)) {
        throw "Bundle path '$resolvedBundle' exists but is not a directory. Refusing to replace a file with a package bundle."
    }

    if (-not (Test-Path -LiteralPath $resolvedBundle -PathType Container)) { return }
    if ((Get-Item -LiteralPath $resolvedBundle -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "Bundle directory '$resolvedBundle' is a link. Refusing to replace user data."
    }
    $ownerMarker = Join-Path $resolvedBundle '.rustolonia-bundle-owner'
    if (Test-Path -LiteralPath $ownerMarker -PathType Leaf) {
        if ((Get-Item -LiteralPath $ownerMarker -Force).Attributes -band [IO.FileAttributes]::ReparsePoint -or
            (Get-Content -LiteralPath $ownerMarker -Raw).Trim() -ne 'Rustolonia-owned bundle marker') {
            throw "Invalid bundle ownership marker in '$resolvedBundle'. Refusing to replace user data."
        }
    }
    elseif (@(Get-ChildItem -LiteralPath $resolvedBundle -Force).Count -gt 0) {
        throw "Bundle directory '$resolvedBundle' already exists and is not an empty or Rustolonia-owned package bundle. Refusing to replace user data."
    }
}

function Prepare-ArtifactBundle {
    param([Parameter(Mandatory)][string]$BundlePath)

    $resolvedBundle = Resolve-CallerRelativePath -PathValue $BundlePath
    Assert-ArtifactBundleDestination -BundlePath $resolvedBundle
    if (-not (Test-Path -LiteralPath $resolvedBundle -PathType Container)) {
        $parent = Split-Path -Parent $resolvedBundle
        if ($parent -and -not (Test-Path -LiteralPath $parent -PathType Container)) {
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
        }
        New-Item -ItemType Directory -Force -Path $resolvedBundle | Out-Null
    }

    $ownerMarker = Join-Path $resolvedBundle '.rustolonia-bundle-owner'
    if (Test-Path -LiteralPath $ownerMarker -PathType Leaf) {
        if ((Get-Content -LiteralPath $ownerMarker -Raw).Trim() -ne 'Rustolonia-owned bundle marker') {
            throw "Invalid bundle ownership marker in '$resolvedBundle'. Refusing to replace user data."
        }
        Get-ChildItem -LiteralPath $resolvedBundle -Force | Where-Object { $_.FullName -ne $ownerMarker } |
            Remove-Item -Recurse -Force -ErrorAction Stop
        return $resolvedBundle
    }

    $existingChildren = @(Get-ChildItem -LiteralPath $resolvedBundle -Force -ErrorAction Stop)
    if ($existingChildren.Count -gt 0) {
        throw "Bundle directory '$resolvedBundle' already exists and is not an empty or Rustolonia-owned package bundle. Refusing to replace user data."
    }

    Set-Content -LiteralPath $ownerMarker -Value 'Rustolonia-owned bundle marker' -Encoding utf8
    return $resolvedBundle
}

function Move-ArtifactBundleDirectory {
    param(
        [Parameter(Mandatory)][string]$LiteralPath,
        [Parameter(Mandatory)][string]$Destination
    )

    # Unlike Move-Item, this cannot nest the bundle inside a concurrently created directory.
    [IO.Directory]::Move($LiteralPath, $Destination)
}

function Invoke-ArtifactBundleTransaction {
    param(
        [Parameter(Mandatory)][string]$BundlePath,
        [Parameter(Mandatory)][string]$Rid,
        [Parameter(Mandatory)][scriptblock]$Build
    )

    $destination = Resolve-CallerRelativePath -PathValue $BundlePath
    Assert-ArtifactBundleDestination -BundlePath $destination
    $transactionRoot = New-IsolatedPackageStagingRoot -OutputRoot (Split-Path -Parent $destination) -Rid $Rid
    $stagedBundle = Join-Path $transactionRoot 'bundle'
    $backup = Join-Path $transactionRoot 'previous'
    $retainBackup = $false
    try {
        Prepare-ArtifactBundle -BundlePath $stagedBundle | Out-Null
        & $Build $stagedBundle
        # Validate again before replacing: building/signing can take a long time.
        Assert-ArtifactBundleDestination -BundlePath $destination
        if (Test-Path -LiteralPath $destination) {
            Move-ArtifactBundleDirectory -LiteralPath $destination -Destination $backup
        }
        try {
            Move-ArtifactBundleDirectory -LiteralPath $stagedBundle -Destination $destination
        }
        catch {
            if (Test-Path -LiteralPath $backup) {
                $retainBackup = $true
                Move-ArtifactBundleDirectory -LiteralPath $backup -Destination $destination
                $retainBackup = $false
            }
            throw
        }
    }
    finally {
        # If rollback itself failed, retain the backup for manual recovery.
        if ($retainBackup -or ((Test-Path -LiteralPath $backup) -and -not (Test-Path -LiteralPath $destination))) {
            Write-Warning "Previous bundle retained for recovery at $backup"
        }
        elseif (Test-Path -LiteralPath $transactionRoot) {
            Remove-Item -LiteralPath $transactionRoot -Recurse -Force
        }
    }
}

function Copy-BundleFiles {
    param(
        [Parameter(Mandatory)][string]$SourceDirectory,
        [Parameter(Mandatory)][string]$DestinationDirectory,
        [Parameter(Mandatory)][string]$HostFile,
        [Parameter(Mandatory)][string]$Rid
    )

    $resolvedSource = (Resolve-Path -LiteralPath $SourceDirectory).Path
    $resolvedDestination = Resolve-CallerRelativePath -PathValue $DestinationDirectory
    $comparison = if ($IsWindows) { [StringComparison]::OrdinalIgnoreCase } else { [StringComparison]::Ordinal }
    $separator = [IO.Path]::DirectorySeparatorChar
    $sourcePrefix = $resolvedSource.TrimEnd($separator) + $separator
    $destinationPrefix = $resolvedDestination.TrimEnd($separator) + $separator
    if ($resolvedSource.Equals($resolvedDestination, $comparison) -or
        $resolvedDestination.StartsWith($sourcePrefix, $comparison) -or
        $resolvedSource.StartsWith($destinationPrefix, $comparison)) {
        throw 'Bundle source and destination must not overlap.'
    }

    if (-not (Test-Path -LiteralPath $HostFile -PathType Leaf)) {
        throw "NativeAOT host is missing: $HostFile"
    }
    $ownerMarker = Join-Path $resolvedDestination '.rustolonia-bundle-owner'
    if (-not (Test-Path -LiteralPath $ownerMarker -PathType Leaf)) {
        throw "Bundle directory '$resolvedDestination' must be prepared before copying."
    }

    $target = Get-RidTargetInfo -Rid $Rid
    if ($target.Platform -eq 'OSX' -and -not (Test-Path -LiteralPath (Join-Path $resolvedSource 'libAvaloniaNative.dylib') -PathType Leaf)) {
        throw 'libAvaloniaNative.dylib was not published with the macOS host.'
    }
    Copy-Item -LiteralPath $HostFile -Destination $resolvedDestination

    $artifactFiles = Get-ChildItem -LiteralPath $resolvedSource -File | Where-Object {
        $_.FullName -ne $HostFile -and (
            $_.Extension -in @('.dll', '.so', '.dylib') -or
            $_.Name -like '*.so.*'
        )
    }

    foreach ($artifact in $artifactFiles) {
        Copy-Item -LiteralPath $artifact.FullName -Destination $resolvedDestination
    }
}

function Copy-BundleNotices {
    param(
        [Parameter(Mandatory)][string]$ProducerRoot,
        [Parameter(Mandatory)][string]$RustoloniaRoot,
        [Parameter(Mandatory)][string]$DestinationDirectory
    )

    Copy-Item -LiteralPath (Join-Path $ProducerRoot 'licence.md') -Destination $DestinationDirectory
    foreach ($name in @('LICENSE', 'THIRD-PARTY-NOTICES.txt')) {
        Copy-Item -LiteralPath (Join-Path $RustoloniaRoot $name) -Destination $DestinationDirectory
    }
}

function New-IsolatedPackageStagingRoot {
    param(
        [Parameter(Mandatory)][string]$OutputRoot,
        [Parameter(Mandatory)][string]$Rid
    )

    $OutputRoot = Resolve-CallerRelativePath -PathValue $OutputRoot
    $null = Get-RidTargetInfo -Rid $Rid
    if (-not (Test-Path -LiteralPath $OutputRoot -PathType Container)) {
        New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
    }

    $stagingRoot = Join-Path $OutputRoot (".${Rid}.avalonia-staging." + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $stagingRoot | Out-Null
    Set-Content -LiteralPath (Join-Path $stagingRoot '.rustolonia-staging-owner') -Value "rustolonia staging for $Rid" -Encoding utf8
    return $stagingRoot
}
