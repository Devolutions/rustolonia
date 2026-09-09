using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Host.Com;
using Avalonia.Host.Desktop;
using Avalonia.Platform.Storage;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

/// <summary>
/// The stage 29 ABI surface: option marshalling, storage item lists, and the
/// shared asynchronous operation registry that carries picker completions.
/// </summary>
public class DesktopFileAbiTests
{
    [Fact]
    public void Picker_options_round_trip_every_field_through_the_abi()
    {
        var options = new AvnFilePickerOptions();

        Assert.Equal(0, options.SetTitle("Open logs"));
        Assert.Equal(0, options.SetAllowMultiple(1));
        Assert.Equal(0, options.SetSuggestedFileName("trace.log"));
        Assert.Equal(0, options.SetSuggestedStartLocation("file:///logs/"));
        Assert.Equal(0, options.SetSuggestedStartWellKnownFolder((int)WellKnownFolder.Downloads));
        Assert.Equal(0, options.SetDefaultExtension("log"));
        Assert.Equal(0, options.SetShowOverwritePrompt(1));
        Assert.Equal(0, options.AddFileType("Log files", out var logs));
        Assert.Equal(0, options.AddFileTypePattern(logs, "*.log"));
        Assert.Equal(0, options.AddFileTypeMimeType(logs, "text/plain"));
        Assert.Equal(0, options.AddFileTypeAppleUniformTypeIdentifier(logs, "public.plain-text"));
        Assert.Equal(0, options.AddFileType("All files", out var all));
        Assert.Equal(0, options.AddFileTypePattern(all, "*.*"));
        Assert.Equal(0, options.SetSuggestedFileTypeIndex(all));

        var request = options.Snapshot();
        Assert.Equal("Open logs", request.Title);
        Assert.True(request.AllowMultiple);
        Assert.Equal("trace.log", request.SuggestedFileName);
        Assert.Equal("file:///logs/", request.SuggestedStartLocation);
        Assert.Equal(WellKnownFolder.Downloads, request.SuggestedStartWellKnownFolder);
        Assert.Equal("log", request.DefaultExtension);
        Assert.True(request.ShowOverwritePrompt);
        Assert.Equal(1, request.SuggestedFileTypeIndex);
        Assert.Equal(2, request.FileTypes.Count);
        Assert.Equal(new[] { "*.log" }, request.FileTypes[0].Patterns);
        Assert.Equal(new[] { "text/plain" }, request.FileTypes[0].MimeTypes);
        Assert.Equal(
            new[] { "public.plain-text" },
            request.FileTypes[0].AppleUniformTypeIdentifiers);
    }

    [Fact]
    public void Snapshotting_options_detaches_them_from_later_mutation()
    {
        var options = new AvnFilePickerOptions();
        options.SetTitle("first");
        options.AddFileType("Log files", out var logs);
        options.AddFileTypePattern(logs, "*.log");

        var snapshot = options.Snapshot();
        options.SetTitle("second");
        options.AddFileTypePattern(logs, "*.txt");
        options.AddFileType("All files", out _);

        Assert.Equal("first", snapshot.Title);
        Assert.Equal(new[] { "*.log" }, Assert.Single(snapshot.FileTypes).Patterns);
    }

    [Fact]
    public void Invalid_option_indices_and_values_are_rejected()
    {
        var options = new AvnFilePickerOptions();

        Assert.Equal(HResults.E_INVALIDARG, options.AddFileTypePattern(0, "*.log"));
        Assert.Equal(HResults.E_INVALIDARG, options.AddFileTypeMimeType(-1, "text/plain"));
        Assert.Equal(
            HResults.E_INVALIDARG,
            options.AddFileTypeAppleUniformTypeIdentifier(3, "public.data"));
        Assert.Equal(HResults.E_INVALIDARG, options.SetSuggestedFileTypeIndex(1));
        Assert.Equal(HResults.E_INVALIDARG, options.SetSuggestedStartWellKnownFolder(99));

        // -1 clears rather than fails, so an optional value has one spelling.
        Assert.Equal(HResults.S_OK, options.SetSuggestedStartWellKnownFolder(-1));
        Assert.Equal(HResults.S_OK, options.SetSuggestedFileTypeIndex(-1));
        Assert.Null(options.Snapshot().SuggestedStartWellKnownFolder);
        Assert.Equal(-1, options.Snapshot().SuggestedFileTypeIndex);
    }

    [Fact]
    public void The_overwrite_prompt_distinguishes_unset_from_false()
    {
        var options = new AvnFilePickerOptions();
        Assert.Null(options.Snapshot().ShowOverwritePrompt);

        options.SetShowOverwritePrompt(0);
        Assert.False(options.Snapshot().ShowOverwritePrompt);

        options.SetShowOverwritePrompt(-1);
        Assert.Null(options.Snapshot().ShowOverwritePrompt);
    }

    [Fact]
    public void A_storage_item_list_exposes_items_and_rejects_bad_indices()
    {
        var list = new AvnStorageItemList(
        [
            new StorageItemSnapshot(false, "a.log", "file:///logs/a.log", "/logs/a.log"),
            new StorageItemSnapshot(true, "logs", "content://logs/", null),
        ]);

        Assert.Equal(0, list.GetCount(out var count));
        Assert.Equal(2, count);

        Assert.Equal(0, list.GetItem(0, out var first));
        Assert.Equal(0, first!.GetKind(out var firstKind));
        Assert.Equal(0, firstKind);
        Assert.Equal(0, first.GetName(out var firstName));
        Assert.Equal("a.log", firstName);
        Assert.Equal(0, first.GetUri(out var firstUri));
        Assert.Equal("file:///logs/a.log", firstUri);
        Assert.Equal(0, first.TryGetLocalPath(out var firstFound, out var firstPath));
        Assert.Equal(1, firstFound);
        Assert.Equal("/logs/a.log", firstPath);

        Assert.Equal(0, list.GetItem(1, out var second));
        Assert.Equal(0, second!.GetKind(out var secondKind));
        Assert.Equal(1, secondKind);
        Assert.Equal(0, second.TryGetLocalPath(out var secondFound, out var secondPath));
        Assert.Equal(0, secondFound);
        Assert.Null(secondPath);

        Assert.Equal(HResults.E_INVALIDARG, list.GetItem(-1, out _));
        Assert.Equal(HResults.E_INVALIDARG, list.GetItem(2, out _));
    }

    [Fact]
    public void A_storage_completion_reports_selected_items_exactly_once()
    {
        var operations = new AvnAsyncOperations();
        var completion = new StorageCompletion();
        var provider = new FakeStorageProvider();
        provider.OpenResult.Add(new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));

        Assert.Equal(
            0,
            operations.StartStorage(
                completion,
                cancellation => DesktopFilePickers.OpenFilesAsync(
                    provider,
                    new DesktopFilePickerRequest(),
                    cancellation),
                out var operationId));

        var result = completion.Wait();
        Assert.Equal(operationId, result.OperationId);
        Assert.Equal(0, result.HResult);
        Assert.Equal((int)AvnStorageOutcome.Completed, result.Outcome);
        Assert.Equal(new[] { "file:///logs/a.log" }, result.Uris);
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void A_dismissed_dialog_completes_successfully_with_the_cancelled_outcome()
    {
        var operations = new AvnAsyncOperations();
        var completion = new StorageCompletion();

        Assert.Equal(
            0,
            operations.StartStorage(
                completion,
                cancellation => DesktopFilePickers.OpenFilesAsync(
                    new FakeStorageProvider(),
                    new DesktopFilePickerRequest(),
                    cancellation),
                out _));

        var result = completion.Wait();
        Assert.Equal(0, result.HResult);
        Assert.Equal((int)AvnStorageOutcome.Cancelled, result.Outcome);
        Assert.Empty(result.Uris);
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void Cancelling_a_storage_operation_completes_with_abort_once()
    {
        var operations = new AvnAsyncOperations();
        var completion = new StorageCompletion();
        var provider = new FakeStorageProvider { Gate = new TaskCompletionSource<bool>() };

        Assert.Equal(
            0,
            operations.StartStorage(
                completion,
                cancellation => DesktopFilePickers.OpenFilesAsync(
                    provider,
                    new DesktopFilePickerRequest(),
                    cancellation),
                out var operationId));

        Assert.Equal(0, operations.Cancel(operationId));
        provider.Gate!.SetResult(true);

        var result = completion.Wait();
        Assert.Equal(HResults.E_ABORT, result.HResult);
        Assert.Equal((int)AvnStorageOutcome.Cancelled, result.Outcome);
        Assert.Empty(result.Uris);
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void Shutdown_cancels_pending_storage_operations()
    {
        var operations = new AvnAsyncOperations();
        var completion = new StorageCompletion();
        var provider = new FakeStorageProvider { Gate = new TaskCompletionSource<bool>() };

        Assert.Equal(
            0,
            operations.StartStorage(
                completion,
                cancellation => DesktopFilePickers.OpenFilesAsync(
                    provider,
                    new DesktopFilePickerRequest(),
                    cancellation),
                out _));

        operations.CancelAll();
        provider.Gate!.SetResult(true);

        Assert.Equal(HResults.E_ABORT, completion.Wait().HResult);
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void Teardown_completes_a_pending_operation_inline()
    {
        // Cancelling alone would leave the completion to a continuation that
        // resumes through the dispatcher; once the application loop has stopped
        // that continuation never runs, so teardown has to deliver the
        // completion itself.
        var operations = new AvnAsyncOperations();
        var completion = new StorageCompletion();
        var provider = new FakeStorageProvider { Gate = new TaskCompletionSource<bool>() };

        Assert.Equal(
            0,
            operations.StartStorage(
                completion,
                cancellation => DesktopFilePickers.OpenFilesAsync(
                    provider,
                    new DesktopFilePickerRequest(),
                    cancellation),
                out var operationId));

        operations.AbortAll();

        // Delivered synchronously, with no dispatcher and with the dialog still
        // open: nothing was waited on.
        Assert.Equal(1, completion.CallCount);
        Assert.False(provider.Gate!.Task.IsCompleted);
        var result = completion.Wait();
        Assert.Equal(operationId, result.OperationId);
        Assert.Equal(HResults.E_ABORT, result.HResult);
        Assert.Equal((int)AvnStorageOutcome.Cancelled, result.Outcome);
        Assert.Empty(result.Uris);

        // The abandoned operation must not complete a second time when its own
        // continuation finally runs, and the operation must be deregistered.
        provider.Gate.SetResult(true);
        Assert.Equal(HResults.E_INVALIDARG, operations.Cancel(operationId));
        Thread.Sleep(50);
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void Teardown_completes_a_pending_delay_operation_inline()
    {
        // The registry is shared, so the same guarantee has to hold for the
        // pre-existing tagged-value completions, not just for pickers.
        var operations = new AvnAsyncOperations();
        var completion = new AsyncCompletion();

        Assert.Equal(
            0,
            operations.Start(
                completion,
                async cancellation =>
                {
                    await Task.Delay(Timeout.Infinite, cancellation);
                    return AvnAsyncValue.None;
                },
                out _));

        operations.AbortAll();

        Assert.Equal(1, completion.CallCount);
        Assert.Equal(HResults.E_ABORT, completion.HResult);
    }

    [Fact]
    public void Teardown_after_an_operation_already_completed_does_nothing()
    {
        var operations = new AvnAsyncOperations();
        var completion = new StorageCompletion();

        Assert.Equal(
            0,
            operations.StartStorage(
                completion,
                _ => Task.FromResult(DesktopPickerResult.Cancel),
                out _));

        Assert.Equal(0, completion.Wait().HResult);
        operations.AbortAll();
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void A_failing_picker_completes_with_the_error_text()
    {
        var operations = new AvnAsyncOperations();
        var completion = new StorageCompletion();
        var provider = new FakeStorageProvider
        {
            FailWith = new NotSupportedException("no storage provider"),
        };

        Assert.Equal(
            0,
            operations.StartStorage(
                completion,
                cancellation => DesktopFilePickers.OpenFilesAsync(
                    provider,
                    new DesktopFilePickerRequest(),
                    cancellation),
                out _));

        var result = completion.Wait();
        Assert.True(result.HResult < 0);
        Assert.NotEqual(HResults.E_ABORT, result.HResult);
        Assert.Contains("no storage provider", result.Error);
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void A_failing_storage_completion_is_not_retried()
    {
        var operations = new AvnAsyncOperations();
        var completion = new FailingStorageCompletion();

        Assert.Equal(
            0,
            operations.StartStorage(
                completion,
                _ => Task.FromResult(DesktopPickerResult.Cancel),
                out _));

        completion.Wait();
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void A_null_storage_completion_is_rejected_without_starting_work()
    {
        var operations = new AvnAsyncOperations();
        var started = false;

        Assert.Equal(
            HResults.E_POINTER,
            operations.StartStorage(
                null,
                _ =>
                {
                    started = true;
                    return Task.FromResult(DesktopPickerResult.Cancel);
                },
                out var operationId));

        Assert.False(started);
        Assert.Equal(0, operationId);
    }

    [Fact]
    public void Cancelling_an_unknown_operation_is_rejected()
    {
        var operations = new AvnAsyncOperations();
        Assert.Equal(HResults.E_INVALIDARG, operations.Cancel(42));
    }

    private sealed class StorageCompletion : IAvnStorageCompletion
    {
        private readonly ManualResetEventSlim _completed = new();
        private Result? _result;

        public int CallCount { get; private set; }

        public Result Wait()
        {
            Assert.True(_completed.Wait(10_000));
            return _result!;
        }

        public int Complete(
            long operationId,
            int hresult,
            int outcome,
            IAvnStorageItemList? items,
            string? error)
        {
            CallCount++;
            var uris = new List<string>();
            if (items is not null)
            {
                Assert.Equal(0, items.GetCount(out var count));
                for (var i = 0; i < count; i++)
                {
                    Assert.Equal(0, items.GetItem(i, out var item));
                    Assert.Equal(0, item!.GetUri(out var uri));
                    uris.Add(uri!);
                }
            }

            _result = new Result(operationId, hresult, outcome, uris, error);
            _completed.Set();
            return 0;
        }
    }

    private sealed record Result(
        long OperationId,
        int HResult,
        int Outcome,
        IReadOnlyList<string> Uris,
        string? Error);

    private sealed class AsyncCompletion : IAvnAsyncCompletion
    {
        public int CallCount { get; private set; }

        public int HResult { get; private set; }

        public int Complete(
            long operationId,
            int hresult,
            int valueKind,
            long integerValue,
            double doubleValue,
            string? stringValue,
            string? error)
        {
            CallCount++;
            HResult = hresult;
            return 0;
        }
    }

    private sealed class FailingStorageCompletion : IAvnStorageCompletion
    {
        private readonly ManualResetEventSlim _called = new();

        public int CallCount { get; private set; }

        public void Wait() => Assert.True(_called.Wait(10_000));

        public int Complete(
            long operationId,
            int hresult,
            int outcome,
            IAvnStorageItemList? items,
            string? error)
        {
            CallCount++;
            _called.Set();
            return HResults.E_FAIL;
        }
    }
}
