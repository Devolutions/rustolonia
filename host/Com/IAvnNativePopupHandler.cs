using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

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
