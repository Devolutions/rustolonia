using System;
using System.Collections.Generic;
using System.IO;
using System.Threading.Tasks;
using Avalonia.Platform.Storage;

namespace Avalonia.Host.Tests.Desktop;

/// <summary>
/// A scripted <see cref="IStorageProvider"/> that lets picker flows be tested
/// end to end without opening a platform dialog.
/// </summary>
internal sealed class FakeStorageProvider : IStorageProvider
{
    public bool CanOpen { get; set; } = true;

    public bool CanSave { get; set; } = true;

    public bool CanPickFolder { get; set; } = true;

    public FilePickerOpenOptions? LastOpenOptions { get; private set; }

    public FolderPickerOpenOptions? LastFolderOptions { get; private set; }

    public FilePickerSaveOptions? LastSaveOptions { get; private set; }

    public List<IStorageFile> OpenResult { get; } = new();

    public List<IStorageFolder> FolderResult { get; } = new();

    public IStorageFile? SaveResult { get; set; }

    public Dictionary<string, IStorageFolder> FoldersByPath { get; } = new(StringComparer.Ordinal);

    public Dictionary<string, IStorageFile> FilesByPath { get; } = new(StringComparer.Ordinal);

    public Dictionary<WellKnownFolder, IStorageFolder> WellKnownFolders { get; } = new();

    public Exception? FailWith { get; set; }

    /// <summary>Completes only when set, so cancellation can be tested.</summary>
    public TaskCompletionSource<bool>? Gate { get; set; }

    public int OpenCallCount { get; private set; }

    public async Task<IReadOnlyList<IStorageFile>> OpenFilePickerAsync(FilePickerOpenOptions options)
    {
        OpenCallCount++;
        LastOpenOptions = options;
        await WaitAsync().ConfigureAwait(true);
        return OpenResult.ToArray();
    }

    public async Task<OpenFilePickerResult> OpenFilePickerWithResultAsync(
        FilePickerOpenOptions options) =>
        new() { Files = await OpenFilePickerAsync(options).ConfigureAwait(true) };

    public async Task<IStorageFile?> SaveFilePickerAsync(FilePickerSaveOptions options)
    {
        LastSaveOptions = options;
        await WaitAsync().ConfigureAwait(true);
        return SaveResult;
    }

    public async Task<SaveFilePickerResult> SaveFilePickerWithResultAsync(
        FilePickerSaveOptions options) =>
        new() { File = await SaveFilePickerAsync(options).ConfigureAwait(true) };

    public async Task<IReadOnlyList<IStorageFolder>> OpenFolderPickerAsync(
        FolderPickerOpenOptions options)
    {
        LastFolderOptions = options;
        await WaitAsync().ConfigureAwait(true);
        return FolderResult.ToArray();
    }

    public Task<IStorageBookmarkFile?> OpenFileBookmarkAsync(string bookmark) =>
        Task.FromResult<IStorageBookmarkFile?>(null);

    public Task<IStorageBookmarkFolder?> OpenFolderBookmarkAsync(string bookmark) =>
        Task.FromResult<IStorageBookmarkFolder?>(null);

    public Task<IStorageFile?> TryGetFileFromPathAsync(Uri filePath) =>
        Task.FromResult(FilesByPath.TryGetValue(filePath.ToString(), out var file) ? file : null);

    public Task<IStorageFolder?> TryGetFolderFromPathAsync(Uri folderPath) =>
        Task.FromResult(
            FoldersByPath.TryGetValue(folderPath.ToString(), out var folder) ? folder : null);

    public Task<IStorageFolder?> TryGetWellKnownFolderAsync(WellKnownFolder wellKnownFolder) =>
        Task.FromResult(
            WellKnownFolders.TryGetValue(wellKnownFolder, out var folder) ? folder : null);

    private async Task WaitAsync()
    {
        if (Gate is not null)
            await Gate.Task.ConfigureAwait(true);
        if (FailWith is not null)
            throw FailWith;
    }
}

/// <summary>A file whose URI may or may not map to a local filesystem path.</summary>
internal sealed class FakeStorageFile(string name, Uri path, IStorageFolder? parent = null)
    : IStorageFile
{
    public string Name { get; } = name;

    public Uri Path { get; } = path;

    public bool CanBookmark => false;

    public IStorageFolder? Parent { get; } = parent;

    public Task<StorageItemProperties> GetBasicPropertiesAsync() =>
        Task.FromResult(new StorageItemProperties());

    public Task<string?> SaveBookmarkAsync() => Task.FromResult<string?>(null);

    public Task<IStorageFolder?> GetParentAsync() => Task.FromResult(Parent);

    public Task DeleteAsync() => Task.CompletedTask;

    public Task<IStorageItem?> MoveAsync(IStorageFolder destination) =>
        Task.FromResult<IStorageItem?>(null);

    public Task<Stream> OpenReadAsync() => Task.FromResult<Stream>(new MemoryStream());

    public Task<Stream> OpenWriteAsync() => Task.FromResult<Stream>(new MemoryStream());

    public void Dispose()
    {
    }
}

internal sealed class FakeStorageFolder(string name, Uri path) : IStorageFolder
{
    public string Name { get; } = name;

    public Uri Path { get; } = path;

    public bool CanBookmark => false;

    public Task<StorageItemProperties> GetBasicPropertiesAsync() =>
        Task.FromResult(new StorageItemProperties());

    public Task<string?> SaveBookmarkAsync() => Task.FromResult<string?>(null);

    public Task<IStorageFolder?> GetParentAsync() => Task.FromResult<IStorageFolder?>(null);

    public Task DeleteAsync() => Task.CompletedTask;

    public Task<IStorageItem?> MoveAsync(IStorageFolder destination) =>
        Task.FromResult<IStorageItem?>(null);

    public async IAsyncEnumerable<IStorageItem> GetItemsAsync()
    {
        await Task.CompletedTask;
        yield break;
    }

    public Task<IStorageFolder?> GetFolderAsync(string name) =>
        Task.FromResult<IStorageFolder?>(null);

    public Task<IStorageFile?> GetFileAsync(string name) => Task.FromResult<IStorageFile?>(null);

    public Task<IStorageFile?> CreateFileAsync(string name) => Task.FromResult<IStorageFile?>(null);

    public Task<IStorageFolder?> CreateFolderAsync(string name) =>
        Task.FromResult<IStorageFolder?>(null);

    public void Dispose()
    {
    }
}
