using System;
using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Host.Desktop;

namespace Avalonia.Host.Com;

internal sealed class AvnAsyncOperations
{
    private readonly ConcurrentDictionary<long, Operation> _operations = new();
    private long _nextOperationId;

    public int Start(
        IAvnAsyncCompletion? completion,
        Func<CancellationToken, Task<AvnAsyncValue>> operation,
        out long operationId) =>
        StartCore(
            completion,
            operation,
            static (completion, id, hresult, value, error) => Complete(
                (IAvnAsyncCompletion)completion,
                id,
                hresult,
                value,
                error),
            AvnAsyncValue.None,
            out operationId);

    /// <summary>
    /// Starts an operation whose result is a storage picker outcome. It shares
    /// the single operation registry with <see cref="Start"/>, so
    /// <see cref="Cancel"/>, <see cref="CancelAll"/> and <see cref="AbortAll"/>
    /// (teardown) apply uniformly to both, and a completion still crosses the
    /// ABI exactly once.
    /// </summary>
    public int StartStorage(
        IAvnStorageCompletion? completion,
        Func<CancellationToken, Task<DesktopPickerResult>> operation,
        out long operationId) =>
        StartCore(
            completion,
            operation,
            static (completion, id, hresult, value, error) => CompleteStorage(
                (IAvnStorageCompletion)completion,
                id,
                hresult,
                value,
                error),
            DesktopPickerResult.Cancel,
            out operationId);

    public int Cancel(long operationId)
    {
        if (!_operations.TryGetValue(operationId, out var operation))
            return HResults.E_INVALIDARG;
        operation.Cancel();
        return HResults.S_OK;
    }

    /// <summary>
    /// Requests cancellation of everything still pending and lets each
    /// operation deliver its own completion.
    /// </summary>
    public void CancelAll()
    {
        foreach (var operation in _operations.Values)
            operation.Cancel();
    }

    /// <summary>
    /// Aborts everything still pending and delivers each completion inline, on
    /// the calling thread.
    /// </summary>
    /// <remarks>
    /// This is the teardown counterpart of <see cref="CancelAll"/>. Cancelling
    /// only makes an operation <em>finish</em>; its completion is still
    /// delivered by a continuation that resumes through the dispatcher. Once the
    /// application loop has stopped the dispatcher rejects new work, so a
    /// cancelled operation would never call back at all and a consumer awaiting
    /// it would wait forever. Claiming each operation first is what keeps the
    /// completion exactly-once when its own continuation does still run.
    /// </remarks>
    public void AbortAll()
    {
        foreach (var pair in _operations)
        {
            var operation = pair.Value;
            operation.Cancel();
            if (!operation.TryClaimCompletion())
                continue;

            _operations.TryRemove(pair.Key, out _);
            try
            {
                operation.Abort();
            }
            catch (Exception e)
            {
                _ = AbiError.Capture(e);
            }
        }
    }

    private int StartCore<T>(
        object? completion,
        Func<CancellationToken, Task<T>> operation,
        Action<object, long, int, T, string?> complete,
        T failureValue,
        out long operationId)
    {
        operationId = 0;
        if (completion is null)
            return HResults.E_POINTER;

        var id = Interlocked.Increment(ref _nextOperationId);
        var cancellation = new CancellationTokenSource();
        var registration = new Operation(
            cancellation,
            (hresult, error) => complete(completion, id, hresult, failureValue, error));
        if (!_operations.TryAdd(id, registration))
        {
            cancellation.Dispose();
            return HResults.E_FAIL;
        }

        operationId = id;
        _ = Run(id, registration, cancellation, completion, operation, complete, failureValue);
        return HResults.S_OK;
    }

    private async Task Run<T>(
        long operationId,
        Operation registration,
        CancellationTokenSource cancellation,
        object completion,
        Func<CancellationToken, Task<T>> operation,
        Action<object, long, int, T, string?> complete,
        T failureValue)
    {
        int hresult;
        T value;
        string? error;
        try
        {
            value = await operation(cancellation.Token);
            cancellation.Token.ThrowIfCancellationRequested();
            hresult = HResults.S_OK;
            error = null;
        }
        catch (OperationCanceledException)
        {
            hresult = HResults.E_ABORT;
            value = failureValue;
            error = "The asynchronous operation was canceled.";
        }
        catch (Exception e)
        {
            hresult = Marshal.GetHRForException(e);
            value = failureValue;
            error = e.ToString();
        }
        finally
        {
            _operations.TryRemove(operationId, out _);
            cancellation.Dispose();
        }

        // Teardown may already have aborted this operation; a completion
        // crosses the ABI exactly once, never twice.
        if (!registration.TryClaimCompletion())
            return;

        try
        {
            complete(completion, operationId, hresult, value, error);
        }
        catch (Exception e)
        {
            _ = AbiError.Capture(e);
        }
    }

    /// <summary>One in-flight operation: its cancellation and its abort path.</summary>
    private sealed class Operation(CancellationTokenSource cancellation, Action<int, string?> abort)
    {
        private int _completed;

        /// <summary>Claims the exclusive right to deliver the one completion.</summary>
        public bool TryClaimCompletion() => Interlocked.Exchange(ref _completed, 1) == 0;

        public void Cancel()
        {
            try
            {
                cancellation.Cancel();
            }
            catch (ObjectDisposedException)
            {
                // The operation finished and disposed its source concurrently,
                // so there is nothing left to cancel.
            }
        }

        public void Abort() =>
            abort(
                HResults.E_ABORT,
                "The application exited while the asynchronous operation was pending.");
    }

    private static void Complete(
        IAvnAsyncCompletion completion,
        long operationId,
        int hresult,
        AvnAsyncValue value,
        string? error)
    {
        var completionResult = completion.Complete(
            operationId,
            hresult,
            value.Kind,
            value.Integer,
            value.Double,
            value.String,
            error);
        if (completionResult < 0)
            Marshal.ThrowExceptionForHR(completionResult);
    }

    private static void CompleteStorage(
        IAvnStorageCompletion completion,
        long operationId,
        int hresult,
        DesktopPickerResult value,
        string? error)
    {
        var outcome = value.Cancelled ? AvnStorageOutcome.Cancelled : AvnStorageOutcome.Completed;
        var completionResult = completion.Complete(
            operationId,
            hresult,
            (int)outcome,
            new AvnStorageItemList(value.Items),
            error);
        if (completionResult < 0)
            Marshal.ThrowExceptionForHR(completionResult);
    }
}

internal readonly record struct AvnAsyncValue(
    int Kind,
    long Integer,
    double Double,
    string? String)
{
    public static AvnAsyncValue None => default;

    public static AvnAsyncValue FromString(string? value) =>
        new(4, 0, 0, value);
}
