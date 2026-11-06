using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface]
[Guid(AvnGuids.IAvnTerminationHandler)]
public partial interface IAvnTerminationHandler
{
    /// <summary>UI thread; flags: 1 = native AppKit request, 2 = OS logout/restart/shutdown.</summary>
    [PreserveSig]
    int OnRequested(long token, int flags);
}
