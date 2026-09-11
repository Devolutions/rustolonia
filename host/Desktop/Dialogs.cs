using System;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Threading;

namespace Avalonia.Host.Desktop;

/// <summary>
/// Runs <see cref="Window.ShowDialog{TResult}(Window)"/> from an ABI request.
/// </summary>
/// <remarks>
/// A modal dialog owns its owner while it is open: the dialog may outlive the originating
/// call, so the caller's window tokens are resolved to managed windows up front and the
/// task is awaited without blocking the UI thread. A cancellation request closes the
/// dialog rather than abandoning the task, because an abandoned <c>ShowDialog</c> task
/// still holds the owner.
/// </remarks>
internal static class Dialogs
{
    public static async Task<TResult?> ShowAsync<TResult>(
        Window dialog,
        Window owner,
        CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(dialog);
        ArgumentNullException.ThrowIfNull(owner);
        cancellationToken.ThrowIfCancellationRequested();

        var task = dialog.ShowDialog<TResult>(owner);
        if (!cancellationToken.CanBeCanceled)
            return await task.ConfigureAwait(true);

        using var registration = cancellationToken.Register(() =>
        {
            if (Dispatcher.UIThread.CheckAccess())
                TryClose(dialog);
            else
                Dispatcher.UIThread.Post(() => TryClose(dialog));
        });
        return await task.ConfigureAwait(true);
    }

    /// <summary>
    /// Converts a dialog result to its ABI string form: <c>null</c> stays null (the dialog
    /// returned no result), a string crosses as-is, and anything else uses its invariant
    /// <see cref="object.ToString"/> form.
    /// </summary>
    public static string? ResultToString(object? result) => result switch
    {
        null => null,
        string text => text,
        _ => Convert.ToString(result, System.Globalization.CultureInfo.InvariantCulture),
    };

    private static void TryClose(Window dialog)
    {
        try
        {
            dialog.Close();
        }
        catch
        {
            // The dialog may already be closing or detached from its owner; a cancellation
            // that races the natural close is satisfied either way.
        }
    }
}
