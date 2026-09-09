using System.Runtime.InteropServices;
using System.Threading;
using Avalonia.Host.Com;

namespace Avalonia.Host;

public readonly record struct ProjectionDiagnosticSnapshot(
    long WrappersCreated,
    int TrackedObjectIds,
    int LiveManagedObjects,
    long ActiveSubscriptions,
    long NativeOwnershipReleases);

[StructLayout(LayoutKind.Sequential)]
public readonly record struct ProjectionDiagnosticNativeSnapshot(
    long WrappersCreated,
    int TrackedObjectIds,
    int LiveManagedObjects,
    long ActiveSubscriptions,
    long NativeOwnershipReleases);

public static class ProjectionDiagnostics
{
    private static long s_wrappersCreated;
    private static long s_activeSubscriptions;
    private static long s_nativeOwnershipReleases;

    internal static void WrapperCreated() =>
        Interlocked.Increment(ref s_wrappersCreated);

    internal static void SubscriptionAdded() =>
        Interlocked.Increment(ref s_activeSubscriptions);

    internal static void SubscriptionRemoved() =>
        Interlocked.Decrement(ref s_activeSubscriptions);

    internal static void NativeOwnershipReleased() =>
        Interlocked.Increment(ref s_nativeOwnershipReleases);

    public static ProjectionDiagnosticSnapshot Capture() => new(
        Interlocked.Read(ref s_wrappersCreated),
        ProjectionRuntime.TrackedObjectIdCount,
        ProjectionRuntime.LiveManagedObjectCount,
        Interlocked.Read(ref s_activeSubscriptions),
        Interlocked.Read(ref s_nativeOwnershipReleases));

    internal static ProjectionDiagnosticNativeSnapshot CaptureNative()
    {
        var snapshot = Capture();
        return new ProjectionDiagnosticNativeSnapshot(
            snapshot.WrappersCreated,
            snapshot.TrackedObjectIds,
            snapshot.LiveManagedObjects,
            snapshot.ActiveSubscriptions,
            snapshot.NativeOwnershipReleases);
    }
}
