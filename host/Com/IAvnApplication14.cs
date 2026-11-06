using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Application naming before initialization. Platform-owned lifetimes that
/// initialize Avalonia before Rust runs return E_NOTIMPL.
/// </summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication14)]
public partial interface IAvnApplication14
{
    [PreserveSig]
    int SetApplicationName([MarshalAs(UnmanagedType.LPWStr)] string? name);
}
