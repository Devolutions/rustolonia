using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>Per-window popup hosting; a separately queried additive capability.</summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication9)]
public partial interface IAvnApplication9
{
    /// <summary>Set before opening popups. Does not migrate already-open popups.</summary>
    [PreserveSig]
    int SetWindowOverlayPopups(IAvnWindow? window, int enabled);
}
