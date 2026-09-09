using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Host.Com;
using Avalonia.Host.Desktop;
using Avalonia.Input;
using Avalonia.Input.Platform;
using Avalonia.Platform.Storage;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

/// <summary>
/// Stage 31 clipboard commands. Everything is driven through Avalonia's own
/// <see cref="IClipboard"/> and <see cref="IStorageProvider"/> abstractions, so
/// a full round trip is exercised without a platform clipboard.
/// </summary>
public class DesktopClipboardTests
{
    [Fact]
    public void The_payload_builder_only_carries_primitives_and_snapshots_them()
    {
        var data = new AvnClipboardData();

        Assert.Equal(HResults.S_OK, data.SetText("copied"));
        Assert.Equal(HResults.S_OK, data.AddFileUri("file:///logs/a.log"));
        Assert.Equal(HResults.E_INVALIDARG, data.AddFileUri("   "));
        Assert.Equal(HResults.E_INVALIDARG, data.AddFileUri(null));

        var snapshot = data.Snapshot();
        data.SetText("changed");
        data.AddFileUri("file:///logs/b.log");

        Assert.Equal("copied", snapshot.Text);
        Assert.Equal(["file:///logs/a.log"], snapshot.FileUris);
    }

    [Fact]
    public async Task Text_and_files_round_trip_through_the_clipboard()
    {
        var clipboard = new FakeClipboard();
        var provider = new FakeStorageProvider();
        provider.FilesByPath["file:///logs/a.log"] =
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log"));

        await DesktopClipboard.WriteAsync(
            clipboard,
            provider,
            new ClipboardPayload("copied", ["file:///logs/a.log"]),
            CancellationToken.None);

        Assert.Equal("copied", await clipboard.Current!.TryGetTextAsync());
        var read = await DesktopClipboard.ReadFilesAsync(clipboard, CancellationToken.None);
        Assert.False(read.Cancelled);
        Assert.Equal("file:///logs/a.log", Assert.Single(read.Items).Uri);
    }

    [Fact]
    public async Task Clearing_removes_the_payload_and_leaves_a_readable_empty_clipboard()
    {
        var clipboard = new FakeClipboard();
        await DesktopClipboard.WriteAsync(
            clipboard,
            provider: null,
            new ClipboardPayload("copied", []),
            CancellationToken.None);

        await DesktopClipboard.ClearAsync(clipboard, CancellationToken.None);

        Assert.Null(clipboard.Current);
        Assert.Equal(1, clipboard.ClearCount);
        var read = await DesktopClipboard.ReadFilesAsync(clipboard, CancellationToken.None);
        Assert.Empty(read.Items);
    }

    [Fact]
    public async Task A_clipboard_with_no_files_is_an_empty_success_not_a_failure()
    {
        var clipboard = new FakeClipboard();
        await DesktopClipboard.WriteAsync(
            clipboard,
            provider: null,
            new ClipboardPayload("text only", []),
            CancellationToken.None);

        var read = await DesktopClipboard.ReadFilesAsync(clipboard, CancellationToken.None);

        Assert.False(read.Cancelled);
        Assert.Empty(read.Items);
    }

    [Fact]
    public async Task An_unresolvable_file_entry_is_dropped_rather_than_failing_the_copy()
    {
        var clipboard = new FakeClipboard();
        var provider = new FakeStorageProvider();
        provider.FilesByPath["file:///logs/a.log"] =
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log"));

        await DesktopClipboard.WriteAsync(
            clipboard,
            provider,
            new ClipboardPayload("copied", ["file:///logs/a.log", "file:///logs/deleted.log"]),
            CancellationToken.None);

        var read = await DesktopClipboard.ReadFilesAsync(clipboard, CancellationToken.None);
        Assert.Equal("a.log", Assert.Single(read.Items).Name);
    }

    [Fact]
    public async Task Writing_without_a_storage_provider_still_writes_the_text()
    {
        var clipboard = new FakeClipboard();

        await DesktopClipboard.WriteAsync(
            clipboard,
            provider: null,
            new ClipboardPayload("copied", ["file:///logs/a.log"]),
            CancellationToken.None);

        Assert.Equal("copied", await clipboard.Current!.TryGetTextAsync());
        Assert.Empty((await DesktopClipboard.ReadFilesAsync(clipboard, CancellationToken.None)).Items);
    }

    [Fact]
    public async Task Cancellation_is_observed_before_the_platform_is_touched()
    {
        var clipboard = new FakeClipboard();
        using var cancellation = new CancellationTokenSource();
        await cancellation.CancelAsync();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => DesktopClipboard.WriteAsync(
            clipboard,
            provider: null,
            new ClipboardPayload("copied", []),
            cancellation.Token));
        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => DesktopClipboard.ClearAsync(clipboard, cancellation.Token));
        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => DesktopClipboard.ReadFilesAsync(clipboard, cancellation.Token));

        Assert.Equal(0, clipboard.ClearCount);
        Assert.Equal(0, clipboard.WriteCount);
    }

    [Fact]
    public void Null_arguments_are_rejected_before_anything_is_started()
    {
        var application = new AvnApplication();

        Assert.Equal(HResults.E_POINTER, application.GetClipboardCapabilities(null, out _));
        Assert.Equal(HResults.E_POINTER, application.StartClipboardWrite(null, null, null, out _));
        Assert.Equal(HResults.E_POINTER, application.StartClipboardClear(null, null, out _));
        Assert.Equal(HResults.E_POINTER, application.StartClipboardReadFiles(null, null, out _));
    }

    [Fact]
    public void The_payload_builder_is_host_owned_and_creatable_without_a_window()
    {
        var application = new AvnApplication();

        Assert.Equal(HResults.S_OK, application.CreateClipboardData(out var data));
        Assert.NotNull(data);
        Assert.Equal(HResults.S_OK, data!.SetText("copied"));
    }

    /// <summary>
    /// A scripted <see cref="IClipboard"/>. The platform clipboard is the one
    /// part of a clipboard command that cannot be exercised in a unit test, so
    /// it is the only thing replaced here.
    /// </summary>
    private sealed class FakeClipboard : IClipboard
    {
        public IAsyncDataTransfer? Current { get; private set; }

        public int ClearCount { get; private set; }

        public int WriteCount { get; private set; }

        public Task ClearAsync()
        {
            ClearCount++;
            Current = null;
            return Task.CompletedTask;
        }

        public Task SetDataAsync(IAsyncDataTransfer? dataTransfer)
        {
            WriteCount++;
            Current = dataTransfer;
            return Task.CompletedTask;
        }

        public Task FlushAsync() => Task.CompletedTask;

        public Task<IAsyncDataTransfer?> TryGetDataAsync() => Task.FromResult(Current);

        public Task<IAsyncDataTransfer?> TryGetInProcessDataAsync() => Task.FromResult(Current);
    }
}
