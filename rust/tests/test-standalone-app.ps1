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

With -GitRepository and -GitTag, skip packaging and use a git dependency
without registry patches. -UseCommittedChecksums verifies the tagged hashes.
Use -LinuxBackend Drm for a fullscreen single-view consumer, and -Linkage
Static to link the static host archive through the `static-host` feature. -BuildOnly
verifies download and packaging without launching on a host lacking DRM.

.EXAMPLE
./rust/build.ps1
./rust/tests/test-standalone-app.ps1

.EXAMPLE
./rust/tests/test-standalone-app.ps1 -Distributable -KeepWorkDirectory
#>
param(
    [ValidateSet('win-x64', 'win-arm64', 'linux-x64', 'linux-arm64', 'linux-musl-x64', 'linux-musl-arm64', 'osx-x64', 'osx-arm64')]
    [string]$Rid,
    [ValidateSet('X11', 'Drm')][string]$LinuxBackend = 'X11',
    [ValidateSet('Shared', 'Static')][string]$Linkage = 'Shared',
    # Published host to pack. Defaults to rust/build.ps1's output for -Rid.
    [string]$PublishDirectory,
    # Existing directory with the release tarball and .sha256 (skips packing).
    [string]$TarballDirectory,
    [string]$RustoloniaRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [string]$WorkDirectory = (Join-Path ([IO.Path]::GetTempPath()) 'rustolonia-standalone-app'),
    # Build with default-features = false and ship the host beside the executable.
    [switch]$Distributable,
    [switch]$BuildOnly,
    # Exercise Cargo's git source and its transitive path dependency, without registry patches.
    [string]$GitRepository,
    [string]$GitTag,
    [switch]$UseCommittedChecksums,
    [switch]$KeepWorkDirectory,
    [int]$LaunchSeconds = 5
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $RustoloniaRoot 'rust' 'package-shared.ps1')

if (-not $Rid) { $Rid = Get-DefaultConsumerRid }
$null = Get-RidTargetInfo -Rid $Rid -LinuxBackend $LinuxBackend
$version = Get-RustoloniaReleaseVersion -RustoloniaRoot $RustoloniaRoot
if ([bool]$GitRepository -ne [bool]$GitTag) { throw 'Pass both -GitRepository and -GitTag.' }
if ($UseCommittedChecksums -and -not $GitRepository) { throw '-UseCommittedChecksums requires a git consumer.' }
$rustDir = Join-Path $RustoloniaRoot 'rust'
$WorkDirectory = [IO.Path]::GetFullPath($WorkDirectory)
if ($WorkDirectory.StartsWith([IO.Path]::GetFullPath($RustoloniaRoot), [StringComparison]::OrdinalIgnoreCase)) {
    # The repository's .cargo/config.toml would disable downloads.
    throw "-WorkDirectory must be outside the repository: $WorkDirectory"
}
if (Test-Path -LiteralPath $WorkDirectory) { Remove-Item -LiteralPath $WorkDirectory -Recurse -Force }
New-Item -ItemType Directory -Force -Path $WorkDirectory | Out-Null

if (-not $TarballDirectory) {
    if (-not $PublishDirectory -and $Linkage -eq 'Static') {
        # rust/build.ps1 publishes a shared host; package-host.ps1 publishes
        # the static one itself.
        $TarballDirectory = Join-Path $WorkDirectory 'host'
        & (Join-Path $rustDir 'package-host.ps1') -Rid $Rid -LinuxBackend $LinuxBackend -Linkage Static `
            -OutputRoot $TarballDirectory -RustoloniaRoot $RustoloniaRoot
    }
}
if (-not $TarballDirectory) {
    if (-not $PublishDirectory) {
        $suffix = if ($LinuxBackend -eq 'Drm') { '-drm' } else { '' }
        $PublishDirectory = Join-Path $rustDir 'target' "dotnet-$Rid$suffix" 'publish' 'Avalonia.Host' "release_$Rid"
    }
    if (-not (Test-Path -LiteralPath $PublishDirectory -PathType Container)) {
        throw "Published host not found at $PublishDirectory. Run rust/build.ps1 first, or pass -PublishDirectory or -TarballDirectory."
    }
    $TarballDirectory = Join-Path $WorkDirectory 'host'
    & (Join-Path $rustDir 'package-host.ps1') -Rid $Rid -LinuxBackend $LinuxBackend -PublishDirectory $PublishDirectory `
        -OutputRoot $TarballDirectory -RustoloniaRoot $RustoloniaRoot -Linkage $Linkage
}

# --no-verify: verification would build rustolonia-sys outside the repository
# and try to download the not-yet-released host from GitHub. The app build
# below is the real verification.
$patchSection = ''
if (-not $GitRepository) {
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
    $patchSection = "[patch.crates-io]`n$($patches -join "`n")"
}

$appDir = Join-Path $WorkDirectory 'app'
$appName = 'standalone_app'
New-Item -ItemType Directory -Force -Path (Join-Path $appDir 'src') | Out-Null
$features = if ($Distributable) { ', default-features = false' } else { '' }
$crateFeatures = @()
if ($LinuxBackend -eq 'Drm') { $crateFeatures += '"linux-drm"' }
if ($Linkage -eq 'Static') { $crateFeatures += '"static-host"' }
if ($crateFeatures) { $features += ", features = [$($crateFeatures -join ', ')]" }
$dependency = if ($GitRepository) {
    # JSON strings are also valid TOML basic strings, including escaped Windows paths.
    "git = $($GitRepository | ConvertTo-Json -Compress), tag = $($GitTag | ConvertTo-Json -Compress)"
} else {
    "version = `"=$version`""
}
@"
[package]
name = "$appName"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
rustolonia = { $dependency$features }

$patchSection

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
if ($Rid -cin $script:MuslStaticHostRids) {
    @'
use rustolonia::{
    App, Button, OpenFilePickerOptions, Orientation, PickerOutcome, StackPanel, TextBlock, TextBox,
    Window,
};

fn main() -> rustolonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let editor = TextBox::new()?.text("initial")?;
        let editor_for_event = editor.clone();
        let editor = editor.on_text_changed(scope, move |_| {
            println!(
                "edited={}",
                editor_for_event.get_text().expect("read text").expect("text is present")
            );
        })?;
        let shutdown = scope.clone();
        let button = Button::new()?
            .height(80.0)?
            .content(Some(&TextBlock::new()?.text("Stop")?))?
            .on_click(scope, move |_| {
                println!("clicked");
                shutdown.shutdown().expect("shutdown failed");
            })?;
        let services_scope = scope.clone();
        let services = Button::new()?
            .height(80.0)?
            .content(Some(&TextBlock::new()?.text("Check desktop services")?))?
            .on_click(scope, move |_| {
                let task_scope = services_scope.clone();
                services_scope
                    .spawn(async move {
                        let window = task_scope.main_window().expect("main window");
                        task_scope
                            .clipboard_set_text(&window, "static-clipboard")
                            .expect("start clipboard write")
                            .await
                            .expect("clipboard write");
                        let text = task_scope
                            .clipboard_get_text(&window)
                            .expect("start clipboard read")
                            .await
                            .expect("clipboard read");
                        println!("clipboard={}", text.expect("clipboard text"));
                        let result = task_scope
                            .open_file_picker(
                                &window,
                                &OpenFilePickerOptions::new().title("Rustolonia static picker probe"),
                            )
                            .expect("start picker")
                            .await
                            .expect("picker result");
                        match result {
                            PickerOutcome::Cancelled => println!("picker=cancelled"),
                            PickerOutcome::Selected(_) => panic!("picker unexpectedly selected a file"),
                        }
                    })
                    .expect("spawn desktop services probe");
            })?;
        scope.mount(
            Window::new()?
                .title("Rustolonia static musl probe")?
                .width(400.0)?
                .height(250.0)?
                .content(Some(
                    &StackPanel::new()?
                        .orientation(Orientation::Vertical)?
                        .child(editor)?
                        .child(button)?
                        .child(services)?,
                ))?,
        )
    })?;
    println!("stopped");
    Ok(())
}
'@ | Set-Content -LiteralPath (Join-Path $appDir 'src' 'main.rs')
}
if ($LinuxBackend -eq 'Drm') {
    Copy-Item -LiteralPath (Join-Path $rustDir 'rustolonia/examples/embedded_linux.rs') `
        -Destination (Join-Path $appDir 'src/main.rs') -Force
}

try {
    & (Join-Path $PSScriptRoot 'test-host-download.ps1') -Rid $Rid -LinuxBackend $LinuxBackend -TarballDirectory $TarballDirectory `
        -RustoloniaRoot $RustoloniaRoot -WorkDirectory (Join-Path $WorkDirectory 'download') -Linkage $Linkage `
        -AppManifest (Join-Path $appDir 'Cargo.toml') -AppName $appName -ShipHost:$Distributable `
        -UseCommittedChecksums:$UseCommittedChecksums `
        -BuildOnly:$BuildOnly `
        -MuslInteraction:($Rid -cin $script:MuslStaticHostRids) `
        -LaunchSeconds $LaunchSeconds
    $source = if ($GitRepository) { "git tag $GitTag" } else { "packaged crates" }
    Write-Host "Standalone app built from rustolonia $version $source and the $Rid ($LinuxBackend, $Linkage) host tarball."
}
finally {
    if (-not $KeepWorkDirectory) {
        Remove-Item -LiteralPath $WorkDirectory -Recurse -Force -ErrorAction SilentlyContinue
    } else {
        Write-Host "Work directory kept at $WorkDirectory"
    }
}
