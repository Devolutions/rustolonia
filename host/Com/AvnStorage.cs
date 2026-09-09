using System;
using System.Collections.Generic;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Host.Desktop;
using Avalonia.Platform.Storage;

namespace Avalonia.Host.Com;

/// <summary>
/// ABI view over an immutable <see cref="StorageItemSnapshot"/>.
/// </summary>
[GeneratedComClass]
public sealed partial class AvnStorageItem : IAvnStorageItem
{
    private readonly StorageItemSnapshot _snapshot;

    internal AvnStorageItem(StorageItemSnapshot snapshot) => _snapshot = snapshot;

    internal StorageItemSnapshot Snapshot => _snapshot;

    public int GetKind(out int value)
    {
        value = (int)(_snapshot.IsFolder ? AvnStorageItemKind.Folder : AvnStorageItemKind.File);
        return HResults.S_OK;
    }

    public int GetName(out string? value)
    {
        value = _snapshot.Name;
        return HResults.S_OK;
    }

    public int GetUri(out string? value)
    {
        value = _snapshot.Uri;
        return HResults.S_OK;
    }

    public int TryGetLocalPath(out int found, out string? value)
    {
        value = _snapshot.LocalPath;
        found = value is null ? 0 : 1;
        return HResults.S_OK;
    }
}

/// <summary>ABI view over an ordered snapshot list.</summary>
[GeneratedComClass]
public sealed partial class AvnStorageItemList : IAvnStorageItemList
{
    private readonly IReadOnlyList<StorageItemSnapshot> _items;

    internal AvnStorageItemList(IReadOnlyList<StorageItemSnapshot> items) => _items = items;

    internal static AvnStorageItemList Empty() =>
        new(Array.Empty<StorageItemSnapshot>());

    public int GetCount(out int value)
    {
        value = _items.Count;
        return HResults.S_OK;
    }

    public int GetItem(int index, out IAvnStorageItem? value)
    {
        value = null;
        if (index < 0 || index >= _items.Count)
            return HResults.E_INVALIDARG;
        value = new AvnStorageItem(_items[index]);
        return HResults.S_OK;
    }
}

/// <summary>
/// Host-owned picker options builder handed to Rust so the ABI only carries
/// primitives and strings.
/// </summary>
[GeneratedComClass]
public sealed partial class AvnFilePickerOptions : IAvnFilePickerOptions
{
    private readonly DesktopFilePickerRequest _request = new();

    /// <summary>
    /// Copies the accumulated options. Pickers run against the copy so a
    /// consumer that keeps mutating the builder after starting an operation
    /// cannot change an in-flight dialog.
    /// </summary>
    internal DesktopFilePickerRequest Snapshot()
    {
        var copy = new DesktopFilePickerRequest
        {
            Title = _request.Title,
            AllowMultiple = _request.AllowMultiple,
            SuggestedFileName = _request.SuggestedFileName,
            SuggestedStartLocation = _request.SuggestedStartLocation,
            SuggestedStartWellKnownFolder = _request.SuggestedStartWellKnownFolder,
            DefaultExtension = _request.DefaultExtension,
            ShowOverwritePrompt = _request.ShowOverwritePrompt,
            SuggestedFileTypeIndex = _request.SuggestedFileTypeIndex,
        };
        foreach (var type in _request.FileTypes)
        {
            var clone = new DesktopFileTypeRequest(type.Name);
            clone.Patterns.AddRange(type.Patterns);
            clone.MimeTypes.AddRange(type.MimeTypes);
            clone.AppleUniformTypeIdentifiers.AddRange(type.AppleUniformTypeIdentifiers);
            copy.FileTypes.Add(clone);
        }

        return copy;
    }

    public int SetTitle(string? value)
    {
        _request.Title = value;
        return HResults.S_OK;
    }

    public int SetAllowMultiple(int value)
    {
        _request.AllowMultiple = value != 0;
        return HResults.S_OK;
    }

    public int SetSuggestedFileName(string? value)
    {
        _request.SuggestedFileName = value;
        return HResults.S_OK;
    }

    public int SetSuggestedStartLocation(string? uriOrPath)
    {
        _request.SuggestedStartLocation = uriOrPath;
        return HResults.S_OK;
    }

    public int SetSuggestedStartWellKnownFolder(int value)
    {
        if (value < 0)
        {
            _request.SuggestedStartWellKnownFolder = null;
            return HResults.S_OK;
        }

        if (!Enum.IsDefined(typeof(WellKnownFolder), value))
            return HResults.E_INVALIDARG;
        _request.SuggestedStartWellKnownFolder = (WellKnownFolder)value;
        return HResults.S_OK;
    }

    public int SetDefaultExtension(string? value)
    {
        _request.DefaultExtension = value;
        return HResults.S_OK;
    }

    public int SetShowOverwritePrompt(int value)
    {
        _request.ShowOverwritePrompt = value < 0 ? null : value != 0;
        return HResults.S_OK;
    }

    public int AddFileType(string? name, out int index)
    {
        index = _request.FileTypes.Count;
        _request.FileTypes.Add(new DesktopFileTypeRequest(name));
        return HResults.S_OK;
    }

    public int AddFileTypePattern(int index, string? pattern)
    {
        if (!TryGetFileType(index, out var type) || pattern is null)
            return HResults.E_INVALIDARG;
        type.Patterns.Add(pattern);
        return HResults.S_OK;
    }

    public int AddFileTypeMimeType(int index, string? mimeType)
    {
        if (!TryGetFileType(index, out var type) || mimeType is null)
            return HResults.E_INVALIDARG;
        type.MimeTypes.Add(mimeType);
        return HResults.S_OK;
    }

    public int AddFileTypeAppleUniformTypeIdentifier(int index, string? identifier)
    {
        if (!TryGetFileType(index, out var type) || identifier is null)
            return HResults.E_INVALIDARG;
        type.AppleUniformTypeIdentifiers.Add(identifier);
        return HResults.S_OK;
    }

    public int SetSuggestedFileTypeIndex(int index)
    {
        if (index >= _request.FileTypes.Count)
            return HResults.E_INVALIDARG;
        _request.SuggestedFileTypeIndex = index < 0 ? -1 : index;
        return HResults.S_OK;
    }

    private bool TryGetFileType(int index, out DesktopFileTypeRequest type)
    {
        if (index < 0 || index >= _request.FileTypes.Count)
        {
            type = null!;
            return false;
        }

        type = _request.FileTypes[index];
        return true;
    }
}
