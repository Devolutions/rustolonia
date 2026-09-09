using System;
using System.Threading;
using Avalonia.Rust;
using Xunit;

namespace Avalonia.Host.Tests;

public class RustVmInboundWriteTrackerTests
{
    [Fact]
    public void Reentrant_write_has_an_independent_publication_frame()
    {
        var tracker = new RustVmInboundWriteTracker();
        var outer = tracker.Begin(1);
        var outerPublication = tracker.MarkPublication(1);
        tracker.CommitPublication(1, outerPublication);

        var inner = tracker.Begin(1);

        Assert.False(tracker.WasPublished(inner));
        tracker.CommitLocal(1);
        tracker.End(inner);
        Assert.True(tracker.WasPublished(outer));
        Assert.False(tracker.ShouldRollback(outer));
        tracker.End(outer);
    }

    [Fact]
    public void Publication_from_another_thread_does_not_mark_the_ui_write()
    {
        var tracker = new RustVmInboundWriteTracker();
        var write = tracker.Begin(1);
        RustVmInboundWriteFrame? publication = null;
        var callerThread = Environment.CurrentManagedThreadId;
        var thread = new Thread(() =>
        {
            Assert.NotEqual(callerThread, Environment.CurrentManagedThreadId);
            publication = tracker.MarkPublication(1);
        });
        thread.Start();
        thread.Join();
        tracker.CommitPublication(1, publication);

        Assert.Null(publication);
        Assert.False(tracker.WasPublished(write));
        tracker.End(write);
    }

    [Fact]
    public void Failed_write_rolls_back_only_its_latest_publication()
    {
        var tracker = new RustVmInboundWriteTracker();
        var write = tracker.Begin(1);
        var publication = tracker.MarkPublication(1);
        tracker.CommitPublication(1, publication);

        Assert.True(tracker.ShouldRollback(write));

        tracker.CommitLocal(1);
        Assert.False(tracker.ShouldRollback(write));
        tracker.End(write);
    }
}
