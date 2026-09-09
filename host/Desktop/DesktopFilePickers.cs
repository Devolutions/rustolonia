using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Platform.Storage;

namespace Avalonia.Host.Desktop;

/// <summary>One entry of <see cref="DesktopFilePickerRequest.FileTypes"/>.</summary>
public sealed class DesktopFileTypeRequest
{
    public DesktopFileTypeRequest(string? name) => Name = name ?? string.Empty;

    public string Name { get; }

    public List<string> Patterns { get; } = new();

    public List<string> MimeTypes { get; } = new();

    public List<string> AppleUniformTypeIdentifiers { get; } = new();

    internal FilePickerFileType ToFilePickerFileType() => new(Name)
    {
        Patterns = Patterns.Count == 0 ? null : Patterns.ToArray(),
        MimeTypes = MimeTypes.Count == 0 ? null : MimeTypes.ToArray(),
        AppleUniformTypeIdentifiers = AppleUniformTypeIdentifiers.Count == 0
            ? null
            : AppleUniformTypeIdentifiers.ToArray(),
    };
}

/// <summary>
/// Platform-neutral picker options, gathered across the ABI and translated into
/// Avalonia's own <see cref="PickerOptions"/> derivatives when the picker runs.
/// </summary>
public sealed class DesktopFilePickerRequest
{
    public string? Title { get; set; }

    public bool AllowMultiple { get; set; }

    public string? SuggestedFileName { get; set; }

    /// <summary>Absolute URI or absolute local path for the start location.</summary>
    public string? SuggestedStartLocation { get; set; }

    /// <summary>A <see cref="WellKnownFolder"/> value, or null for none.</summary>
    public WellKnownFolder? SuggestedStartWellKnownFolder { get; set; }

    public string? DefaultExtension { get; set; }

    public bool? ShowOverwritePrompt { get; set; }

    public int SuggestedFileTypeIndex { get; set; } = -1;

    public List<DesktopFileTypeRequest> FileTypes { get; } = new();

    internal IReadOnlyList<FilePickerFileType>? BuildFileTypes()
    {
        if (FileTypes.Count == 0)
            return null;
        var result = new FilePickerFileType[FileTypes.Count];
        for (var i = 0; i < FileTypes.Count; i++)
            result[i] = FileTypes[i].ToFilePickerFileType();
        return result;
    }

    internal static FilePickerFileType? SelectSuggested(
        IReadOnlyList<FilePickerFileType>? types,
        int index) =>
        types is not null && index >= 0 && index < types.Count ? types[index] : null;
}

/// <summary>
/// The outcome of one picker operation. A user-dismissed dialog is
/// <see cref="Cancelled"/> and never surfaces as an error.
/// </summary>
public sealed record DesktopPickerResult(
    bool Cancelled,
    IReadOnlyList<StorageItemSnapshot> Items)
{
    public static DesktopPickerResult Cancel { get; } =
        new(true, Array.Empty<StorageItemSnapshot>());

    public static DesktopPickerResult From(IReadOnlyList<IStorageItem>? items)
    {
        if (items is null || items.Count == 0)
            return Cancel;
        return new DesktopPickerResult(false, StorageItemSnapshot.FromStorageItems(items));
    }

    public static DesktopPickerResult From(IStorageItem? item) =>
        item is null
            ? Cancel
            : new DesktopPickerResult(false, new[] { StorageItemSnapshot.FromStorageItem(item) });

    /// <summary>
    /// A completed, possibly empty result.
    /// </summary>
    /// <remarks>
    /// An empty <em>picker</em> result means the user dismissed the dialog, so
    /// <see cref="From(IReadOnlyList{IStorageItem})"/> reports
    /// <see cref="Cancelled"/>. A clipboard read has no dialog to dismiss: a
    /// clipboard that simply carries no files is a successful empty result, and
    /// conflating the two would make "nothing to paste" indistinguishable from
    /// "the user cancelled".
    /// </remarks>
    public static DesktopPickerResult Completed(IReadOnlyList<IStorageItem>? items) =>
        new(false, StorageItemSnapshot.FromStorageItems(items));
}

/// <summary>
/// Runs Avalonia's <see cref="IStorageProvider"/> pickers from an ABI request.
/// </summary>
/// <remarks>
/// This type deliberately depends on nothing but <see cref="IStorageProvider"/>,
/// so tests drive the whole picker surface through a fake provider instead of
/// opening a real platform dialog. No raw platform dialog API is used anywhere.
/// </remarks>
public static class DesktopFilePickers
{
    public static AvnStorageCapabilityFlags GetCapabilities(IStorageProvider provider)
    {
        ArgumentNullException.ThrowIfNull(provider);
        var flags = AvnStorageCapabilityFlags.None;
        if (provider.CanOpen)
            flags |= AvnStorageCapabilityFlags.CanOpen;
        if (provider.CanSave)
            flags |= AvnStorageCapabilityFlags.CanSave;
        if (provider.CanPickFolder)
            flags |= AvnStorageCapabilityFlags.CanPickFolder;
        return flags;
    }

    public static async Task<DesktopPickerResult> OpenFilesAsync(
        IStorageProvider provider,
        DesktopFilePickerRequest request,
        CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(provider);
        ArgumentNullException.ThrowIfNull(request);
        cancellationToken.ThrowIfCancellationRequested();

        var types = request.BuildFileTypes();
        var options = new FilePickerOpenOptions
        {
            Title = request.Title,
            AllowMultiple = request.AllowMultiple,
            SuggestedFileName = request.SuggestedFileName,
            SuggestedStartLocation = await ResolveStartLocationAsync(provider, request)
                .ConfigureAwait(true),
            FileTypeFilter = types,
            SuggestedFileType = DesktopFilePickerRequest.SelectSuggested(
                types,
                request.SuggestedFileTypeIndex),
        };

        var files = await AwaitDialogAsync(provider.OpenFilePickerAsync(options), cancellationToken)
            .ConfigureAwait(true);
        return DesktopPickerResult.From(files);
    }

    public static async Task<DesktopPickerResult> OpenFoldersAsync(
        IStorageProvider provider,
        DesktopFilePickerRequest request,
        CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(provider);
        ArgumentNullException.ThrowIfNull(request);
        cancellationToken.ThrowIfCancellationRequested();

        var options = new FolderPickerOpenOptions
        {
            Title = request.Title,
            AllowMultiple = request.AllowMultiple,
            SuggestedFileName = request.SuggestedFileName,
            SuggestedStartLocation = await ResolveStartLocationAsync(provider, request)
                .ConfigureAwait(true),
        };

        var folders = await AwaitDialogAsync(provider.OpenFolderPickerAsync(options), cancellationToken)
            .ConfigureAwait(true);
        return DesktopPickerResult.From(folders);
    }

    public static async Task<DesktopPickerResult> SaveFileAsync(
        IStorageProvider provider,
        DesktopFilePickerRequest request,
        CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(provider);
        ArgumentNullException.ThrowIfNull(request);
        cancellationToken.ThrowIfCancellationRequested();

        var types = request.BuildFileTypes();
        var options = new FilePickerSaveOptions
        {
            Title = request.Title,
            SuggestedFileName = request.SuggestedFileName,
            SuggestedStartLocation = await ResolveStartLocationAsync(provider, request)
                .ConfigureAwait(true),
            DefaultExtension = request.DefaultExtension,
            ShowOverwritePrompt = request.ShowOverwritePrompt,
            FileTypeChoices = types,
            SuggestedFileType = DesktopFilePickerRequest.SelectSuggested(
                types,
                request.SuggestedFileTypeIndex),
        };

        // Deliberately returns the picked file as a snapshot (name/URI plus an
        // optional local path) instead of opening a managed stream: writing is
        // the consumer's job, and forcing stream IO here would break non-local
        // items and large exports.
        var file = await AwaitDialogAsync(provider.SaveFilePickerAsync(options), cancellationToken)
            .ConfigureAwait(true);
        return DesktopPickerResult.From(file);
    }

    /// <summary>
    /// Runs one picker and cancels it when the thing that owns it goes away —
    /// in practice the window the dialog is parented to.
    /// </summary>
    /// <param name="observeClosed">
    /// Subscribes a "the owner closed" callback and returns the registration.
    /// It is disposed before the linked source is, so the callback can never
    /// run against a disposed <see cref="CancellationTokenSource"/>.
    /// </param>
    /// <remarks>
    /// Closing the parent window while a dialog is up is a lifetime event, not
    /// a user choice, so it aborts (<c>E_ABORT</c>) instead of reporting the
    /// <see cref="DesktopPickerResult.Cancel"/> outcome a dismissed dialog
    /// produces. The operation still completes exactly once either way.
    /// </remarks>
    public static async Task<DesktopPickerResult> RunWhileOpenAsync(
        Func<Action, IDisposable> observeClosed,
        Func<CancellationToken, Task<DesktopPickerResult>> run,
        CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(observeClosed);
        ArgumentNullException.ThrowIfNull(run);

        using var linked = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        using (observeClosed(linked.Cancel))
        {
            return await run(linked.Token).ConfigureAwait(true);
        }
    }

    /// <summary>
    /// Awaits a platform dialog but stops waiting as soon as the operation is
    /// cancelled.
    /// </summary>
    /// <remarks>
    /// A storage provider's picker task has no cancellation token: it completes
    /// when the dialog closes and not before. Racing it against the token is
    /// what lets a cancel, a closing parent window, or shutdown resolve the
    /// caller's completion promptly instead of waiting for a dialog that may
    /// never be dismissed. The abandoned task is explicitly observed so a later
    /// failure cannot surface as an unobserved task exception.
    /// </remarks>
    private static async Task<T> AwaitDialogAsync<T>(
        Task<T> dialog,
        CancellationToken cancellationToken)
    {
        if (!cancellationToken.CanBeCanceled)
            return await dialog.ConfigureAwait(true);

        try
        {
            return await dialog.WaitAsync(cancellationToken).ConfigureAwait(true);
        }
        catch (OperationCanceledException)
        {
            Observe(dialog);
            throw;
        }
    }

    private static void Observe<T>(Task<T> task) =>
        _ = task.ContinueWith(
            static completed => _ = completed.Exception,
            CancellationToken.None,
            TaskContinuationOptions.OnlyOnFaulted | TaskContinuationOptions.ExecuteSynchronously,
            TaskScheduler.Default);

    private static async Task<IStorageFolder?> ResolveStartLocationAsync(
        IStorageProvider provider,
        DesktopFilePickerRequest request)
    {        if (request.SuggestedStartWellKnownFolder is { } wellKnown)
        {
            var folder = await provider.TryGetWellKnownFolderAsync(wellKnown).ConfigureAwait(true);
            if (folder is not null)
                return folder;
        }

        if (string.IsNullOrWhiteSpace(request.SuggestedStartLocation))
            return null;

        var location = request.SuggestedStartLocation!;

        // A start location may point at a file the user last used; fall back to
        // its parent folder rather than failing the whole picker. Locations with
        // an explicit URI scheme go through the Uri overloads (which handle
        // non-file schemes); anything else is a filesystem path and uses the
        // string overloads, which avoid double-escaping it.
        if (StorageItemSnapshot.LooksLikeUri(location) &&
            Uri.TryCreate(location, UriKind.Absolute, out var uri))
        {
            var uriFolder = await provider.TryGetFolderFromPathAsync(uri).ConfigureAwait(true);
            if (uriFolder is not null)
                return uriFolder;
            var uriFile = await provider.TryGetFileFromPathAsync(uri).ConfigureAwait(true);
            return uriFile is null
                ? null
                : await uriFile.GetParentAsync().ConfigureAwait(true);
        }

        var direct = await provider.TryGetFolderFromPathAsync(location).ConfigureAwait(true);
        if (direct is not null)
            return direct;

        var file = await provider.TryGetFileFromPathAsync(location).ConfigureAwait(true);
        if (file is null)
            return null;
        return await file.GetParentAsync().ConfigureAwait(true);
    }
}

/// <summary>Public mirror of <c>AvnStorageCapabilities</c> for the picker core.</summary>
[Flags]
public enum AvnStorageCapabilityFlags
{
    None = 0,
    CanOpen = 1,
    CanSave = 2,
    CanPickFolder = 4,
}
