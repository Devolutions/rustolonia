# Builds jiffshot.exe into app/artifacts/<rid> with Rustolonia's consumer
# build. Regenerates view-model.ir.json and the adapters first.
#
#   pwsh apps/jiffshot/app/build.ps1 [-Rid win-x64|win-arm64] [-ProducerRoot <avalonia-src checkout>]
#
# It builds against this repository's Rustolonia and its avalonia-src
# submodule (git submodule update --init avalonia-src); -ProducerRoot
# points at another checkout of the producer instead.
#
# win-arm64 cross-builds from x64: it needs the aarch64-pc-windows-msvc
# Rust target and Visual Studio's ARM64 build tools.
param(
    [ValidateSet('win-x64', 'win-arm64')][string]$Rid = 'win-x64',
    [string]$RustoloniaRoot = (Join-Path $PSScriptRoot '..' '..' '..'),
    [string]$ProducerRoot = (Join-Path $PSScriptRoot '..' '..' '..' 'avalonia-src'),
    [switch]$UpdateLockFile
)
$ErrorActionPreference = 'Stop'
python (Join-Path $PSScriptRoot 'tools' 'gen_ir.py')
$arguments = @{
    ProducerRoot   = (Resolve-Path $ProducerRoot).Path
    Manifest       = (Join-Path $PSScriptRoot ($Rid -eq 'win-x64' ? 'avalonia-app.json' : "avalonia-app.$Rid.json"))
    RustoloniaRoot = (Resolve-Path $RustoloniaRoot).Path
}
if ($UpdateLockFile) { $arguments.UpdateLockFile = $true }
& (Join-Path $arguments.RustoloniaRoot 'rust' 'build-app.ps1') @arguments
