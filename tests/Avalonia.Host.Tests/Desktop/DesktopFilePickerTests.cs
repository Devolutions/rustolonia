using System;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Host.Desktop;
using Avalonia.Platform.Storage;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

/// <summary>
/// Picker behaviour, driven entirely through a fake
/// <see cref="IStorageProvider"/> so no platform dialog is ever opened.
/// </summary>
public class DesktopFilePickerTests
{
    [Fact]
    public async Task Open_files_returns_every_selected_item_in_order()
    {
        var provider = new FakeStorageProvider();
        provider.OpenResult.Add(new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        provider.OpenResult.Add(new FakeStorageFile("b.log", new Uri("file:///logs/b.log")));

        var request = new DesktopFilePickerRequest { AllowMultiple = true, Title = "Open" };
        var result = await DesktopFilePickers.OpenFilesAsync(provider, request, CancellationToken.None);

        Assert.False(result.Cancelled);
        Assert.Equal(
            new[] { "file:///logs/a.log", "file:///logs/b.log" },
            result.Items.Select(item => item.Uri));
        Assert.True(provider.LastOpenOptions!.AllowMultiple);
        Assert.Equal("Open", provider.LastOpenOptions.Title);
    }

    [Fact]
    public async Task A_dismissed_dialog_is_cancelled_not_an_error()
    {
        var provider = new FakeStorageProvider();

        var files = await DesktopFilePickers.OpenFilesAsync(
            provider,
            new DesktopFilePickerRequest(),
            CancellationToken.None);
        var folders = await DesktopFilePickers.OpenFoldersAsync(
            provider,
            new DesktopFilePickerRequest(),
            CancellationToken.None);
        var save = await DesktopFilePickers.SaveFileAsync(
            provider,
            new DesktopFilePickerRequest(),
            CancellationToken.None);

        Assert.True(files.Cancelled);
        Assert.True(folders.Cancelled);
        Assert.True(save.Cancelled);
        Assert.Empty(files.Items);
        Assert.Empty(folders.Items);
        Assert.Empty(save.Items);
    }

    [Fact]
    public async Task Folder_picker_supports_multi_select_and_reports_folder_items()
    {
        var provider = new FakeStorageProvider();
        provider.FolderResult.Add(new FakeStorageFolder("one", new Uri("file:///data/one/")));
        provider.FolderResult.Add(new FakeStorageFolder("two", new Uri("file:///data/two/")));

        var result = await DesktopFilePickers.OpenFoldersAsync(
            provider,
            new DesktopFilePickerRequest { AllowMultiple = true },
            CancellationToken.None);

        Assert.Equal(2, result.Items.Count);
        Assert.All(result.Items, item => Assert.True(item.IsFolder));
        Assert.True(provider.LastFolderOptions!.AllowMultiple);
    }

    [Fact]
    public async Task Save_returns_a_path_and_uri_without_opening_a_managed_stream()
    {
        var provider = new FakeStorageProvider
        {
            SaveResult = new FakeStorageFile("export.log", new Uri("file:///exports/export.log")),
        };
        var request = new DesktopFilePickerRequest
        {
            SuggestedFileName = "export.log",
            DefaultExtension = "log",
            ShowOverwritePrompt = true,
        };

        var result = await DesktopFilePickers.SaveFileAsync(provider, request, CancellationToken.None);

        var item = Assert.Single(result.Items);
        Assert.False(item.IsFolder);
        Assert.Equal("file:///exports/export.log", item.Uri);
        Assert.EndsWith("export.log", item.LocalPath);
        Assert.Equal("export.log", provider.LastSaveOptions!.SuggestedFileName);
        Assert.Equal("log", provider.LastSaveOptions.DefaultExtension);
        Assert.True(provider.LastSaveOptions.ShowOverwritePrompt);
    }

    [Fact]
    public async Task Non_local_items_report_a_uri_and_no_path()
    {
        var provider = new FakeStorageProvider();
        provider.OpenResult.Add(
            new FakeStorageFile("doc", new Uri("content://media/documents/7")));

        var result = await DesktopFilePickers.OpenFilesAsync(
            provider,
            new DesktopFilePickerRequest(),
            CancellationToken.None);

        var item = Assert.Single(result.Items);
        Assert.Equal("content://media/documents/7", item.Uri);
        Assert.Null(item.LocalPath);
    }

    [Fact]
    public async Task File_type_filters_carry_patterns_mime_types_and_utis()
    {
        var provider = new FakeStorageProvider();
        var request = new DesktopFilePickerRequest { SuggestedFileTypeIndex = 1 };
        var logs = new DesktopFileTypeRequest("Log files");
        logs.Patterns.Add("*.log");
        logs.MimeTypes.Add("text/plain");
        logs.AppleUniformTypeIdentifiers.Add("public.plain-text");
        var all = new DesktopFileTypeRequest("All files");
        all.Patterns.Add("*.*");
        request.FileTypes.Add(logs);
        request.FileTypes.Add(all);

        await DesktopFilePickers.OpenFilesAsync(provider, request, CancellationToken.None);

        var filters = provider.LastOpenOptions!.FileTypeFilter!;
        Assert.Equal(2, filters.Count);
        Assert.Equal("Log files", filters[0].Name);
        Assert.Equal(new[] { "*.log" }, filters[0].Patterns);
        Assert.Equal(new[] { "text/plain" }, filters[0].MimeTypes);
        Assert.Equal(new[] { "public.plain-text" }, filters[0].AppleUniformTypeIdentifiers);
        Assert.Same(filters[1], provider.LastOpenOptions.SuggestedFileType);
    }

    [Fact]
    public async Task A_well_known_start_folder_wins_over_a_path()
    {
        var provider = new FakeStorageProvider();
        var documents = new FakeStorageFolder("Documents", new Uri("file:///users/me/documents/"));
        provider.WellKnownFolders[WellKnownFolder.Documents] = documents;
        provider.FoldersByPath["file:///other/"] = new FakeStorageFolder(
            "other",
            new Uri("file:///other/"));

        var request = new DesktopFilePickerRequest
        {
            SuggestedStartWellKnownFolder = WellKnownFolder.Documents,
            SuggestedStartLocation = "file:///other/",
        };
        await DesktopFilePickers.OpenFilesAsync(provider, request, CancellationToken.None);

        Assert.Same(documents, provider.LastOpenOptions!.SuggestedStartLocation);
    }

    [Fact]
    public async Task A_start_location_naming_a_file_falls_back_to_its_parent_folder()
    {
        var provider = new FakeStorageProvider();
        var parent = new FakeStorageFolder("logs", new Uri("file:///logs/"));
        provider.FilesByPath["file:///logs/a.log"] =
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log"), parent);

        var request = new DesktopFilePickerRequest
        {
            SuggestedStartLocation = "file:///logs/a.log",
        };
        await DesktopFilePickers.OpenFilesAsync(provider, request, CancellationToken.None);

        Assert.Same(parent, provider.LastOpenOptions!.SuggestedStartLocation);
    }

    [Fact]
    public async Task An_unresolvable_start_location_does_not_fail_the_picker()
    {
        var provider = new FakeStorageProvider();
        provider.OpenResult.Add(new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));

        var request = new DesktopFilePickerRequest
        {
            SuggestedStartLocation = "file:///nowhere/",
        };
        var result = await DesktopFilePickers.OpenFilesAsync(
            provider,
            request,
            CancellationToken.None);

        Assert.Null(provider.LastOpenOptions!.SuggestedStartLocation);
        Assert.Single(result.Items);
    }

    [Fact]
    public async Task A_provider_failure_surfaces_as_an_exception_not_a_cancellation()
    {
        var provider = new FakeStorageProvider
        {
            Gate = new TaskCompletionSource<bool>(),
            FailWith = new InvalidOperationException("provider exploded"),
        };
        var picker = DesktopFilePickers.OpenFilesAsync(
            provider,
            new DesktopFilePickerRequest(),
            CancellationToken.None);
        provider.Gate.SetResult(true);

        var error = await Assert.ThrowsAsync<InvalidOperationException>(() => picker);
        Assert.Equal("provider exploded", error.Message);
    }

    [Fact]
    public async Task A_cancelled_token_stops_the_picker_before_it_starts()
    {
        var provider = new FakeStorageProvider();
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => DesktopFilePickers.OpenFilesAsync(
                provider,
                new DesktopFilePickerRequest(),
                cancellation.Token));
        Assert.Equal(0, provider.OpenCallCount);
    }

    [Fact]
    public async Task Cancelling_after_the_dialog_opened_still_aborts_the_result()
    {
        var provider = new FakeStorageProvider { Gate = new TaskCompletionSource<bool>() };
        provider.OpenResult.Add(new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        using var cancellation = new CancellationTokenSource();

        var picker = DesktopFilePickers.OpenFilesAsync(
            provider,
            new DesktopFilePickerRequest(),
            cancellation.Token);
        cancellation.Cancel();
        provider.Gate.SetResult(true);

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => picker);
        Assert.Equal(1, provider.OpenCallCount);
    }

    [Fact]
    public async Task Cancelling_resolves_without_waiting_for_the_dialog_to_close()
    {
        // A storage provider's picker task has no cancellation token, so a
        // dialog that is never dismissed would otherwise leave the consumer's
        // completion pending forever.
        var provider = new FakeStorageProvider { Gate = new TaskCompletionSource<bool>() };
        using var cancellation = new CancellationTokenSource();

        var picker = DesktopFilePickers.OpenFilesAsync(
            provider,
            new DesktopFilePickerRequest(),
            cancellation.Token);
        cancellation.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => picker);
        Assert.Equal(1, provider.OpenCallCount);
        Assert.False(provider.Gate!.Task.IsCompleted, "the dialog is still open");

        // The abandoned dialog may still fail; that must not resurface anywhere.
        provider.FailWith = new InvalidOperationException("late dialog failure");
        provider.Gate.SetResult(true);
    }

    [Fact]
    public async Task Closing_the_owning_window_aborts_a_pending_picker()
    {
        var provider = new FakeStorageProvider { Gate = new TaskCompletionSource<bool>() };
        provider.OpenResult.Add(new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        Action? closed = null;

        var picker = DesktopFilePickers.RunWhileOpenAsync(
            callback =>
            {
                closed = callback;
                return new CallbackDisposable(() => closed = null);
            },
            cancellation => DesktopFilePickers.OpenFilesAsync(
                provider,
                new DesktopFilePickerRequest(),
                cancellation),
            CancellationToken.None);

        Assert.NotNull(closed);
        closed!();

        // Closing the window is a lifetime event, not a user choice, so it
        // aborts instead of reporting the "dismissed" outcome.
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => picker);
        Assert.Null(closed);
    }

    [Fact]
    public async Task A_window_that_stays_open_lets_the_picker_finish_normally()
    {
        var provider = new FakeStorageProvider();
        provider.OpenResult.Add(new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        var unsubscribed = false;

        var result = await DesktopFilePickers.RunWhileOpenAsync(
            _ => new CallbackDisposable(() => unsubscribed = true),
            cancellation => DesktopFilePickers.OpenFilesAsync(
                provider,
                new DesktopFilePickerRequest(),
                cancellation),
            CancellationToken.None);

        Assert.False(result.Cancelled);
        Assert.Equal("file:///logs/a.log", Assert.Single(result.Items).Uri);
        Assert.True(unsubscribed, "the close observer must not outlive the picker");
    }

    [Fact]
    public async Task An_outer_cancellation_still_reaches_a_window_scoped_picker()
    {
        var provider = new FakeStorageProvider { Gate = new TaskCompletionSource<bool>() };
        using var cancellation = new CancellationTokenSource();

        var picker = DesktopFilePickers.RunWhileOpenAsync(
            _ => new CallbackDisposable(static () => { }),
            token => DesktopFilePickers.OpenFilesAsync(
                provider,
                new DesktopFilePickerRequest(),
                token),
            cancellation.Token);
        cancellation.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => picker);
    }

    [Theory]
    [InlineData(true, true, true, 7)]
    [InlineData(true, false, false, 1)]
    [InlineData(false, false, true, 4)]
    [InlineData(false, false, false, 0)]
    public void Capabilities_are_reported_from_the_provider(
        bool canOpen,
        bool canSave,
        bool canPickFolder,
        int expected)
    {
        var provider = new FakeStorageProvider
        {
            CanOpen = canOpen,
            CanSave = canSave,
            CanPickFolder = canPickFolder,
        };

        Assert.Equal(
            (AvnStorageCapabilityFlags)expected,
            DesktopFilePickers.GetCapabilities(provider));
    }

    private sealed class CallbackDisposable(Action dispose) : IDisposable
    {
        public void Dispose() => dispose();
    }
}
