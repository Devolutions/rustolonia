using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// The separately versioned dialog capability, queried from <c>IAvnApplication</c>.
/// Nothing here is ever added to an already published vtable; a consumer that predates
/// the capability reads <c>E_NOINTERFACE</c> from the query instead.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnApplication5)]
public partial interface IAvnApplication5
{
    /// <summary>
    /// Shows <paramref name="dialog"/> modally over <paramref name="owner"/> and completes
    /// through the shared async operation registry: exactly one completion, cancellable
    /// through <c>CancelAsyncOperation</c>, never blocking the UI thread. The result string
    /// is the dialog result converted by the host (null when the dialog returned no result).
    /// </summary>
    [PreserveSig]
    int StartShowDialog(
        IAvnWindow? owner,
        IAvnWindow? dialog,
        IAvnAsyncCompletion? completion,
        out long operationId);
}
