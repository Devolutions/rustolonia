using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Ir.Tests;

public class AbiBaselineTests
{
    [Fact]
    public void Released_identities_are_still_present_in_current_ir()
    {
        var root = FindRepositoryRoot();
        var ir = ProjectionIr.FromJson(File.ReadAllText(Path.Combine(root, "rust", "projection.ir.json")));
        using var baseline = JsonDocument.Parse(File.ReadAllText(Path.Combine(root, "rust", "abi-baseline.json")));
        var snapshot = baseline.RootElement;

        Assert.Equal(3, snapshot.GetProperty("schemaVersion").GetInt32());
        Assert.Equal("released", snapshot.GetProperty("kind").GetString());
        Assert.Equal(
            "9654332a79f637473da96054f75b2a16deaa557e",
            snapshot.GetProperty("producerPin").GetString());
        Assert.Equal("int32_t", snapshot.GetProperty("abiDefinitions").GetProperty("hresult").GetString());
        Assert.Equal("__stdcall", snapshot.GetProperty("abiDefinitions").GetProperty("callWin32").GetString());
        Assert.Equal(
            "const-and-indirection-only",
            snapshot.GetProperty("abiDefinitions").GetProperty("pointerModel").GetString());

        var currentByIid = ir.Types
            .Where(type => type.Iid is not null)
            .ToDictionary(type => type.Iid!, type => type, StringComparer.OrdinalIgnoreCase);

        foreach (var released in snapshot.GetProperty("interfaces").EnumerateArray())
        {
            var iid = released.GetProperty("iid").GetString()!;
            var name = released.GetProperty("name").GetString()!;
            if (!released.TryGetProperty("irBases", out var bases) || bases.GetArrayLength() == 0)
                continue;

            Assert.True(
                currentByIid.TryGetValue(iid, out var current),
                $"Released IID {iid} ({name}) is missing from the current IR.");
            Assert.Equal(name, current.Name);
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
