using System;
using System.IO;
using System.Linq;
using Avalonia.Host.Desktop;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

/// <summary>
/// Startup/open-with argument normalization. These rules must stay
/// platform-neutral: '/' introduces an absolute Unix path and is never treated
/// as a switch, and absolute non-file URIs are preserved verbatim.
/// </summary>
public class StorageActivationTests
{
    [Fact]
    public void A_relative_path_becomes_an_absolute_file_uri_and_local_path()
    {
        var snapshot = StorageItemSnapshot.TryFromActivationArgument("relative-open-with.log");

        Assert.NotNull(snapshot);
        Assert.False(snapshot!.IsFolder);
        Assert.Equal("relative-open-with.log", snapshot.Name);
        Assert.StartsWith("file:///", snapshot.Uri);
        Assert.Equal(Path.GetFullPath("relative-open-with.log"), snapshot.LocalPath);
    }

    [Fact]
    public void An_existing_directory_is_reported_as_a_folder()
    {
        var directory = Directory.CreateTempSubdirectory("avalonia-activation");
        try
        {
            var snapshot = StorageItemSnapshot.TryFromActivationArgument(directory.FullName);

            Assert.NotNull(snapshot);
            Assert.True(snapshot!.IsFolder);
            Assert.Equal(directory.FullName, snapshot.LocalPath);
        }
        finally
        {
            directory.Delete(recursive: true);
        }
    }

    [Fact]
    public void A_non_file_uri_is_preserved_and_has_no_local_path()
    {
        var snapshot = StorageItemSnapshot.TryFromActivationArgument("content://media/documents/7");

        Assert.NotNull(snapshot);
        Assert.Equal("content://media/documents/7", snapshot!.Uri);
        Assert.Null(snapshot.LocalPath);
        Assert.Equal("7", snapshot.Name);
    }

    [Fact]
    public void A_file_uri_keeps_its_uri_and_resolves_a_local_path()
    {
        var snapshot = StorageItemSnapshot.TryFromActivationArgument("file:///logs/a%20b.log");

        Assert.NotNull(snapshot);
        Assert.Equal("file:///logs/a%20b.log", snapshot!.Uri);
        Assert.NotNull(snapshot.LocalPath);
        Assert.Contains("a b.log", snapshot.LocalPath);
    }

    [Theory]
    [InlineData("-v")]
    [InlineData("--verbose")]
    [InlineData("   ")]
    [InlineData("")]
    [InlineData(null)]
    public void Option_switches_and_blanks_are_not_activation_items(string? argument) =>
        Assert.Null(StorageItemSnapshot.TryFromActivationArgument(argument));

    [Fact]
    public void A_unix_style_absolute_path_is_not_mistaken_for_a_switch()
    {
        var snapshot = StorageItemSnapshot.TryFromActivationArgument("/var/log/system.log");

        Assert.NotNull(snapshot);
        Assert.EndsWith("system.log", snapshot!.Uri);
    }

    [Fact]
    public void Activation_arguments_keep_order_and_drop_duplicates()
    {
        var items = StorageItemSnapshot.FromActivationArguments(
        [
            "--flag",
            "first.log",
            "myapp://open/7",
            "first.log",
            "second.log",
            "myapp://open/7",
        ]);

        Assert.Equal(3, items.Count);
        Assert.Equal("first.log", items[0].Name);
        Assert.Equal("myapp://open/7", items[1].Uri);
        Assert.Equal("second.log", items[2].Name);
        Assert.Equal(items.Select(item => item.Uri).Distinct().Count(), items.Count);
    }

    [Fact]
    public void No_arguments_produce_no_activation_items()
    {
        Assert.Empty(StorageItemSnapshot.FromActivationArguments(null));
        Assert.Empty(StorageItemSnapshot.FromActivationArguments(Array.Empty<string>()));
    }

    [Fact]
    public void Storage_items_snapshot_their_name_uri_and_optional_path()
    {
        var local = StorageItemSnapshot.FromStorageItem(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        var remote = StorageItemSnapshot.FromStorageItem(
            new FakeStorageFolder("bucket", new Uri("s3://bucket/prefix/")));

        Assert.Equal("a.log", local.Name);
        Assert.Equal("file:///logs/a.log", local.Uri);
        Assert.NotNull(local.LocalPath);

        Assert.True(remote.IsFolder);
        Assert.Equal("s3://bucket/prefix/", remote.Uri);
        Assert.Null(remote.LocalPath);
    }
}
