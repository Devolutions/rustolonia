using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using Avalonia.Projection.Generator;
using Xunit;

namespace Avalonia.Projection.Generator.Tests;

public class OwnedOutputWriterTests
{
    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void Explicit_directory_tracks_removal_of_its_last_output(bool keepOutputElsewhere)
    {
        using var scratch = new Scratch();
        var directory = Path.Combine(scratch.Root, "adapters");
        var stale = Path.Combine(directory, "LastAdapter.g.cs");
        var unrelated = Path.Combine(directory, "notes.txt");
        var other = Path.Combine(directory, "Other.g.cs");
        OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string> { [stale] = "stale\n" });
        OwnedOutputs.Write(OwnedOutputs.ProjectionGeneratorId, new Dictionary<string, string> { [other] = "other\n" });
        File.WriteAllText(unrelated, "user\n");
        var expected = new Dictionary<string, string>();
        if (keepOutputElsewhere)
            expected.Add(Path.Combine(scratch.Root, "registry", "Registry.g.cs"), "registry\n");
        var directories = new[] { directory };

        var check = OwnedOutputs.Check(OwnedOutputs.ViewModelGeneratorId, expected, directories);
        Assert.Contains(check.Mismatches, item => item == $"OBSOLETE: {stale}");
        Assert.True(File.Exists(stale));

        OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, expected, directories);
        Assert.False(File.Exists(stale));
        Assert.Equal("user\n", File.ReadAllText(unrelated));
        Assert.Equal("other\n", File.ReadAllText(other));
        Assert.True(OwnedOutputs.Check(OwnedOutputs.ViewModelGeneratorId, expected, directories).Success);
        Assert.Contains("\"files\": []", File.ReadAllText(Path.Combine(directory,
            OwnedOutputs.ManifestFileName(OwnedOutputs.ViewModelGeneratorId))), StringComparison.Ordinal);
    }

    [Fact]
    public void Empty_explicit_directory_preserves_modified_obsolete_output()
    {
        using var scratch = new Scratch();
        var stale = Path.Combine(scratch.Root, "Last.g.cs");
        OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string> { [stale] = "generated\n" });
        File.WriteAllText(stale, "edited\n");
        var expected = new Dictionary<string, string>();
        var directories = new[] { scratch.Root };

        Assert.Contains(OwnedOutputs.Check(OwnedOutputs.ViewModelGeneratorId, expected, directories).Mismatches,
            item => item == $"OBSOLETE-MODIFIED: {stale}");
        Assert.Throws<InvalidOperationException>(() =>
            OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, expected, directories));
        Assert.Equal("edited\n", File.ReadAllText(stale));
    }

    [Fact]
    public void Checking_empty_explicit_directory_does_not_create_it()
    {
        using var scratch = new Scratch();
        var missing = Path.Combine(scratch.Root, "missing");
        Assert.True(OwnedOutputs.Check(OwnedOutputs.ViewModelGeneratorId,
            new Dictionary<string, string>(), [missing]).Success);
        Assert.False(Directory.Exists(missing));
    }

    [Fact]
    public void Check_mode_does_not_create_missing_directories_or_files()
    {
        using var scratch = new Scratch();
        var missingRoot = Path.Combine(scratch.Root, "absent");
        var path = Path.Combine(missingRoot, "owned.g.cs");

        var result = OwnedOutputs.Check(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string>
        {
            [path] = "generated\n",
        });

        Assert.False(result.Success);
        Assert.Contains(result.Mismatches, item => item.StartsWith("MISSING:", StringComparison.Ordinal) && item.Contains(path, StringComparison.Ordinal));
        Assert.False(Directory.Exists(missingRoot));
        Assert.False(File.Exists(path));
        Assert.Empty(Directory.GetFileSystemEntries(scratch.Root));
    }

    [Fact]
    public void Check_reports_changed_missing_and_obsolete_outputs()
    {
        using var scratch = new Scratch();
        var changed = Path.Combine(scratch.Root, "changed.g.cs");
        var missing = Path.Combine(scratch.Root, "missing.g.cs");
        var obsolete = Path.Combine(scratch.Root, "obsolete.g.cs");
        File.WriteAllText(changed, "old\n");
        File.WriteAllText(obsolete, "stale\n");
        OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string>
        {
            [changed] = "old\n",
            [obsolete] = "stale\n",
        });

        var result = OwnedOutputs.Check(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string>
        {
            [changed] = "new\n",
            [missing] = "fresh\n",
        });

        Assert.Contains(result.Mismatches, item => item.StartsWith("DIFFERENT:", StringComparison.Ordinal) && item.Contains(changed, StringComparison.Ordinal));
        Assert.Contains(result.Mismatches, item => item.StartsWith("MISSING:", StringComparison.Ordinal) && item.Contains(missing, StringComparison.Ordinal));
        Assert.Contains(result.Mismatches, item => item.StartsWith("OBSOLETE:", StringComparison.Ordinal) && item.Contains(obsolete, StringComparison.Ordinal));
        Assert.Equal("old\n", File.ReadAllText(changed).Replace("\r\n", "\n"));
        Assert.Equal("stale\n", File.ReadAllText(obsolete).Replace("\r\n", "\n"));
        Assert.False(File.Exists(missing));
    }

    [Fact]
    public void Write_prunes_only_manifest_owned_stale_files_and_keeps_unrelated_and_other_generator_outputs()
    {
        using var scratch = new Scratch();
        var kept = Path.Combine(scratch.Root, "kept.g.cs");
        var stale = Path.Combine(scratch.Root, "stale.g.cs");
        var unrelated = Path.Combine(scratch.Root, "notes.txt");
        var other = Path.Combine(scratch.Root, "other.g.cs");
        File.WriteAllText(unrelated, "leave me\n");
        OwnedOutputs.Write(OwnedOutputs.ProjectionGeneratorId, new Dictionary<string, string>
        {
            [other] = "projection-owned\n",
        });
        OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string>
        {
            [kept] = "keep\n",
            [stale] = "remove\n",
        });

        OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string>
        {
            [kept] = "keep-updated\n",
        });

        Assert.Equal("keep-updated\n", File.ReadAllText(kept).Replace("\r\n", "\n"));
        Assert.False(File.Exists(stale));
        Assert.Equal("leave me\n", File.ReadAllText(unrelated).Replace("\r\n", "\n"));
        Assert.Equal("projection-owned\n", File.ReadAllText(other).Replace("\r\n", "\n"));
        Assert.True(File.Exists(Path.Combine(scratch.Root, OwnedOutputs.ManifestFileName(OwnedOutputs.ProjectionGeneratorId))));
    }

    [Fact]
    public void Write_refuses_to_delete_obsolete_file_modified_outside_the_generator()
    {
        using var scratch = new Scratch();
        var kept = Path.Combine(scratch.Root, "kept.g.cs");
        var stale = Path.Combine(scratch.Root, "stale.g.cs");
        OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string>
        {
            [kept] = "keep\n",
            [stale] = "owned-stale\n",
        });
        File.WriteAllText(stale, "edited by hand\n");
        var beforeKept = File.ReadAllBytes(kept);
        var beforeStale = File.ReadAllBytes(stale);
        var manifestBefore = File.ReadAllBytes(Path.Combine(scratch.Root, OwnedOutputs.ManifestFileName(OwnedOutputs.ViewModelGeneratorId)));

        var error = Assert.Throws<InvalidOperationException>(() => OwnedOutputs.Write(
            OwnedOutputs.ViewModelGeneratorId,
            new Dictionary<string, string> { [kept] = "keep-updated\n" }));

        Assert.Contains("modified outside the generator", error.Message, StringComparison.Ordinal);
        Assert.Equal(beforeKept, File.ReadAllBytes(kept));
        Assert.Equal(beforeStale, File.ReadAllBytes(stale));
        Assert.Equal(manifestBefore, File.ReadAllBytes(Path.Combine(scratch.Root, OwnedOutputs.ManifestFileName(OwnedOutputs.ViewModelGeneratorId))));
    }

    [Fact]
    public void Malformed_ownership_paths_are_rejected_without_changing_existing_bytes()
    {
        using var scratch = new Scratch();
        var owned = Path.Combine(scratch.Root, "owned.g.cs");
        File.WriteAllText(owned, "original\n");
        var manifestPath = Path.Combine(scratch.Root, OwnedOutputs.ManifestFileName(OwnedOutputs.ViewModelGeneratorId));
        File.WriteAllText(manifestPath, """
            {
              "generator": "avalonia-viewmodel",
              "files": [
                { "path": "../escape.g.cs", "sha256": "00" }
              ]
            }
            """);
        var ownedBytes = File.ReadAllBytes(owned);
        var manifestBytes = File.ReadAllBytes(manifestPath);

        var error = Assert.Throws<InvalidOperationException>(() => OwnedOutputs.Write(
            OwnedOutputs.ViewModelGeneratorId,
            new Dictionary<string, string> { [owned] = "updated\n" }));

        Assert.Contains("unsafe path", error.Message, StringComparison.Ordinal);
        Assert.Equal(ownedBytes, File.ReadAllBytes(owned));
        Assert.Equal(manifestBytes, File.ReadAllBytes(manifestPath));
    }

    [Fact]
    public void Absolute_manifest_path_is_rejected()
    {
        using var scratch = new Scratch();
        var owned = Path.Combine(scratch.Root, "owned.g.cs");
        File.WriteAllText(owned, "original\n");
        var absolute = Path.Combine(scratch.Root, "other.g.cs");
        File.WriteAllText(Path.Combine(scratch.Root, OwnedOutputs.ManifestFileName(OwnedOutputs.ViewModelGeneratorId)), $$"""
            {
              "generator": "avalonia-viewmodel",
              "files": [
                { "path": {{ToJson(absolute)}}, "sha256": "00" }
              ]
            }
            """);

        var error = Assert.Throws<InvalidOperationException>(() => OwnedOutputs.Check(
            OwnedOutputs.ViewModelGeneratorId,
            new Dictionary<string, string> { [owned] = "original\n" }));
        Assert.Contains("unsafe path", error.Message, StringComparison.Ordinal);
        Assert.Equal("original\n", File.ReadAllText(owned).Replace("\r\n", "\n"));
    }

    [Fact]
    public void Check_with_normalize_is_rejected_by_flag_helper()
    {
        Assert.True(GeneratorCli.ContainsCheckAndNormalize(["--check", "--normalize", "view-model.ir.json"]));
        Assert.True(GeneratorCli.ContainsCheckAndNormalize(["--normalize", "--check", "view-model.ir.json"]));
        Assert.False(GeneratorCli.ContainsCheckAndNormalize(["--check", "a", "b", "c", "d", "e"]));
        Assert.Equal("Cannot combine --check with --normalize.", GeneratorCli.CheckWithNormalizeMessage);
    }

    [Fact]
    public void Assembling_outputs_rejects_physical_path_collisions()
    {
        using var scratch = new Scratch();
        var files = new Dictionary<string, string>(StringComparer.Ordinal);
        var first = Path.Combine(scratch.Root, "owned.g.cs");
        OwnedOutputs.Add(files, first, "one\n");
        var error = Assert.Throws<InvalidOperationException>(() =>
            OwnedOutputs.Add(files, Path.Combine(scratch.Root, "OWNED.g.cs"), "two\n"));
        Assert.Contains("Duplicate generated output", error.Message, StringComparison.Ordinal);
        Assert.Equal("one\n", files[first]);
    }

    [Fact]
    public void Write_refuses_to_overwrite_another_generators_owned_file()
    {
        using var scratch = new Scratch();
        var shared = Path.Combine(scratch.Root, "shared.g.cs");
        OwnedOutputs.Write(OwnedOutputs.ProjectionGeneratorId, new Dictionary<string, string>
        {
            [shared] = "projection\n",
        });

        var error = Assert.Throws<InvalidOperationException>(() =>
            OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string>
            {
                [shared] = "viewmodel\n",
            }));
        Assert.Contains("owned by avalonia-projection", error.Message, StringComparison.Ordinal);
        Assert.Equal("projection\n", File.ReadAllText(shared).Replace("\r\n", "\n"));
    }

    [Fact]
    public void Check_reports_another_generators_claim_without_writing()
    {
        using var scratch = new Scratch();
        var shared = Path.Combine(scratch.Root, "shared.g.cs");
        var files = new Dictionary<string, string> { [shared] = "same\n" };
        OwnedOutputs.Write(OwnedOutputs.ProjectionGeneratorId, files);

        var result = OwnedOutputs.Check(OwnedOutputs.ViewModelGeneratorId, files);

        Assert.False(result.Success);
        Assert.Contains(result.Mismatches, mismatch => mismatch.Contains("CONFLICT", StringComparison.Ordinal));
        Assert.Equal("same\n", File.ReadAllText(shared));
        Assert.False(File.Exists(Path.Combine(scratch.Root, OwnedOutputs.ManifestFileName(OwnedOutputs.ViewModelGeneratorId))));
    }

    [Fact]
    public void Write_does_not_delete_a_file_still_owned_by_another_generator()
    {
        using var scratch = new Scratch();
        var shared = Path.Combine(scratch.Root, "shared.g.cs");
        var extra = Path.Combine(scratch.Root, "extra.g.cs");
        OwnedOutputs.Write(OwnedOutputs.ProjectionGeneratorId, new Dictionary<string, string>
        {
            [shared] = "same\n",
        });
        File.WriteAllText(
            Path.Combine(scratch.Root, OwnedOutputs.ManifestFileName(OwnedOutputs.ViewModelGeneratorId)),
            $$"""
            {
              "generator": "avalonia-viewmodel",
              "files": [
                { "path": "shared.g.cs", "sha256": "{{OwnedOutputs.HashContent("same\n")}}" },
                { "path": "extra.g.cs", "sha256": "{{OwnedOutputs.HashContent("extra\n")}}" }
              ]
            }
            """ + "\n");
        File.WriteAllText(extra, "extra\n");

        OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string>
        {
            [extra] = "extra\n",
        });

        Assert.True(File.Exists(shared));
        Assert.Equal("same\n", File.ReadAllText(shared).Replace("\r\n", "\n"));
        Assert.True(File.Exists(extra));
    }

    [Theory]
    [InlineData("""{ "generator": "avalonia-viewmodel", "files": null }""", "null files list")]
    [InlineData("""{ "generator": "avalonia-viewmodel", "files": [null] }""", "null files[")]
    [InlineData("""{ "generator": "avalonia-viewmodel", "files": [{ "path": "a.g.cs", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }, { "path": "a.g.cs", "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }] }""", "duplicate file name")]
    [InlineData("""{ "generator": "avalonia-viewmodel", "files": [{ "path": "a.g.cs", "sha256": "not-a-hash" }] }""", "invalid hash")]
    [InlineData("""{ "generator": "Not Safe", "files": [] }""", "unsafe generator")]
    [InlineData("""{ "generator": "avalonia-viewmodel", "files": [{ "path": "nested/a.g.cs", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }] }""", "unsafe path")]
    public void Corrupted_ownership_manifests_fail_closed(string manifest, string expected)
    {
        using var scratch = new Scratch();
        var owned = Path.Combine(scratch.Root, "owned.g.cs");
        File.WriteAllText(owned, "original\n");
        File.WriteAllText(Path.Combine(scratch.Root, OwnedOutputs.ManifestFileName(OwnedOutputs.ViewModelGeneratorId)), manifest);
        var before = File.ReadAllBytes(owned);

        var error = Assert.Throws<InvalidOperationException>(() => OwnedOutputs.Write(
            OwnedOutputs.ViewModelGeneratorId,
            new Dictionary<string, string> { [owned] = "updated\n" }));
        Assert.Contains(expected, error.Message, StringComparison.Ordinal);
        Assert.Equal(before, File.ReadAllBytes(owned));
    }

    [Fact]
    public void Bootstrap_does_not_claim_unrelated_gcs_files()
    {
        using var scratch = new Scratch();
        var generated = Path.Combine(scratch.Root, "SampleAdapter.g.cs");
        var leftover = Path.Combine(scratch.Root, "HandWritten.g.cs");
        File.WriteAllText(leftover, "not generated\n");

        OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, new Dictionary<string, string>
        {
            [generated] = "adapter\n",
        });

        Assert.True(File.Exists(leftover));
        var manifest = File.ReadAllText(Path.Combine(scratch.Root, OwnedOutputs.ManifestFileName(OwnedOutputs.ViewModelGeneratorId)));
        Assert.Contains("SampleAdapter.g.cs", manifest, StringComparison.Ordinal);
        Assert.DoesNotContain("HandWritten.g.cs", manifest, StringComparison.Ordinal);
    }

    private static string ToJson(string value)
    {
        var builder = new StringBuilder("\"");
        foreach (var character in value)
        {
            builder.Append(character switch
            {
                '\\' => "\\\\",
                '"' => "\\\"",
                _ => character.ToString(),
            });
        }

        builder.Append('"');
        return builder.ToString();
    }

    private sealed class Scratch : IDisposable
    {
        public string Root { get; } = Path.Combine(AppContext.BaseDirectory, "rustolonia-owned-" + Guid.NewGuid().ToString("N"));

        public Scratch() => Directory.CreateDirectory(Root);

        public void Dispose()
        {
            if (Directory.Exists(Root))
                Directory.Delete(Root, recursive: true);
        }
    }
}
