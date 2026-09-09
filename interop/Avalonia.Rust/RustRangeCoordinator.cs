using System;
using System.Collections.Generic;
using Avalonia.Rust.Interop;
using Avalonia.Threading;

namespace Avalonia.Rust;

/// <summary>
/// Queues, decodes and applies windowed range batches.
///
/// It mirrors <see cref="RustVmBatchCoordinator"/>'s discipline at a much
/// smaller scale: <see cref="Publish"/> never reads the batch on the
/// submitting (Rust worker) stack, decoding fully validates the batch before
/// anything is realized, and a stale generation is completed as
/// <see cref="RustVmBatchOutcome.Stale"/> with no managed state touched and no
/// element adapters leaked.
/// </summary>
public sealed class RustRangeCoordinator
{
    private const int InvalidArgument = unchecked((int)0x80070057);
    private const int Failure = unchecked((int)0x80004005);

    /// <summary>Wire tag for a batch that republishes a dataset's identity.</summary>
    public const int RangeReset = 0;

    /// <summary>Wire tag for a batch that realizes one page.</summary>
    public const int RangeFill = 1;

    /// <summary>
    /// Wire tag for a batch that re-requests realized pages at the current
    /// generation without dropping dataset identity.
    /// </summary>
    public const int RangeInvalidate = 2;

    private readonly Func<int, RustWindowedCollection?> _resolve;
    private readonly Action<Action> _post;
    private volatile bool _closed;

    /// <param name="resolve">Maps a schema collection ID to its windowed projection.</param>
    /// <param name="post">Nonblocking UI-thread post; defaults to the Avalonia dispatcher.</param>
    public RustRangeCoordinator(Func<int, RustWindowedCollection?> resolve, Action<Action>? post = null)
    {
        _resolve = resolve ?? throw new ArgumentNullException(nameof(resolve));
        _post = post ?? (action => Dispatcher.UIThread.Post(action));
    }

    /// <summary>Stops accepting range batches; queued batches complete as cancelled.</summary>
    public void Close() => _closed = true;

    /// <summary>Enqueues one range batch without reading or completing it inline.</summary>
    public int Publish(IAvnRustVmRangeBatch? batch)
    {
        if (batch is null)
            return InvalidArgument;
        if (_closed)
        {
            PostComplete(batch, RustVmBatchOutcome.Cancelled, 0);
            return 0;
        }
        try
        {
            _post(() => Apply(batch));
            return 0;
        }
        catch
        {
            PostComplete(batch, RustVmBatchOutcome.Error, Failure);
            return Failure;
        }
    }

    private void PostComplete(IAvnRustVmRangeBatch batch, RustVmBatchOutcome outcome, int error)
    {
        try
        {
            _post(() => Complete(batch, outcome, error));
        }
        catch
        {
            // The dispatcher is gone; Rust's completion resolves when the batch drops.
        }
    }

    private void Apply(IAvnRustVmRangeBatch batch)
    {
        if (_closed)
        {
            Complete(batch, RustVmBatchOutcome.Cancelled, 0);
            return;
        }

        try
        {
            var hr = batch.GetCollectionId(out var collectionId);
            if (hr < 0)
            {
                Complete(batch, RustVmBatchOutcome.Error, hr);
                return;
            }
            hr = batch.GetGeneration(out var generation);
            if (hr < 0)
            {
                Complete(batch, RustVmBatchOutcome.Error, hr);
                return;
            }
            var (outcome, error) = Read(batch, collectionId, generation);
            Complete(batch, outcome, error);
        }
        catch (Exception error)
        {
            Complete(batch, RustVmBatchOutcome.Error, error.HResult < 0 ? error.HResult : Failure);
        }
    }

    /// <summary>
    /// Decodes and applies one batch. Every element is staged first, so a
    /// failure or a stale generation discovered mid-decode releases the staged
    /// adapters and leaves the window exactly as it was.
    /// </summary>
    private (RustVmBatchOutcome Outcome, int Error) Read(IAvnRustVmRangeBatch batch, int collectionId, long generation)
    {
        var window = _resolve(collectionId);
        if (window is null)
            return (RustVmBatchOutcome.Error, InvalidArgument);

        var hr = batch.GetKind(out var kind);
        if (hr < 0)
            return (RustVmBatchOutcome.Error, hr);
        hr = batch.GetTotalCount(out var totalCount);
        if (hr < 0)
            return (RustVmBatchOutcome.Error, hr);
        hr = batch.GetOffset(out var offset);
        if (hr < 0)
            return (RustVmBatchOutcome.Error, hr);
        hr = batch.GetItemCount(out var count);
        if (hr < 0)
        {
            window.AbandonPage(generation, offset);
            return (RustVmBatchOutcome.Error, hr);
        }
        if (count < 0 || totalCount < 0 || offset < 0)
        {
            window.AbandonPage(generation, offset);
            return (RustVmBatchOutcome.Error, InvalidArgument);
        }

        if (kind == RangeReset)
        {
            // Rust is the authority on dataset identity, so a reset is always
            // accepted; it invalidates every realized page.
            window.ResetTo(generation, totalCount);
            return (RustVmBatchOutcome.Applied, 0);
        }
        if (kind == RangeInvalidate)
        {
            if (generation != window.Generation)
                return (RustVmBatchOutcome.Stale, 0);
            window.RefreshRealized();
            return (RustVmBatchOutcome.Applied, 0);
        }
        if (kind != RangeFill)
        {
            // A producer newer than this host: reject the batch, but leave the
            // page requestable rather than blank forever.
            window.AbandonPage(generation, offset);
            return (RustVmBatchOutcome.Error, InvalidArgument);
        }

        // Reject before creating a single adapter: a stale generation must not
        // allocate managed presentation objects at all.
        if (generation != window.Generation)
        {
            window.AbandonPage(generation, offset);
            return (RustVmBatchOutcome.Stale, 0);
        }

        var staged = new List<object?>(count);
        for (var index = 0; index < count; index++)
        {
            hr = batch.GetItemModel(index, out var model);
            if (hr < 0)
            {
                DisposeStaged(staged);
                window.AbandonPage(generation, offset);
                return (RustVmBatchOutcome.Error, hr);
            }
            string? text = null;
            if (model is null)
            {
                hr = RustRangeReader.ReadText(batch, index, out text);
                if (hr < 0)
                {
                    DisposeStaged(staged);
                    window.AbandonPage(generation, offset);
                    return (RustVmBatchOutcome.Error, hr);
                }
            }
            try
            {
                staged.Add(window.CreateElement(model, text));
            }
            catch
            {
                // The element never took ownership of the model, so release it
                // here rather than leaving it to finalization.
                (model as IDisposable)?.Dispose();
                DisposeStaged(staged);
                window.AbandonPage(generation, offset);
                throw;
            }
        }

        if (window.ApplyRange(generation, totalCount, offset, staged))
            return (RustVmBatchOutcome.Applied, 0);
        DisposeStaged(staged);
        return (RustVmBatchOutcome.Stale, 0);
    }

    private static void DisposeStaged(List<object?>? staged)
    {
        if (staged is null)
            return;
        foreach (var item in staged)
        {
            if (item is IDisposable disposable)
            {
                try { disposable.Dispose(); }
                catch { /* Rollback must not surface a secondary failure. */ }
            }
        }
    }

    private static void Complete(IAvnRustVmRangeBatch batch, RustVmBatchOutcome outcome, int error)
    {
        try
        {
            _ = batch.Complete((int)outcome, error);
        }
        catch
        {
            // Rust owns the completion channel; a foreign failure here must not
            // turn an already applied range into a managed exception.
        }
    }
}

internal static unsafe class RustRangeReader
{
    private const int InvalidArgument = unchecked((int)0x80070057);

    internal static int ReadText(IAvnRustVmRangeBatch batch, int index, out string value)
    {
        value = string.Empty;
        var hr = batch.GetItemStringLength(index, out var length);
        if (hr < 0)
            return hr;
        if (length < 0)
            return InvalidArgument;
        var buffer = new char[length + 1];
        fixed (char* pointer = buffer)
            hr = batch.CopyItemString(index, pointer, buffer.Length);
        if (hr >= 0)
            value = new string(buffer, 0, length);
        return hr;
    }
}
