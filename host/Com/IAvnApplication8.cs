using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Stage 35 native-window operations. This is a separately queried capability
/// so no previously published application vtable changes.
/// </summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication8)]
public partial interface IAvnApplication8
{
    [PreserveSig]
    int GetNativeWindowHandle(IAvnWindow? window, out long handle, out int kind);

    [PreserveSig]
    int GetMacOSWindowNumber(IAvnWindow? window, out int number);

    [PreserveSig]
    int SetMacOSWindowIgnoresMouseEvents(IAvnWindow? window, int ignores);

    [PreserveSig]
    int ShowWindow(IAvnWindow? window, int activate);

    [PreserveSig]
    int ActivateWindow(IAvnWindow? window);
}
