#Requires -Version 7.0
Set-StrictMode -Version Latest

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

$script:StaticDepsDir = $PSScriptRoot
$script:StaticDepsRepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$script:DefaultStaticDepsRepository = 'Devolutions/rustolonia'
$script:StaticDepsComponents = @('angle', 'skiasharp')

$script:StaticDepsRids = @{
    angle     = @('win-x64', 'win-arm64')
    skiasharp = @('win-x64', 'win-arm64', 'linux-x64', 'linux-arm64', 'osx-x64', 'osx-arm64')
}

$script:StaticDepsRecipePaths = @{
    angle     = @(
        'rust/static-deps/common.ps1',
        'rust/static-deps/artifacts.ps1',
        'rust/static-deps/angle',
        '.github/workflows/build-angle.yml'
    )
    skiasharp = @(
        'rust/static-deps/common.ps1',
        'rust/static-deps/artifacts.ps1',
        'rust/static-deps/skiasharp',
        'rust/static-skia-linux.gn',
        'rust/build-linux-static-graphics.sh',
        'rust/collect-linux-graphics-notices.sh',
        '.github/workflows/build-skiasharp.yml'
    )
}

$script:StaticDepsProvenanceFile = @{
    angle     = 'STATIC-ANGLE-PROVENANCE.json'
    skiasharp = 'STATIC-SKIASHARP-PROVENANCE.json'
}

function Get-BytesSha256 {
    param([Parameter(Mandatory)][byte[]]$Bytes)
    $hash = [System.Security.Cryptography.SHA256]::HashData($Bytes)
    return [System.Convert]::ToHexString($hash).ToLowerInvariant()
}

function Get-FileSha256Hex {
    param([Parameter(Mandatory)][string]$Path)
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $hash = [System.Security.Cryptography.SHA256]::HashData($stream)
        return [System.Convert]::ToHexString($hash).ToLowerInvariant()
    }
    finally {
        $stream.Dispose()
    }
}

function Write-CanonicalJsonValue {
    param(
        [Parameter(Mandatory)][System.Text.Json.Utf8JsonWriter]$Writer,
        $Value
    )

    if ($null -eq $Value) {
        $Writer.WriteNullValue()
        return
    }
    if ($Value -is [bool]) {
        $Writer.WriteBooleanValue($Value)
        return
    }
    if ($Value -is [string]) {
        $Writer.WriteStringValue($Value)
        return
    }
    if ($Value -is [int] -or $Value -is [long] -or $Value -is [int16] -or $Value -is [byte]) {
        $Writer.WriteNumberValue([long]$Value)
        return
    }
    if ($Value -is [double] -or $Value -is [single] -or $Value -is [decimal]) {
        $Writer.WriteNumberValue([double]$Value)
        return
    }
    if ($Value -is [System.Collections.IDictionary]) {
        $Writer.WriteStartObject()
        $keys = @($Value.Keys | ForEach-Object { [string]$_ })
        [Array]::Sort($keys, [System.StringComparer]::Ordinal)
        foreach ($key in $keys) {
            $Writer.WritePropertyName($key)
            Write-CanonicalJsonValue -Writer $Writer -Value $Value[$key]
        }
        $Writer.WriteEndObject()
        return
    }
    if ($Value -is [System.Management.Automation.PSCustomObject]) {
        $Writer.WriteStartObject()
        $props = @($Value.PSObject.Properties | Where-Object { $_.MemberType -eq 'NoteProperty' } | ForEach-Object { $_.Name })
        [Array]::Sort($props, [System.StringComparer]::Ordinal)
        foreach ($name in $props) {
            $Writer.WritePropertyName($name)
            Write-CanonicalJsonValue -Writer $Writer -Value $Value.$name
        }
        $Writer.WriteEndObject()
        return
    }
    if ($Value -is [System.Collections.IEnumerable]) {
        $Writer.WriteStartArray()
        foreach ($item in $Value) {
            Write-CanonicalJsonValue -Writer $Writer -Value $item
        }
        $Writer.WriteEndArray()
        return
    }
    $Writer.WriteStringValue([string]$Value)
}

function ConvertTo-CanonicalJsonBytes {
    param($Value)

    $stream = [System.IO.MemoryStream]::new()
    try {
        $options = [System.Text.Json.JsonWriterOptions]::new()
        $options.Indented = $false
        $options.Encoder = [System.Text.Encodings.Web.JavaScriptEncoder]::UnsafeRelaxedJsonEscaping
        $writer = [System.Text.Json.Utf8JsonWriter]::new($stream, $options)
        try {
            Write-CanonicalJsonValue -Writer $writer -Value $Value
        }
        finally {
            $writer.Dispose()
        }
        return $stream.ToArray()
    }
    finally {
        $stream.Dispose()
    }
}

function Write-StaticDepsJson {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)]$Value
    )

    $parent = Split-Path -Parent $Path
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $stream = [System.IO.MemoryStream]::new()
    try {
        $options = [System.Text.Json.JsonWriterOptions]::new()
        $options.Indented = $true
        $options.Encoder = [System.Text.Encodings.Web.JavaScriptEncoder]::UnsafeRelaxedJsonEscaping
        $writer = [System.Text.Json.Utf8JsonWriter]::new($stream, $options)
        try {
            Write-CanonicalJsonValue -Writer $writer -Value $Value
        }
        finally {
            $writer.Dispose()
        }
        $text = [System.Text.Encoding]::UTF8.GetString($stream.ToArray()).Replace("`r`n", "`n") + "`n"
        [System.IO.File]::WriteAllText($Path, $text, [System.Text.UTF8Encoding]::new($false))
    }
    finally {
        $stream.Dispose()
    }
}

function Read-StaticDepsJson {
    param([Parameter(Mandatory)][string]$Path)
    return (Get-Content -LiteralPath $Path -Raw -Encoding utf8 | ConvertFrom-Json -AsHashtable)
}

function Get-StaticDepsSourceConfig {
    param(
        [Parameter(Mandatory)][ValidateSet('angle', 'skiasharp')][string]$Component,
        [string]$RepoRoot = $script:StaticDepsRepoRoot
    )
    $path = Join-Path $RepoRoot 'rust' 'static-deps' $Component 'source.json'
    return Read-StaticDepsJson -Path $path
}

function Get-StaticDepsRecipeHashes {
    param(
        [Parameter(Mandatory)][ValidateSet('angle', 'skiasharp')][string]$Component,
        [string]$RepoRoot = $script:StaticDepsRepoRoot
    )

    $resolvedRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
    $files = [ordered]@{}
    $discovered = [System.Collections.Generic.List[string]]::new()
    foreach ($entry in $script:StaticDepsRecipePaths[$Component]) {
        $full = Join-Path $resolvedRoot $entry
        if (Test-Path -LiteralPath $full -PathType Container) {
            foreach ($item in Get-ChildItem -LiteralPath $full -File -Recurse) {
                $rel = [System.IO.Path]::GetRelativePath($resolvedRoot, $item.FullName).Replace('\', '/')
                $parts = $rel -split '/'
                if ($parts -contains 'bin' -or $parts -contains 'obj' -or $parts -contains '__pycache__' -or $item.Extension -eq '.pyc') {
                    continue
                }
                $discovered.Add($rel)
            }
        }
        elseif (Test-Path -LiteralPath $full -PathType Leaf) {
            $rel = [System.IO.Path]::GetRelativePath($resolvedRoot, $full).Replace('\', '/')
            $discovered.Add($rel)
        }
        else {
            throw "Recipe input is missing for component '$Component': $entry"
        }
    }
    $sorted = @($discovered | Select-Object -Unique)
    [Array]::Sort($sorted, [System.StringComparer]::Ordinal)
    foreach ($rel in $sorted) {
        $files[$rel] = Get-FileSha256Hex -Path (Join-Path $resolvedRoot $rel)
    }
    return $files
}

function Get-StaticDepsIdentity {
    param(
        [Parameter(Mandatory)][ValidateSet('angle', 'skiasharp')][string]$Component,
        [string]$Commit,
        [string]$Version,
        [string]$RepoRoot = $script:StaticDepsRepoRoot
    )

    if (-not $Commit -or -not $Version) {
        $config = Get-StaticDepsSourceConfig -Component $Component -RepoRoot $RepoRoot
        if (-not $Commit) { $Commit = [string]$config['commit'] }
        if (-not $Version) {
            $Version = if ($config.ContainsKey('version') -and $config['version']) {
                [string]$config['version']
            } else {
                ([string]$config['ref']).Replace('/', '-')
            }
        }
    }
    if ($Commit -notmatch '^[0-9a-f]{40}$') {
        throw "Component '$Component' requires an exact 40-character lowercase upstream commit; found '$Commit'."
    }
    $manifest = [ordered]@{
        schema    = 1
        component = $Component
        commit    = $Commit
        version   = $Version
        recipe    = Get-StaticDepsRecipeHashes -Component $Component -RepoRoot $RepoRoot
    }
    $hash = Get-BytesSha256 -Bytes (ConvertTo-CanonicalJsonBytes -Value $manifest)
    $manifest['tag'] = "native-$Component-$($hash.Substring(0, 32))"
    return $manifest
}

function Get-StaticDepsRequiredFiles {
    param([Parameter(Mandatory)][ValidateSet('angle', 'skiasharp')][string]$Component)

    $required = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($rid in $script:StaticDepsRids[$Component]) {
        $names = if ($Component -eq 'angle') {
            @('libANGLE_static.lib', 'libGLESv2_static.lib', 'STATIC-ANGLE-NOTICES.txt')
        }
        elseif ($rid.StartsWith('win-')) {
            @('skia.lib', 'SkiaSharp.lib', 'libHarfBuzzSharp.lib')
        }
        elseif ($rid.StartsWith('linux-')) {
            @('libskia.a', 'libSkiaSharp.a', 'libHarfBuzzSharp.a', 'STATIC-SKIASHARP-PROVENANCE.json', 'STATIC-SKIASHARP-SOURCE-NOTICES.txt')
        }
        else {
            @('libskia.a', 'libSkiaSharp.a', 'libHarfBuzzSharp.a')
        }
        foreach ($name in $names) {
            $null = $required.Add("$rid/$name")
        }
    }
    return $required
}

function Assert-StaticDepsManifest {
    param([Parameter(Mandatory)][System.Collections.IDictionary]$Manifest)

    foreach ($requiredKey in @('schema', 'component', 'commit', 'version', 'recipe', 'tag')) {
        if (-not $Manifest.Contains($requiredKey)) {
            throw "Component manifest is missing required field '$requiredKey'."
        }
    }
    $component = [string]$Manifest['component']
    if ([int]$Manifest['schema'] -ne 1 -or $component -notin $script:StaticDepsComponents) {
        throw "Unsupported component manifest (schema=$($Manifest['schema']), component=$component)."
    }
    $seed = [ordered]@{
        schema    = [int]$Manifest['schema']
        component = $component
        commit    = [string]$Manifest['commit']
        version   = [string]$Manifest['version']
        recipe    = $Manifest['recipe']
    }
    $expectedTag = "native-$component-$((Get-BytesSha256 -Bytes (ConvertTo-CanonicalJsonBytes -Value $seed)).Substring(0, 32))"
    if ([string]$Manifest['tag'] -ne $expectedTag) {
        throw "Component identity mismatch: manifest tag '$($Manifest['tag'])' != expected '$expectedTag'."
    }
    if ($Manifest.Contains('files') -and $null -ne $Manifest['files']) {
        $required = Get-StaticDepsRequiredFiles -Component $component
        $missing = @($required | Where-Object { -not $Manifest['files'].Contains($_) })
        if ($missing.Count -gt 0) {
            [Array]::Sort($missing, [System.StringComparer]::Ordinal)
            throw "Component '$component' is missing required files: $($missing -join ', ')."
        }
    }
}

function Get-DirectoryInventory {
    param([Parameter(Mandatory)][string]$Directory)

    $resolved = (Resolve-Path -LiteralPath $Directory).Path
    $files = @(Get-ChildItem -LiteralPath $resolved -File -Recurse)
    $relPaths = @($files | ForEach-Object { [System.IO.Path]::GetRelativePath($resolved, $_.FullName).Replace('\', '/') })
    [Array]::Sort($relPaths, [System.StringComparer]::Ordinal)
    $result = [ordered]@{}
    foreach ($rel in $relPaths) {
        $result[$rel] = Get-FileSha256Hex -Path (Join-Path $resolved $rel)
    }
    return $result
}

function Get-StaticDepsProducer {
    return [ordered]@{
        repository = if ($env:GITHUB_REPOSITORY) { $env:GITHUB_REPOSITORY } else { 'local' }
        commit     = if ($env:GITHUB_SHA) { $env:GITHUB_SHA } else { 'local' }
        workflow   = if ($env:GITHUB_WORKFLOW_REF) { $env:GITHUB_WORKFLOW_REF } else { 'local' }
        run_id     = if ($env:GITHUB_RUN_ID) { $env:GITHUB_RUN_ID } else { 'local' }
    }
}

function New-StaticDepsBundle {
    param(
        [Parameter(Mandatory)][System.Collections.IDictionary]$Manifest,
        [Parameter(Mandatory)][string]$IncomingDir,
        [Parameter(Mandatory)][string]$OutputDir
    )

    Assert-StaticDepsManifest -Manifest $Manifest
    $component = [string]$Manifest['component']
    $staging = Join-Path ([System.IO.Path]::GetTempPath()) ("rustolonia-bundle-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $staging | Out-Null
    try {
        foreach ($rid in $script:StaticDepsRids[$component]) {
            $source = Join-Path $IncomingDir "native-$component-$rid"
            if ($component -eq 'skiasharp' -and $rid.StartsWith('osx-') -and -not (Test-Path -LiteralPath $source -PathType Container)) {
                $source = Join-Path $IncomingDir 'native-skiasharp-osx' $rid
            }
            if (-not (Test-Path -LiteralPath $source -PathType Container)) {
                throw "Missing incoming artifact directory: $source"
            }
            $dest = Join-Path $staging $rid
            New-Item -ItemType Directory -Force -Path $dest | Out-Null
            Copy-Item -Path (Join-Path $source '*') -Destination $dest -Recurse -Force
        }

        $bundled = [ordered]@{}
        foreach ($key in $Manifest.Keys) { $bundled[$key] = $Manifest[$key] }
        $bundled['files'] = Get-DirectoryInventory -Directory $staging
        $bundled['producer'] = Get-StaticDepsProducer
        Assert-StaticDepsManifest -Manifest $bundled

        New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
        $archiveName = "$component-native.zip"
        $archivePath = Join-Path $OutputDir $archiveName
        if (Test-Path -LiteralPath $archivePath) { Remove-Item -LiteralPath $archivePath -Force }

        $zipStream = [System.IO.File]::Create($archivePath)
        try {
            $zip = [System.IO.Compression.ZipArchive]::new($zipStream, [System.IO.Compression.ZipArchiveMode]::Create)
            try {
                $fixedTime = [System.DateTimeOffset]::new(1980, 1, 1, 0, 0, 0, [System.TimeSpan]::Zero)
                $entryNames = @($bundled['files'].Keys)
                [Array]::Sort($entryNames, [System.StringComparer]::Ordinal)
                foreach ($name in $entryNames) {
                    $entry = $zip.CreateEntry($name, [System.IO.Compression.CompressionLevel]::Optimal)
                    $entry.LastWriteTime = $fixedTime
                    $bytes = [System.IO.File]::ReadAllBytes((Join-Path $staging $name))
                    $entryStream = $entry.Open()
                    try {
                        $entryStream.Write($bytes, 0, $bytes.Length)
                    }
                    finally {
                        $entryStream.Dispose()
                    }
                }
            }
            finally {
                $zip.Dispose()
            }
        }
        finally {
            $zipStream.Dispose()
        }

        $bundled['archive'] = $archiveName
        $bundled['sha256'] = Get-FileSha256Hex -Path $archivePath
        Write-StaticDepsJson -Path (Join-Path $OutputDir 'component.json') -Value $bundled
        return $bundled
    }
    finally {
        Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Expand-StaticDepsBundle {
    param(
        [Parameter(Mandatory)][System.Collections.IDictionary]$Manifest,
        [Parameter(Mandatory)][string]$ArchivePath,
        [Parameter(Mandatory)][string]$DestinationDir
    )

    Assert-StaticDepsManifest -Manifest $Manifest
    $component = [string]$Manifest['component']
    $actualArchiveSha = Get-FileSha256Hex -Path $ArchivePath
    if ($actualArchiveSha -ne [string]$Manifest['sha256']) {
        throw "Component archive checksum mismatch: expected '$($Manifest['sha256'])', got '$actualArchiveSha'."
    }

    New-Item -ItemType Directory -Force -Path $DestinationDir | Out-Null
    $zip = [System.IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        $entryNames = @($zip.Entries | ForEach-Object { $_.FullName })
        $expectedNames = @($Manifest['files'].Keys)
        $uniqueEntryNames = @($entryNames | Select-Object -Unique)
        if ($entryNames.Count -ne $uniqueEntryNames.Count -or $entryNames.Count -ne $expectedNames.Count) {
            throw 'Component archive inventory mismatch.'
        }
        foreach ($name in $entryNames) {
            if (-not $Manifest['files'].Contains($name)) {
                throw "Component archive contains unexpected entry: $name"
            }
            if ($name.StartsWith('/') -or $name.Contains('\') -or $name.Contains(':') -or (($name -split '/') -contains '..')) {
                throw "Unsafe component archive path: $name"
            }
        }
        foreach ($entry in $zip.Entries) {
            $ms = [System.IO.MemoryStream]::new()
            try {
                $es = $entry.Open()
                try { $es.CopyTo($ms) } finally { $es.Dispose() }
                $bytes = $ms.ToArray()
            }
            finally {
                $ms.Dispose()
            }
            $actualHash = Get-BytesSha256 -Bytes $bytes
            $expectedHash = [string]$Manifest['files'][$entry.FullName]
            if ($actualHash -ne $expectedHash) {
                throw "Component file checksum mismatch for '$($entry.FullName)': expected '$expectedHash', got '$actualHash'."
            }
            $target = Join-Path $DestinationDir $entry.FullName
            if (Test-Path -LiteralPath $target -PathType Leaf) {
                $existingHash = Get-FileSha256Hex -Path $target
                if ($existingHash -ne $actualHash) {
                    throw "Components contain conflicting files: $($entry.FullName)"
                }
            }
            $parent = Split-Path -Parent $target
            if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
            [System.IO.File]::WriteAllBytes($target, $bytes)
        }
    }
    finally {
        $zip.Dispose()
    }

    Write-StaticDepsJson -Path (Join-Path $DestinationDir "$component.json") -Value $Manifest
    $provName = $script:StaticDepsProvenanceFile[$component]
    foreach ($rid in $script:StaticDepsRids[$component]) {
        $ridDir = Join-Path $DestinationDir $rid
        if (Test-Path -LiteralPath $ridDir -PathType Container) {
            $provPath = Join-Path $ridDir $provName
            # Preserve per-RID provenance already embedded in the archive (e.g. Linux glibc 2.35).
            if (-not (Test-Path -LiteralPath $provPath -PathType Leaf)) {
                Write-StaticDepsJson -Path $provPath -Value $Manifest
            }
        }
    }
    return $Manifest
}

function Get-StaticDepsRepository {
    if ($env:GITHUB_REPOSITORY) { return $env:GITHUB_REPOSITORY }
    return $script:DefaultStaticDepsRepository
}

function Invoke-GhCapture {
    param([Parameter(Mandatory)][string[]]$Arguments)

    $previous = $PSNativeCommandUseErrorActionPreference
    $PSNativeCommandUseErrorActionPreference = $false
    try {
        $stderrFile = [System.IO.Path]::GetTempFileName()
        try {
            $stdout = (& gh @Arguments 2>$stderrFile) -join "`n"
            $code = $LASTEXITCODE
            $global:LASTEXITCODE = 0
            $stderr = if (Test-Path -LiteralPath $stderrFile) { Get-Content -LiteralPath $stderrFile -Raw } else { '' }
            return [pscustomobject]@{ ExitCode = $code; StdOut = $stdout; StdErr = $stderr }
        }
        finally {
            Remove-Item -LiteralPath $stderrFile -Force -ErrorAction SilentlyContinue
        }
    }
    finally {
        $PSNativeCommandUseErrorActionPreference = $previous
    }
}

function Get-StaticDepsRelease {
    param([Parameter(Mandatory)][string]$Tag)

    $repo = Get-StaticDepsRepository
    $escaped = [System.Uri]::EscapeDataString($Tag)
    $res = Invoke-GhCapture -Arguments @('api', "repos/$repo/releases/tags/$escaped")
    if ($res.ExitCode -ne 0) {
        if ($res.StdErr -match 'HTTP 404') { return $null }
        throw "Failed to query GitHub release '$Tag': $($res.StdErr)"
    }
    $value = $res.StdOut | ConvertFrom-Json
    if ($value.draft) {
        throw "Draft release '$Tag' is not a reusable build input."
    }
    return $value
}

function Receive-StaticDepsReleaseAsset {
    param(
        [Parameter(Mandatory)][string]$Tag,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Directory
    )

    if ([System.IO.Path]::GetFileName($Name) -ne $Name -or $Name.StartsWith('-')) {
        throw "Invalid release asset name: $Name"
    }
    $repo = Get-StaticDepsRepository
    $res = Invoke-GhCapture -Arguments @('release', 'download', $Tag, '--repo', $repo, '--pattern', $Name, '--dir', $Directory, '--clobber')
    if ($res.ExitCode -ne 0) {
        throw "Failed to download asset '$Name' from release '$Tag': $($res.StdErr)"
    }
    return (Join-Path $Directory $Name)
}

function Test-StaticDepsCachedRelease {
    param([Parameter(Mandatory)][System.Collections.IDictionary]$Expected)

    $tag = [string]$Expected['tag']
    $existing = Get-StaticDepsRelease -Tag $tag
    if ($null -eq $existing) { return $false }

    $archiveName = "$($Expected['component'])-native.zip"
    $assetNames = @($existing.assets | ForEach-Object { $_.name })
    if (($assetNames -notcontains 'component.json') -or ($assetNames -notcontains $archiveName)) {
        throw "Existing component release '$tag' is incomplete."
    }
    $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("rustolonia-probe-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    try {
        $manifestPath = Receive-StaticDepsReleaseAsset -Tag $tag -Name 'component.json' -Directory $tmp
        $actual = Read-StaticDepsJson -Path $manifestPath
        Assert-StaticDepsManifest -Manifest $actual
        foreach ($key in $Expected.Keys) {
            $expectedBytes = ConvertTo-CanonicalJsonBytes -Value $Expected[$key]
            $actualBytes = ConvertTo-CanonicalJsonBytes -Value $actual[$key]
            if ((Get-BytesSha256 -Bytes $expectedBytes) -ne (Get-BytesSha256 -Bytes $actualBytes)) {
                throw "Existing component release '$tag' has a different identity for key '$key'."
            }
        }
        return $true
    }
    finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Publish-StaticDepsRelease {
    param(
        [Parameter(Mandatory)][string]$Tag,
        [Parameter(Mandatory)][string[]]$Files,
        [Parameter(Mandatory)][string]$Title,
        [Parameter(Mandatory)][string]$Notes,
        [string]$TargetCommit
    )

    $existing = Get-StaticDepsRelease -Tag $Tag
    if ($null -ne $existing) {
        $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("rustolonia-verify-" + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Force -Path $tmp | Out-Null
        try {
            foreach ($file in $Files) {
                $name = [System.IO.Path]::GetFileName($file)
                $downloaded = Receive-StaticDepsReleaseAsset -Tag $Tag -Name $name -Directory $tmp
                if ((Get-FileSha256Hex -Path $downloaded) -ne (Get-FileSha256Hex -Path $file)) {
                    throw "Refusing to replace existing release asset: $name"
                }
            }
            return
        }
        finally {
            Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    $commit = if ($TargetCommit) { $TargetCommit } else { $env:GITHUB_SHA }
    if (-not $commit) {
        throw 'Release persistence requires a GitHub workflow commit (GITHUB_SHA).'
    }
    $repo = Get-StaticDepsRepository
    $argsList = @('release', 'create', $Tag) + $Files + @('--repo', $repo, '--target', $commit, '--title', $Title, '--notes', $Notes, '--prerelease', '--latest=false')
    $res = Invoke-GhCapture -Arguments $argsList
    if ($res.ExitCode -ne 0) {
        throw "Failed to create release '$Tag': $($res.StdErr)"
    }
}

function Get-CoffArchiveMachine {
    param([Parameter(Mandatory)][string]$Path)

    $data = [System.IO.File]::ReadAllBytes($Path)
    $magic = [System.Text.Encoding]::ASCII.GetBytes("!<arch>`n")
    if ($data.Length -lt $magic.Length) { throw "Not a COFF archive: $Path" }
    for ($i = 0; $i -lt $magic.Length; $i++) {
        if ($data[$i] -ne $magic[$i]) { throw "Not a COFF archive: $Path" }
    }
    $known = [System.Collections.Generic.HashSet[int]]::new([int[]]@(0x8664, 0xAA64, 0x014C, 0x01C0))
    $machines = [System.Collections.Generic.HashSet[int]]::new()
    $offset = 8
    while ($offset + 60 -le $data.Length) {
        $name = [System.Text.Encoding]::ASCII.GetString($data, $offset, 16).Trim()
        $sizeText = [System.Text.Encoding]::ASCII.GetString($data, $offset + 48, 10).Trim()
        $size = [int]$sizeText
        $memberStart = $offset + 60
        if ($name -notin @('/', '//', '/SYM64/') -and $size -ge 8 -and ($memberStart + 8) -le $data.Length) {
            $isAnon = ($data[$memberStart] -eq 0 -and $data[$memberStart + 1] -eq 0 -and $data[$memberStart + 2] -eq 0xFF -and $data[$memberStart + 3] -eq 0xFF)
            $machineOffset = if ($isAnon) { $memberStart + 6 } else { $memberStart }
            $machine = [int][System.BitConverter]::ToUInt16($data, $machineOffset)
            if ($known.Contains($machine)) {
                $null = $machines.Add($machine)
            }
        }
        $offset = $memberStart + $size + ($size % 2)
    }
    if ($machines.Count -ne 1) {
        $found = ($machines | ForEach-Object { '0x{0:X4}' -f $_ }) -join ', '
        throw "Expected one COFF architecture in $Path, found [$found]."
    }
    return @($machines)[0]
}

function Get-MachOArchiveCpuType {
    param([Parameter(Mandatory)][string]$Path)

    $data = [System.IO.File]::ReadAllBytes($Path)
    $magic = [System.Text.Encoding]::ASCII.GetBytes("!<arch>`n")
    if ($data.Length -lt $magic.Length) { throw "Not a static archive: $Path" }
    for ($i = 0; $i -lt $magic.Length; $i++) {
        if ($data[$i] -ne $magic[$i]) { throw "Not a static archive: $Path" }
    }
    $cputypes = [System.Collections.Generic.HashSet[uint32]]::new()
    $offset = 8
    while ($offset + 60 -le $data.Length) {
        $name = [System.Text.Encoding]::ASCII.GetString($data, $offset, 16).Trim()
        $sizeText = [System.Text.Encoding]::ASCII.GetString($data, $offset + 48, 10).Trim()
        $size = [int]$sizeText
        $payloadStart = $offset + 60
        $payloadLen = $size
        if ($name.StartsWith('#1/')) {
            $nameLen = [int]$name.Substring(3)
            $payloadStart += $nameLen
            $payloadLen -= $nameLen
        }
        if ($payloadLen -ge 8 -and ($payloadStart + 8) -le $data.Length) {
            $hdrMagic = [System.BitConverter]::ToUInt32($data, $payloadStart)
            if ($hdrMagic -eq 0xFEEDFACFu -or $hdrMagic -eq 0xFEEDFACEu) {
                $cpu = [System.BitConverter]::ToUInt32($data, $payloadStart + 4)
                $null = $cputypes.Add($cpu)
            }
        }
        $offset = ($offset + 60) + $size + ($size % 2)
    }
    if ($cputypes.Count -ne 1) {
        $found = ($cputypes | ForEach-Object { '0x{0:X8}' -f $_ }) -join ', '
        throw "Expected one Mach-O architecture in $Path, found [$found]."
    }
    return @($cputypes)[0]
}

function Get-AvaloniaPackageVersions {
    param([Parameter(Mandatory)][string]$PackagesPropsPath)

    if (-not (Test-Path -LiteralPath $PackagesPropsPath -PathType Leaf)) {
        throw "Avalonia Directory.Packages.props not found: $PackagesPropsPath"
    }
    [xml]$xml = Get-Content -LiteralPath $PackagesPropsPath -Raw -Encoding utf8
    $props = @{}
    foreach ($pg in $xml.SelectNodes('//*[local-name()="PropertyGroup"]')) {
        foreach ($node in $pg.ChildNodes) {
            if ($node.NodeType -eq [System.Xml.XmlNodeType]::Element -and $node.InnerText) {
                $props[$node.LocalName] = $node.InnerText.Trim()
            }
        }
    }
    $versions = @{}
    $targets = @('SkiaSharp', 'HarfBuzzSharp', 'Avalonia.Angle.Windows.Natives')
    foreach ($el in $xml.SelectNodes('//*[local-name()="PackageVersion"]')) {
        $include = $el.GetAttribute('Include')
        if ($include -notin $targets) { continue }
        $ver = $el.GetAttribute('Version')
        if (-not $ver) {
            $verNode = $el.SelectSingleNode('*[local-name()="Version"]')
            if ($verNode) { $ver = $verNode.InnerText.Trim() }
        }
        if (-not $ver) { continue }
        for ($i = 0; $i -lt 10; $i++) {
            $expanded = [regex]::Replace($ver, '\$\(([^)]+)\)', {
                param($m)
                $k = $m.Groups[1].Value
                if ($props.ContainsKey($k)) { $props[$k] } else { $m.Value }
            })
            if ($expanded -eq $ver) { break }
            $ver = $expanded
        }
        $versions[$include] = $ver
    }
    foreach ($target in $targets) {
        if (-not $versions.ContainsKey($target)) {
            throw "PackageVersion for '$target' was not found in $PackagesPropsPath."
        }
    }
    return $versions
}

function Enter-MsvcEnvironment {
    param([Parameter(Mandatory)][string]$Arch)

    $targetArch = switch ($Arch.ToLowerInvariant()) {
        'amd64_arm64' { 'arm64' }
        'x64_arm64'   { 'arm64' }
        default       { $Arch.ToLowerInvariant() }
    }

    if (-not (Get-Module -ListAvailable -Name VsDevShell)) {
        Set-PSRepository -Name PSGallery -InstallationPolicy Trusted -ErrorAction SilentlyContinue
        Install-Module -Name VsDevShell -Repository PSGallery -Scope CurrentUser -Force -AllowClobber
    }
    Import-Module VsDevShell -Force

    $baselinePath = $env:PATH
    try {
        $vsDevEnv = Get-VsDevEnv -Arch $targetArch -HostArch x64 -NoLogo
    }
    catch {
        $sysRoot = $env:SystemRoot
        $minPath = (@("$sysRoot\System32", $sysRoot, "$sysRoot\System32\Wbem", "$sysRoot\System32\WindowsPowerShell\v1.0") -join ';')
        try {
            $env:PATH = $minPath
            $vsDevEnv = Get-VsDevEnv -Arch $targetArch -HostArch x64 -NoLogo
        }
        finally {
            $env:PATH = $baselinePath
        }
        foreach ($pathKey in @($vsDevEnv.Keys | Where-Object { $_ -ieq 'PATH' })) {
            $vsDevEnv[$pathKey] = ([string]$vsDevEnv[$pathKey]).Replace($minPath, $baselinePath)
        }
    }
    $vsDevEnv | Enter-VsDevShell -Force

    if ($env:GITHUB_ENV) {
        foreach ($entry in $vsDevEnv.GetEnumerator()) {
            $name = [string]$entry.Key
            $val = if ($null -eq $entry.Value) { '' } else { [string]$entry.Value }
            if ([string]::IsNullOrWhiteSpace($name) -or $name.Contains('=')) {
                continue
            }
            $delim = "VSDEVENV_" + [guid]::NewGuid().ToString('N')
            "$name<<$delim`n$val`n$delim" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
        }
    }
}

function Assert-StaticDepsSyncedWithAvalonia {
    param(
        [string]$RepoRoot = $script:StaticDepsRepoRoot,
        [string]$AvaloniaPackagesProps
    )

    $resolvedRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
    if (-not $AvaloniaPackagesProps) {
        $AvaloniaPackagesProps = Join-Path $resolvedRoot 'avalonia-src' 'Directory.Packages.props'
    }
    $errors = [System.Collections.Generic.List[string]]::new()

    $skiaConfig = Get-StaticDepsSourceConfig -Component 'skiasharp' -RepoRoot $resolvedRoot
    $angleConfig = Get-StaticDepsSourceConfig -Component 'angle' -RepoRoot $resolvedRoot

    # Always verify rust/build-linux-static-graphics.sh matches rust/static-deps/skiasharp/source.json.
    $linuxScriptPath = Join-Path $resolvedRoot 'rust' 'build-linux-static-graphics.sh'
    if (Test-Path -LiteralPath $linuxScriptPath -PathType Leaf) {
        $linuxScript = Get-Content -LiteralPath $linuxScriptPath -Raw -Encoding utf8
        if ($linuxScript -match '(?m)^skiasharp_commit=([0-9a-f]{40})\r?$') {
            if ($Matches[1] -ne [string]$skiaConfig['commit']) {
                $errors.Add("rust/build-linux-static-graphics.sh skiasharp_commit ($($Matches[1])) != rust/static-deps/skiasharp/source.json commit ($($skiaConfig['commit'])).")
            }
        }
        else {
            $errors.Add('Cannot find skiasharp_commit in rust/build-linux-static-graphics.sh.')
        }
        if ($linuxScript -match '(?m)^skia_commit=([0-9a-f]{40})\r?$') {
            if ($Matches[1] -ne [string]$skiaConfig['skiaCommit']) {
                $errors.Add("rust/build-linux-static-graphics.sh skia_commit ($($Matches[1])) != rust/static-deps/skiasharp/source.json skiaCommit ($($skiaConfig['skiaCommit'])).")
            }
        }
        else {
            $errors.Add('Cannot find skia_commit in rust/build-linux-static-graphics.sh.')
        }
        if ($linuxScript -match '--branch\s+v([^\s]+)') {
            if ($Matches[1] -ne [string]$skiaConfig['version']) {
                $errors.Add("rust/build-linux-static-graphics.sh SkiaSharp branch (v$($Matches[1])) != rust/static-deps/skiasharp/source.json version (v$($skiaConfig['version'])).")
            }
        }
    }

    if (Test-Path -LiteralPath $AvaloniaPackagesProps -PathType Leaf) {
        $avalonia = Get-AvaloniaPackageVersions -PackagesPropsPath $AvaloniaPackagesProps
        if ([string]$skiaConfig['version'] -ne $avalonia['SkiaSharp']) {
            $errors.Add("SkiaSharp version drift: rust/static-deps/skiasharp/source.json has '$($skiaConfig['version'])', but Avalonia declares '$($avalonia['SkiaSharp'])'. Run ./rust/static-deps/sync-from-avalonia.ps1 -Update.")
        }
        if ([string]$skiaConfig['harfbuzzVersion'] -ne $avalonia['HarfBuzzSharp']) {
            $errors.Add("HarfBuzzSharp version drift: rust/static-deps/skiasharp/source.json has '$($skiaConfig['harfbuzzVersion'])', but Avalonia declares '$($avalonia['HarfBuzzSharp'])'. Run ./rust/static-deps/sync-from-avalonia.ps1 -Update.")
        }
        if ([string]$angleConfig['avaloniaAnglePackageVersion'] -ne $avalonia['Avalonia.Angle.Windows.Natives']) {
            $errors.Add("ANGLE version drift: rust/static-deps/angle/source.json tracks Avalonia.Angle.Windows.Natives '$($angleConfig['avaloniaAnglePackageVersion'])', but Avalonia declares '$($avalonia['Avalonia.Angle.Windows.Natives'])'. Review ANGLE commit/patches and run ./rust/static-deps/sync-from-avalonia.ps1 -Update.")
        }
    }

    if ($errors.Count -gt 0) {
        throw ("Static dependency sync check failed:`n - " + ($errors -join "`n - "))
    }
}

function Write-GitHubWorkflowOutputs {
    param([Parameter(Mandatory)][System.Collections.IDictionary]$Values)

    Write-Output ($Values | ConvertTo-Json -Depth 4)
    if ($env:GITHUB_OUTPUT) {
        $lines = [System.Collections.Generic.List[string]]::new()
        foreach ($key in $Values.Keys) {
            $raw = $Values[$key]
            $rendered = if ($raw -is [bool]) { $raw.ToString().ToLowerInvariant() } else { [string]$raw }
            if ($rendered.Contains("`n") -or $rendered.Contains("`r")) {
                throw "Multiline workflow output for '$key'."
            }
            $lines.Add("$key=$rendered")
        }
        [System.IO.File]::AppendAllLines($env:GITHUB_OUTPUT, $lines, [System.Text.UTF8Encoding]::new($false))
    }
}
