using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnActivationFactory)]
public partial interface IAvnActivationFactory
{
    [PreserveSig]
    int CreateEcho(out IAvnEcho? echo);

    [PreserveSig]
    int CreateApplication(out IAvnApplication? application);

    [PreserveSig]
    int CreateControlFactory(out IAvnControlFactory? factory);

    [PreserveSig]
    int CreateDispatcher(out IAvnDispatcher? dispatcher);
}
