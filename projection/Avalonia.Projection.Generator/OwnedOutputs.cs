using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Avalonia.Projection.Generator;

public sealed class OwnedOutputCheckResult
{
    public IReadOnlyList<string> Mismatches { get; init; } = [];
    public bool Success => Mismatches.Count == 0;
}

public static class OwnedOutputs
{
    public const string ProjectionGeneratorId = "avalonia-projection";
    public const string ViewModelGeneratorId = "avalonia-viewmodel";
    public const string BindgenGeneratorId = "avalonia-bindgen";

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    public static string ManifestFileName(string generatorId)
    {
        if (!IsSafeGeneratorId(generatorId))
            throw new InvalidOperationException($"Unsafe generator id '{generatorId}'.");
        return $".{generatorId}.owned.json";
    }

    public static string HashContent(string content)
    {
        var bytes = Encoding.UTF8.GetBytes(content);
        return Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
    }

    public static bool IsSafeGeneratorId(string generatorId) =>
        !string.IsNullOrWhiteSpace(generatorId)
        && generatorId.Length <= 64
        && generatorId.All(character => character is (>= 'a' and <= 'z') or (>= '0' and <= '9') or '-')
        && generatorId[0] is >= 'a' and <= 'z';

    public static bool IsSafeLeafFileName(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return false;
        if (path.Contains('/') || path.Contains('\\') || path.Contains(':'))
            return false;
        return IsSafeRelativePath(path) && Path.GetFileName(path) == path;
    }

    public static bool IsSafeRelativePath(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return false;
        if (Path.IsPathRooted(path))
            return false;

        var normalized = path.Replace('\\', '/');
        if (normalized.StartsWith('/') || normalized.Contains(':'))
            return false;

        var parts = normalized.Split('/', StringSplitOptions.None);
        return parts.Length > 0 && parts.All(part => part is not "" and not "." and not "..");
    }

    public static void Add(IDictionary<string, string> files, string path, string content)
    {
        ArgumentNullException.ThrowIfNull(files);
        var full = Path.GetFullPath(path);
        foreach (var existing in files.Keys)
        {
            if (string.Equals(Path.GetFullPath(existing), full, StringComparison.OrdinalIgnoreCase))
            {
                throw new InvalidOperationException(
                    $"Duplicate generated output '{full}' collides with '{Path.GetFullPath(existing)}'.");
            }
        }

        files.Add(path, content);
    }

    public static OwnedOutputCheckResult Check(
        string generatorId,
        IReadOnlyDictionary<string, string> files,
        IEnumerable<string>? outputDirectories = null)
    {
        outputDirectories = outputDirectories?.ToArray();
        var mismatches = new List<string>();
        ValidateDestinations(generatorId, files, outputDirectories);

        foreach (var (path, expected) in files.OrderBy(entry => entry.Key, StringComparer.OrdinalIgnoreCase))
        {
            var directory = Path.GetDirectoryName(Path.GetFullPath(path))!;
            var otherOwners = OtherOwners(directory, Path.GetFileName(path), generatorId);
            if (otherOwners.Count > 0)
                mismatches.Add($"CONFLICT: {path} is owned by {string.Join(", ", otherOwners)}");
            if (!File.Exists(path))
            {
                mismatches.Add($"MISSING: {path}");
                continue;
            }

            var actual = File.ReadAllText(path).Replace("\r\n", "\n", StringComparison.Ordinal);
            if (!string.Equals(actual, expected, StringComparison.Ordinal))
                mismatches.Add($"DIFFERENT: {path}");
        }

        foreach (var directory in GroupByDirectory(files, outputDirectories).Keys)
        {
            if (!Directory.Exists(directory))
                continue;

            var manifest = ReadManifest(directory, generatorId);
            if (manifest is null)
                continue;

            var expectedNames = ExpectedRelativeNames(directory, files);
            foreach (var owned in manifest.Files!)
            {
                ValidateManifestEntry(directory, owned);
                if (expectedNames.Contains(owned.Path))
                    continue;

                var fullPath = Path.GetFullPath(Path.Combine(directory, owned.Path));
                if (!File.Exists(fullPath))
                {
                    mismatches.Add($"OBSOLETE: {fullPath}");
                    continue;
                }

                var onDisk = HashBytes(File.ReadAllBytes(fullPath));
                mismatches.Add(
                    onDisk == owned.Sha256
                        ? $"OBSOLETE: {fullPath}"
                        : $"OBSOLETE-MODIFIED: {fullPath}");
            }
        }

        return new OwnedOutputCheckResult { Mismatches = mismatches };
    }

    public static void Write(string generatorId, IReadOnlyDictionary<string, string> files,
        IEnumerable<string>? outputDirectories = null)
    {
        outputDirectories = outputDirectories?.ToArray();
        ValidateDestinations(generatorId, files, outputDirectories);
        var groups = GroupByDirectory(files, outputDirectories);
        var obsolete = new List<string>();

        foreach (var (path, _) in files)
        {
            var directory = Path.GetDirectoryName(Path.GetFullPath(path))!;
            var leaf = Path.GetFileName(path);
            var other = OtherOwners(directory, leaf, generatorId);
            if (other.Count > 0)
            {
                throw new InvalidOperationException(
                    $"Refusing to overwrite '{leaf}' owned by {string.Join(", ", other)}.");
            }
        }

        foreach (var (directory, _) in groups)
        {
            var expectedNames = ExpectedRelativeNames(directory, files);
            var manifest = Directory.Exists(directory) ? ReadManifest(directory, generatorId) : null;
            if (manifest is null)
                continue;

            foreach (var owned in manifest.Files!)
            {
                ValidateManifestEntry(directory, owned);
                if (expectedNames.Contains(owned.Path))
                    continue;

                var fullPath = Path.GetFullPath(Path.Combine(directory, owned.Path));
                if (!File.Exists(fullPath))
                    continue;

                if (OtherOwners(directory, owned.Path, generatorId).Count > 0)
                    continue;

                var onDisk = HashBytes(File.ReadAllBytes(fullPath));
                if (onDisk != owned.Sha256)
                {
                    throw new InvalidOperationException(
                        $"Refusing to delete '{owned.Path}' owned by {generatorId}: file was modified outside the generator.");
                }

                obsolete.Add(fullPath);
            }
        }

        var staged = new List<(string Destination, string Temp)>();
        try
        {
            foreach (var (path, content) in files.OrderBy(entry => entry.Key, StringComparer.OrdinalIgnoreCase))
            {
                var directory = Path.GetDirectoryName(path)
                    ?? throw new InvalidOperationException($"Output path '{path}' has no directory.");
                Directory.CreateDirectory(directory);
                var temp = path + $".owned-tmp-{Guid.NewGuid():N}";
                staged.Add((path, temp));
                File.WriteAllText(temp, content, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
            }
        }
        catch
        {
            DeleteTemps(staged);
            throw;
        }

        var committed = new List<string>();
        try
        {
            foreach (var (destination, temp) in staged)
            {
                File.Move(temp, destination, overwrite: true);
                committed.Add(destination);
            }
        }
        catch (Exception exception)
        {
            DeleteTemps(staged);
            throw new InvalidOperationException(
                $"Partial write failure after updating {committed.Count} file(s): {exception.Message}",
                exception);
        }

        foreach (var path in obsolete)
            File.Delete(path);

        var manifestTemps = new List<(string Destination, string Temp)>();
        try
        {
            foreach (var (directory, directoryFiles) in groups)
            {
                Directory.CreateDirectory(directory);
                var manifest = new OwnedOutputManifest
                {
                    Generator = generatorId,
                    Files = directoryFiles
                        .Select(entry => new OwnedOutputFile
                        {
                            Path = Path.GetFileName(entry.Key),
                            Sha256 = HashContent(entry.Value),
                        })
                        .OrderBy(file => file.Path, StringComparer.OrdinalIgnoreCase)
                        .ToList(),
                };
                var json = JsonSerializer.Serialize(manifest, JsonOptions).Replace("\r\n", "\n", StringComparison.Ordinal) + "\n";
                var destination = Path.Combine(directory, ManifestFileName(generatorId));
                var temp = destination + $".owned-tmp-{Guid.NewGuid():N}";
                manifestTemps.Add((destination, temp));
                File.WriteAllText(temp, json, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
            }

            foreach (var (destination, temp) in manifestTemps)
                File.Move(temp, destination, overwrite: true);
        }
        catch
        {
            DeleteTemps(manifestTemps);
            throw;
        }
    }

    private static void ValidateDestinations(string generatorId, IReadOnlyDictionary<string, string> files,
        IEnumerable<string>? outputDirectories)
    {
        if (!IsSafeGeneratorId(generatorId))
            throw new InvalidOperationException($"Unsafe generator id '{generatorId}'.");
        if (files.Count == 0 && (outputDirectories is null || !outputDirectories.Any()))
            throw new InvalidOperationException("Generation produced no outputs.");

        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var path in files.Keys)
        {
            if (string.IsNullOrWhiteSpace(path))
                throw new InvalidOperationException("Generated output path must not be empty.");
            var full = Path.GetFullPath(path);
            if (!seen.Add(full))
                throw new InvalidOperationException($"Duplicate generated output '{full}'.");
            var relative = Path.GetFileName(full);
            if (!IsSafeLeafFileName(relative))
                throw new InvalidOperationException($"Generated output '{path}' is not a safe file name.");
            if (string.Equals(relative, ManifestFileName(generatorId), StringComparison.OrdinalIgnoreCase))
                throw new InvalidOperationException($"Generator '{generatorId}' cannot emit its ownership manifest as content.");
        }
    }

    private static Dictionary<string, List<KeyValuePair<string, string>>> GroupByDirectory(
        IReadOnlyDictionary<string, string> files, IEnumerable<string>? outputDirectories)
    {
        var groups = new Dictionary<string, List<KeyValuePair<string, string>>>(StringComparer.OrdinalIgnoreCase);
        // Keep configured directories in scope even when their last generated file was removed.
        foreach (var directory in outputDirectories ?? [])
        {
            if (string.IsNullOrWhiteSpace(directory))
                throw new InvalidOperationException("Generated output directory must not be empty.");
            groups.TryAdd(Path.GetFullPath(directory), []);
        }
        foreach (var entry in files)
        {
            var directory = Path.GetDirectoryName(Path.GetFullPath(entry.Key))
                ?? throw new InvalidOperationException($"Output path '{entry.Key}' has no directory.");
            if (!groups.TryGetValue(directory, out var list))
            {
                list = [];
                groups[directory] = list;
            }

            list.Add(entry);
        }

        return groups;
    }

    private static HashSet<string> ExpectedRelativeNames(
        string directory,
        IReadOnlyDictionary<string, string> files)
    {
        var names = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var path in files.Keys)
        {
            if (string.Equals(Path.GetDirectoryName(Path.GetFullPath(path)), directory, StringComparison.OrdinalIgnoreCase))
                names.Add(Path.GetFileName(path));
        }

        return names;
    }

    private static OwnedOutputManifest? ReadManifest(string directory, string generatorId)
    {
        var path = Path.Combine(directory, ManifestFileName(generatorId));
        if (!File.Exists(path))
            return null;

        var parsed = JsonSerializer.Deserialize<OwnedOutputManifest>(File.ReadAllText(path), JsonOptions)
            ?? throw new InvalidOperationException($"Ownership manifest '{path}' deserialized to null.");
        ValidateManifest(directory, generatorId, parsed);
        return parsed;
    }

    private static void ValidateManifest(string directory, string generatorId, OwnedOutputManifest parsed)
    {
        if (!IsSafeGeneratorId(parsed.Generator))
            throw new InvalidOperationException($"Ownership manifest in '{directory}' has an unsafe generator id '{parsed.Generator}'.");
        if (!string.Equals(parsed.Generator, generatorId, StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                $"Ownership manifest in '{directory}' belongs to '{parsed.Generator}', not '{generatorId}'.");
        }

        if (parsed.Files is null)
            throw new InvalidOperationException($"Ownership manifest in '{directory}' has a null files list.");

        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        for (var index = 0; index < parsed.Files.Count; index++)
        {
            var owned = parsed.Files[index]
                ?? throw new InvalidOperationException($"Ownership manifest in '{directory}' contains a null files[{index}] element.");
            ValidateManifestEntry(directory, owned);
            if (!seen.Add(owned.Path))
            {
                throw new InvalidOperationException(
                    $"Ownership manifest in '{directory}' contains a duplicate file name '{owned.Path}'.");
            }
        }
    }

    private static void ValidateManifestEntry(string directory, OwnedOutputFile owned)
    {
        if (!IsSafeLeafFileName(owned.Path))
        {
            throw new InvalidOperationException(
                $"Ownership manifest in '{directory}' contains an unsafe path '{owned.Path}'.");
        }

        if (owned.Sha256 is null || !IsSha256(owned.Sha256))
        {
            throw new InvalidOperationException(
                $"Ownership manifest in '{directory}' contains an invalid hash for '{owned.Path}'.");
        }
    }

    private static bool IsSha256(string value) =>
        value.Length == 64 && value.All(character => character is (>= '0' and <= '9') or (>= 'a' and <= 'f'));

    private static IReadOnlyList<string> OtherOwners(string directory, string leaf, string exceptGenerator)
    {
        if (!Directory.Exists(directory))
            return [];

        var owners = new List<string>();
        foreach (var path in Directory.EnumerateFiles(directory, ".*.owned.json"))
        {
            var fileName = Path.GetFileName(path);
            if (!fileName.StartsWith('.') || !fileName.EndsWith(".owned.json", StringComparison.Ordinal))
                continue;
            var candidateId = fileName[1..^".owned.json".Length];
            if (!IsSafeGeneratorId(candidateId) || string.Equals(candidateId, exceptGenerator, StringComparison.Ordinal))
                continue;

            var manifest = ReadManifest(directory, candidateId);
            if (manifest?.Files is { } owned
                && owned.Any(file => string.Equals(file.Path, leaf, StringComparison.OrdinalIgnoreCase)))
                owners.Add(candidateId);
        }

        return owners;
    }

    private static string HashBytes(byte[] bytes) =>
        Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();

    private static void DeleteTemps(IEnumerable<(string Destination, string Temp)> staged)
    {
        foreach (var (_, temp) in staged)
        {
            if (File.Exists(temp))
                File.Delete(temp);
        }
    }

    private sealed class OwnedOutputManifest
    {
        public string Generator { get; set; } = "";
        public List<OwnedOutputFile>? Files { get; set; }
    }

    private sealed class OwnedOutputFile
    {
        public string Path { get; set; } = "";
        public string Sha256 { get; set; } = "";
    }
}
