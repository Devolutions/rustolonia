using System;
using System.Runtime.InteropServices;
using Avalonia.Rust.Interop;

namespace Avalonia.Rust;

/// <summary>
/// Shared plumbing for the stage 30 capabilities queried from a Rust view
/// model: tracked async invocation with cancellation, and windowed range
/// sources.
///
/// An invocation handle identifies one specific async command run and is never
/// reused, so a cancellation raised after that run already finished is dropped
/// instead of aborting its successor. Both capabilities are optional: a host
/// attached to a producer that predates them still starts commands through
/// <see cref="IAvnRustViewModel.BeginAsync"/> and simply projects no windowed
/// data, rather than failing attach.
/// </summary>
public static class RustAsyncCommands
{
    private const int NoInterface = unchecked((int)0x80004002);

    /// <summary>
    /// Resolves the optional tracked-async capability. Returns null when the
    /// producer does not implement it.
    /// </summary>
    public static IAvnRustViewModel2? TryResolve(IAvnRustViewModel model) =>
        TryCast<IAvnRustViewModel2>(model);

    /// <summary>
    /// Resolves the optional range-source capability. Returns null when the
    /// producer publishes no windowed collection.
    /// </summary>
    public static IAvnRustRangeSource? TryResolveRangeSource(IAvnRustViewModel model) =>
        TryCast<IAvnRustRangeSource>(model);

    /// <summary>
    /// Starts an async command, preferring the tracked capability. Returns the
    /// invocation handle, or 0 when the producer only supports the untracked
    /// entry point.
    /// </summary>
    public static long Begin(
        IAvnRustViewModel model,
        IAvnRustViewModel2? tracked,
        int commandId,
        string? parameter)
    {
        ArgumentNullException.ThrowIfNull(model);
        if (tracked is null)
        {
            ThrowIfFailed(model.BeginAsync(commandId, parameter));
            return 0;
        }
        ThrowIfFailed(tracked.BeginAsyncTracked(commandId, parameter, out var operationId));
        return operationId;
    }

    /// <summary>
    /// Requests cancellation of one in-flight invocation. A zero handle (no
    /// invocation in flight, or an untracked producer) is a no-op rather than
    /// an error, so a bound Cancel button can never throw at the user.
    /// </summary>
    public static void Cancel(IAvnRustViewModel2? tracked, int commandId, long operationId)
    {
        if (tracked is null || operationId == 0)
            return;
        ThrowIfFailed(tracked.CancelAsync(commandId, operationId));
    }

    /// <summary>Clamps a published progress fraction into the 0..1 domain.</summary>
    public static double ClampProgress(double value) =>
        double.IsNaN(value) ? 0 : value < 0 ? 0 : value > 1 ? 1 : value;

    /// <summary>
    /// Casts to an optional capability. A <c>ComWrappers</c> RCW answers this
    /// with a real <c>QueryInterface</c>, so a producer that does not expose
    /// the IID reports "absent" rather than throwing at attach time.
    /// </summary>
    private static T? TryCast<T>(IAvnRustViewModel model) where T : class
    {
        ArgumentNullException.ThrowIfNull(model);
        try
        {
            return model as T;
        }
        catch (COMException error) when (error.HResult == NoInterface)
        {
            return null;
        }
        catch (InvalidCastException)
        {
            return null;
        }
    }

    private static void ThrowIfFailed(int hresult)
    {
        if (hresult < 0)
            Marshal.ThrowExceptionForHR(hresult);
    }
}
