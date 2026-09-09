using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Threading;
using Avalonia.Controls;
using Avalonia.Host.Desktop;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

/// <summary>
/// Attaches Avalonia's drag-and-drop routed events to a projected control and
/// forwards file/folder payloads to a Rust <see cref="IAvnFileDropHandler"/>.
/// </summary>
/// <remarks>
/// The managed handlers answer the platform synchronously using the accepted
/// effect mask captured at subscription time (see <see cref="DesktopFileDrop"/>),
/// then post the notification through <see cref="Dispatcher.UIThread"/>. Posting
/// keeps the drag loop responsive no matter what the consumer does, and the
/// dispatcher's FIFO ordering at one priority preserves enter/over/leave/drop
/// order. Payload items are snapshotted before posting because the platform
/// <c>IDataTransfer</c> is only valid for the duration of the managed event.
/// </remarks>
internal sealed class AvnFileDropRegistry
{
    private readonly Dictionary<long, Subscription> _subscriptions = new();
    private readonly Dictionary<Control, DropState> _targets = new();
    private long _nextSubscriptionId;

    public int Subscribe(
        Control target,
        DragDropEffects acceptedEffects,
        IAvnFileDropHandler handler,
        out long subscriptionId)
    {
        subscriptionId = 0;
        Dispatcher.UIThread.VerifyAccess();

        var id = Interlocked.Increment(ref _nextSubscriptionId);
        if (!_targets.TryGetValue(target, out var state))
        {
            state = new DropState(DragDrop.GetAllowDrop(target));
            _targets.Add(target, state);
            DragDrop.SetAllowDrop(target, true);
        }
        state.Count++;
        var subscription = new Subscription(id, target, acceptedEffects, handler);
        _subscriptions.Add(id, subscription);
        subscription.Attach();
        subscriptionId = id;
        return HResults.S_OK;
    }

    public int Unsubscribe(long subscriptionId)
    {
        Dispatcher.UIThread.VerifyAccess();
        if (!_subscriptions.Remove(subscriptionId, out var subscription))
            return HResults.E_INVALIDARG;
        subscription.Detach();
        ReleaseTarget(subscription.Target);
        return HResults.S_OK;
    }

    public void Clear()
    {
        Dispatcher.UIThread.VerifyAccess();
        foreach (var subscription in _subscriptions.Values)
        {
            subscription.Detach();
            ReleaseTarget(subscription.Target);
        }
        _subscriptions.Clear();
    }

    private void ReleaseTarget(Control target)
    {
        var state = _targets[target];
        if (--state.Count == 0)
        {
            _targets.Remove(target);
            DragDrop.SetAllowDrop(target, state.PreviousAllowDrop);
        }
    }

    private sealed class DropState(bool previousAllowDrop)
    {
        public readonly bool PreviousAllowDrop = previousAllowDrop;
        public int Count;
    }

    private sealed class Subscription
    {
        private readonly long _id;
        private readonly Control _target;
        private readonly DragDropEffects _acceptedEffects;
        private readonly IAvnFileDropHandler _handler;
        private readonly EventHandler<DragEventArgs> _onEnter;
        private readonly EventHandler<DragEventArgs> _onOver;
        private readonly EventHandler<DragEventArgs> _onLeave;
        private readonly EventHandler<DragEventArgs> _onDrop;
        public Control Target => _target;
        private IDataTransfer? _payloadSource;
        private IReadOnlyList<StorageItemSnapshot> _payload = Array.Empty<StorageItemSnapshot>();
        private bool _attached;

        public Subscription(
            long id,
            Control target,
            DragDropEffects acceptedEffects,
            IAvnFileDropHandler handler)
        {
            _id = id;
            _target = target;
            _acceptedEffects = acceptedEffects;
            _handler = handler;
            _onEnter = (_, e) => Handle(DesktopDropEventKind.Enter, e);
            _onOver = (_, e) => Handle(DesktopDropEventKind.Over, e);
            _onLeave = (_, e) => Handle(DesktopDropEventKind.Leave, e);
            _onDrop = (_, e) => Handle(DesktopDropEventKind.Drop, e);
        }

        public void Attach()
        {
            _target.AddHandler(DragDrop.DragEnterEvent, _onEnter);
            _target.AddHandler(DragDrop.DragOverEvent, _onOver);
            _target.AddHandler(DragDrop.DragLeaveEvent, _onLeave);
            _target.AddHandler(DragDrop.DropEvent, _onDrop);
            _attached = true;
        }

        public void Detach()
        {
            if (!_attached)
                return;
            _attached = false;
            ForgetPayload();
            _target.RemoveHandler(DragDrop.DragEnterEvent, _onEnter);
            _target.RemoveHandler(DragDrop.DragOverEvent, _onOver);
            _target.RemoveHandler(DragDrop.DragLeaveEvent, _onLeave);
            _target.RemoveHandler(DragDrop.DropEvent, _onDrop);
        }

        private void Handle(DesktopDropEventKind kind, DragEventArgs e)
        {
            if (!_attached)
                return;

            var notification = DesktopFileDrop.PrepareFrom(
                kind,
                e.DragEffects,
                _acceptedEffects,
                CapturePayload(kind, e.DataTransfer));

            if (kind != DesktopDropEventKind.Leave)
            {
                e.DragEffects = notification.EffectiveEffects;
                if (notification.EffectiveEffects != DragDropEffects.None)
                    e.Handled = true;
            }

            var handler = _handler;
            var id = _id;
            Dispatcher.UIThread.Post(
                () =>
                {
                    // Unsubscribe (and shutdown, through Clear) must stop
                    // delivery, including for notifications already queued when
                    // the subscription went away.
                    if (!_attached)
                        return;
                    try
                    {
                        var hr = handler.OnDragEvent(
                            id,
                            (int)notification.Kind,
                            (int)notification.AllowedEffects,
                            (int)notification.EffectiveEffects,
                            new AvnStorageItemList(notification.Items));
                        if (hr < 0)
                            Marshal.ThrowExceptionForHR(hr);
                    }
                    catch (Exception error)
                    {
                        // A failing consumer must not tear down the UI thread;
                        // the drag itself already completed on the platform side.
                        _ = AbiError.Capture(error);
                    }
                },
                DispatcherPriority.Input);
        }

        /// <summary>
        /// Captures the payload of one drag, once.
        /// </summary>
        /// <remarks>
        /// <c>DragOver</c> fires continuously while the pointer moves and the
        /// platform reuses the same <see cref="IDataTransfer"/> for the whole
        /// drag, so the snapshot list is materialized on the first notification
        /// and reused for the rest. Doing it per event would allocate a fresh
        /// snapshot (and re-probe a local path) for every file, many times a
        /// second, inside the platform drag loop.
        /// </remarks>
        private IReadOnlyList<StorageItemSnapshot> CapturePayload(
            DesktopDropEventKind kind,
            IDataTransfer? dataTransfer)
        {
            if (kind == DesktopDropEventKind.Leave || dataTransfer is null)
            {
                ForgetPayload();
                return Array.Empty<StorageItemSnapshot>();
            }

            if (!ReferenceEquals(_payloadSource, dataTransfer))
            {
                _payloadSource = dataTransfer;
                _payload = DesktopFileDrop.CaptureItems(dataTransfer);
            }

            var payload = _payload;
            if (kind == DesktopDropEventKind.Drop)
                ForgetPayload();
            return payload;
        }

        private void ForgetPayload()
        {
            _payloadSource = null;
            _payload = Array.Empty<StorageItemSnapshot>();
        }
    }
}
