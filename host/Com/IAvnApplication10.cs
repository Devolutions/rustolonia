using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication10)]
public partial interface IAvnApplication10
{
    /// <summary>macOS UI thread; close popups before reconfiguration. Null removes the gate.</summary>
    [PreserveSig]
    int SetNativePopupGate(IAvnWindow? window, IAvnNativePopupHandler? handler);
    /// <summary>UI thread; stale/duplicate tokens return E_INVALIDARG.</summary>
    [PreserveSig]
    int AcknowledgeNativePopup(long token);
}

[GeneratedComInterface]
[Guid(AvnGuids.IAvnNativePopupHandler)]
public partial interface IAvnNativePopupHandler
{
    /// <summary>Popup exists but has never been shown. Token/window number are values, not pointers.</summary>
    [PreserveSig]
    int OnPreparing(long token, int windowNumber);
    /// <summary>Invalidates the token; remove its exclusion after stream updates safely complete.</summary>
    [PreserveSig]
    int OnClosed(long token);
}
