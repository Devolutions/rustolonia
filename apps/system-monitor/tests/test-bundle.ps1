<#
.SYNOPSIS
Smoke-tests the built Windows portable bundle through Windows UI Automation.
.DESCRIPTION
Requires Windows PowerShell 5.1 Desktop, .NET Framework UIAutomationClient/
UIAutomationTypes, and an unlocked interactive Windows desktop in the same user
session and integrity level as the app. Headless/service CI sessions are not
supported. No winapp, SDK, installed package, or screenshots are required.
Run with powershell.exe -NoProfile -STA -File .\avalonia\tests\test-bundle.ps1.
Other hosts/apartment states fail before launch; there is no implicit relaunch.
If local execution policy blocks scripts, add -ExecutionPolicy Bypass to that
invocation after reviewing this file (process-local only; no policy is changed).

The bundle must already be built. This script does not build or modify it.
AVN_HOST_NATIVE_LIB is removed only from ProcessStartInfo.EnvironmentVariables;
the calling process environment is never changed. Only the newly started process
and its window are targeted. No process inventory, command lines, environment
values, or full UI trees are printed. No process termination is ever attempted:
finally requests a normal window close, waits, and reports the exact PID if it
needs manual intervention. A nonzero app exit or failed assertion fails the test.

Polling and shutdown waits are bounded. An unresponsive out-of-process Windows
UIA provider can itself block a synchronous UIA call beyond those deadlines;
this is an interactive smoke test, not a hung-provider watchdog.
Live refresh requires observable percentage changes within TimeoutSeconds.
A perfectly static machine can therefore fail the resume assertion.
Deterministic model state and refresh cancellation belong to the parent tests.
.PARAMETER BundlePath
Path to the existing portable executable, relative to the caller if supplied.
.PARAMETER TimeoutSeconds
Deadline for each readiness/transition assertion (default 30 seconds).
.PARAMETER CloseTimeoutSeconds
Maximum normal-exit wait after requesting window close (default 15 seconds).
.EXAMPLE
powershell.exe -NoProfile -STA -File .\avalonia\tests\test-bundle.ps1
#>
[CmdletBinding()]
param(
    [string] $BundlePath = '',
    [ValidateRange(10, 120)] [int] $TimeoutSeconds = 30,
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
    $BundlePath = Join-Path $PSScriptRoot '..\artifacts\win-x64\neohtop-avalonia.exe'
}
$exe = (Resolve-Path -LiteralPath $BundlePath).ProviderPath
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Bundle not found: $exe" }

$script:app = $null
$script:window = $null
$script:stage = 'launch'
$failure = $null
$cleanupFailure = $null

function Assert-Smoke([bool] $Condition, [string] $Message) {
    if (-not $Condition) { throw "[$script:stage] $Message" }
}

function Wait-Smoke([string] $Description, [scriptblock] $Probe) {
    $script:stage = $Description
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
            # Stale UIA references during refresh are retried, without dumping UI.
            $lastError = $_.Exception.GetType().Name
        }
        Start-Sleep -Milliseconds 200
    }
    throw "[$Description] Timed out after ${TimeoutSeconds}s; PID $($script:app.Id); last probe: $lastError."
}

function Find-Control([string] $Id, [string] $Name = '') {
    $condition = [Windows.Automation.PropertyCondition]::new(
        [Windows.Automation.AutomationElement]::AutomationIdProperty, $Id)
    $element = $script:window.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
    if ($null -eq $element -and $Name) {
        $condition = [Windows.Automation.PropertyCondition]::new(
            [Windows.Automation.AutomationElement]::NameProperty, $Name)
        $element = $script:window.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
    }
    return $element
}

function Get-TextElements($Root) {
    $condition = [Windows.Automation.PropertyCondition]::new(
        [Windows.Automation.AutomationElement]::ControlTypeProperty,
        [Windows.Automation.ControlType]::Text)
    return $Root.FindAll([Windows.Automation.TreeScope]::Descendants, $condition)
}

function Get-ProcessCount {
    foreach ($element in (Get-TextElements $script:window)) {
        if ($element.Current.Name -match '^([0-9]+) processes?$') { return [int] $Matches[1] }
    }
    return -1
}

function Get-StatsFingerprint {
    # Only numeric percentages, never process commands/user/environment values.
    $values = @(
        foreach ($element in (Get-TextElements $script:window)) {
            $text = $element.Current.Name
            if ($text -match '^\d+([.,]\d+)?%$') { $text }
        }
    )
    if ($values.Count -lt 3) { return '' }
    return $values -join '|'
}

function Set-Search([string] $Value) {
    $search = Find-Control 'SearchBox' 'SearchText'
    Assert-Smoke ($null -ne $search) 'SearchBox/SearchText not found.'
    $pattern = $search.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern)
    $pattern.SetValue($Value)
    Assert-Smoke ($pattern.Current.Value -eq $Value) 'Search ValuePattern did not accept the requested text.'
}

function Invoke-Control([string] $Id) {
    $control = Find-Control $Id
    Assert-Smoke ($null -ne $control) "Control '$Id' not found."
    $control.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
}

function Get-OwnRow {
    $table = Find-Control 'ProcessTable' 'ProcessTable'
    if ($null -eq $table) { return $null }
    $condition = [Windows.Automation.PropertyCondition]::new(
        [Windows.Automation.AutomationElement]::NameProperty, [string] $script:app.Id)
    $cells = $table.FindAll([Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($cell in $cells) {
        $node = $cell
        for ($depth = 0; $depth -lt 8 -and $null -ne $node; $depth++) {
            if ([Windows.Automation.Automation]::Compare($node, $table)) { break }
            $selection = $null
            if ($node.TryGetCurrentPattern([Windows.Automation.SelectionItemPattern]::Pattern, [ref] $selection)) {
                # PID substring search may match other processes; require the exact
                # PID cell AND our executable's name in this selectable row.
                $nameCondition = [Windows.Automation.PropertyCondition]::new(
                    [Windows.Automation.AutomationElement]::NameProperty, [IO.Path]::GetFileName($exe))
                $nameCell = $node.FindFirst([Windows.Automation.TreeScope]::Descendants, $nameCondition)
                if ($null -ne $nameCell -or $node.Current.Name -eq [IO.Path]::GetFileName($exe)) {
                    return $node
                }
            }
            $node = [Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($node)
        }
    }
    return $null
}

try {
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $exe
    $start.WorkingDirectory = Split-Path -Parent $exe
    $start.UseShellExecute = $false
    $start.EnvironmentVariables.Remove('AVN_HOST_NATIVE_LIB')
    $script:app = [Diagnostics.Process]::Start($start)
    Write-Host "Started bundle PID $($script:app.Id)."
    $script:window = Wait-Smoke 'main window' {
        $script:app.Refresh()
        if ($script:app.MainWindowHandle -ne [IntPtr]::Zero) {
            $candidate = [Windows.Automation.AutomationElement]::FromHandle($script:app.MainWindowHandle)
            if ($candidate.Current.ProcessId -eq $script:app.Id) { return $candidate }
        }
    }
    $null = Wait-Smoke 'real process statistics' {
        (Get-ProcessCount) -gt 0 -and (Get-StatsFingerprint) -ne '' -and
            $null -ne (Find-Control 'CpuCoresList' 'CpuCores')
    }
    $initialCount = Get-ProcessCount
    Write-Host "PASS readiness: $initialCount processes and populated percentage statistics."
    $rate = Find-Control 'RefreshRateButton'
    Assert-Smoke ($null -ne $rate) 'RefreshRateButton not found.'
    $rateTexts = @($rate.Current.Name) + @(Get-TextElements $rate | ForEach-Object { $_.Current.Name })
    Assert-Smoke ($rateTexts -contains '3s') 'Expected default 3s refresh interval before 7s freeze observation.'

    # Runtime-generated tokens never appear in a helper process command line.
    $impossible = 'neohtopsmokeabsent' + [Guid]::NewGuid().ToString('N')
    Set-Search $impossible
    $null = Wait-Smoke 'impossible search yields zero processes' { (Get-ProcessCount) -eq 0 }
    Set-Search ([string] $script:app.Id)
    $null = Wait-Smoke 'PID search exposes exact own process row' { Get-OwnRow }
    Write-Host 'PASS search: impossible token excludes all rows; PID query includes exact own row.'

    # Freeze before acquiring/selecting a row: live updates replace containers.
    Invoke-Control 'FreezeButton'
    Start-Sleep -Milliseconds 500
    $row = Wait-Smoke 'frozen own process row' { Get-OwnRow }
    $selection = $row.GetCurrentPattern([Windows.Automation.SelectionItemPattern]::Pattern)
    $selection.Select()
    $null = Wait-Smoke 'own row selected' {
        $selectedRow = Get-OwnRow
        $null -ne $selectedRow -and
            $selectedRow.GetCurrentPattern([Windows.Automation.SelectionItemPattern]::Pattern).Current.IsSelected
    }
    $frozenStats = Get-StatsFingerprint
    $frozenCount = Get-ProcessCount
    Assert-Smoke ($frozenStats -ne '') 'Missing numeric statistics while frozen.'
    $clock = [Diagnostics.Stopwatch]::StartNew()
    while ($clock.Elapsed.TotalSeconds -lt 7) {
        Start-Sleep -Milliseconds 250
        Assert-Smoke (-not $script:app.HasExited) 'App exited during freeze observation.'
        Assert-Smoke ((Get-StatsFingerprint) -eq $frozenStats) 'Statistics changed while frozen.'
        Assert-Smoke ((Get-ProcessCount) -eq $frozenCount) 'Process count changed while frozen.'
        Assert-Smoke ($selection.Current.IsSelected) 'Own row selection was lost while frozen.'
    }
    Write-Host 'PASS frozen selection: exact own PID selected; statistics/count/selection stable for 7s.'

    # Clear while frozen so collection changes cannot race this assertion.
    Set-Search ''
    $null = Wait-Smoke 'clear search restores process list' { (Get-ProcessCount) -gt $frozenCount }
    $beforeResume = Get-StatsFingerprint
    Invoke-Control 'FreezeButton'
    $null = Wait-Smoke 'resume changes live statistics' {
        $stats = Get-StatsFingerprint
        $stats -ne '' -and $stats -ne $beforeResume
    }
    Write-Host 'PASS clear search and resumed live statistics.'
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
            Write-Host "Exit evidence: PID $($script:app.Id), HasExited=$($script:app.HasExited), ExitCode=$($script:app.ExitCode)."
            if ($script:app.ExitCode -ne 0) { throw "Bundle PID $($script:app.Id) exited with code $($script:app.ExitCode)." }
        } catch {
            $cleanupFailure = "Natural-close cleanup failed for PID $($script:app.Id): $($_.Exception.Message) Manual intervention may be required; no termination attempted."
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
Write-Host 'PASS portable bundle smoke (including natural exit).'
