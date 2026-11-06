using System;
using System.Runtime.InteropServices;
using Avalonia.Rust.Interop;

namespace Avalonia.Rust;

/// <summary>
/// Shared plumbing for tracked async command invocation with cancellation, and
/// for windowed range sources queried from a Rust view model.
///
/// An invocation handle identifies one specific async command run and is never
/// reused, so a cancellation raised after that run already finished is dropped
/// instead of aborting its successor.
/// </summary>
public static class RustAsyncCommands
{
    private const int NoInterface = unchecked((int)0x80004002);

    /// <summary>
    /// Resolves the optional range-source capability. Returns null when the
    /// producer publishes no windowed collection.
    /// </summary>
    public static IAvnRustRangeSource? TryResolveRangeSource(IAvnRustViewModel model) =>
        TryCast<IAvnRustRangeSource>(model);

    /// <summary>Starts an async command and returns its invocation handle.</summary>
    public static long Begin(IAvnRustViewModel model, int commandId, string? parameter)
    {
        ArgumentNullException.ThrowIfNull(model);
        ThrowIfFailed(model.BeginAsyncTracked(commandId, parameter, out var operationId));
        return operationId;
    }

    /// <summary>
    /// Requests cancellation of one in-flight invocation. A zero handle (no
    /// invocation in flight) is a no-op rather than an error, so a bound Cancel
    /// button can never throw at the user.
    /// </summary>
    public static void Cancel(IAvnRustViewModel model, int commandId, long operationId)
    {
        ArgumentNullException.ThrowIfNull(model);
        if (operationId == 0)
            return;
        ThrowIfFailed(model.CancelAsync(commandId, operationId));
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
