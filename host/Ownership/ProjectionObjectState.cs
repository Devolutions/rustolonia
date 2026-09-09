using System;
using System.Collections.Generic;
using System.Threading;
using Avalonia.Host.Com;
using Avalonia.Threading;

namespace Avalonia.Host.Ownership;

internal sealed class ProjectionObjectState
{
    private readonly object _gate = new();
    private AvaloniaObject? _target;
    private ProjectionLifetimeToken? _lifetimeToken;
    private List<Action>? _cleanup = [];
    private int _activeCalls;
    private bool _releasePending;

    public ProjectionObjectState(AvaloniaObject target, long objectId)
    {
        _target = target;
        ObjectId = objectId;
    }

    public long ObjectId { get; }

    public IDisposable EnterCall()
    {
        lock (_gate)
        {
            ObjectDisposedException.ThrowIf(_releasePending || _target is null, this);
            _activeCalls++;
            return new CallLease(this);
        }
    }

    public T GetTarget<T>()
        where T : AvaloniaObject
    {
        lock (_gate)
            return (T)(_target ?? throw new ObjectDisposedException(GetType().Name));
    }

    public long GetLifetimeToken()
    {
        lock (_gate)
        {
            ObjectDisposedException.ThrowIf(_releasePending || _target is null, this);
            _lifetimeToken ??= new ProjectionLifetimeToken(this);
            return (long)_lifetimeToken.GetNativePointer();
        }
    }

    public void RegisterCleanup(Action cleanup)
    {
        ArgumentNullException.ThrowIfNull(cleanup);
        lock (_gate)
        {
            ObjectDisposedException.ThrowIf(_releasePending, this);
            _cleanup!.Add(cleanup);
        }
    }

    public void ReleaseNativeOwnership()
    {
        ReleaseWork? work;
        lock (_gate)
        {
            if (_releasePending)
                return;
            _releasePending = true;
            work = CreateReleaseWorkIfReadyLocked();
        }
        RunRelease(work);
    }

    private void ReleaseCall()
    {
        ReleaseWork? work;
        lock (_gate)
        {
            _activeCalls--;
            work = CreateReleaseWorkIfReadyLocked();
        }
        RunRelease(work);
    }

    private ReleaseWork? CreateReleaseWorkIfReadyLocked()
    {
        if (!_releasePending || _activeCalls != 0 || _target is null)
            return null;

        var work = new ReleaseWork(_target, ObjectId, _cleanup!);
        _target = null;
        _lifetimeToken = null;
        _cleanup = null;
        return work;
    }

    private static void RunRelease(ReleaseWork? work)
    {
        if (work is null)
            return;

        ProjectionRuntime.Forget(work.Target, work.ObjectId);
        if (work.Cleanup.Count == 0)
            return;

        void Cleanup()
        {
            foreach (var action in work.Cleanup)
                action();
        }

        if (Dispatcher.UIThread.CheckAccess())
            Cleanup();
        else
            Dispatcher.UIThread.Post(Cleanup);
    }

    private sealed record ReleaseWork(
        AvaloniaObject Target,
        long ObjectId,
        IReadOnlyList<Action> Cleanup);

    private sealed class CallLease(ProjectionObjectState owner) : IDisposable
    {
        private ProjectionObjectState? _owner = owner;

        public void Dispose() =>
            Interlocked.Exchange(ref _owner, null)?.ReleaseCall();
    }
}
