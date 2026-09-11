using System;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Host.Desktop;

namespace Avalonia.Host.Com;

/// <summary>
/// Stage 32 modal dialog activation. <c>Window.ShowDialog</c> returns a
/// <see cref="Task{TResult}"/> and never completes synchronously, so it composes onto the
/// shared async operation registry the same way the storage pickers and clipboard do:
/// exactly one completion, cancellable, and the UI thread is never blocked waiting on the
/// dialog.
/// </summary>
public partial class AvnApplication : IAvnApplication5
{
    public int StartShowDialog(
        IAvnWindow? owner,
        IAvnWindow? dialog,
        IAvnAsyncCompletion? completion,
        out long operationId)
    {
        operationId = 0;
        if (owner is null || dialog is null)
            return HResults.E_POINTER;
        try
        {
            Avalonia.Threading.Dispatcher.UIThread.VerifyAccess();
            var ownerWindow = (Window?)ProjectionRuntime.Unwrap(owner)
                ?? throw new ObjectDisposedException(nameof(owner));
            var dialogWindow = (Window?)ProjectionRuntime.Unwrap(dialog)
                ?? throw new ObjectDisposedException(nameof(dialog));

            return _asyncOperations.Start(
                completion,
                async cancellation =>
                {
                    var result = await global::Avalonia.Host.Desktop.Dialogs
                        .ShowAsync<object?>(dialogWindow, ownerWindow, cancellation)
                        .ConfigureAwait(true);
                    return AvnAsyncValue.FromString(global::Avalonia.Host.Desktop.Dialogs.ResultToString(result));
                },
                out operationId);
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }
}
