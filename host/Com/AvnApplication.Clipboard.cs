using System;
using Avalonia.Controls;
using Avalonia.Host.Desktop;
using Avalonia.Input.Platform;
using Avalonia.Platform.Storage;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

/// <summary>
/// Stage 31 clipboard commands, routed through Avalonia's own
/// <see cref="IClipboard"/> and <see cref="Avalonia.Input.IDataTransfer"/>
/// abstractions. No platform clipboard API is touched here.
/// </summary>
/// <remarks>
/// Every operation is asynchronous and goes through the one shared async
/// operation registry, so a clipboard command is cancellable, completes exactly
/// once, and never blocks the UI thread waiting on the platform - which matters
/// because a clipboard read can block for as long as the owning application
/// takes to render the requested format.
/// </remarks>
public partial class AvnApplication : IAvnApplication4
{
    public int CreateClipboardData(out IAvnClipboardData? data)
    {
        data = null;
        try
        {
            data = new AvnClipboardData();
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int GetClipboardCapabilities(IAvnWindow? window, out int capabilities)
    {
        capabilities = 0;
        if (window is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var managedWindow = (Window?)ProjectionRuntime.Unwrap(window)
                ?? throw new ObjectDisposedException(nameof(window));
            var flags = AvnClipboardCapabilityFlags.None;
            var topLevel = TopLevel.GetTopLevel(managedWindow);
            if (topLevel?.Clipboard is not null)
                flags |= AvnClipboardCapabilityFlags.Available;
            if (topLevel?.StorageProvider is not null)
                flags |= AvnClipboardCapabilityFlags.Files;
            capabilities = (int)flags;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int StartClipboardWrite(
        IAvnWindow? window,
        IAvnClipboardData? data,
        IAvnAsyncCompletion? completion,
        out long operationId)
    {
        operationId = 0;
        if (window is null || data is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var clipboard = ResolveClipboard(window, out var managedWindow);
            var provider = TopLevel.GetTopLevel(managedWindow)?.StorageProvider;

            // The payload is snapshotted now: a consumer that keeps mutating
            // the builder after starting the write cannot change what lands on
            // the clipboard.
            var payload = ((AvnClipboardData)data).Snapshot();
            if (payload.Text is null && payload.FileUris.Count == 0)
                return HResults.E_INVALIDARG;

            return _asyncOperations.Start(
                completion,
                async cancellation =>
                {
                    await DesktopClipboard.WriteAsync(clipboard, provider, payload, cancellation);
                    return AvnAsyncValue.None;
                },
                out operationId);
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int StartClipboardClear(
        IAvnWindow? window,
        IAvnAsyncCompletion? completion,
        out long operationId)
    {
        operationId = 0;
        if (window is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var clipboard = ResolveClipboard(window, out _);
            return _asyncOperations.Start(
                completion,
                async cancellation =>
                {
                    await DesktopClipboard.ClearAsync(clipboard, cancellation);
                    return AvnAsyncValue.None;
                },
                out operationId);
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int StartClipboardReadFiles(
        IAvnWindow? window,
        IAvnStorageCompletion? completion,
        out long operationId)
    {
        operationId = 0;
        if (window is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var clipboard = ResolveClipboard(window, out _);
            return _asyncOperations.StartStorage(
                completion,
                cancellation => DesktopClipboard.ReadFilesAsync(clipboard, cancellation),
                out operationId);
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    private static IClipboard ResolveClipboard(IAvnWindow window, out Window managedWindow)
    {
        managedWindow = (Window?)ProjectionRuntime.Unwrap(window)
            ?? throw new ObjectDisposedException(nameof(window));
        return TopLevel.GetTopLevel(managedWindow)?.Clipboard
            ?? throw new NotSupportedException("The top-level has no clipboard.");
    }
}
