using System;
using System.Threading;
using MicroCom.Runtime;

namespace Avalonia.Host.Ownership;

internal abstract class HostNativeOwned :
    IUnknown,
    IMicroComShadowContainer,
    IMicroComExceptionCallback
{
    private readonly object _gate = new();
    private MicroComShadow? _shadow;
    private readonly Action<Action> _scheduleCleanup;
    private int _activeCalls;
    private bool _nativeOwnershipReleased;
    private bool _cleanupScheduled;

    protected HostNativeOwned(Action<Action>? scheduleCleanup = null)
    {
        _scheduleCleanup = scheduleCleanup ?? (static cleanup => cleanup());
    }

    MicroComShadow? IMicroComShadowContainer.Shadow
    {
        get => _shadow;
        set => _shadow = value;
    }

    void IMicroComShadowContainer.OnReferencedFromNative()
    {
    }

    void IMicroComShadowContainer.OnUnreferencedFromNative()
    {
        var scheduleCleanup = false;
        lock (_gate)
        {
            if (_nativeOwnershipReleased)
                return;
            _nativeOwnershipReleased = true;
            ProjectionDiagnostics.NativeOwnershipReleased();
            scheduleCleanup = TryScheduleCleanupLocked();
        }
        if (scheduleCleanup)
            _scheduleCleanup(FinishCleanup);
    }

    protected IDisposable EnterCall()
    {
        lock (_gate)
        {
            ObjectDisposedException.ThrowIf(_nativeOwnershipReleased, this);
            _activeCalls++;
            return new CallLease(this);
        }
    }

    protected abstract void Destroyed();

    void IDisposable.Dispose()
    {
    }

    void IMicroComExceptionCallback.RaiseException(Exception e) =>
        _ = AbiError.Capture(e);

    private void ReleaseCall()
    {
        var scheduleCleanup = false;
        lock (_gate)
        {
            _activeCalls--;
            scheduleCleanup = TryScheduleCleanupLocked();
        }
        if (scheduleCleanup)
            _scheduleCleanup(FinishCleanup);
    }

    private bool TryScheduleCleanupLocked()
    {
        if (!_nativeOwnershipReleased || _activeCalls != 0 || _cleanupScheduled)
            return false;
        _cleanupScheduled = true;
        return true;
    }

    private void FinishCleanup()
    {
        _shadow?.Dispose();
        _shadow = null;
        Destroyed();
    }

    private sealed class CallLease(HostNativeOwned owner) : IDisposable
    {
        private HostNativeOwned? _owner = owner;

        public void Dispose() =>
            Interlocked.Exchange(ref _owner, null)?.ReleaseCall();
    }
}
