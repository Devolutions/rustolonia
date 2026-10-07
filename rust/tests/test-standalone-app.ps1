#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
Builds a standalone app against the packaged crates and a locally built host
tarball, as if both were already released.

.DESCRIPTION
Rehearses the crates.io + GitHub release experience before anything is
published:

1. `cargo package` the three crates and extract the .crate files, exactly as
   crates.io would serve them.
2. Pack the locally published host into the release tarball with
   package-host.ps1, unless -TarballDirectory already has one.
3. Generate a fresh Cargo project outside the repository that depends on
   `rustolonia = "=X.Y.Z"` and redirects crates.io to the packaged crates
   with [patch.crates-io].
4. Build and launch it through test-host-download.ps1, which serves the
   tarball from a file:// release tree with its checksum.

Build the host first with `rust/build.ps1`.

.EXAMPLE
./rust/build.ps1
./rust/tests/test-standalone-app.ps1

.EXAMPLE
./rust/tests/test-standalone-app.ps1 -Distributable -KeepWorkDirectory
#>
param(
    [ValidateSet('win-x64', 'win-arm64', 'linux-x64', 'linux-arm64', 'osx-x64', 'osx-arm64')]
    [string]$Rid,
    # Published host to pack. Defaults to rust/build.ps1's output for -Rid.
    [string]$PublishDirectory,
    # Existing directory with the release tarball and .sha256 (skips packing).
    [string]$TarballDirectory,
    [string]$RustoloniaRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [string]$WorkDirectory = (Join-Path ([IO.Path]::GetTempPath()) 'rustolonia-standalone-app'),
    # Build with default-features = false and ship the host beside the executable.
    [switch]$Distributable,
    [switch]$KeepWorkDirectory,
    [int]$LaunchSeconds = 5
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $RustoloniaRoot 'rust' 'package-shared.ps1')

if (-not $Rid) { $Rid = Get-DefaultConsumerRid }
$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $RustoloniaRoot
$rustDir = Join-Path $RustoloniaRoot 'rust'
$WorkDirectory = [IO.Path]::GetFullPath($WorkDirectory)
if ($WorkDirectory.StartsWith([IO.Path]::GetFullPath($RustoloniaRoot), [StringComparison]::OrdinalIgnoreCase)) {
    # The repository's .cargo/config.toml would disable downloads.
    throw "-WorkDirectory must be outside the repository: $WorkDirectory"
}
if (Test-Path -LiteralPath $WorkDirectory) { Remove-Item -LiteralPath $WorkDirectory -Recurse -Force }
New-Item -ItemType Directory -Force -Path $WorkDirectory | Out-Null

if (-not $TarballDirectory) {
    if (-not $PublishDirectory) {
        $PublishDirectory = Join-Path $rustDir 'target' "dotnet-$Rid" 'publish' 'Avalonia.Host' "release_$Rid"
    }
    if (-not (Test-Path -LiteralPath $PublishDirectory -PathType Container)) {
        throw "Published host not found at $PublishDirectory. Run rust/build.ps1 first, or pass -PublishDirectory or -TarballDirectory."
    }
    $TarballDirectory = Join-Path $WorkDirectory 'host'
    & (Join-Path $rustDir 'package-host.ps1') -Rid $Rid -PublishDirectory $PublishDirectory `
        -OutputRoot $TarballDirectory -RustoloniaRoot $RustoloniaRoot
}

# --no-verify: verification would build rustolonia-sys outside the repository
# and try to download the not-yet-released host from GitHub. The app build
# below is the real verification.
$packageTarget = Join-Path $WorkDirectory 'package-target'
cargo package --locked --allow-dirty --no-verify --manifest-path (Join-Path $rustDir 'Cargo.toml') `
    -p rustolonia-sys -p rustolonia-bindgen -p rustolonia --target-dir $packageTarget

$cratesDir = Join-Path $WorkDirectory 'crates'
New-Item -ItemType Directory -Force -Path $cratesDir | Out-Null
$patches = foreach ($crate in @('rustolonia-sys', 'rustolonia-bindgen', 'rustolonia')) {
    $crateFile = Join-Path $packageTarget 'package' "$crate-$version.crate"
    if (-not (Test-Path -LiteralPath $crateFile -PathType Leaf)) { throw "Packaged crate not found: $crateFile" }
    tar -xzf $crateFile -C $cratesDir
    $path = (Join-Path $cratesDir "$crate-$version").Replace('\', '/')
    "$crate = { path = `"$path`" }"
}

$appDir = Join-Path $WorkDirectory 'app'
$appName = 'standalone_app'
New-Item -ItemType Directory -Force -Path (Join-Path $appDir 'src') | Out-Null
$features = if ($Distributable) { ', default-features = false' } else { '' }
@"
[package]
name = "$appName"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
rustolonia = { version = "=$version"$features }

# Stand-in for crates.io: the exact .crate contents that would be published.
[patch.crates-io]
$($patches -join "`n")

[workspace]
"@ | Set-Content -LiteralPath (Join-Path $appDir 'Cargo.toml')

@'
use rustolonia::{App, Button, Orientation, StackPanel, TextBlock, Window};

fn main() -> rustolonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        scope.mount(
            Window::new()?.title("Standalone rustolonia app")?.content(Some(
                &StackPanel::new()?
                    .orientation(Orientation::Vertical)?
                    .child(TextBlock::new()?.text("Built from the packaged crates")?)?
                    .child(Button::new()?.content(Some(&TextBlock::new()?.text("Hello")?))?)?,
            ))?,
        )
    })
}
'@ | Set-Content -LiteralPath (Join-Path $appDir 'src' 'main.rs')

try {
    & (Join-Path $PSScriptRoot 'test-host-download.ps1') -Rid $Rid -TarballDirectory $TarballDirectory `
        -RustoloniaRoot $RustoloniaRoot -WorkDirectory (Join-Path $WorkDirectory 'download') `
        -AppManifest (Join-Path $appDir 'Cargo.toml') -AppName $appName -ShipHost:$Distributable `
        -LaunchSeconds $LaunchSeconds
    Write-Host "Standalone app built from packaged rustolonia $version crates and the $Rid host tarball."
}
finally {
    if (-not $KeepWorkDirectory) {
        Remove-Item -LiteralPath $WorkDirectory -Recurse -Force -ErrorAction SilentlyContinue
    } else {
        Write-Host "Work directory kept at $WorkDirectory"
    }
}
