using System;
using System.Collections.Generic;

namespace Avalonia.Rust;

public sealed class RustVmInboundWriteTracker
{
    private readonly object _gate = new();
    private readonly Dictionary<int, Stack<RustVmInboundWriteFrame>> _framesByThread = [];
    private readonly Dictionary<int, long> _versions = [];

    public RustVmInboundWriteFrame Begin(int propertyId)
    {
        var frame = new RustVmInboundWriteFrame(
            propertyId,
            Environment.CurrentManagedThreadId);
        lock (_gate)
        {
            if (!_framesByThread.TryGetValue(frame.ThreadId, out var frames))
                _framesByThread.Add(frame.ThreadId, frames = new Stack<RustVmInboundWriteFrame>());
            frames.Push(frame);
        }
        return frame;
    }

    public RustVmInboundWriteFrame? MarkPublication(int propertyId)
    {
        lock (_gate)
        {
            if (_framesByThread.TryGetValue(Environment.CurrentManagedThreadId, out var frames))
            {
                foreach (var frame in frames)
                {
                    if (frame.PropertyId == propertyId)
                    {
                        frame.Published = true;
                        return frame;
                    }
                }
            }
        }
        return null;
    }

    public void CommitPublication(int propertyId, RustVmInboundWriteFrame? frame)
    {
        lock (_gate)
        {
            var version = NextVersion(propertyId);
            if (frame is not null)
                frame.LastPublicationVersion = version;
        }
    }

    public void CommitLocal(int propertyId)
    {
        lock (_gate)
            NextVersion(propertyId);
    }

    public bool WasPublished(RustVmInboundWriteFrame frame)
    {
        lock (_gate)
            return frame.Published;
    }

    public bool ShouldRollback(RustVmInboundWriteFrame frame)
    {
        lock (_gate)
        {
            return frame.Published &&
                frame.LastPublicationVersion is { } publication &&
                _versions.GetValueOrDefault(frame.PropertyId) == publication;
        }
    }

    public void End(RustVmInboundWriteFrame frame)
    {
        lock (_gate)
        {
            var frames = _framesByThread[frame.ThreadId];
            if (!ReferenceEquals(frames.Pop(), frame))
                throw new InvalidOperationException("Inbound write frames ended out of order.");
            if (frames.Count == 0)
                _framesByThread.Remove(frame.ThreadId);
        }
    }

    private long NextVersion(int propertyId)
    {
        var version = _versions.GetValueOrDefault(propertyId) + 1;
        _versions[propertyId] = version;
        return version;
    }
}

public sealed class RustVmInboundWriteFrame
{
    internal RustVmInboundWriteFrame(int propertyId, int threadId)
    {
        PropertyId = propertyId;
        ThreadId = threadId;
    }

    internal int PropertyId { get; }
    internal int ThreadId { get; }
    internal bool Published { get; set; }
    internal long? LastPublicationVersion { get; set; }
}
