using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class HostInfoTests
{
    [Fact]
    public void Export_reports_version_and_abi_fingerprint()
    {
        var info = new HostInfoNative { StructSize = (uint)sizeof(HostInfoNative) };
        delegate* unmanaged<HostInfoNative*, int> getHostInfo = &Exports.GetHostInfo;

        Assert.Equal(HResults.S_OK, getHostInfo(&info));
        Assert.Equal((uint)sizeof(HostInfoNative), info.StructSize);
        Assert.Equal(HostBuildInfo.Version, Marshal.PtrToStringUTF8((nint)info.Version));
        Assert.Equal(HostBuildInfo.AbiFingerprint, Marshal.PtrToStringUTF8((nint)info.AbiFingerprint));
    }

    [Fact]
    public void Export_rejects_null_and_undersized_structs()
    {
        delegate* unmanaged<HostInfoNative*, int> getHostInfo = &Exports.GetHostInfo;
        Assert.Equal(HResults.E_POINTER, getHostInfo(null));

        var info = new HostInfoNative { StructSize = 4 };
        Assert.Equal(HResults.E_INVALIDARG, getHostInfo(&info));
        Assert.True(info.Version is null);
    }

    [Fact]
    public void Abi_fingerprint_is_sha256_of_the_checked_in_header()
    {
        var root = FindRepositoryRoot();
        var header = File.ReadAllBytes(
            Path.Combine(root, "rust", "rustolonia-sys", "include", "avalonia-rust-abi.h"));
        Assert.Equal(Convert.ToHexStringLower(SHA256.HashData(header)), HostBuildInfo.AbiFingerprint);
    }

    [Fact]
    public void Version_matches_release_manifest()
    {
        var root = FindRepositoryRoot();
        using var manifest = JsonDocument.Parse(
            File.ReadAllText(Path.Combine(root, "rust", "release-manifest.json")));
        Assert.Equal(manifest.RootElement.GetProperty("rustoloniaVersion").GetString(), HostBuildInfo.Version);
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
