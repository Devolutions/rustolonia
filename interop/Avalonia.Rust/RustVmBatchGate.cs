using System;
using System.Threading;

namespace Avalonia.Rust;

/// <summary>Result of trying to start a batch on a <see cref="RustVmBatchGate"/>.</summary>
public enum RustVmBatchEntry
{
    /// <summary>The batch owns the target until <see cref="RustVmBatchGate.ExitBatch"/>.</summary>
    Entered,

    /// <summary>Disposal has been requested; the batch must be cancelled.</summary>
    Disposed,

    /// <summary>A batch is already running on this target; nesting is not permitted.</summary>
    Reentrant,
}

/// <summary>
/// The explicit, non-reentrant batch lifecycle gate shared by the IR-generated
/// adapters and the reflectable adapter.
///
/// It exists because notifications published at the end of a batch can run
/// arbitrary observer code, and that code may dispose the adapter. A plain
/// (reentrant) monitor would let such a nested <c>Dispose</c> detach the model
/// and tear down nested adapters while the batch is still publishing. This gate
/// instead marks disposal pending and runs the cleanup exactly once, after the
/// batch's commit and notifications have finished. Concurrent disposal from
/// other threads serializes on the same cleanup and never runs it twice.
/// </summary>
public sealed class RustVmBatchGate
{
    private readonly object _sync = new();
    private readonly object _cleanupSync = new();
    private int _closed;
    private bool _batchActive;
    private bool _cleanupClaimed;
    private Action? _deferredCleanup;

    /// <summary>
    /// True once disposal has been requested, even when the cleanup itself is
    /// still deferred behind an active batch. Sink entry points use this so a
    /// disposing adapter stops accepting work immediately.
    /// </summary>
    public bool IsClosed => Volatile.Read(ref _closed) != 0;

    /// <summary>Attempts to take exclusive batch ownership of the target.</summary>
    public RustVmBatchEntry TryEnterBatch()
    {
        lock (_sync)
        {
            if (_closed != 0)
                return RustVmBatchEntry.Disposed;
            if (_batchActive)
                return RustVmBatchEntry.Reentrant;
            _batchActive = true;
            return RustVmBatchEntry.Entered;
        }
    }

    /// <summary>
    /// Releases batch ownership and runs a disposal that arrived while the
    /// batch was committing or publishing notifications.
    /// </summary>
    public void ExitBatch()
    {
        Action? deferred;
        lock (_sync)
        {
            _batchActive = false;
            deferred = _deferredCleanup;
            _deferredCleanup = null;
        }
        if (deferred is null)
            return;
        lock (_cleanupSync)
            deferred();
    }

    /// <summary>
    /// Requests disposal. <paramref name="cleanup"/> runs at most once: now when
    /// no batch is active, otherwise from <see cref="ExitBatch"/>.
    /// </summary>
    public void Dispose(Action cleanup)
    {
        ArgumentNullException.ThrowIfNull(cleanup);
        var run = false;
        lock (_sync)
        {
            Volatile.Write(ref _closed, 1);
            if (!_cleanupClaimed)
            {
                _cleanupClaimed = true;
                if (_batchActive)
                    _deferredCleanup = cleanup;
                else
                    run = true;
            }
        }
        // Serializes concurrent disposers against the thread actually running
        // cleanup, so no caller observes a half-detached adapter.
        lock (_cleanupSync)
        {
            if (run)
                cleanup();
        }
    }
}
