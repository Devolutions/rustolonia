<#
.SYNOPSIS
Smoke-tests the archive explorer Windows portable bundle through UI Automation.
.DESCRIPTION
Requires Windows PowerShell 5.1, STA, and an unlocked interactive desktop.
The test starts with no arguments, verifies the idle empty state, then
requests a normal close. It never opens a picker dialog.
#>
[CmdletBinding()]
param(
    [string] $BundlePath = '',
    [ValidateRange(10, 180)] [int] $TimeoutSeconds = 60,
    [ValidateRange(1, 60)] [int] $CloseTimeoutSeconds = 15
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSEdition -ne 'Desktop' -or $PSVersionTable.PSVersion.Major -ne 5) {
    throw 'Use Windows PowerShell 5.1: powershell.exe -NoProfile -STA -File <script>.'
}
if ([Threading.Thread]::CurrentThread.GetApartmentState() -ne 'STA') {
    throw 'STA is required. Run powershell.exe -NoProfile -STA -File <script>.'
}
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
if (-not $BundlePath) {
    $BundlePath = Join-Path $PSScriptRoot '..\artifacts\win-x64\archive-tool.exe'
}
$exe = (Resolve-Path -LiteralPath $BundlePath).ProviderPath
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Bundle not found: $exe" }

$script:app = $null
$script:window = $null
$failure = $null
$cleanupFailure = $null

function Wait-Condition([string] $Description, [scriptblock] $Probe) {
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $lastError = 'condition not yet satisfied'
    while ($clock.Elapsed.TotalSeconds -lt $TimeoutSeconds) {
        if ($script:app.HasExited) {
            throw "[$Description] PID $($script:app.Id) exited early; exit code $($script:app.ExitCode)."
        }
        try {
            $result = & $Probe
            if ($result) { return $result }
        } catch {
            $lastError = $_.Exception.GetType().Name
        }
        Start-Sleep -Milliseconds 200
    }
    throw "[$Description] Timed out after ${TimeoutSeconds}s; last probe: $lastError."
}

function Find-Control([string] $Name) {
    $condition = [Windows.Automation.PropertyCondition]::new(
        [Windows.Automation.AutomationElement]::AutomationIdProperty, $Name)
    $element = $script:window.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
    if ($null -eq $element) {
        $condition = [Windows.Automation.PropertyCondition]::new(
            [Windows.Automation.AutomationElement]::NameProperty, $Name)
        $element = $script:window.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
    }
    return $element
}

function Set-Value([string] $Name, [string] $Value) {
    $control = Find-Control $Name
    if ($null -eq $control) { throw "Control '$Name' not found." }
    $pattern = $control.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern)
    $pattern.SetValue($Value)
}

function Get-Text([string] $Name) {
    $control = Find-Control $Name
    if ($null -eq $control) { return '' }
    return [string] $control.Current.Name
}

try {
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $exe
    $start.WorkingDirectory = Split-Path -Parent $exe
    $start.UseShellExecute = $false
    $start.EnvironmentVariables.Remove('AVN_HOST_NATIVE_LIB')
    $script:app = [Diagnostics.Process]::Start($start)
    $script:window = Wait-Condition 'main window' {
        $script:app.Refresh()
        if ($script:app.MainWindowHandle -ne [IntPtr]::Zero) {
            $candidate = [Windows.Automation.AutomationElement]::FromHandle($script:app.MainWindowHandle)
            if ($candidate.Current.ProcessId -eq $script:app.Id) { return $candidate }
        }
    }

    $null = Wait-Condition 'idle empty state' {
        (Get-Text 'EntryCountLabel') -eq 'No archive loaded' -and
            (Get-Text 'Status') -match 'Open an archive' -and
            $null -ne (Find-Control 'OpenButton') -and
            $null -ne (Find-Control 'EntriesTable')
    }
    Write-Host 'PASS startup: idle window, no sample archive.'
} catch {
    $failure = $_
} finally {
    if ($null -ne $script:app) {
        try {
            if (-not $script:app.HasExited) {
                $script:app.Refresh()
                $requested = $script:app.CloseMainWindow()
                Write-Host "Normal close requested for PID $($script:app.Id): $requested."
                if (-not $script:app.WaitForExit($CloseTimeoutSeconds * 1000)) {
                    throw "PID $($script:app.Id) did not exit within ${CloseTimeoutSeconds}s. Close it manually; no termination attempted."
                }
            }
            if ($script:app.ExitCode -ne 0) {
                throw "Bundle PID $($script:app.Id) exited with code $($script:app.ExitCode)."
            }
        } catch {
            $cleanupFailure = $_.Exception.Message
        } finally {
            $script:app.Dispose()
        }
    }
}
if ($null -ne $failure) {
    if ($cleanupFailure) { Write-Warning $cleanupFailure }
    throw $failure
}
if ($cleanupFailure) { throw $cleanupFailure }
Write-Host 'PASS archive explorer portable bundle smoke.'
