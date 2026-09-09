using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Host.Com;
using Avalonia.Input;
using Avalonia.Input.Platform;
using Avalonia.Platform.Storage;

namespace Avalonia.Host.Desktop;

/// <summary>
/// Clipboard command implementation, expressed purely in terms of Avalonia's
/// <see cref="IClipboard"/> and <see cref="IStorageProvider"/> abstractions.
/// </summary>
/// <remarks>
/// This type deliberately depends on nothing but those two abstractions, so
/// tests drive the whole clipboard surface through fakes instead of a real
/// platform clipboard. No raw platform clipboard API is used anywhere.
/// </remarks>
public static class DesktopClipboard
{
    /// <summary>Writes text and resolved file entries to the clipboard.</summary>
    public static async Task WriteAsync(
        IClipboard clipboard,
        IStorageProvider? provider,
        ClipboardPayload payload,
        CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(clipboard);
        ArgumentNullException.ThrowIfNull(payload);
        cancellationToken.ThrowIfCancellationRequested();
        var transfer = new DataTransfer();
        if (payload.Text is { } text)
            transfer.Add(DataTransferItem.CreateText(text));
        foreach (var item in await ResolveFilesAsync(provider, payload.FileUris, cancellationToken)
            .ConfigureAwait(true))
        {
            transfer.Add(DataTransferItem.CreateFile(item));
        }

        cancellationToken.ThrowIfCancellationRequested();
        await clipboard.SetDataAsync(transfer).ConfigureAwait(true);
        cancellationToken.ThrowIfCancellationRequested();
    }

    /// <summary>Clears the clipboard.</summary>
    public static async Task ClearAsync(IClipboard clipboard, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(clipboard);
        cancellationToken.ThrowIfCancellationRequested();
        await clipboard.ClearAsync().ConfigureAwait(true);
        cancellationToken.ThrowIfCancellationRequested();
    }

    /// <summary>
    /// Reads file entries from the clipboard as immutable snapshots.
    /// </summary>
    /// <remarks>
    /// A clipboard carrying no files is a successful, empty result - not a
    /// failure and not a cancellation, so the three outcomes stay distinct.
    /// </remarks>
    public static async Task<DesktopPickerResult> ReadFilesAsync(
        IClipboard clipboard,
        CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(clipboard);
        cancellationToken.ThrowIfCancellationRequested();
        using var transfer = await clipboard.TryGetDataAsync().ConfigureAwait(true);
        cancellationToken.ThrowIfCancellationRequested();
        if (transfer is null)
            return DesktopPickerResult.Completed(Array.Empty<IStorageItem>());
        var files = await transfer.TryGetFilesAsync().ConfigureAwait(true);
        cancellationToken.ThrowIfCancellationRequested();
        return DesktopPickerResult.Completed(Materialize(files));
    }

    private static IReadOnlyList<IStorageItem> Materialize(IEnumerable<IStorageItem>? files)
    {
        var items = new List<IStorageItem>();
        if (files is null)
            return items;
        foreach (var file in files)
        {
            if (file is not null)
                items.Add(file);
        }

        return items;
    }

    /// <summary>
    /// Resolves clipboard file entries to storage items.
    /// </summary>
    /// <remarks>
    /// An entry the provider cannot resolve is dropped rather than failing the
    /// write: a recent-file URI may point at something that has since been
    /// deleted, and losing one entry is a better outcome than losing the copy.
    /// </remarks>
    private static async Task<IReadOnlyList<IStorageItem>> ResolveFilesAsync(
        IStorageProvider? provider,
        IReadOnlyList<string> uris,
        CancellationToken cancellationToken)
    {
        var items = new List<IStorageItem>(uris.Count);
        if (provider is null || uris.Count == 0)
            return items;
        foreach (var value in uris)
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (await ResolveFileAsync(provider, value).ConfigureAwait(true) is { } item)
                items.Add(item);
        }

        return items;
    }

    private static async Task<IStorageItem?> ResolveFileAsync(IStorageProvider provider, string value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return null;
        try
        {
            // Locations with an explicit URI scheme go through the Uri
            // overloads (which handle non-file schemes); anything else is a
            // filesystem path and uses the string overloads, which avoid
            // double-escaping it. This mirrors stage 29's picker start location.
            if (StorageItemSnapshot.LooksLikeUri(value) &&
                Uri.TryCreate(value, UriKind.Absolute, out var uri))
            {
                return await provider.TryGetFileFromPathAsync(uri).ConfigureAwait(true) as IStorageItem
                    ?? await provider.TryGetFolderFromPathAsync(uri).ConfigureAwait(true);
            }

            return await provider.TryGetFileFromPathAsync(value).ConfigureAwait(true) as IStorageItem
                ?? await provider.TryGetFolderFromPathAsync(value).ConfigureAwait(true);
        }
        catch (Exception)
        {
            // A provider may throw for a path it cannot map; that is the
            // "unresolvable entry" case, not a clipboard failure.
            return null;
        }
    }
}
