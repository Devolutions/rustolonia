using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Single-view application capability. Hosts whose lifetime exposes one main
/// view instead of windows (the browser host) accept the root control here;
/// desktop hosts return E_NOTIMPL.
/// </summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication13)]
public partial interface IAvnApplication13
{
    [PreserveSig]
    int SetMainView(IAvnControl? view);
}
