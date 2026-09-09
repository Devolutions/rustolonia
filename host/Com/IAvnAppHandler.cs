using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnAppHandler)]
public partial interface IAvnAppHandler
{
    [PreserveSig]
    int OnStarted();
}
