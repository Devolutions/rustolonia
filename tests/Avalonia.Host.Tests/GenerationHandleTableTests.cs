using System;
using System.Threading;
using Avalonia.Host.Ownership;
using Xunit;

namespace Avalonia.Host.Tests;

public class GenerationHandleTableTests
{
    [Fact]
    public void Final_release_retires_without_gc_and_reuses_slot_with_new_generation()
    {
        var table = new GenerationHandleTable();
        var firstTarget = new AvaloniaObject();
        var first = table.Project(firstTarget);
        var releases = ProjectionDiagnostics.Capture().NativeOwnershipReleases;

        Assert.True(table.TryRelease(first));
        Assert.Equal(
            releases + 1,
            ProjectionDiagnostics.Capture().NativeOwnershipReleases);
        Assert.False(table.TryLease<AvaloniaObject>(first, out _));

        var second = table.Project(firstTarget);
        Assert.Equal(first.Slot, second.Slot);
        Assert.NotEqual(first.Generation, second.Generation);
        Assert.False(table.TryRetain(first));
        Assert.True(table.TryRelease(second));
    }

    [Fact]
    public void Projection_is_canonical_while_native_references_are_alive()
    {
        var table = new GenerationHandleTable();
        var target = new AvaloniaObject();
        var first = table.Project(target);
        var second = table.Project(target);

        Assert.Equal(first, second);
        Assert.True(table.TryRelease(first));
        Assert.True(table.TryLease<AvaloniaObject>(second, out var lease));
        lease!.Dispose();
        Assert.True(table.TryRelease(second));
        Assert.False(table.TryRelease(second));
    }

    [Fact]
    public void Active_lease_defers_retirement_but_handle_is_immediately_stale()
    {
        var table = new GenerationHandleTable();
        var handle = table.Project(new AvaloniaObject());
        Assert.True(table.TryLease<AvaloniaObject>(handle, out var lease));
        var releases = ProjectionDiagnostics.Capture().NativeOwnershipReleases;

        Assert.True(table.TryRelease(handle));
        Assert.False(table.TryRetain(handle));
        Assert.False(table.TryLease<AvaloniaObject>(handle, out _));
        Assert.Equal(
            releases,
            ProjectionDiagnostics.Capture().NativeOwnershipReleases);
        Assert.NotNull(lease!.Target);

        lease.Dispose();
        Assert.Equal(
            releases + 1,
            ProjectionDiagnostics.Capture().NativeOwnershipReleases);
    }

    [Fact]
    public void Final_release_and_cleanup_are_safe_on_worker_thread()
    {
        Action? scheduledCleanup = null;
        var table = new GenerationHandleTable(cleanup => scheduledCleanup = cleanup);
        var handle = table.Project(new AvaloniaObject());
        var cleanupThread = 0;
        Assert.True(table.TryRegisterCleanup(
            handle,
            () => cleanupThread = Environment.CurrentManagedThreadId));

        var workerThread = 0;
        var released = false;
        var thread = new Thread(() =>
        {
            workerThread = Environment.CurrentManagedThreadId;
            released = table.TryRelease(handle);
        });
        thread.Start();
        thread.Join();

        Assert.True(released);
        Assert.Equal(0, cleanupThread);
        Assert.False(table.TryLease<AvaloniaObject>(handle, out _));
        Assert.NotNull(scheduledCleanup);

        scheduledCleanup();
        Assert.Equal(Environment.CurrentManagedThreadId, cleanupThread);
        Assert.NotEqual(workerThread, cleanupThread);
    }

    [Fact]
    public void Release_during_active_call_runs_cleanup_exactly_once_after_call()
    {
        var table = new GenerationHandleTable();
        var handle = table.Project(new AvaloniaObject());
        var cleanupCount = 0;
        Assert.True(table.TryRegisterCleanup(handle, () => cleanupCount++));
        Assert.True(table.TryLease<AvaloniaObject>(handle, out var lease));

        Assert.True(table.TryRelease(handle));
        Assert.Equal(0, cleanupCount);
        lease!.Dispose();
        lease.Dispose();

        Assert.Equal(1, cleanupCount);
        Assert.False(table.TryRelease(handle));
        Assert.Equal((0, 1), table.Capture());
    }
}
