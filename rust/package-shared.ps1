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
    'win-x64'     = @{ Triple = 'x86_64-pc-windows-msvc'; Platform = 'Win32'; HostExtension = '.dll'; HostFileName = 'rustolonia_host.dll'; ExeExtension = '.exe'; OS = 'Windows'; Arch = 'x64' }
    'win-arm64'   = @{ Triple = 'aarch64-pc-windows-msvc'; Platform = 'Win32'; HostExtension = '.dll'; HostFileName = 'rustolonia_host.dll'; ExeExtension = '.exe'; OS = 'Windows'; Arch = 'arm64' }
    'linux-x64'   = @{ Triple = 'x86_64-unknown-linux-gnu'; Platform = 'X11'; HostExtension = '.so'; HostFileName = 'librustolonia_host.so'; ExeExtension = ''; OS = 'Linux'; Arch = 'x64' }
    'linux-arm64' = @{ Triple = 'aarch64-unknown-linux-gnu'; Platform = 'X11'; HostExtension = '.so'; HostFileName = 'librustolonia_host.so'; ExeExtension = ''; OS = 'Linux'; Arch = 'arm64' }
    'linux-musl-x64'   = @{ Triple = 'x86_64-unknown-linux-musl'; Platform = 'X11'; HostExtension = '.so'; HostFileName = 'librustolonia_host.so'; ExeExtension = ''; OS = 'Linux'; Arch = 'x64' }
    'linux-musl-arm64' = @{ Triple = 'aarch64-unknown-linux-musl'; Platform = 'X11'; HostExtension = '.so'; HostFileName = 'librustolonia_host.so'; ExeExtension = ''; OS = 'Linux'; Arch = 'arm64' }
    'osx-x64'     = @{ Triple = 'x86_64-apple-darwin'; Platform = 'OSX'; HostExtension = '.dylib'; HostFileName = 'librustolonia_host.dylib'; ExeExtension = ''; OS = 'macOS'; Arch = 'x64' }
    'osx-arm64'   = @{ Triple = 'aarch64-apple-darwin'; Platform = 'OSX'; HostExtension = '.dylib'; HostFileName = 'librustolonia_host.dylib'; ExeExtension = ''; OS = 'macOS'; Arch = 'arm64' }
}

$script:MuslStaticHostRids = @('linux-musl-x64', 'linux-musl-arm64')
$script:SharedHostRids = @($script:RidTargets.Keys | Where-Object { $_ -cnotin $script:MuslStaticHostRids })
$script:IosHostRids = @('ios-arm64', 'iossimulator-arm64', 'iossimulator-x64')
# The browser relink kit (rust/package-browser-host.ps1): one for every browser.
$script:BrowserHostRid = 'browser-wasm'

function Get-RidTargetInfo {
    param(
        [Parameter(Mandatory)][string]$Rid,
        [ValidateSet('X11', 'Drm')][string]$LinuxBackend = 'X11'
    )

    if (-not $script:RidTargets.ContainsKey($Rid)) {
        throw "Unsupported RID '$Rid'. Supported values: $((($script:RidTargets.Keys | Sort-Object) -join ', '))"
    }

    $target = $script:RidTargets[$Rid].Clone()
    # NativeLib=Static output of host/Avalonia.Host.csproj (StageRustoloniaStaticHost).
    $target.StaticHostFileName = if ($target.OS -eq 'Windows') { 'rustolonia_host.lib' } else { 'librustolonia_host.a' }
    if ($LinuxBackend -eq 'Drm') {
        if ($target.OS -ne 'Linux') {
            throw 'The Drm backend requires a Linux RID.'
        }
        if ($Rid -cin $script:MuslStaticHostRids) {
            throw 'The Drm backend requires a glibc Linux RID.'
        }
        $target.Platform = 'LinuxDrm'
    }
    return $target
}

function Assert-RidLinkage {
    param([string]$Rid, [string]$Linkage)
    if ($Rid -cin $script:MuslStaticHostRids -and $Linkage -ne 'Static') {
        throw "The $Rid host is available only with static linkage."
    }
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

    $project = Join-Path $ProducerRoot 'native' 'Avalonia.Native' 'Avalonia.Native.macOS.proj'
    if (-not (Test-Path -LiteralPath $project -PathType Leaf)) {
        throw "Producer root is missing the native header project: $project"
    }

    return @('dotnet', 'build', $project, '-t:GenerateMicroComItems', '-p:BuildAvaloniaNativeXcodeProject=false')
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
        [switch]$DeveloperTools,
        [string]$CurrentArchitecture = (Get-CurrentRuntimeArchitecture),
        [string]$ObjCopyName
    )

    $properties = @(
        "-p:AvaloniaProducerRoot=$ProducerRoot",
        "-p:RustoloniaRoot=$RustoloniaRoot",
        "-p:AvaloniaRustHostPlatform=$HostPlatform",
        "-p:AvaloniaRustDeveloperTools=$($DeveloperTools.IsPresent.ToString().ToLowerInvariant())"
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
        [string]$BinaryName,
        [string[]]$Features = @()
    )

    $args = @('build', '--manifest-path', $ManifestPath, '-p', $PackageName)
    $Features = @($Features | Where-Object { $_ })
    if ($Features.Count -gt 0) {
        $args += @('--features', ($Features -join ','))
    }
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

function Get-StaticHostCargoFeatures {
    param(
        [Parameter(Mandatory)][hashtable]$Metadata,
        [Parameter(Mandatory)][string]$PackageName
    )

    $packages = @($Metadata.packages | Where-Object {
        $_.name -ceq $PackageName -and $_.id -in $Metadata.workspace_members
    })
    if ($packages.Count -ne 1) {
        throw "Cannot find consumer package '$PackageName' in the Cargo workspace."
    }
    $aliases = @($packages[0].dependencies | Where-Object {
        $_.name -ceq 'rustolonia' -and $null -eq $_.kind
    } | ForEach-Object {
        if ($_.rename) { $_.rename } else { $_.name }
    } | Select-Object -Unique)
    if ($aliases.Count -eq 0) {
        throw "Static consumer package '$PackageName' must depend directly on rustolonia."
    }
    return @($aliases | ForEach-Object { "$_/static-host" })
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

function Initialize-LocalProducerSubmodule {
    param(
        [Parameter(Mandatory)][string]$RustoloniaRoot,
        [Parameter(Mandatory)][string]$ProducerRoot
    )

    $rootPath = Resolve-CallerRelativePath $RustoloniaRoot
    $producerPath = Resolve-CallerRelativePath $ProducerRoot
    $expectedProducerPath = [IO.Path]::GetFullPath((Join-Path $rootPath 'avalonia-src'))
    if (-not [String]::Equals($producerPath, $expectedProducerPath, [StringComparison]::OrdinalIgnoreCase)) {
        return
    }

    $expectedRevision = & git -C $rootPath rev-parse 'HEAD:avalonia-src' 2>$null
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($expectedRevision)) {
        throw "Cannot determine the pinned avalonia-src revision from $rootPath."
    }

    $actualRevision = if (Test-Path -LiteralPath $producerPath -PathType Container) {
        & git -C $producerPath rev-parse HEAD 2>$null
    }
    else {
        ''
    }
    if ($LASTEXITCODE -ne 0 -or $actualRevision -ne $expectedRevision) {
        Write-Host "==> Initializing avalonia-src at $expectedRevision"
        & git -C $rootPath submodule update --init --recursive -- avalonia-src
        if ($LASTEXITCODE -ne 0) {
            throw 'Cannot initialize avalonia-src at the pinned producer revision.'
        }
    }

    $actualRevision = & git -C $producerPath rev-parse HEAD 2>$null
    if ($LASTEXITCODE -ne 0 -or $actualRevision -ne $expectedRevision) {
        throw "avalonia-src must be at $expectedRevision; found '$actualRevision'."
    }
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
        [Parameter(Mandatory)][string]$Rid,
        [switch]$Static
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
    if (-not $Static -and $target.Platform -eq 'OSX' -and -not (Test-Path -LiteralPath (Join-Path $resolvedSource 'libAvaloniaNative.dylib') -PathType Leaf)) {
        throw 'libAvaloniaNative.dylib was not published with the macOS host.'
    }
    Copy-Item -LiteralPath $HostFile -Destination $resolvedDestination

    # A static host also carries the NativeAOT runtime libraries and the
    # link directives rustolonia-sys reads (rustolonia-host-link.txt).
    $staticExtensions = if (-not $Static) { @() } elseif ($target.OS -eq 'Windows') { @('.lib') } else { @('.a') }
    if ($Static -and -not (Test-Path -LiteralPath (Join-Path $resolvedSource 'rustolonia-host-link.txt') -PathType Leaf)) {
        throw 'rustolonia-host-link.txt was not published with the static host.'
    }
    $artifactFiles = Get-ChildItem -LiteralPath $resolvedSource -File | Where-Object {
        $_.FullName -ne $HostFile -and (
            ($Static -and ($_.Extension -in $staticExtensions -or $_.Name -eq 'rustolonia-host-link.txt')) -or
            (-not $Static -and ($_.Extension -in @('.dll', '.so', '.dylib') -or $_.Name -like '*.so.*'))
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

function Assert-LinuxMuslStaticProvenance {
    param([string]$Rid, [string]$Directory)

    $sources = Join-Path $Directory 'STATIC-LINUX-MUSL-SOURCES.tsv'
    $pinned = Join-Path $PSScriptRoot 'linux-musl-static-sources.tsv'
    $provenance = Join-Path $Directory 'STATIC-LINUX-PROVENANCE.tsv'
    $notices = Join-Path $Directory 'STATIC-LINUX-NOTICES.txt'
    foreach ($path in @($sources, $pinned, $provenance, $notices)) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing static musl metadata: $path" }
    }
    $sourceHash = (Get-FileHash -LiteralPath $sources -Algorithm SHA256).Hash
    if ($sourceHash -ne (Get-FileHash -LiteralPath $pinned -Algorithm SHA256).Hash) {
        throw 'Static musl source metadata does not match the pinned source manifest.'
    }
    $pins = @{}
    foreach ($line in Get-Content -LiteralPath $pinned) {
        if ([string]::IsNullOrWhiteSpace($line) -or $line.StartsWith('#')) { continue }
        $fields = $line -split "`t"
        if ($fields.Count -ne 5 -or $fields[0] -notmatch '^lib[A-Za-z0-9_+.-]+\.a$' -or
            $fields[4] -notmatch '^[0-9a-f]{128}$' -or $pins.ContainsKey($fields[0])) {
            throw "Invalid pinned musl source metadata: $line"
        }
        $pins[$fields[0]] = $fields
    }
    $architecture = if ($Rid -eq 'linux-musl-x64') { 'x86_64' } else { 'aarch64' }
    $seen = @{}
    $noticeHash = $null
    $recordedSourceHash = $null
    foreach ($line in Get-Content -LiteralPath $provenance) {
        if ($line -match '^# notices-sha256\t([0-9a-f]{64})$') {
            if ($noticeHash) { throw 'Duplicate static musl notice hash.' }
            $noticeHash = $Matches[1]
            continue
        }
        if ($line -match '^# sources-sha256\t([0-9a-f]{64})$') {
            if ($recordedSourceHash) { throw 'Duplicate static musl source hash.' }
            $recordedSourceHash = $Matches[1]
            continue
        }
        if ([string]::IsNullOrWhiteSpace($line) -or $line.StartsWith('#')) { continue }
        $fields = $line -split "`t"
        if ($fields.Count -ne 5 -or -not $pins.ContainsKey($fields[0]) -or $seen.ContainsKey($fields[0])) {
            throw "Invalid static musl package provenance: $line"
        }
        $pin = $pins[$fields[0]]
        $distribution = if ($pin[1].StartsWith('xorg-')) { 'xorg' } else { 'alpine' }
        if ($fields[1] -cne "$($pin[1]):$architecture" -or $fields[2] -cne $pin[2] -or
            $fields[3] -notmatch '^[0-9a-f]{64}$' -or $fields[4] -cne $distribution) {
            throw "Static musl package provenance does not match its pin: $line"
        }
        $archive = Join-Path $Directory $fields[0]
        if (-not (Test-Path -LiteralPath $archive -PathType Leaf) -or
            (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $fields[3]) {
            throw "Static musl archive does not match its provenance: $archive"
        }
        $seen[$fields[0]] = $true
    }
    foreach ($archive in $pins.Keys) {
        if (-not $seen.ContainsKey($archive)) { throw "Static musl provenance is missing $archive." }
    }
    if (-not $noticeHash -or $noticeHash -ne (Get-FileHash -LiteralPath $notices -Algorithm SHA256).Hash -or
        -not $recordedSourceHash -or $recordedSourceHash -ne $sourceHash) {
        throw 'Static musl notice or source metadata does not match its provenance.'
    }
}

function Copy-StaticNativeNotices {
    param(
        [Parameter(Mandatory)][string]$ProjectAssetsJsonPath,
        [Parameter(Mandatory)][string]$DestinationDirectory,
        [Parameter(Mandatory)][string]$Rid,
        [string]$StaticPublishDirectory
    )

    if (-not (Test-Path -LiteralPath $ProjectAssetsJsonPath -PathType Leaf)) {
        throw "Cannot find restored package metadata: $ProjectAssetsJsonPath"
    }
    $assets = Get-Content -LiteralPath $ProjectAssetsJsonPath -Raw | ConvertFrom-Json
    $platform = if ($Rid.StartsWith('win-')) { 'Win32' } elseif ($Rid.StartsWith('osx-')) { 'macOS' } else { 'Linux' }
    $runtimeName = "runtime.$Rid.Microsoft.DotNet.ILCompiler"
    $packageNames = @("SkiaSharp.NativeAssets.$platform", "HarfBuzzSharp.NativeAssets.$platform", 'Summpot.StaticAvalonia', $runtimeName)
    $packagePaths = @{}
    $packageVersions = @{}
    foreach ($name in $packageNames) {
        $package = @($assets.libraries.PSObject.Properties | Where-Object { $_.Name -like "$name/*" -and $_.Value.type -eq 'package' })
        if ($package.Count -ne 1) { throw "Cannot find the restored $name package in $ProjectAssetsJsonPath." }
        $packageVersions[$name] = $package[0].Name.Substring($name.Length + 1)
        foreach ($folder in $assets.packageFolders.PSObject.Properties.Name) {
            $candidate = Join-Path $folder $package[0].Value.path
            if (Test-Path -LiteralPath $candidate -PathType Container) {
                $packagePaths[$name] = $candidate
                break
            }
        }
        if (-not $packagePaths.ContainsKey($name)) { throw "Cannot locate the restored $name package on disk." }
    }

    $notices = @()
    foreach ($name in $packageNames[0..1]) {
        foreach ($file in @('LICENSE.txt', 'THIRD-PARTY-NOTICES.txt')) {
            $path = Join-Path $packagePaths[$name] $file
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing $name notice: $path" }
            $notices += @{ Label = "$name/$file"; Path = $path }
        }
    }
    $runtimeNotices = Join-Path $packagePaths[$runtimeName] 'THIRD-PARTY-NOTICES.TXT'
    if (-not (Test-Path -LiteralPath $runtimeNotices -PathType Leaf)) { throw "Missing NativeAOT runtime notices: $runtimeNotices" }
    $notices += @{ Label = "$runtimeName/THIRD-PARTY-NOTICES.TXT"; Path = $runtimeNotices }
    $runtimeLicense = $null
    foreach ($folder in $assets.packageFolders.PSObject.Properties.Name) {
        $candidate = Join-Path $folder "microsoft.netcore.app.runtime.$Rid" $packageVersions[$runtimeName] 'LICENSE.TXT'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { $runtimeLicense = $candidate; break }
    }
    if (-not $runtimeLicense) { throw "Cannot find the .NET runtime license for $Rid $($packageVersions[$runtimeName])." }
    $notices += @{ Label = "Microsoft.NETCore.App.Runtime.$Rid/LICENSE.TXT"; Path = $runtimeLicense }
    $staticPackage = $packagePaths['Summpot.StaticAvalonia']
    if ($Rid.StartsWith('win-')) {
        $angleNotices = Join-Path $staticPackage 'licenses'
        if (-not (Test-Path -LiteralPath $angleNotices -PathType Container)) {
            throw "Missing static ANGLE notices: $angleNotices"
        }
        $angleFiles = @(Get-ChildItem -LiteralPath $angleNotices -File -Recurse | Sort-Object FullName -CaseSensitive)
        if ($angleFiles.Count -eq 0) { throw "Static ANGLE notices are empty: $angleNotices" }
        foreach ($file in $angleFiles) {
            $relative = [IO.Path]::GetRelativePath($staticPackage, $file.FullName).Replace('\', '/')
            $notices += @{ Label = "Summpot.StaticAvalonia/$relative"; Path = $file.FullName }
        }
    }

    $components = @(@{ Source = 'skiasharp.json'; Destination = 'STATIC-SKIASHARP-PROVENANCE.json' })
    if ($Rid.StartsWith('win-')) {
        $components += @{ Source = 'angle.json'; Destination = 'STATIC-ANGLE-PROVENANCE.json' }
    }
    foreach ($component in $components) {
        $source = if ($Rid.StartsWith('linux-')) {
            if ([string]::IsNullOrWhiteSpace($StaticPublishDirectory)) {
                throw 'The static Linux publish directory is required to collect native provenance.'
            }
            Join-Path $StaticPublishDirectory $component.Destination
        } else {
            Join-Path $staticPackage 'build' 'components' $component.Source
        }
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing static native provenance: $source" }
        if ($component.Source -eq 'skiasharp.json') {
            $metadata = Get-Content -LiteralPath $source -Raw | ConvertFrom-Json
            if ($metadata.version -ne $packageVersions["SkiaSharp.NativeAssets.$platform"]) {
                throw "Static SkiaSharp $($metadata.version) does not match the restored managed graphics version $($packageVersions["SkiaSharp.NativeAssets.$platform"])."
            }
            if ($Rid.StartsWith('linux-')) {
                if ($Rid -cin $script:MuslStaticHostRids) {
                    $patch = Join-Path $PSScriptRoot 'static-skia-musl-dng.patch'
                    if ($metadata.rid -ne $Rid -or -not $metadata.PSObject.Properties['libc'] -or $metadata.libc -ne 'musl' -or
                        -not $metadata.PSObject.Properties['dngPatchSha256'] -or
                        $metadata.dngPatchSha256 -ne (Get-FileHash -LiteralPath $patch -Algorithm SHA256).Hash) {
                        throw "Static Linux graphics provenance does not match $Rid and the musl source patch."
                    }
                }
                elseif ($metadata.rid -ne $Rid -or $metadata.glibcBuild -ne '2.35') {
                    throw "Static Linux graphics provenance does not match $Rid and the glibc 2.35 baseline."
                }
                if (-not $metadata.PSObject.Properties['archives']) {
                    throw 'Static Linux graphics provenance does not contain archive hashes.'
                }
                foreach ($archive in @('libSkiaSharp.a', 'libskia.a', 'libHarfBuzzSharp.a')) {
                    $property = $metadata.archives.PSObject.Properties[$archive]
                    if ($null -eq $property) { throw "Static Linux graphics provenance is missing $archive." }
                    $expected = $property.Value
                    $actual = (Get-FileHash -LiteralPath (Join-Path $StaticPublishDirectory $archive) -Algorithm SHA256).Hash
                    if ($expected -notmatch '^[0-9a-f]{64}$' -or $actual -ne $expected) {
                        throw "Static Linux graphics provenance does not match $archive."
                    }
                }
                $sourceNotices = Join-Path $StaticPublishDirectory 'STATIC-SKIASHARP-SOURCE-NOTICES.txt'
                if (-not (Test-Path -LiteralPath $sourceNotices -PathType Leaf)) { throw "Missing static graphics source notices: $sourceNotices" }
                $noticeHash = (Get-FileHash -LiteralPath $sourceNotices -Algorithm SHA256).Hash
                if (-not $metadata.PSObject.Properties['noticesSha256'] -or
                    $metadata.noticesSha256 -notmatch '^[0-9a-f]{64}$' -or $noticeHash -ne $metadata.noticesSha256) {
                    throw 'Static Linux graphics provenance does not match the source notices.'
                }
            }
        }
        Copy-Item -LiteralPath $source -Destination (Join-Path $DestinationDirectory $component.Destination)
    }
    if ($Rid.StartsWith('linux-')) {
        if ([string]::IsNullOrWhiteSpace($StaticPublishDirectory)) {
            throw 'The static Linux publish directory is required to collect system-library notices.'
        }
        $linuxMetadata = @('STATIC-LINUX-NOTICES.txt', 'STATIC-LINUX-PROVENANCE.tsv', 'STATIC-SKIASHARP-SOURCE-NOTICES.txt')
        if ($Rid -cin $script:MuslStaticHostRids) {
            Assert-LinuxMuslStaticProvenance -Rid $Rid -Directory $StaticPublishDirectory
            $linuxMetadata += 'STATIC-LINUX-MUSL-SOURCES.tsv'
        }
        foreach ($name in $linuxMetadata) {
            $source = Join-Path $StaticPublishDirectory $name
            if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing static Linux metadata: $source" }
            Copy-Item -LiteralPath $source -Destination (Join-Path $DestinationDirectory $name)
        }
    }
    $output = Join-Path $DestinationDirectory 'STATIC-NATIVE-NOTICES.txt'
    $writer = [IO.StreamWriter]::new($output, $false, [Text.UTF8Encoding]::new($false))
    try {
        foreach ($notice in $notices) {
            $writer.WriteLine("===== $($notice.Label) =====")
            $writer.WriteLine([IO.File]::ReadAllText($notice.Path).TrimEnd("`r", "`n"))
            $writer.WriteLine()
        }
        $writer.WriteLine('===== Wuffs =====')
        $writer.WriteLine('Copyright 2017 The Wuffs Authors.')
        $writer.WriteLine('Licensed under the Apache License, Version 2.0, included in the SkiaSharp third-party notices above.')
    }
    finally { $writer.Dispose() }
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

function Get-RustoloniaReleaseVersion {
    param([Parameter(Mandatory)][string]$RustoloniaRoot)

    $release = Get-Content -LiteralPath (Join-Path $RustoloniaRoot 'rust' 'release-manifest.json') -Raw | ConvertFrom-Json
    $version = [string]$release.rustoloniaVersion
    foreach ($crate in @('rustolonia-sys', 'rustolonia', 'rustolonia-bindgen')) {
        $manifest = Join-Path $RustoloniaRoot 'rust' $crate 'Cargo.toml'
        $match = Select-String -LiteralPath $manifest -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
        if (-not $match -or $match.Matches[0].Groups[1].Value -ne $version) {
            throw "$crate version does not match release-manifest.json rustoloniaVersion '$version'."
        }
    }
    return $version
}

function Get-AbiFingerprint {
    param([Parameter(Mandatory)][string]$RustoloniaRoot)

    # Raw bytes, exactly like rustolonia-sys/build.rs (the header is checked out with eol=lf).
    $header = Join-Path $RustoloniaRoot 'rust' 'rustolonia-sys' 'include' 'avalonia-rust-abi.h'
    return (Get-FileHash -LiteralPath $header -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-HostAssetName {
    param(
        [Parameter(Mandatory)][string]$Version,
        [Parameter(Mandatory)][string]$Rid,
        [ValidateSet('default', 'devtools')][string]$Flavor = 'default',
        [ValidateSet('X11', 'Drm')][string]$LinuxBackend = 'X11',
        [ValidateSet('Shared', 'Static')][string]$Linkage = 'Shared',
        [switch]$Symbols
    )

    if ($Rid -cnotin $script:IosHostRids -and $Rid -cne $script:BrowserHostRid) {
        $null = Get-RidTargetInfo -Rid $Rid -LinuxBackend $LinuxBackend
        Assert-RidLinkage -Rid $Rid -Linkage $Linkage
    }
    elseif ($LinuxBackend -eq 'Drm') {
        throw 'The Drm backend requires a Linux RID.'
    }
    elseif ($Linkage -eq 'Static') {
        throw 'Only desktop RIDs have a separate static host.'
    }
    $name = "rustolonia-host-$Version-$Rid"
    if ($LinuxBackend -eq 'Drm') { $name += '-drm' }
    if ($Linkage -eq 'Static') { $name += '-static' }
    if ($Flavor -ne 'default') { $name += "-$Flavor" }
    if ($Symbols) { $name += '-symbols' }
    return "$name.tar.gz"
}

# The prebuilt Android host is one APK for every ABI; rustolonia-apk
# (rust/rustolonia-apk/src/host.rs) downloads it by this name.
function Get-AndroidHostAssetName {
    param([Parameter(Mandatory)][string]$Version)
    return "rustolonia-host-$Version-android.apk"
}

function Write-HostManifest {
    param(
        [Parameter(Mandatory)][string]$Directory,
        [Parameter(Mandatory)][string]$Version,
        [Parameter(Mandatory)][string]$Rid,
        [Parameter(Mandatory)][string]$AbiFingerprint,
        [Parameter(Mandatory)][string]$RustoloniaRoot,
        [ValidateSet('default', 'devtools')][string]$Flavor = 'default',
        [ValidateSet('X11', 'Drm')][string]$LinuxBackend = 'X11',
        [ValidateSet('Shared', 'Static')][string]$Linkage = 'Shared',
        [string]$SourceRevision
    )

    $target = Get-RidTargetInfo -Rid $Rid -LinuxBackend $LinuxBackend
    $release = Get-Content -LiteralPath (Join-Path $RustoloniaRoot 'rust' 'release-manifest.json') -Raw | ConvertFrom-Json
    $files = @(Get-ChildItem -LiteralPath $Directory -File -Force |
        Where-Object { $_.Name -ne 'host-manifest.json' } |
        Sort-Object Name |
        ForEach-Object {
            [ordered]@{ name = $_.Name; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
        })
    # version, rid and abiFingerprint come first and are flat strings: rustolonia-sys
    # reads them with a minimal parser.
    $manifest = [ordered]@{
        version        = $Version
        rid            = $Rid
        abiFingerprint = $AbiFingerprint
        platform       = $target.Platform
        linkage        = $Linkage
        hostFile       = if ($Linkage -eq 'Static') { $target.StaticHostFileName } else { $target.HostFileName }
        flavor         = $Flavor
        schemaVersion  = 1
        sourceRevision = $SourceRevision
        producerPin    = [string]$release.producerPin
        producerRemote = [string]$release.producerRemote
        patches        = @($release.patches | ForEach-Object { [ordered]@{ path = $_.path; sha256 = $_.sha256 } })
        files          = $files
    }
    $json = ($manifest | ConvertTo-Json -Depth 6) -replace "`r`n", "`n"
    [IO.File]::WriteAllText((Join-Path $Directory 'host-manifest.json'), "$json`n", [Text.UTF8Encoding]::new($false))
}

function Get-HostSymbolFiles {
    param([Parameter(Mandatory)][string]$PublishDirectory)

    # NativeAOT symbols: .pdb (Windows), .dbg (Linux), .dSYM bundles (macOS).
    $root = (Resolve-Path -LiteralPath $PublishDirectory).Path
    $items = @(Get-ChildItem -LiteralPath $root -File | Where-Object { $_.Extension -in @('.pdb', '.dbg') })
    foreach ($bundle in @(Get-ChildItem -LiteralPath $root -Directory -Filter '*.dSYM')) {
        $items += @(Get-ChildItem -LiteralPath $bundle.FullName -File -Recurse)
    }
    return @($items | ForEach-Object { [IO.Path]::GetRelativePath($root, $_.FullName) } | Sort-Object -CaseSensitive)
}

function New-DeterministicTarGz {
    param(
        [Parameter(Mandatory)][string]$SourceDirectory,
        [Parameter(Mandatory)][string[]]$RelativePaths,
        [Parameter(Mandatory)][string]$Destination,
        [long]$Timestamp = 0,
        [string[]]$ExecutablePaths = @()
    )

    # Fixed order, owner, mode and mtime, and no gzip timestamp, so identical inputs
    # produce byte-identical archives.
    $root = (Resolve-Path -LiteralPath $SourceDirectory).Path
    $mtime = [DateTimeOffset]::FromUnixTimeSeconds($Timestamp)
    $executable = [IO.UnixFileMode]'UserRead, UserWrite, UserExecute, GroupRead, GroupExecute, OtherRead, OtherExecute'
    $regular = [IO.UnixFileMode]'UserRead, UserWrite, GroupRead, OtherRead'
    $explicitExecutables = @($ExecutablePaths | ForEach-Object { $_.Replace('\', '/') })
    $file = [IO.File]::Create($Destination)
    try {
        $gzip = [IO.Compression.GZipStream]::new($file, [IO.Compression.CompressionLevel]::SmallestSize)
        try {
            $writer = [Formats.Tar.TarWriter]::new($gzip, [Formats.Tar.TarEntryFormat]::Ustar, $false)
            try {
                foreach ($relative in @($RelativePaths | Sort-Object -CaseSensitive -Unique)) {
                    $entryName = $relative.Replace('\', '/')
                    if ($entryName.StartsWith('/') -or $entryName -match '(^|/)\.\.(/|$)') {
                        throw "Archive entry must be a relative path inside the source directory: $relative"
                    }
                    $path = Join-Path $root $relative
                    $entry = [Formats.Tar.UstarTarEntry]::new([Formats.Tar.TarEntryType]::RegularFile, $entryName)
                    $entry.ModificationTime = $mtime
                    $entry.Uid = 0
                    $entry.Gid = 0
                    $entry.Mode = if ($entryName -match '\.(dll|so|dylib|exe)$|\.so\.|\.dSYM/Contents/Resources/DWARF/' -or
                        $entryName -cin $explicitExecutables) { $executable } else { $regular }
                    $stream = [IO.File]::OpenRead($path)
                    try {
                        $entry.DataStream = $stream
                        $writer.WriteEntry($entry)
                    }
                    finally { $stream.Dispose() }
                }
            }
            finally { $writer.Dispose() }
        }
        finally { $gzip.Dispose() }
    }
    finally { $file.Dispose() }
}

function Write-AssetChecksum {
    param([Parameter(Mandatory)][string]$Asset)

    $hash = (Get-FileHash -LiteralPath $Asset -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$Asset.sha256", "$hash  $([IO.Path]::GetFileName($Asset))`n", [Text.UTF8Encoding]::new($false))
    return $hash
}
