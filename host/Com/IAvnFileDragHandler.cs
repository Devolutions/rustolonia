using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface]
[Guid(AvnGuids.IAvnFileDragHandler)]
public partial interface IAvnFileDragHandler
{
    [PreserveSig]
    int OnPreparing(long token);
    /// <summary>effect: 0 cancelled/no drop, 1 copied; status HRESULT.</summary>
    [PreserveSig]
    int OnCompleted(long token, int effect, int status);
}
