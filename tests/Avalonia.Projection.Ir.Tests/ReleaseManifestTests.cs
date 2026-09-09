using System;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Ir.Tests;

/// <summary>
/// Ties together the several independently-versioned identities this
/// repository publishes (the framework projection IR, the view-model schema,
/// the released ABI snapshot, the consumer manifest schema, the pinned
/// producer commit, and the additive producer patch set) into one
/// authoritative, machine-readable release record, and fails closed if any
/// of them drift out of sync with the actual source of truth.
/// </summary>
public class ReleaseManifestTests
{
    [Fact]
    public void Release_manifest_matches_the_actual_schema_and_producer_sources_of_truth()
    {
        var root = FindRepositoryRoot();
        using var manifest = JsonDocument.Parse(File.ReadAllText(Path.Combine(root, "rust", "release-manifest.json")));
        var element = manifest.RootElement;

        Assert.Equal(1, element.GetProperty("schemaVersion").GetInt32());
        Assert.False(string.IsNullOrWhiteSpace(element.GetProperty("rustoloniaVersion").GetString()));

        var schemas = element.GetProperty("schemas");
        Assert.Equal(ProjectionIr.CurrentVersion, schemas.GetProperty("projectionIr").GetInt32());
        Assert.Equal(ViewModelIr.CurrentVersion, schemas.GetProperty("viewModelIr").GetInt32());

        var consumerManifestSchema = JsonDocument.Parse(
            File.ReadAllText(Path.Combine(root, "rust", "consumer-app-manifest.schema.json")));
        var consumerManifestVersion = consumerManifestSchema.RootElement
            .GetProperty("properties").GetProperty("version").GetProperty("const").GetInt32();
        Assert.Equal(consumerManifestVersion, schemas.GetProperty("consumerAppManifest").GetInt32());

        using var abiBaseline = JsonDocument.Parse(
            File.ReadAllText(Path.Combine(root, "rust", "abi-baseline.json")));
        Assert.Equal(
            abiBaseline.RootElement.GetProperty("schemaVersion").GetInt32(),
            schemas.GetProperty("abiBaseline").GetInt32());
        Assert.Equal(
            abiBaseline.RootElement.GetProperty("producerPin").GetString(),
            element.GetProperty("producerPin").GetString());

        var patches = element.GetProperty("patches").EnumerateArray().ToArray();
        Assert.NotEmpty(patches);
        foreach (var patch in patches)
        {
            var relativePath = patch.GetProperty("path").GetString()!;
            var patchPath = Path.Combine(root, relativePath.Replace('/', Path.DirectorySeparatorChar));
            Assert.True(File.Exists(patchPath), $"Release manifest references a missing patch file: {relativePath}");
            var actualHash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(patchPath))).ToLowerInvariant();
            Assert.Equal(
                patch.GetProperty("sha256").GetString(),
                actualHash);
        }
    }

    private static string FindRepositoryRoot()
    {
        for (var current = new DirectoryInfo(AppContext.BaseDirectory);
             current is not null;
             current = current.Parent)
        {
            var gitPath = Path.Combine(current.FullName, ".git");
            if (Directory.Exists(gitPath) || File.Exists(gitPath))
                return current.FullName;
        }

        throw new DirectoryNotFoundException("Could not locate repository root.");
    }
}
