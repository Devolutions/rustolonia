using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication11)]
public partial interface IAvnApplication11
{
    [PreserveSig]
    int SetTerminationHandler(IAvnTerminationHandler? handler);
    [PreserveSig]
    int CompleteTermination(long token, int accept);
}

[GeneratedComInterface]
[Guid(AvnGuids.IAvnTerminationHandler)]
public partial interface IAvnTerminationHandler
{
    /// <summary>UI thread; flags: 1 = native AppKit request, 2 = OS logout/restart/shutdown.</summary>
    [PreserveSig]
    int OnRequested(long token, int flags);
}
