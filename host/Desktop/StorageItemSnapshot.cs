using System;
using System.Collections.Generic;
using System.IO;
using Avalonia.Platform.Storage;
using SystemUri = System.Uri;

namespace Avalonia.Host.Desktop;

/// <summary>
/// Immutable, transport-ready description of one storage item.
/// </summary>
/// <remarks>
/// Snapshots decouple the ABI from managed object lifetime. A drag-and-drop
/// <c>IDataTransfer</c> is only valid while the managed event is on the stack,
/// and a picker result must outlive the window that produced it, so everything
/// the Rust side can observe is captured eagerly and copied.
/// <para>
/// <see cref="Uri"/> is always present. <see cref="LocalPath"/> is null whenever
/// the platform has no filesystem path for the item, which is the normal case
/// for Android <c>content:</c> URIs, browser handles and other non-local items.
/// </para>
/// </remarks>
public sealed record StorageItemSnapshot(
    bool IsFolder,
    string Name,
    string Uri,
    string? LocalPath)
{
    /// <summary>
    /// Captures an <see cref="IStorageItem"/> without taking ownership of it.
    /// </summary>
    public static StorageItemSnapshot FromStorageItem(IStorageItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        string? localPath;
        try
        {
            localPath = item.TryGetLocalPath();
        }
        catch (Exception)
        {
            // A platform provider may throw for items it cannot map to a path;
            // that is the "no local path" case, not a picker failure.
            localPath = null;
        }

        return new StorageItemSnapshot(
            item is IStorageFolder,
            item.Name,
            item.Path.IsAbsoluteUri ? item.Path.AbsoluteUri : item.Path.ToString(),
            string.IsNullOrEmpty(localPath) ? null : localPath);
    }

    /// <summary>
    /// Parses one verbatim command-line argument into an activation item, or
    /// returns null when the argument is not a file/folder reference.
    /// </summary>
    /// <remarks>
    /// Absolute non-file URIs are preserved verbatim, which is how "open with"
    /// survives on platforms that hand an application a content or custom-scheme
    /// URI rather than a path. Everything else is treated as a filesystem path
    /// and made absolute against the current directory. Arguments beginning with
    /// '-' are treated as option switches; '/' is deliberately <em>not</em>
    /// treated as a switch prefix, because it introduces every absolute path on
    /// Unix.
    /// </remarks>
    public static StorageItemSnapshot? TryFromActivationArgument(string? argument)
    {
        if (string.IsNullOrWhiteSpace(argument))
            return null;
        if (argument.StartsWith('-'))
            return null;

        if (LooksLikeUri(argument) &&
            SystemUri.TryCreate(argument, UriKind.Absolute, out var uri))
        {
            if (!uri.IsFile)
            {
                // Non-local activation: keep the URI, report no local path, and
                // do not guess whether it is a folder.
                return new StorageItemSnapshot(false, NameFromUri(uri), uri.AbsoluteUri, null);
            }

            var path = uri.LocalPath;
            return new StorageItemSnapshot(
                Directory.Exists(path),
                NameFromPath(path),
                uri.AbsoluteUri,
                path);
        }

        string full;
        try
        {
            full = Path.GetFullPath(argument);
        }
        catch (Exception e) when (
            e is ArgumentException or NotSupportedException or PathTooLongException or
                IOException or System.Security.SecurityException)
        {
            return null;
        }

        if (!SystemUri.TryCreate(full, UriKind.Absolute, out var fileUri))
            return null;

        return new StorageItemSnapshot(
            Directory.Exists(full),
            NameFromPath(full),
            fileUri.AbsoluteUri,
            full);
    }

    /// <summary>
    /// Normalizes verbatim startup arguments into ordered activation items,
    /// dropping duplicates by URI while preserving first-seen order.
    /// </summary>
    public static IReadOnlyList<StorageItemSnapshot> FromActivationArguments(
        IEnumerable<string?>? arguments)
    {
        var results = new List<StorageItemSnapshot>();
        if (arguments is null)
            return results;

        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var argument in arguments)
        {
            if (TryFromActivationArgument(argument) is not { } snapshot)
                continue;
            if (!seen.Add(snapshot.Uri))
                continue;
            results.Add(snapshot);
        }

        return results;
    }

    public static IReadOnlyList<StorageItemSnapshot> FromStorageItems(
        IEnumerable<IStorageItem>? items)
    {
        var results = new List<StorageItemSnapshot>();
        if (items is null)
            return results;
        foreach (var item in items)
            results.Add(FromStorageItem(item));
        return results;
    }

    /// <summary>
    /// Recognizes an explicit URI scheme. A bare Windows path ("C:\dir\file")
    /// also parses as an absolute <c>file:</c> URI on every OS, so the scheme
    /// has to be spelled out for an argument to take the URI path; otherwise
    /// every platform normalizes paths through one code path.
    /// </summary>
    internal static bool LooksLikeUri(string argument)
    {
        var separator = argument.IndexOf(':');
        // A single-character "scheme" is a Windows drive letter, not a scheme.
        if (separator < 2)
            return false;
        for (var i = 0; i < separator; i++)
        {
            var c = argument[i];
            var valid = char.IsAsciiLetterOrDigit(c) || c is '+' or '-' or '.';
            if (!valid)
                return false;
        }

        return char.IsAsciiLetter(argument[0]);
    }

    private static string NameFromPath(string path)
    {
        var name = Path.GetFileName(path);
        return string.IsNullOrEmpty(name) ? path : name;
    }

    private static string NameFromUri(SystemUri uri)
    {
        var segments = uri.Segments;
        for (var i = segments.Length - 1; i >= 0; i--)
        {
            var segment = SystemUri.UnescapeDataString(segments[i]).TrimEnd('/');
            if (!string.IsNullOrEmpty(segment))
                return segment;
        }

        return uri.AbsoluteUri;
    }
}
