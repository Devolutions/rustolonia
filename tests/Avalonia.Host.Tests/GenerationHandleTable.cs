using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Threading;

namespace Avalonia.Host.Ownership;

internal readonly record struct OwnershipHandle(ulong Value)
{
    public static OwnershipHandle Invalid => default;

    public bool IsValid => Value != 0;

    internal int Slot => unchecked((int)((uint)Value - 1));

    internal uint Generation => (uint)(Value >> 32);

    internal static OwnershipHandle Create(int slot, uint generation) =>
        new(((ulong)generation << 32) | checked((uint)slot + 1));
}

internal sealed class GenerationHandleTable
{
    private enum EntryState
    {
        Alive,
        ReleasePending,
        Retired,
    }

    private sealed class Identity
    {
        public OwnershipHandle Current;
    }

    private sealed class Entry(AvaloniaObject target, Identity identity, uint generation)
    {
        public AvaloniaObject? Target = target;
        public readonly Identity Identity = identity;
        public readonly List<Action> Cleanup = [];
        public uint Generation = generation;
        public int NativeReferences = 1;
        public int ActiveCalls;
        public EntryState State = EntryState.Alive;
    }

    private sealed class Slot
    {
        public uint Generation;
        public Entry? Entry;
    }

    private readonly object _gate = new();
    private readonly List<Slot> _slots = [];
    private readonly Stack<int> _freeSlots = [];
    private readonly ConditionalWeakTable<AvaloniaObject, Identity> _identities = new();
    private readonly Action<Action> _scheduleCleanup;

    public GenerationHandleTable(Action<Action>? scheduleCleanup = null)
    {
        _scheduleCleanup = scheduleCleanup ?? (static cleanup => cleanup());
    }

    public OwnershipHandle Project(AvaloniaObject target)
    {
        ArgumentNullException.ThrowIfNull(target);
        lock (_gate)
        {
            var identity = _identities.GetValue(target, static _ => new Identity());
            if (identity.Current.IsValid && TryRetainLocked(identity.Current))
                return identity.Current;

            var slotIndex = _freeSlots.Count > 0 ? _freeSlots.Pop() : _slots.Count;
            Slot slot;
            if (slotIndex == _slots.Count)
            {
                slot = new Slot { Generation = 1 };
                _slots.Add(slot);
            }
            else
            {
                slot = _slots[slotIndex];
                slot.Generation = NextGeneration(slot.Generation);
            }

            var handle = OwnershipHandle.Create(slotIndex, slot.Generation);
            slot.Entry = new Entry(target, identity, slot.Generation);
            identity.Current = handle;
            return handle;
        }
    }

    public bool TryRetain(OwnershipHandle handle)
    {
        lock (_gate)
            return TryRetainLocked(handle);
    }

    public bool TryRelease(OwnershipHandle handle)
    {
        List<Action>? cleanup = null;
        lock (_gate)
        {
            if (!TryGetAliveEntryLocked(handle, out var entry))
                return false;

            entry.NativeReferences--;
            if (entry.NativeReferences > 0)
                return true;

            entry.State = EntryState.ReleasePending;
            if (entry.Identity.Current == handle)
                entry.Identity.Current = OwnershipHandle.Invalid;
            cleanup = RetireIfReadyLocked(handle.Slot, entry);
        }

        RunCleanup(cleanup);
        return true;
    }

    public bool TryRegisterCleanup(OwnershipHandle handle, Action cleanup)
    {
        ArgumentNullException.ThrowIfNull(cleanup);
        lock (_gate)
        {
            if (!TryGetAliveEntryLocked(handle, out var entry))
                return false;
            entry.Cleanup.Add(cleanup);
            return true;
        }
    }

    public bool TryLease<T>(OwnershipHandle handle, out HandleLease<T>? lease)
        where T : AvaloniaObject
    {
        lock (_gate)
        {
            if (!TryGetAliveEntryLocked(handle, out var entry) || entry.Target is not T target)
            {
                lease = null;
                return false;
            }

            entry.ActiveCalls++;
            lease = new HandleLease<T>(this, handle, target);
            return true;
        }
    }

    internal (int LiveEntries, int FreeSlots) Capture()
    {
        lock (_gate)
            return (_slots.Count(slot => slot.Entry is not null), _freeSlots.Count);
    }

    private bool TryRetainLocked(OwnershipHandle handle)
    {
        if (!TryGetAliveEntryLocked(handle, out var entry))
            return false;
        entry.NativeReferences++;
        return true;
    }

    private bool TryGetAliveEntryLocked(OwnershipHandle handle, out Entry entry)
    {
        entry = null!;
        if (!handle.IsValid || handle.Slot < 0 || handle.Slot >= _slots.Count)
            return false;
        var candidate = _slots[handle.Slot].Entry;
        if (candidate is null ||
            candidate.Generation != handle.Generation ||
            candidate.State != EntryState.Alive)
        {
            return false;
        }
        entry = candidate;
        return true;
    }

    private void ReleaseLease(OwnershipHandle handle)
    {
        List<Action>? cleanup = null;
        lock (_gate)
        {
            if (!handle.IsValid || handle.Slot < 0 || handle.Slot >= _slots.Count)
                return;
            var entry = _slots[handle.Slot].Entry;
            if (entry is null || entry.Generation != handle.Generation)
                return;

            entry.ActiveCalls--;
            cleanup = RetireIfReadyLocked(handle.Slot, entry);
        }
        RunCleanup(cleanup);
    }

    private List<Action>? RetireIfReadyLocked(int slotIndex, Entry entry)
    {
        if (entry.State != EntryState.ReleasePending || entry.ActiveCalls != 0)
            return null;

        entry.State = EntryState.Retired;
        entry.Target = null;
        var cleanup = entry.Cleanup.Count == 0 ? null : new List<Action>(entry.Cleanup);
        entry.Cleanup.Clear();
        _slots[slotIndex].Entry = null;
        _freeSlots.Push(slotIndex);
        ProjectionDiagnostics.NativeOwnershipReleased();
        return cleanup;
    }

    private void RunCleanup(List<Action>? cleanup)
    {
        if (cleanup is null)
            return;
        _scheduleCleanup(() =>
        {
            foreach (var action in cleanup)
                action();
        });
    }

    private static uint NextGeneration(uint generation) =>
        generation == uint.MaxValue ? 1 : generation + 1;

    internal sealed class HandleLease<T> : IDisposable
        where T : AvaloniaObject
    {
        private GenerationHandleTable? _owner;
        private readonly OwnershipHandle _handle;

        internal HandleLease(
            GenerationHandleTable owner,
            OwnershipHandle handle,
            T target)
        {
            _owner = owner;
            _handle = handle;
            Target = target;
        }

        public T Target { get; }

        public void Dispose() =>
            Interlocked.Exchange(ref _owner, null)?.ReleaseLease(_handle);
    }
}
