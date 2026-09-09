<#
.SYNOPSIS
Smoke-tests the PDF viewer's Windows portable bundle through UI Automation.
.DESCRIPTION
Requires Windows PowerShell 5.1, STA, and an unlocked interactive desktop.
The test starts the no-argument sample mode, verifies a rendered page and its
bookmarks pane, jumps via a table-of-contents entry, moves to the next page,
searches for a later-page phrase, then requests a normal close. Pass -PdfPath
to exercise a real document such as [MS-RDPBCGR].pdf.
#>
[CmdletBinding()]
param(
    [string] $BundlePath = '',
    [string] $PdfPath = '',
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
    $BundlePath = Join-Path $PSScriptRoot '..\artifacts\win-x64\pdf-viewer.exe'
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

function Invoke-Control([string] $Name) {
    $control = Find-Control $Name
    if ($null -eq $control) { throw "Control '$Name' not found." }
    try {
        $control.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
        return
    } catch {
        $child = $control.FindFirst(
            [Windows.Automation.TreeScope]::Descendants,
            [Windows.Automation.PropertyCondition]::new(
                [Windows.Automation.AutomationElement]::IsInvokePatternAvailableProperty, $true))
        if ($null -eq $child) { throw "Control '$Name' is not invokable." }
        $child.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
    }
}

function Invoke-Bookmark([string] $Title) {
    $tree = Find-Control 'OutlineTree'
    if ($null -eq $tree) { throw 'OutlineTree not found.' }
    $button = $tree.FindFirst(
        [Windows.Automation.TreeScope]::Descendants,
        [Windows.Automation.AndCondition]::new(
            [Windows.Automation.PropertyCondition]::new(
                [Windows.Automation.AutomationElement]::ControlTypeProperty,
                [Windows.Automation.ControlType]::Button),
            [Windows.Automation.PropertyCondition]::new(
                [Windows.Automation.AutomationElement]::NameProperty, $Title)))
    if ($null -eq $button) { throw "Bookmark '$Title' not found." }
    $button.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
}

function Set-Value([string] $Name, [string] $Value) {
    $control = Find-Control $Name
    if ($null -eq $control) { throw "Control '$Name' not found." }
    $pattern = $control.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern)
    $pattern.SetValue($Value)
    if ($pattern.Current.Value -ne $Value) {
        throw "Control '$Name' did not accept the requested value."
    }
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
    if ($PdfPath) {
        $start.Arguments = '"' + (Resolve-Path -LiteralPath $PdfPath).ProviderPath + '"'
    }
    $start.EnvironmentVariables.Remove('AVN_HOST_NATIVE_LIB')
    $script:app = [Diagnostics.Process]::Start($start)
    $script:window = Wait-Condition 'main window' {
        $script:app.Refresh()
        if ($script:app.MainWindowHandle -ne [IntPtr]::Zero) {
            $candidate = [Windows.Automation.AutomationElement]::FromHandle($script:app.MainWindowHandle)
            if ($candidate.Current.ProcessId -eq $script:app.Id) { return $candidate }
        }
    }

    $null = Wait-Condition 'first rendered page' {
        (Get-Text 'PageLabel') -match '^Page 1 of (?!1$)[1-9][0-9]*$' -and
            $null -ne (Find-Control 'PageImage') -and
            (
                ($PdfPath -and (Get-Text 'PageText') -match 'Remote Desktop|RDP|MS-RDPBCGR') -or
                (-not $PdfPath -and (Get-Text 'PageText') -match 'Rustolonia PDF Viewer')
            )
    }
    Write-Host 'PASS startup: PDF rendered and extracted text is visible.'

    $null = Wait-Condition 'bookmarks pane' {
        (Get-Text 'OutlineStatus') -match 'bookmark' -and $null -ne (Find-Control 'OutlineTree')
    }
    $bookmarkName = if ($PdfPath -match 'RDPBCGR') { '1 Introduction' } else { 'Sample section 2' }
    $bookmarkPage = if ($PdfPath -match 'RDPBCGR') { 'Page 17 of ' } else { 'Page 2 of ' }
    Invoke-Bookmark $bookmarkName
    $null = Wait-Condition 'bookmark navigation' { (Get-Text 'PageLabel') -match "^$bookmarkPage" }
    Write-Host "PASS bookmarks: table of contents jumped to $bookmarkName."

    if (-not $PdfPath) {
        Invoke-Control 'PreviousPageButton'
        $null = Wait-Condition 'bookmark return' { (Get-Text 'PageLabel') -match '^Page 1 of ' }

        Invoke-Control 'NextPageButton'
        $null = Wait-Condition 'next page navigation' { (Get-Text 'PageLabel') -match '^Page 2 of ' }
        Write-Host 'PASS navigation: next page rendered.'

        Invoke-Control 'PreviousPageButton'
        $null = Wait-Condition 'previous page navigation' { (Get-Text 'PageLabel') -match '^Page 1 of ' }
        Write-Host 'PASS navigation: previous page rendered.'
    }

    $searchQuery = if ($PdfPath) { 'Introduction' } else { 'Page 4' }
    Set-Value 'SearchBox' $searchQuery
    Invoke-Control 'SearchButton'
    $null = Wait-Condition 'search result navigation' {
        (Get-Text 'SearchMatchLabel') -match '^[1-9][0-9]* of [1-9][0-9]*$' -and
            (
                ($PdfPath -and (Get-Text 'SearchStatus') -match 'match') -or
                (-not $PdfPath -and (Get-Text 'PageLabel') -match '^Page [2-9][0-9]* of ')
            )
    }
    Write-Host "PASS search: '$searchQuery' selected on $((Get-Text 'PageLabel'))."
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
Write-Host 'PASS PDF viewer portable bundle smoke.'
